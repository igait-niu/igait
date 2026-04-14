//! K8s Job orchestrator for the iGait processing pipeline.
//!
//! Instead of running 7 always-on stage Deployments that poll Firebase RTDB,
//! the backend now acts as the central orchestrator:
//!
//! 1. Polls all stage queues in Firebase RTDB
//! 2. Claims available jobs
//! 3. Spawns K8s Jobs with the appropriate stage container image
//! 4. Monitors `job_results/` in RTDB for completion
//! 5. Handles stage transitions (move to next queue or finalize)
//! 6. Cleans up failed/timed-out K8s Jobs

use anyhow::{Context, Result};
use igait_lib::microservice::{
    ClaimResult, FinalizeQueueItem, FirebaseRtdb, JobResult, JobMetadata, QueueItem, QueueOps,
    StageNumber, job_result_path, queue_item_path,
};
use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{
    Container, EnvFromSource, EnvVar, PodSpec, PodTemplateSpec, ResourceRequirements,
    SecretEnvSource,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{
    api::{Api, ListParams, PostParams},
    Client as KubeClient,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// The K8s namespace where pipeline Jobs are created.
const NAMESPACE: &str = "igait";

/// The K8s secret containing pipeline environment variables.
const SECRET_NAME: &str = "igait-secrets";

/// How often the orchestration loop polls for new work (seconds).
const POLL_INTERVAL_SECS: u64 = 3;

/// How often the completion monitor checks for results (seconds).
const COMPLETION_POLL_INTERVAL_SECS: u64 = 2;

/// How often to check for stale/failed K8s Jobs (seconds).
const STALE_JOB_CHECK_INTERVAL_SECS: u64 = 60;

/// Resource configuration for each stage's K8s Job.
struct StageResources {
    cpu_request: &'static str,
    cpu_limit: &'static str,
    memory_request: &'static str,
    memory_limit: &'static str,
    ephemeral_request: Option<&'static str>,
    ephemeral_limit: Option<&'static str>,
    /// Maximum time the Job can run before K8s kills it.
    active_deadline_secs: i64,
}

/// Returns the resource configuration for a given stage.
fn stage_resources(stage: StageNumber) -> StageResources {
    match stage {
        StageNumber::Stage1MediaConversion => StageResources {
            cpu_request: "250m",
            cpu_limit: "2",
            memory_request: "512Mi",
            memory_limit: "2Gi",
            ephemeral_request: Some("256Mi"),
            ephemeral_limit: Some("1Gi"),
            active_deadline_secs: 600,
        },
        StageNumber::Stage2ValidityCheck => StageResources {
            cpu_request: "15m",
            cpu_limit: "8",
            memory_request: "256Mi",
            memory_limit: "8Gi",
            ephemeral_request: None,
            ephemeral_limit: None,
            active_deadline_secs: 1800,
        },
        StageNumber::Stage3Reframing => StageResources {
            cpu_request: "50m",
            cpu_limit: "250m",
            memory_request: "64Mi",
            memory_limit: "256Mi",
            ephemeral_request: None,
            ephemeral_limit: None,
            active_deadline_secs: 600,
        },
        StageNumber::Stage4PoseEstimation => StageResources {
            cpu_request: "250m",
            cpu_limit: "1",
            memory_request: "512Mi",
            memory_limit: "2Gi",
            ephemeral_request: None,
            ephemeral_limit: None,
            active_deadline_secs: 1200,
        },
        StageNumber::Stage5CycleDetection => StageResources {
            cpu_request: "1",
            cpu_limit: "1200m",
            memory_request: "1Gi",
            memory_limit: "4Gi",
            ephemeral_request: None,
            ephemeral_limit: None,
            active_deadline_secs: 1800,
        },
        StageNumber::Stage6Prediction => StageResources {
            cpu_request: "50m",
            cpu_limit: "250m",
            memory_request: "64Mi",
            memory_limit: "256Mi",
            ephemeral_request: None,
            ephemeral_limit: None,
            active_deadline_secs: 600,
        },
        StageNumber::Stage7Finalize => StageResources {
            cpu_request: "50m",
            cpu_limit: "250m",
            memory_request: "64Mi",
            memory_limit: "256Mi",
            ephemeral_request: None,
            ephemeral_limit: None,
            active_deadline_secs: 300,
        },
    }
}

/// Maps StageNumber to its environment variable name for the container image.
fn stage_image_env_var(stage: StageNumber) -> &'static str {
    match stage {
        StageNumber::Stage1MediaConversion => "STAGE1_IMAGE",
        StageNumber::Stage2ValidityCheck => "STAGE2_IMAGE",
        StageNumber::Stage3Reframing => "STAGE3_IMAGE",
        StageNumber::Stage4PoseEstimation => "STAGE4_IMAGE",
        StageNumber::Stage5CycleDetection => "STAGE5_IMAGE",
        StageNumber::Stage6Prediction => "STAGE6_IMAGE",
        StageNumber::Stage7Finalize => "STAGE7_IMAGE",
    }
}

/// The central pipeline orchestrator.
///
/// Manages the lifecycle of K8s Jobs for each pipeline stage:
/// claiming work from Firebase RTDB queues, spawning Jobs,
/// monitoring completion, and handling stage transitions.
pub struct Orchestrator {
    kube_client: KubeClient,
    rtdb: FirebaseRtdb,
    /// Container images for each stage, read from env vars at startup.
    stage_images: HashMap<StageNumber, String>,
    /// Set of job IDs that currently have a K8s Job running.
    /// Prevents double-dispatch (in addition to RTDB claiming).
    in_flight: RwLock<HashMap<String, StageNumber>>,
}

impl Orchestrator {
    /// Creates a new Orchestrator.
    ///
    /// Reads stage container images from environment variables (STAGE1_IMAGE..STAGE7_IMAGE)
    /// and initializes the K8s client (auto-discovers in-cluster config).
    pub async fn new() -> Result<Self> {
        let kube_client = KubeClient::try_default()
            .await
            .context("Failed to create K8s client (are we running in-cluster?)")?;

        let rtdb = FirebaseRtdb::from_env()
            .context("Failed to create Firebase RTDB client")?;

        // Load stage images from environment
        let stages = [
            StageNumber::Stage1MediaConversion,
            StageNumber::Stage2ValidityCheck,
            StageNumber::Stage3Reframing,
            StageNumber::Stage4PoseEstimation,
            StageNumber::Stage5CycleDetection,
            StageNumber::Stage6Prediction,
            StageNumber::Stage7Finalize,
        ];

        let mut stage_images = HashMap::new();
        for stage in &stages {
            let env_var = stage_image_env_var(*stage);
            match std::env::var(env_var) {
                Ok(image) => {
                    info!("Stage {} image: {}", stage.as_u8(), image);
                    stage_images.insert(*stage, image);
                }
                Err(_) => {
                    warn!("Missing {} env var — stage {} Jobs cannot be created", env_var, stage.as_u8());
                }
            }
        }

        Ok(Self {
            kube_client,
            rtdb,
            stage_images,
            in_flight: RwLock::new(HashMap::new()),
        })
    }

    /// Creates a K8s Job for a standard processing stage (1-6).
    async fn create_stage_job(&self, stage: StageNumber, job: &QueueItem) -> Result<String> {
        let image = self.stage_images.get(&stage)
            .ok_or_else(|| anyhow::anyhow!("No image configured for stage {}", stage.as_u8()))?;

        let payload = serde_json::to_string(job)
            .context("Failed to serialize QueueItem")?;

        let job_name = self.make_job_name(stage, &job.job_id);
        let resources = stage_resources(stage);

        let k8s_job = self.build_job_spec(
            &job_name,
            image,
            "IGAIT_JOB_PAYLOAD",
            &payload,
            &resources,
            stage,
            &job.job_id,
            &job.user_id,
        );

        let jobs_api: Api<Job> = Api::namespaced(self.kube_client.clone(), NAMESPACE);
        jobs_api.create(&PostParams::default(), &k8s_job).await
            .context(format!("Failed to create K8s Job {}", job_name))?;

        info!("Created K8s Job {} for stage {} job {}", job_name, stage.as_u8(), job.job_id);
        Ok(job_name)
    }

    /// Creates a K8s Job for the finalize stage (stage 7).
    async fn create_finalize_job(&self, job: &FinalizeQueueItem) -> Result<String> {
        let stage = StageNumber::Stage7Finalize;
        let image = self.stage_images.get(&stage)
            .ok_or_else(|| anyhow::anyhow!("No image configured for stage 7 (finalize)"))?;

        let payload = serde_json::to_string(job)
            .context("Failed to serialize FinalizeQueueItem")?;

        let job_name = self.make_job_name(stage, &job.job_id);
        let resources = stage_resources(stage);

        let k8s_job = self.build_job_spec(
            &job_name,
            image,
            "IGAIT_FINALIZE_PAYLOAD",
            &payload,
            &resources,
            stage,
            &job.job_id,
            &job.user_id,
        );

        let jobs_api: Api<Job> = Api::namespaced(self.kube_client.clone(), NAMESPACE);
        jobs_api.create(&PostParams::default(), &k8s_job).await
            .context(format!("Failed to create K8s Job {}", job_name))?;

        info!("Created K8s Job {} for finalize job {}", job_name, job.job_id);
        Ok(job_name)
    }

    /// Builds a K8s Job spec.
    fn build_job_spec(
        &self,
        job_name: &str,
        image: &str,
        payload_env_var: &str,
        payload: &str,
        resources: &StageResources,
        stage: StageNumber,
        job_id: &str,
        user_id: &str,
    ) -> Job {
        let mut resource_requests = std::collections::BTreeMap::new();
        resource_requests.insert("cpu".to_string(), Quantity(resources.cpu_request.to_string()));
        resource_requests.insert("memory".to_string(), Quantity(resources.memory_request.to_string()));
        if let Some(eph) = resources.ephemeral_request {
            resource_requests.insert("ephemeral-storage".to_string(), Quantity(eph.to_string()));
        }

        let mut resource_limits = std::collections::BTreeMap::new();
        resource_limits.insert("cpu".to_string(), Quantity(resources.cpu_limit.to_string()));
        resource_limits.insert("memory".to_string(), Quantity(resources.memory_limit.to_string()));
        if let Some(eph) = resources.ephemeral_limit {
            resource_limits.insert("ephemeral-storage".to_string(), Quantity(eph.to_string()));
        }

        Job {
            metadata: ObjectMeta {
                name: Some(job_name.to_string()),
                namespace: Some(NAMESPACE.to_string()),
                labels: Some(std::collections::BTreeMap::from([
                    ("app".to_string(), "igait-pipeline".to_string()),
                    ("managed-by".to_string(), "igait-backend".to_string()),
                    ("igait.niu.edu/stage".to_string(), stage.as_u8().to_string()),
                ])),
                annotations: Some(std::collections::BTreeMap::from([
                    ("igait.niu.edu/job-id".to_string(), job_id.to_string()),
                    ("igait.niu.edu/user-id".to_string(), user_id.to_string()),
                ])),
                ..Default::default()
            },
            spec: Some(JobSpec {
                backoff_limit: Some(2),
                active_deadline_seconds: Some(resources.active_deadline_secs),
                ttl_seconds_after_finished: Some(300),
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(std::collections::BTreeMap::from([
                            ("app".to_string(), "igait-pipeline".to_string()),
                        ])),
                        ..Default::default()
                    }),
                    spec: Some(PodSpec {
                        restart_policy: Some("Never".to_string()),
                        containers: vec![Container {
                            name: "worker".to_string(),
                            image: Some(image.to_string()),
                            env: Some(vec![EnvVar {
                                name: payload_env_var.to_string(),
                                value: Some(payload.to_string()),
                                ..Default::default()
                            }]),
                            env_from: Some(vec![EnvFromSource {
                                secret_ref: Some(SecretEnvSource {
                                    name: SECRET_NAME.to_string(),
                                    optional: Some(false),
                                }),
                                ..Default::default()
                            }]),
                            resources: Some(ResourceRequirements {
                                requests: Some(resource_requests),
                                limits: Some(resource_limits),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }),
                },
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Generates a DNS-safe K8s Job name from stage number and job ID.
    ///
    /// Format: `igait-s{N}-{sanitized_job_id}-{random}`
    /// Max 63 characters (K8s DNS label limit).
    fn make_job_name(&self, stage: StageNumber, job_id: &str) -> String {
        let sanitized = job_id
            .to_lowercase()
            .replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "-");
        let random = format!("{:05x}", rand::random::<u32>() & 0xFFFFF);
        let prefix = format!("igait-s{}-", stage.as_u8());
        // Leave room for prefix + random suffix + separating dash
        let max_id_len = 63 - prefix.len() - random.len() - 1;
        let truncated_id = if sanitized.len() > max_id_len {
            &sanitized[..max_id_len]
        } else {
            &sanitized
        };
        format!("{}{}-{}", prefix, truncated_id, random)
    }

    /// Attempts to claim and dispatch a job for a standard stage (1-6).
    async fn poll_and_dispatch_stage(&self, stage: StageNumber) -> Result<bool> {
        let queue_ops = QueueOps::new(self.rtdb.clone(), "orchestrator".to_string());

        let job = match queue_ops.claim_job(stage).await {
            ClaimResult::Claimed(job) => job,
            ClaimResult::QueueEmpty | ClaimResult::AllClaimed => return Ok(false),
            ClaimResult::Error(e) => {
                error!("Failed to claim job for stage {}: {}", stage.as_u8(), e);
                return Ok(false);
            }
        };

        info!("Claimed job {} for stage {}", job.job_id, stage.as_u8());

        // Track as in-flight
        self.in_flight.write().await.insert(job.job_id.clone(), stage);

        // Create K8s Job
        match self.create_stage_job(stage, &job).await {
            Ok(job_name) => {
                info!("Dispatched K8s Job {} for job {}", job_name, job.job_id);
                Ok(true)
            }
            Err(e) => {
                error!("Failed to create K8s Job for job {}: {}", job.job_id, e);
                // Release the claim so another attempt can be made
                let _ = queue_ops.release_job(stage, &job.job_id).await;
                self.in_flight.write().await.remove(&job.job_id);
                Err(e)
            }
        }
    }

    /// Attempts to claim and dispatch a finalize job (stage 7).
    async fn poll_and_dispatch_finalize(&self) -> Result<bool> {
        let queue_ops = QueueOps::new(self.rtdb.clone(), "orchestrator".to_string());

        let job = match queue_ops.claim_finalize_job().await {
            ClaimResult::Claimed(job) => job,
            ClaimResult::QueueEmpty | ClaimResult::AllClaimed => return Ok(false),
            ClaimResult::Error(e) => {
                error!("Failed to claim finalize job: {}", e);
                return Ok(false);
            }
        };

        info!("Claimed finalize job {}", job.job_id);

        self.in_flight.write().await.insert(job.job_id.clone(), StageNumber::Stage7Finalize);

        match self.create_finalize_job(&job).await {
            Ok(job_name) => {
                info!("Dispatched K8s finalize Job {} for job {}", job_name, job.job_id);
                Ok(true)
            }
            Err(e) => {
                error!("Failed to create K8s finalize Job for job {}: {}", job.job_id, e);
                self.in_flight.write().await.remove(&job.job_id);
                Err(e)
            }
        }
    }

    /// Checks `job_results/` in Firebase RTDB for completed results and handles transitions.
    async fn check_completions(&self) -> Result<()> {
        let results: Option<HashMap<String, JobResult>> = self.rtdb.get("job_results").await
            .context("Failed to read job_results from RTDB")?;

        let Some(results) = results else {
            return Ok(());
        };

        let queue_ops = QueueOps::new(self.rtdb.clone(), "orchestrator".to_string());

        for (safe_job_id, result) in results {
            let job_id = &result.job_id;
            let Some(stage) = StageNumber::from_u8(result.stage) else {
                error!("Invalid stage number {} in job result for {}", result.stage, job_id);
                let _ = self.rtdb.delete(&format!("job_results/{}", safe_job_id)).await;
                continue;
            };

            if result.success {
                info!("Job {} stage {} completed successfully", job_id, result.stage);

                match stage {
                    // Stage 6 success → finalize queue
                    StageNumber::Stage6Prediction => {
                        let finalize_item = FinalizeQueueItem::success(
                            result.job_id.clone(),
                            result.user_id.clone(),
                            result.output_keys.clone(),
                            result.metadata.clone(),
                        );
                        let current_path = queue_item_path(stage, job_id);
                        let finalize_path = queue_item_path(StageNumber::Stage7Finalize, job_id);
                        let mut updates = HashMap::new();
                        updates.insert(current_path, serde_json::Value::Null);
                        updates.insert(finalize_path, serde_json::to_value(&finalize_item)?);
                        self.rtdb.multi_update(updates).await
                            .context("Failed to move job to finalize queue")?;
                    }
                    // Stage 7 (finalize) success → just remove from finalize queue
                    StageNumber::Stage7Finalize => {
                        let _ = queue_ops.complete_finalize(job_id).await;
                    }
                    // Stages 1-5 success → move to next stage queue
                    _ => {
                        let next_job = QueueItem::new(
                            result.job_id.clone(),
                            result.user_id.clone(),
                            result.output_keys.clone(),
                            result.metadata.clone(),
                            result.requires_approval,
                        );
                        // Preserve approval
                        let mut next_job = next_job;
                        next_job.approved = result.approved;

                        let current_path = queue_item_path(stage, job_id);
                        let next_stage = igait_lib::microservice::next_stage(stage);
                        let next_path = queue_item_path(next_stage, job_id);
                        let mut updates = HashMap::new();
                        updates.insert(current_path, serde_json::Value::Null);
                        updates.insert(next_path, serde_json::to_value(&next_job)?);
                        self.rtdb.multi_update(updates).await
                            .context("Failed to move job to next stage")?;
                    }
                }
            } else {
                // Failure → move to finalize queue (unless already finalize)
                let error_msg = result.error.clone().unwrap_or_else(|| "Unknown error".to_string());
                warn!("Job {} stage {} failed: {}", job_id, result.stage, error_msg);

                if stage == StageNumber::Stage7Finalize {
                    // Finalize itself failed — just log it and remove
                    error!("Finalize job {} failed: {}", job_id, error_msg);
                    let _ = queue_ops.complete_finalize(job_id).await;
                } else {
                    let finalize_item = FinalizeQueueItem::failure(
                        result.job_id.clone(),
                        result.user_id.clone(),
                        result.stage,
                        error_msg,
                        Some(result.logs.clone()),
                        result.metadata.clone(),
                    );
                    let current_path = queue_item_path(stage, job_id);
                    let finalize_path = queue_item_path(StageNumber::Stage7Finalize, job_id);
                    let mut updates = HashMap::new();
                    updates.insert(current_path, serde_json::Value::Null);
                    updates.insert(finalize_path, serde_json::to_value(&finalize_item)?);
                    self.rtdb.multi_update(updates).await
                        .context("Failed to move failed job to finalize queue")?;
                }
            }

            // Clean up the result from RTDB
            let result_path = job_result_path(job_id);
            let _ = self.rtdb.delete(&result_path).await;

            // Remove from in-flight tracking
            self.in_flight.write().await.remove(job_id);
        }

        Ok(())
    }

    /// Checks for stale K8s Jobs that have failed/timed out without writing a result.
    async fn check_stale_jobs(&self) -> Result<()> {
        let jobs_api: Api<Job> = Api::namespaced(self.kube_client.clone(), NAMESPACE);
        let lp = ListParams::default()
            .labels("app=igait-pipeline,managed-by=igait-backend");

        let job_list = jobs_api.list(&lp).await
            .context("Failed to list K8s Jobs")?;

        for k8s_job in job_list.items {
            let job_name = k8s_job.metadata.name.as_deref().unwrap_or("unknown");
            let status = k8s_job.status.as_ref();

            let is_failed = status
                .and_then(|s| s.failed)
                .map(|f| f > 0)
                .unwrap_or(false);

            let conditions = status
                .and_then(|s| s.conditions.as_ref());
            let is_deadline_exceeded = conditions
                .map(|conds| {
                    conds.iter().any(|c| c.type_ == "Failed" && c.reason.as_deref() == Some("DeadlineExceeded"))
                })
                .unwrap_or(false);

            if is_failed || is_deadline_exceeded {
                warn!("K8s Job {} is failed/timed out, cleaning up", job_name);

                // Extract job_id and stage from annotations/labels so we can
                // write a synthetic failure result for the completion monitor.
                let annotations = k8s_job.metadata.annotations.as_ref();
                let labels = k8s_job.metadata.labels.as_ref();

                let job_id = annotations.and_then(|a| a.get("igait.niu.edu/job-id"));
                let stage_str = labels.and_then(|l| l.get("igait.niu.edu/stage"));

                if let (Some(job_id), Some(stage_str)) = (job_id, stage_str) {
                    let stage_num: u8 = stage_str.parse().unwrap_or(0);

                    // Only write a failure result if one doesn't already exist
                    let result_path = job_result_path(job_id);
                    let existing: Option<serde_json::Value> = self.rtdb.get(&result_path).await.unwrap_or(None);

                    if existing.is_none() {
                        let reason = if is_deadline_exceeded { "timed out" } else { "crashed" };
                        warn!("Writing synthetic failure result for job {} stage {} ({})", job_id, stage_num, reason);

                        let user_id = annotations
                            .and_then(|a| a.get("igait.niu.edu/user-id"))
                            .cloned()
                            .unwrap_or_default();

                        let failure_result = JobResult {
                            stage: stage_num,
                            success: false,
                            output_keys: HashMap::new(),
                            error: Some(format!("K8s Job {}: pod {}", reason, job_name)),
                            logs: format!("Stage {} pod {} without writing a result", stage_num, reason),
                            duration_ms: 0,
                            job_id: job_id.clone(),
                            user_id,
                            metadata: JobMetadata::default(),
                            input_keys: HashMap::new(),
                            requires_approval: false,
                            approved: false,
                        };

                        if let Err(e) = self.rtdb.set(&result_path, &failure_result).await {
                            error!("Failed to write synthetic failure result for {}: {}", job_id, e);
                        }
                    }
                } else {
                    warn!("K8s Job {} missing igait annotations, cannot recover — will be cleaned up by TTL", job_name);
                }

                // Remove from in-flight tracking regardless
                if let Some(job_id) = annotations.and_then(|a| a.get("igait.niu.edu/job-id")) {
                    self.in_flight.write().await.remove(job_id);
                }
            }
        }

        Ok(())
    }
}

/// Main orchestration loop — polls all stage queues and dispatches K8s Jobs.
pub async fn orchestration_loop(orchestrator: Arc<Orchestrator>) {
    let stages = [
        StageNumber::Stage1MediaConversion,
        StageNumber::Stage2ValidityCheck,
        StageNumber::Stage3Reframing,
        StageNumber::Stage4PoseEstimation,
        StageNumber::Stage5CycleDetection,
        StageNumber::Stage6Prediction,
    ];

    info!("Orchestration loop started");

    loop {
        // Scan all processing stage queues
        for stage in &stages {
            match orchestrator.poll_and_dispatch_stage(*stage).await {
                Ok(true) => {
                    // Dispatched a job — immediately check for more in same stage
                    continue;
                }
                Ok(false) => {}
                Err(e) => {
                    error!("Orchestrator error for stage {}: {:?}", stage.as_u8(), e);
                }
            }
        }

        // Also check finalize queue
        match orchestrator.poll_and_dispatch_finalize().await {
            Ok(_) => {}
            Err(e) => {
                error!("Orchestrator error for finalize: {:?}", e);
            }
        }

        tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}

/// Completion monitor loop — polls `job_results/` in RTDB and handles transitions.
pub async fn completion_monitor_loop(orchestrator: Arc<Orchestrator>) {
    info!("Completion monitor loop started");

    let mut stale_check_counter: u64 = 0;

    loop {
        // Check for completed results
        if let Err(e) = orchestrator.check_completions().await {
            error!("Completion monitor error: {:?}", e);
        }

        // Periodically check for stale K8s Jobs
        stale_check_counter += COMPLETION_POLL_INTERVAL_SECS;
        if stale_check_counter >= STALE_JOB_CHECK_INTERVAL_SECS {
            stale_check_counter = 0;
            if let Err(e) = orchestrator.check_stale_jobs().await {
                error!("Stale job check error: {:?}", e);
            }
        }

        tokio::time::sleep(Duration::from_secs(COMPLETION_POLL_INTERVAL_SECS)).await;
    }
}
