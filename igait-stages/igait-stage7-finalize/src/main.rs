//! Stage 7: Finalize Microservice
//!
//! Handles post-processing completion tasks:
//! - Checks for prediction.json in S3 to determine success/failure
//! - Sends success/failure emails to users
//! - Archives processing results
//!
//! This is the terminal stage that receives jobs from the finalize queue.

use anyhow::{Context, Result};
use async_trait::async_trait;
use igait_lib::microservice::{
    EmailClient, EmailTemplates, FinalizeQueueItem, ProcessingResult, StorageClient,
    JobStatus, StageStatus, QueueOps, FirebaseRtdb,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{Instant, SystemTime};
use chrono::{DateTime, Utc};

/// The expected format of prediction.json from Stage 6.
///
/// This matches the raw output of the Python ensemble in `iGAIT_MODEL_IO`.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PredictionResult {
    status: String,
    class: Option<i32>,
    probabilities: Option<Vec<f64>>,
    message: Option<String>,
    // Error fields (present when status == "error")
    error_type: Option<String>,
    error_message: Option<String>,
}

/// ASD threshold - scores >= this value indicate ASD markers.
const ASD_THRESHOLD: f64 = 0.5;

/// The finalize worker handles the final stage of the pipeline.
pub struct FinalizeStageWorker {
    email_client: EmailClient,
    storage: StorageClient,
    queue_ops: QueueOps,
}

impl FinalizeStageWorker {
    /// Creates a new finalize worker with required clients.
    pub async fn new() -> Result<Self> {
        let email_client = EmailClient::from_env()
            .await
            .context("Failed to create email client")?;
        let storage = StorageClient::new()
            .await
            .context("Failed to create storage client")?;
        let db = FirebaseRtdb::from_env()
            .context("Failed to create Firebase RTDB client")?;
        let queue_ops = QueueOps::new(db, "stage7-finalize".to_string());

        Ok(Self {
            email_client,
            storage,
            queue_ops,
        })
    }

    /// Attempts to read prediction.json from S3 for a given job.
    ///
    /// Parses the full ensemble result and computes the score by averaging
    /// the individual model probabilities. Returns `Some(score)` if found
    /// and valid, `None` otherwise.
    async fn get_prediction_score(&self, job_id: &str) -> Option<f64> {
        let prediction_path = format!("jobs/{}/stage_6/prediction.json", job_id);
        
        match self.storage.download(&prediction_path).await {
            Ok(data) => {
                match serde_json::from_slice::<PredictionResult>(&data) {
                    Ok(result) => {
                        println!("Found prediction.json for {}: {:?}", job_id, result);

                        // Check if the ensemble itself reported an error
                        if result.status != "success" {
                            eprintln!(
                                "Prediction failed for {}: {} - {}",
                                job_id,
                                result.error_type.as_deref().unwrap_or("unknown"),
                                result.error_message.as_deref().unwrap_or("no details"),
                            );
                            return None;
                        }

                        // Average the probabilities from all ensemble models
                        if let Some(ref probs) = result.probabilities {
                            if probs.is_empty() {
                                eprintln!("Empty probabilities array for {}", job_id);
                                return None;
                            }
                            let score = probs.iter().sum::<f64>() / probs.len() as f64;
                            println!(
                                "Computed score for {} by averaging {} probabilities: {:.4}",
                                job_id,
                                probs.len(),
                                score
                            );
                            Some(score)
                        } else {
                            // Fallback: use class value (0 or 1) as score
                            let score = result.class.unwrap_or(0) as f64;
                            println!(
                                "No probabilities for {}, using class as score: {}",
                                job_id, score
                            );
                            Some(score)
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse prediction.json for {}: {}", job_id, e);
                        None
                    }
                }
            }
            Err(e) => {
                println!("No prediction.json found for {} (error: {})", job_id, e);
                None
            }
        }
    }

    /// Sends a success email with the prediction results.
    async fn send_success_email(
        &self,
        job: &FinalizeQueueItem,
        score: f64,
        logs: &mut String,
    ) -> Result<()> {
        let email = job.metadata.email.as_deref()
            .ok_or_else(|| anyhow::anyhow!("No email address in job metadata"))?;
        
        let dt_now_utc: DateTime<Utc> = SystemTime::now().into();
        let dt_now_cst = dt_now_utc.with_timezone(&chrono_tz::US::Central);
        
        let is_asd = score >= ASD_THRESHOLD;
        
        let (subject, body) = EmailTemplates::prediction_success(
            &dt_now_cst.to_string(),
            score,
            is_asd,
            job.metadata.age,
            job.metadata.ethnicity.as_deref(),
            job.metadata.sex,
            job.metadata.height.as_deref(),
            job.metadata.weight,
            &job.user_id,
            &job.job_id,
        );

        logs.push_str(&format!("Sending success email to {}\n", email));
        logs.push_str(&format!("Score: {:.2}, ASD indicator: {}\n", score, is_asd));
        
        self.email_client.send(email, &subject, &body).await?;
        logs.push_str("Success email sent\n");
        
        Ok(())
    }

    /// Sends a failure email with error information.
    async fn send_failure_email(
        &self,
        job: &FinalizeQueueItem,
        error: &str,
        logs: &mut String,
    ) -> Result<()> {
        let email = job.metadata.email.as_deref()
            .ok_or_else(|| anyhow::anyhow!("No email address in job metadata"))?;
        
        let dt_now_utc: DateTime<Utc> = SystemTime::now().into();
        let dt_now_cst = dt_now_utc.with_timezone(&chrono_tz::US::Central);
        
        let (subject, body) = EmailTemplates::processing_failure(
            &dt_now_cst.to_string(),
            job.failed_at_stage,
            error,
            &job.user_id,
            &job.job_id,
        );

        logs.push_str(&format!("Sending failure email to {}\n", email));
        logs.push_str(&format!("Failed at stage: {:?}, Error: {}\n", job.failed_at_stage, error));
        
        self.email_client.send(email, &subject, &body).await?;
        logs.push_str("Failure email sent\n");
        
        Ok(())
    }

    /// Update job status in RTDB
    async fn update_job_status(&self, job_id: &str, status: JobStatus) {
        match QueueOps::parse_job_id(job_id) {
            Ok((user_id, job_index)) => {
                if let Err(e) = self.queue_ops.update_job_status(&user_id, job_index, &status).await {
                    eprintln!("Failed to update job status in RTDB: {:?}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to parse job_id: {:?}", e);
            }
        }
    }

    /// Upload stage logs to Firebase RTDB
    async fn upload_stage_logs(&self, job_id: &str, logs: &str) {
        match QueueOps::parse_job_id(job_id) {
            Ok((user_id, job_index)) => {
                if let Err(e) = self.queue_ops.update_stage_logs(&user_id, job_index, 7, logs).await {
                    eprintln!("Failed to upload stage 7 logs to RTDB: {:?}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to parse job_id for log upload: {:?}", e);
            }
        }
    }

    /// Update per-stage status in RTDB
    async fn update_stage_status(&self, job_id: &str, stage: u8, status: StageStatus) {
        match QueueOps::parse_job_id(job_id) {
            Ok((user_id, job_index)) => {
                if let Err(e) = self.queue_ops.update_stage_status(&user_id, job_index, stage, &status).await {
                    eprintln!("Failed to update stage {} status in RTDB: {:?}", stage, e);
                }
            }
            Err(e) => {
                eprintln!("Failed to parse job_id for stage status update: {:?}", e);
            }
        }
    }
}

/// Trait for finalize workers (separate from regular StageWorker).
#[async_trait]
pub trait FinalizeWorker: Send + Sync + 'static {
    /// Human-readable service name.
    fn service_name(&self) -> &'static str;

    /// Process a finalize job.
    async fn process(&self, job: &FinalizeQueueItem) -> ProcessingResult;
}

#[async_trait]
impl FinalizeWorker for FinalizeStageWorker {
    fn service_name(&self) -> &'static str {
        "igait-stage7-finalize"
    }

    async fn process(&self, job: &FinalizeQueueItem) -> ProcessingResult {
        let start_time = Instant::now();
        let mut logs = String::new();

        println!("Processing finalize job {}", job.job_id);
        logs.push_str(&format!("Starting finalization for job {}\n", job.job_id));
        logs.push_str(&format!("Queue item success flag: {}\n", job.success));

        // Update status to stage 7 on entry
        self.update_job_status(&job.job_id, JobStatus::processing(7)).await;
        self.update_stage_status(&job.job_id, 7, StageStatus::Running).await;

        // Check for prediction.json in S3 - this is the source of truth
        let prediction_score = self.get_prediction_score(&job.job_id).await;

        let result = if let Some(score) = prediction_score {
            // Prediction file exists - this was a successful pipeline run
            logs.push_str(&format!("Prediction found: score = {:.4}\n", score));
            
            match self.send_success_email(job, score, &mut logs).await {
                Ok(_) => {
                    logs.push_str("Job completed successfully\n");
                }
                Err(e) => {
                    // Log email failure but don't fail the job
                    eprintln!("Failed to send success email for {}: {}", job.job_id, e);
                    logs.push_str(&format!("WARNING: Failed to send email: {}\n", e));
                }
            }
            
            // Update job status to Complete
            let is_asd = score >= ASD_THRESHOLD;
            self.update_job_status(&job.job_id, JobStatus::complete(score as f32, is_asd)).await;
            self.update_stage_status(&job.job_id, 7, StageStatus::Complete).await;

            // Upload stage 7 logs to Firebase RTDB
            self.upload_stage_logs(&job.job_id, &logs).await;
            
            ProcessingResult::Success {
                output_keys: HashMap::from([
                    ("score".to_string(), score.to_string()),
                    ("is_asd".to_string(), (score >= ASD_THRESHOLD).to_string()),
                ]),
                logs,
                duration_ms: start_time.elapsed().as_millis() as u64,
            }
        } else {
            // No prediction file - pipeline failed somewhere
            let error_msg = job.error.clone()
                .or_else(|| job.error_logs.clone())
                .unwrap_or_else(|| "Unknown error - no prediction.json found".to_string());
            
            logs.push_str(&format!("No prediction found, treating as failure\n"));
            logs.push_str(&format!("Error info: {}\n", error_msg));
            
            if let Some(stage) = job.failed_at_stage {
                logs.push_str(&format!("Failed at stage: {}\n", stage));
            }
            
            match self.send_failure_email(job, &error_msg, &mut logs).await {
                Ok(_) => {
                    logs.push_str("Failure notification sent\n");
                }
                Err(e) => {
                    eprintln!("Failed to send failure email for {}: {}", job.job_id, e);
                    logs.push_str(&format!("WARNING: Failed to send email: {}\n", e));
                }
            }
            
            // Update job status to Error
            self.update_job_status(&job.job_id, JobStatus::error(error_msg.clone())).await;
            self.update_stage_status(&job.job_id, 7, StageStatus::Error).await;

            // Upload stage 7 logs to Firebase RTDB
            self.upload_stage_logs(&job.job_id, &logs).await;
            
            // Return success because finalization completed (even though the job itself failed)
            ProcessingResult::Success {
                output_keys: HashMap::new(),
                logs,
                duration_ms: start_time.elapsed().as_millis() as u64,
            }
        };

        result
    }
}

/// Runs the finalize worker in a continuous loop.
/// 
/// This is a standalone worker loop since FinalizeWorker has different
/// queue handling than regular StageWorker.
pub async fn run_finalize_worker(worker: FinalizeStageWorker) -> Result<()> {
    use igait_lib::microservice::{
        ClaimResult, FirebaseRtdb, QueueOps,
        generate_worker_id,
    };
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

    let db = FirebaseRtdb::from_env()?;
    let worker_id = generate_worker_id(worker.service_name());
    let queue_ops = QueueOps::new(db, worker_id.clone());
    let shutdown_token = CancellationToken::new();
    
    // Setup signal handler
    let shutdown_signal = shutdown_token.clone();
    tokio::spawn(async move {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                println!("\nReceived Ctrl+C, shutting down gracefully...");
            },
            _ = terminate => {
                println!("\nReceived SIGTERM, shutting down gracefully...");
            },
        }
        
        shutdown_signal.cancel();
    });
    
    println!("[{}] Starting Finalize worker...", worker_id);

    loop {
        // Check for shutdown signal
        if shutdown_token.is_cancelled() {
            println!("[{}] Shutdown signal received, stopping worker loop", worker_id);
            break;
        }

        // Try to claim a job from the finalize queue
        match queue_ops.claim_finalize_job().await {
            ClaimResult::Claimed(job) => {
                println!("[{}] Claimed finalize job {}", worker_id, job.job_id);
                
                // Process the job with cancellation support
                let process_result = tokio::select! {
                    result = worker.process(&job) => result,
                    _ = shutdown_token.cancelled() => {
                        println!(
                            "[{}] Finalize job {} processing cancelled due to shutdown",
                            worker_id, job.job_id
                        );
                        // Job will remain claimed and be picked up by another worker
                        // or timeout and be re-claimed later
                        break;
                    }
                };
                
                match process_result {
                    ProcessingResult::Success { duration_ms, .. } => {
                        println!(
                            "[{}] Finalize job {} completed in {}ms",
                            worker_id, job.job_id, duration_ms
                        );
                        
                        // Remove from finalize queue (job is done)
                        if let Err(e) = queue_ops.complete_finalize(&job.job_id).await {
                            eprintln!("Failed to remove job from finalize queue: {}", e);
                        }
                    }
                    ProcessingResult::Failure { error, duration_ms, .. } => {
                        // This shouldn't really happen since we always return Success
                        eprintln!(
                            "[{}] Finalize job {} failed after {}ms: {}",
                            worker_id, job.job_id, duration_ms, error
                        );
                    }
                }
            }
            ClaimResult::QueueEmpty | ClaimResult::AllClaimed => {
                // No jobs available, wait before polling again (or until shutdown)
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {},
                    _ = shutdown_token.cancelled() => {
                        println!("[{}] Shutdown signal received during sleep", worker_id);
                        break;
                    }
                }
            }
            ClaimResult::Error(e) => {
                eprintln!("[{}] Error claiming job: {}", worker_id, e);
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(10)) => {},
                    _ = shutdown_token.cancelled() => {
                        println!("[{}] Shutdown signal received during error backoff", worker_id);
                        break;
                    }
                }
            }
        }
    }
    
    println!("[{}] Finalize worker stopped gracefully", worker_id);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting Stage 7 Finalize worker...");
    
    let worker = FinalizeStageWorker::new()
        .await
        .context("Failed to create finalize worker")?;
    
    run_finalize_worker(worker).await
}
