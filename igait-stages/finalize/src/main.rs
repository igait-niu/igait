//! Finalize Microservice
//!
//! Handles post-processing completion tasks:
//! - Checks for prediction.json in S3 to determine success/failure
//! - Sends success/failure emails to users
//! - Archives processing results
//!
//! This is the terminal stage that receives jobs from the finalize queue.

use anyhow::{Context, Result};
use igait_lib::microservice::{
    check_env, ClaimResult, EmailClient, EmailTemplates, FinalizeQueueItem, JobResult,
    ProcessingResult, StageId, StorageClient, JobStatus, StageStatus, QueueOps, FirebaseRtdb,
    job_result_path, FINALIZE_REQUIRED_ENV, STAGE_REQUIRED_ENV,
};
use std::time::Duration;
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
        let queue_ops = QueueOps::new(db, "finalize".to_string());

        Ok(Self {
            email_client,
            storage,
            queue_ops,
        })
    }

    /// Attempts to read prediction.json from S3 for a given job.
    ///
    /// Parses the ensemble result and returns the binary `class` field
    /// (1 = ASD, 0 = no ASD). Returns `Some(is_asd)` if found and valid,
    /// `None` otherwise.
    async fn get_prediction_class(&self, job_id: &str) -> Option<bool> {
        let prediction_path = format!("jobs/{}/prediction/prediction.json", job_id);

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

                        // Use the ensemble's binary classification directly
                        match result.class {
                            Some(class) => {
                                let is_asd = class == 1;
                                println!(
                                    "Prediction for {}: class={}, is_asd={}",
                                    job_id, class, is_asd
                                );
                                Some(is_asd)
                            }
                            None => {
                                eprintln!("No class field in prediction.json for {}", job_id);
                                None
                            }
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
        is_asd: bool,
        logs: &mut String,
    ) -> Result<()> {
        let email = job.metadata.email.as_deref()
            .ok_or_else(|| anyhow::anyhow!("No email address in job metadata"))?;

        let (user_id, job_key) = QueueOps::parse_job_id(&job.job_id)
            .context("Failed to parse job_id for email dedup marker")?;
        let outcome = if is_asd { "success_asd" } else { "success_no_asd" };
        let Some(dedup_id) = self.queue_ops.try_claim_email_send(&user_id, &job_key, outcome).await? else {
            logs.push_str("Result email already sent by another worker; skipping\n");
            return Ok(());
        };

        let dt_now_utc: DateTime<Utc> = SystemTime::now().into();
        let dt_now_cst = dt_now_utc.with_timezone(&chrono_tz::US::Central);

        let (subject, body) = EmailTemplates::prediction_success(
            &dt_now_cst.to_string(),
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
        logs.push_str(&format!("ASD indicator: {}\n", is_asd));

        self.email_client.send_with_dedup(email, &subject, &body, Some(&dedup_id)).await?;
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

        let (user_id, job_key) = QueueOps::parse_job_id(&job.job_id)
            .context("Failed to parse job_id for email dedup marker")?;
        let Some(dedup_id) = self.queue_ops.try_claim_email_send(&user_id, &job_key, "failure").await? else {
            logs.push_str("Result email already sent by another worker; skipping\n");
            return Ok(());
        };

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

        self.email_client.send_with_dedup(email, &subject, &body, Some(&dedup_id)).await?;
        logs.push_str("Failure email sent\n");

        Ok(())
    }

    /// Update job status in RTDB
    async fn update_job_status(&self, job_id: &str, status: JobStatus) {
        match QueueOps::parse_job_id(job_id) {
            Ok((user_id, job_index)) => {
                if let Err(e) = self.queue_ops.update_job_status(&user_id, &job_index, &status).await {
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
        let finalize_id = StageId::new("finalize");
        match QueueOps::parse_job_id(job_id) {
            Ok((user_id, job_index)) => {
                if let Err(e) = self.queue_ops.update_stage_logs(&user_id, &job_index, finalize_id, logs).await {
                    eprintln!("Failed to upload finalize logs to RTDB: {:?}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to parse job_id for log upload: {:?}", e);
            }
        }
    }

    /// Update per-stage status in RTDB
    async fn update_stage_status(&self, job_id: &str, stage: StageId, status: StageStatus) {
        match QueueOps::parse_job_id(job_id) {
            Ok((user_id, job_index)) => {
                if let Err(e) = self.queue_ops.update_stage_status(&user_id, &job_index, stage, &status).await {
                    eprintln!("Failed to update {} status in RTDB: {:?}", stage, e);
                }
            }
            Err(e) => {
                eprintln!("Failed to parse job_id for stage status update: {:?}", e);
            }
        }
    }
}

impl FinalizeStageWorker {
    async fn process(&self, job: &FinalizeQueueItem) -> ProcessingResult {
        let start_time = Instant::now();
        let mut logs = String::new();

        println!("Processing finalize job {}", job.job_id);
        logs.push_str(&format!("Starting finalization for job {}\n", job.job_id));
        logs.push_str(&format!("Queue item success flag: {}\n", job.success));

        // Mark the finalize stage as running on entry.
        let finalize_id = StageId::new("finalize");
        self.update_job_status(&job.job_id, JobStatus::processing(finalize_id)).await;
        self.update_stage_status(&job.job_id, finalize_id, StageStatus::Running).await;

        // Check for prediction.json in S3 - this is the source of truth
        let prediction_class = self.get_prediction_class(&job.job_id).await;

        let result = if let Some(is_asd) = prediction_class {
            // Prediction file exists - this was a successful pipeline run
            logs.push_str(&format!("Prediction found: is_asd = {}\n", is_asd));

            match self.send_success_email(job, is_asd, &mut logs).await {
                Ok(_) => {
                    logs.push_str("Job completed successfully\n");
                }
                Err(e) => {
                    // Log email failure but don't fail the job
                    eprintln!("Failed to send success email for {}: {}", job.job_id, e);
                    logs.push_str(&format!("WARNING: Failed to send email: {}\n", e));
                }
            }
            
            self.upload_stage_logs(&job.job_id, &logs).await;

            ProcessingResult::Success {
                output_keys: HashMap::from([
                    ("is_asd".to_string(), is_asd.to_string()),
                    ("prediction".to_string(), if is_asd { "1.0" } else { "0.0" }.to_string()),
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

#[tokio::main]
async fn main() -> Result<()> {
    let required: Vec<&str> = STAGE_REQUIRED_ENV
        .iter()
        .chain(FINALIZE_REQUIRED_ENV.iter())
        .copied()
        .collect();
    check_env(&required)?;

    // Dual-mode dispatch mirroring the 4 processing stages: the presence
    // of IGAIT_FINALIZE_PAYLOAD selects K8s single-job mode, and its
    // absence falls through to a long-running worker that polls the
    // finalize queue (used by the hermetic local compose stack).
    if std::env::var("IGAIT_FINALIZE_PAYLOAD").is_ok() {
        run_finalize_job_mode().await
    } else {
        run_finalize_worker_mode().await
    }
}

/// Polls the finalize queue indefinitely, processing one job at a time.
///
/// Used when no K8s orchestrator is spawning per-job pods — e.g. the
/// docker-compose stack. Claim is handled by `QueueOps::claim_finalize_job`
/// (which already does CAS-based lease acquisition); completion removes the
/// item from the queue rather than writing a JobResult, since no
/// orchestrator is listening for one.
async fn run_finalize_worker_mode() -> Result<()> {
    println!("[worker-mode] Starting finalize worker (polling queue)");

    let worker = FinalizeStageWorker::new()
        .await
        .context("Failed to create finalize worker")?;

    let poll_interval = Duration::from_secs(5);

    loop {
        match worker.queue_ops.claim_finalize_job().await {
            ClaimResult::Claimed(job) => {
                println!("[worker-mode] Claimed finalize job {}", job.job_id);
                let _ = worker.process(&job).await;
                if let Err(e) = worker.queue_ops.complete_finalize(&job.job_id).await {
                    eprintln!(
                        "[worker-mode] Failed to remove {} from finalize queue: {:?}",
                        job.job_id, e
                    );
                }
            }
            ClaimResult::QueueEmpty | ClaimResult::AllClaimed => {
                tokio::time::sleep(poll_interval).await;
            }
            ClaimResult::Error(e) => {
                eprintln!("[worker-mode] Claim error: {} — backing off", e);
                tokio::time::sleep(poll_interval).await;
            }
        }
    }
}

/// Runs the finalize worker in single-job mode for K8s Job execution.
async fn run_finalize_job_mode() -> Result<()> {
    println!("[job-mode] Starting finalize worker");

    let payload = std::env::var("IGAIT_FINALIZE_PAYLOAD")
        .context("Missing IGAIT_FINALIZE_PAYLOAD environment variable")?;
    let job: FinalizeQueueItem = serde_json::from_str(&payload)
        .context("Failed to deserialize IGAIT_FINALIZE_PAYLOAD as FinalizeQueueItem")?;

    println!("[job-mode] Processing finalize job {}", job.job_id);

    let worker = FinalizeStageWorker::new()
        .await
        .context("Failed to create finalize worker")?;

    let process_result = worker.process(&job).await;

    let db = FirebaseRtdb::from_env()
        .context("Failed to create Firebase RTDB client")?;

    let job_epoch: u64 = std::env::var("IGAIT_JOB_EPOCH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let job_result = match &process_result {
        ProcessingResult::Success { output_keys, logs, duration_ms } => {
            println!("[job-mode] Finalize job {} completed in {}ms", job.job_id, duration_ms);
            JobResult {
                stage: StageId::new("finalize"),
                success: true,
                output_keys: output_keys.clone(),
                error: None,
                logs: logs.clone(),
                duration_ms: *duration_ms,
                job_id: job.job_id.clone(),
                user_id: job.user_id.clone(),
                metadata: job.metadata.clone(),
                input_keys: HashMap::new(),
                requires_approval: false,
                approved: false,
                epoch: job.epoch,
                job_epoch,
                taken_by: None,
                taken_at: None,
            }
        }
        ProcessingResult::Failure { error, logs, duration_ms } => {
            eprintln!("[job-mode] Finalize job {} failed after {}ms: {}", job.job_id, duration_ms, error);
            JobResult {
                stage: StageId::new("finalize"),
                success: false,
                output_keys: HashMap::new(),
                error: Some(error.clone()),
                logs: logs.clone(),
                duration_ms: *duration_ms,
                job_id: job.job_id.clone(),
                user_id: job.user_id.clone(),
                metadata: job.metadata.clone(),
                input_keys: HashMap::new(),
                requires_approval: false,
                approved: false,
                epoch: job.epoch,
                job_epoch,
                taken_by: None,
                taken_at: None,
            }
        }
    };

    let result_path = job_result_path(&job.job_id);
    db.set(&result_path, &job_result).await
        .context("Failed to write finalize JobResult to Firebase RTDB")?;

    println!("[job-mode] Finalize result written to RTDB at {}", result_path);
    Ok(())
}
