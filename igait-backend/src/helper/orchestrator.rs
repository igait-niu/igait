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
    generate_worker_id, job_result_path, job_status_path, now_ms, queue_item_path, retry_transient,
    stage_logs_path, stage_status_path, CasResult, ClaimResult, FinalizeQueueItem, FirebaseRtdb,
    JobMetadata, JobResult, JobStatus, QueueItem, QueueOps, RetryPolicy, StageId, StageStatus,
    JOB_RESULT_TAKE_TIMEOUT_MS,
};
use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{
    Container, EnvFromSource, EnvVar, PodSpec, PodTemplateSpec, ResourceRequirements,
    SecretEnvSource,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{
    api::{Api, DeleteParams, ListParams, PostParams, PropagationPolicy},
    Client as KubeClient,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, instrument, warn};

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
fn stage_resources(stage: StageId) -> StageResources {
    match stage.key() {
        "media-conversion" => StageResources {
            cpu_request: "250m",
            cpu_limit: "2",
            memory_request: "512Mi",
            memory_limit: "2Gi",
            ephemeral_request: Some("256Mi"),
            ephemeral_limit: Some("1Gi"),
            active_deadline_secs: 600,
        },
        "pose-estimation" => StageResources {
            cpu_request: "250m",
            cpu_limit: "1",
            memory_request: "512Mi",
            memory_limit: "2Gi",
            ephemeral_request: None,
            ephemeral_limit: None,
            active_deadline_secs: 1200,
        },
        "cycle-detection" => StageResources {
            cpu_request: "1",
            cpu_limit: "1200m",
            memory_request: "1Gi",
            memory_limit: "4Gi",
            ephemeral_request: None,
            ephemeral_limit: None,
            active_deadline_secs: 1800,
        },
        "prediction" => StageResources {
            cpu_request: "250m",
            cpu_limit: "1",
            memory_request: "256Mi",
            memory_limit: "1Gi",
            ephemeral_request: None,
            ephemeral_limit: None,
            active_deadline_secs: 600,
        },
        "finalize" => StageResources {
            cpu_request: "50m",
            cpu_limit: "250m",
            memory_request: "64Mi",
            memory_limit: "256Mi",
            ephemeral_request: None,
            ephemeral_limit: None,
            active_deadline_secs: 300,
        },
        other => panic!("no StageResources configured for stage {:?}", other),
    }
}

/// Environment variable name for a stage's container image, derived
/// from the stage key: `STAGE_<UPPER_SNAKE_KEY>_IMAGE` (e.g.
/// `STAGE_MEDIA_CONVERSION_IMAGE`). Keeping the Rust side keyed
/// means adding a new stage doesn't require touching this function —
/// just set the matching env var on the K8s deployment.
fn stage_image_env_var(stage: StageId) -> String {
    let upper = stage.key().replace('-', "_").to_ascii_uppercase();
    format!("STAGE_{}_IMAGE", upper)
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
    stage_images: HashMap<StageId, String>,
    /// Set of job IDs that currently have a K8s Job running.
    /// Prevents double-dispatch (in addition to RTDB claiming).
    in_flight: RwLock<HashMap<String, StageId>>,
    /// Unique identifier for this replica. Stamped on `taken_by` when
    /// CAS-claiming a JobResult, so sibling replicas skip it.
    orchestrator_id: String,
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

        let rtdb = FirebaseRtdb::from_env().context("Failed to create Firebase RTDB client")?;

        // Load stage images from environment — every stage in the registry
        // expects a matching STAGE_<KEY>_IMAGE env var.
        let mut stage_images = HashMap::new();
        for spec in igait_lib::microservice::STAGES {
            let stage = spec.id();
            let env_var = stage_image_env_var(stage);
            match std::env::var(&env_var) {
                Ok(image) => {
                    info!("Image for {}: {}", stage, image);
                    stage_images.insert(stage, image);
                }
                Err(_) => {
                    warn!(
                        "Missing {} env var — {} Jobs cannot be created",
                        env_var, stage
                    );
                }
            }
        }

        let orchestrator_id = generate_worker_id("orchestrator");
        info!("Orchestrator id: {}", orchestrator_id);

        Ok(Self {
            kube_client,
            rtdb,
            stage_images,
            in_flight: RwLock::new(HashMap::new()),
            orchestrator_id,
        })
    }

    /// Reads the live job-generation epoch for a job. Returns 0 if no
    /// coordination record exists (job has never been rerun).
    async fn current_job_epoch(&self, job_id: &str) -> u64 {
        let Ok((user_id, job_key)) = QueueOps::parse_job_id(job_id) else {
            return 0;
        };
        let queue_ops = QueueOps::new(self.rtdb.clone(), self.orchestrator_id.clone());
        queue_ops
            .read_job_epoch(&user_id, &job_key)
            .await
            .unwrap_or(0)
    }

    /// Creates a K8s Job for a standard processing stage (1-6).
    #[instrument(skip_all, fields(stage = %stage, job_id = %job.job_id, epoch = job.epoch))]
    async fn create_stage_job(&self, stage: StageId, job: &QueueItem) -> Result<String> {
        let image = self
            .stage_images
            .get(&stage)
            .ok_or_else(|| anyhow::anyhow!("No image configured for {}", stage))?;

        let payload = serde_json::to_string(job).context("Failed to serialize QueueItem")?;

        let job_name = self.make_job_name(stage, &job.job_id);
        let resources = stage_resources(stage);
        let job_epoch = self.current_job_epoch(&job.job_id).await;

        let k8s_job = self.build_job_spec(
            &job_name,
            image,
            "IGAIT_JOB_PAYLOAD",
            &payload,
            &resources,
            stage,
            &job.job_id,
            &job.user_id,
            job_epoch,
        );

        let jobs_api: Api<Job> = Api::namespaced(self.kube_client.clone(), NAMESPACE);
        jobs_api
            .create(&PostParams::default(), &k8s_job)
            .await
            .context(format!("Failed to create K8s Job {}", job_name))?;

        info!(
            "Created K8s Job {} for {} job {} (job_epoch={})",
            job_name, stage, job.job_id, job_epoch
        );
        Ok(job_name)
    }

    /// Creates a K8s Job for the finalize (terminal) stage.
    #[instrument(skip_all, fields(stage = "finalize", job_id = %job.job_id))]
    async fn create_finalize_job(&self, job: &FinalizeQueueItem) -> Result<String> {
        let stage = StageId::new("finalize");
        let image = self
            .stage_images
            .get(&stage)
            .ok_or_else(|| anyhow::anyhow!("No image configured for stage 7 (finalize)"))?;

        let payload =
            serde_json::to_string(job).context("Failed to serialize FinalizeQueueItem")?;

        let job_name = self.make_job_name(stage, &job.job_id);
        let resources = stage_resources(stage);
        let job_epoch = self.current_job_epoch(&job.job_id).await;

        let k8s_job = self.build_job_spec(
            &job_name,
            image,
            "IGAIT_FINALIZE_PAYLOAD",
            &payload,
            &resources,
            stage,
            &job.job_id,
            &job.user_id,
            job_epoch,
        );

        let jobs_api: Api<Job> = Api::namespaced(self.kube_client.clone(), NAMESPACE);
        jobs_api
            .create(&PostParams::default(), &k8s_job)
            .await
            .context(format!("Failed to create K8s Job {}", job_name))?;

        info!(
            "Created K8s Job {} for finalize job {} (job_epoch={})",
            job_name, job.job_id, job_epoch
        );
        Ok(job_name)
    }

    /// Builds a K8s Job spec.
    #[allow(clippy::too_many_arguments)] // Each arg maps to a distinct field in the K8s pod spec; bundling into a struct would just shift the same parameters one level deeper.
    fn build_job_spec(
        &self,
        job_name: &str,
        image: &str,
        payload_env_var: &str,
        payload: &str,
        resources: &StageResources,
        stage: StageId,
        job_id: &str,
        user_id: &str,
        job_epoch: u64,
    ) -> Job {
        let mut resource_requests = std::collections::BTreeMap::new();
        resource_requests.insert(
            "cpu".to_string(),
            Quantity(resources.cpu_request.to_string()),
        );
        resource_requests.insert(
            "memory".to_string(),
            Quantity(resources.memory_request.to_string()),
        );
        if let Some(eph) = resources.ephemeral_request {
            resource_requests.insert("ephemeral-storage".to_string(), Quantity(eph.to_string()));
        }

        let mut resource_limits = std::collections::BTreeMap::new();
        resource_limits.insert("cpu".to_string(), Quantity(resources.cpu_limit.to_string()));
        resource_limits.insert(
            "memory".to_string(),
            Quantity(resources.memory_limit.to_string()),
        );
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
                    ("igait.niu.edu/stage".to_string(), stage.key().to_string()),
                ])),
                annotations: Some(std::collections::BTreeMap::from([
                    ("igait.niu.edu/job-id".to_string(), job_id.to_string()),
                    ("igait.niu.edu/user-id".to_string(), user_id.to_string()),
                    ("igait.niu.edu/job-epoch".to_string(), job_epoch.to_string()),
                ])),
                ..Default::default()
            },
            spec: Some(JobSpec {
                backoff_limit: Some(2),
                active_deadline_seconds: Some(resources.active_deadline_secs),
                ttl_seconds_after_finished: Some(300),
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(std::collections::BTreeMap::from([(
                            "app".to_string(),
                            "igait-pipeline".to_string(),
                        )])),
                        ..Default::default()
                    }),
                    spec: Some(PodSpec {
                        restart_policy: Some("Never".to_string()),
                        containers: vec![Container {
                            name: "worker".to_string(),
                            image: Some(image.to_string()),
                            env: Some(vec![
                                EnvVar {
                                    name: payload_env_var.to_string(),
                                    value: Some(payload.to_string()),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "IGAIT_JOB_EPOCH".to_string(),
                                    value: Some(job_epoch.to_string()),
                                    ..Default::default()
                                },
                            ]),
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
    fn make_job_name(&self, stage: StageId, job_id: &str) -> String {
        let sanitized = job_id
            .to_lowercase()
            .replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "-");
        let random = format!("{:05x}", rand::random::<u32>() & 0xFFFFF);
        let prefix = format!("igait-{}-", stage.key());
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
    #[instrument(skip(self), fields(stage = %stage, orchestrator_id = %self.orchestrator_id))]
    async fn poll_and_dispatch_stage(&self, stage: StageId) -> Result<bool> {
        let queue_ops = QueueOps::new(self.rtdb.clone(), "orchestrator".to_string());

        let job = match queue_ops.claim_job(stage).await {
            ClaimResult::Claimed(job) => job,
            ClaimResult::QueueEmpty | ClaimResult::AllClaimed => return Ok(false),
            ClaimResult::Error(e) => {
                error!("Failed to claim job for {}: {}", stage, e);
                return Ok(false);
            }
        };

        info!("Claimed job {} for {}", job.job_id, stage);

        // Track as in-flight
        self.in_flight
            .write()
            .await
            .insert(job.job_id.clone(), stage);

        // Create K8s Job
        match self.create_stage_job(stage, &job).await {
            Ok(job_name) => {
                info!("Dispatched K8s Job {} for job {}", job_name, job.job_id);
                Ok(true)
            }
            Err(e) => {
                error!("Failed to create K8s Job for job {}: {}", job.job_id, e);
                if let Err(release_err) = queue_ops.release_job(stage, &job.job_id, job.epoch).await
                {
                    warn!(
                        "Failed to release claim on job {} after K8s Job creation failure: {:?} — will be reclaimed after claim TTL",
                        job.job_id, release_err
                    );
                }
                self.in_flight.write().await.remove(&job.job_id);
                Err(e)
            }
        }
    }

    /// Attempts to claim and dispatch a finalize job (stage 7).
    #[instrument(skip(self), fields(stage = 7, orchestrator_id = %self.orchestrator_id))]
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

        self.in_flight
            .write()
            .await
            .insert(job.job_id.clone(), StageId::new("finalize"));

        match self.create_finalize_job(&job).await {
            Ok(job_name) => {
                info!(
                    "Dispatched K8s finalize Job {} for job {}",
                    job_name, job.job_id
                );
                Ok(true)
            }
            Err(e) => {
                error!(
                    "Failed to create K8s finalize Job for job {}: {}",
                    job.job_id, e
                );
                self.in_flight.write().await.remove(&job.job_id);
                Err(e)
            }
        }
    }

    /// CAS-claims a JobResult so only one orchestrator replica processes it.
    /// Returns `Ok(None)` if already taken by another replica within the stale window,
    /// or if a concurrent replica beat us to the CAS write.
    #[instrument(skip(self), fields(job_id = %safe_job_id, orchestrator_id = %self.orchestrator_id))]
    async fn try_take_completion_result(&self, safe_job_id: &str) -> Result<Option<JobResult>> {
        let path = format!("job_results/{}", safe_job_id);
        let (current, etag) = self.rtdb.get_with_etag::<JobResult>(&path).await?;

        let Some(mut result) = current else {
            return Ok(None);
        };

        if let (Some(_), Some(taken_at)) = (&result.taken_by, result.taken_at) {
            if now_ms().saturating_sub(taken_at) < JOB_RESULT_TAKE_TIMEOUT_MS {
                return Ok(None);
            }
        }

        result.taken_by = Some(self.orchestrator_id.clone());
        result.taken_at = Some(now_ms());

        match self.rtdb.put_if_match(&path, &result, &etag).await? {
            CasResult::Ok => Ok(Some(result)),
            CasResult::PreconditionFailed => Ok(None),
        }
    }

    /// Checks `job_results/` in Firebase RTDB for completed results and handles transitions.
    #[instrument(skip(self))]
    async fn check_completions(&self) -> Result<()> {
        let results: Option<HashMap<String, JobResult>> = self
            .rtdb
            .get("job_results")
            .await
            .context("Failed to read job_results from RTDB")?;

        let Some(results) = results else {
            return Ok(());
        };

        for (safe_job_id, _peek) in results {
            let result = match self.try_take_completion_result(&safe_job_id).await {
                Ok(Some(r)) => r,
                Ok(None) => continue,
                Err(e) => {
                    warn!("Failed to CAS-claim job_results/{}: {}", safe_job_id, e);
                    continue;
                }
            };

            let job_id = &result.job_id;
            let stage = result.stage;

            let (user_id, job_key) = match QueueOps::parse_job_id(job_id) {
                Ok(parsed) => parsed,
                Err(e) => {
                    error!("Failed to parse job_id {}: {}", job_id, e);
                    if let Err(e) = self
                        .rtdb
                        .delete(&format!("job_results/{}", safe_job_id))
                        .await
                    {
                        warn!(
                            "Failed to delete unparseable job_result {}: {}",
                            safe_job_id, e
                        );
                    }
                    continue;
                }
            };

            let live_epoch = self.current_job_epoch(job_id).await;
            if result.job_epoch < live_epoch {
                warn!(
                    "Discarding stale JobResult for {} {} (result_epoch={} live_epoch={})",
                    job_id, stage, result.job_epoch, live_epoch
                );
                if let Err(e) = self
                    .rtdb
                    .delete(&format!("job_results/{}", safe_job_id))
                    .await
                {
                    warn!("Failed to delete stale job_result {}: {}", safe_job_id, e);
                }
                self.in_flight.write().await.remove(job_id);
                continue;
            }

            if let Err(e) = self
                .apply_completion_transition(&result, stage, &user_id, &job_key)
                .await
            {
                error!(
                    "Failed to apply completion transition for {}: {:?}",
                    job_id, e
                );
                continue;
            }

            let result_path = job_result_path(job_id);
            if let Err(e) = self.rtdb.delete(&result_path).await {
                // Not fatal: the take-once CAS on job_results prevents a
                // double-apply on retry, so the worst case is a spammy warn
                // until the next scan wins the delete.
                warn!(
                    "Failed to delete processed job_result {}: {}",
                    safe_job_id, e
                );
            }

            self.in_flight.write().await.remove(job_id);
        }

        Ok(())
    }

    /// Performs the single atomic multi-path update for a stage completion.
    ///
    /// Every transition now bundles: the queue move, the per-stage status seal,
    /// the per-stage logs, and (where applicable) the top-level job status.
    /// Either the whole transition commits or none of it does, which eliminates
    /// the "queue advanced but status stayed Running" class of stale-state bug.
    #[instrument(skip_all, fields(
        job_id = %result.job_id,
        stage = %stage,
        epoch = result.job_epoch,
        success = result.success,
    ))]
    async fn apply_completion_transition(
        &self,
        result: &JobResult,
        stage: StageId,
        user_id: &str,
        job_key: &str,
    ) -> Result<()> {
        let job_id = &result.job_id;
        let mut updates: HashMap<String, serde_json::Value> = HashMap::new();

        if result.success {
            info!("Job {} {} completed successfully", job_id, stage);

            updates.insert(
                stage_status_path(user_id, job_key, stage),
                serde_json::to_value(StageStatus::Complete)?,
            );
            updates.insert(
                stage_logs_path(user_id, job_key, stage),
                serde_json::Value::String(result.logs.clone()),
            );

            if stage.terminal() {
                // Terminal stage completing: clear the finalize queue item
                // and stamp the final job status.
                updates.insert(queue_item_path(stage, job_id), serde_json::Value::Null);
                if let Some(is_asd_raw) = result.output_keys.get("is_asd") {
                    let is_asd = is_asd_raw == "true";
                    let prediction = result
                        .output_keys
                        .get("prediction")
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.0);
                    updates.insert(
                        job_status_path(user_id, job_key),
                        serde_json::to_value(JobStatus::complete(prediction, is_asd))?,
                    );
                }
            } else if stage.next().terminal() {
                // Last non-terminal stage: move the job into the
                // finalize (terminal) queue with a FinalizeQueueItem.
                let finalize_item = FinalizeQueueItem::success(
                    result.job_id.clone(),
                    result.user_id.clone(),
                    result.output_keys.clone(),
                    result.metadata.clone(),
                    result.requires_approval,
                    result.approved,
                );
                updates.insert(queue_item_path(stage, job_id), serde_json::Value::Null);
                updates.insert(
                    queue_item_path(stage.next(), job_id),
                    serde_json::to_value(&finalize_item)?,
                );
            } else {
                let mut next_job = QueueItem::new(
                    result.job_id.clone(),
                    result.user_id.clone(),
                    result.output_keys.clone(),
                    result.metadata.clone(),
                    result.requires_approval,
                );
                next_job.approved = result.approved;

                let next_stage = igait_lib::microservice::next_stage(stage);
                updates.insert(queue_item_path(stage, job_id), serde_json::Value::Null);
                updates.insert(
                    queue_item_path(next_stage, job_id),
                    serde_json::to_value(&next_job)?,
                );
            }
        } else {
            let error_msg = result
                .error
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            warn!("Job {} {} failed: {}", job_id, stage, error_msg);

            updates.insert(
                stage_status_path(user_id, job_key, stage),
                serde_json::to_value(StageStatus::Error)?,
            );
            updates.insert(
                stage_logs_path(user_id, job_key, stage),
                serde_json::Value::String(result.logs.clone()),
            );
            updates.insert(
                job_status_path(user_id, job_key),
                serde_json::to_value(JobStatus::error(result.logs.clone()))?,
            );

            if stage.terminal() {
                error!(
                    "Terminal stage {:?} failed for job {}: {}",
                    stage.key(),
                    job_id,
                    error_msg
                );
                updates.insert(queue_item_path(stage, job_id), serde_json::Value::Null);
            } else {
                let finalize_item = FinalizeQueueItem::failure(
                    result.job_id.clone(),
                    result.user_id.clone(),
                    stage,
                    error_msg,
                    Some(result.logs.clone()),
                    result.metadata.clone(),
                );
                updates.insert(queue_item_path(stage, job_id), serde_json::Value::Null);
                updates.insert(
                    queue_item_path(StageId::new("finalize"), job_id),
                    serde_json::to_value(&finalize_item)?,
                );
            }
        }

        let rtdb = &self.rtdb;
        retry_transient(
            &RetryPolicy::default(),
            "apply_completion_transition",
            || {
                let updates = updates.clone();
                async move { rtdb.multi_update(updates).await }
            },
        )
        .await
        .context("Failed to apply completion transition")?;
        Ok(())
    }

    /// Deletes all in-flight K8s Jobs for a given `job_id`, propagating the
    /// delete to the owned pods. Returns the number of K8s Jobs cancelled.
    ///
    /// Called by the rerun endpoint before writing the new queue item so that
    /// a still-running stage can't finish and contaminate the rerun with a
    /// stale result. The epoch bump is the ultimate safety net; this is the
    /// proactive "please stop now" that avoids the wasted compute.
    #[instrument(skip(self), fields(job_id = %job_id))]
    pub async fn cancel_in_flight_jobs(&self, job_id: &str) -> Result<usize> {
        let jobs_api: Api<Job> = Api::namespaced(self.kube_client.clone(), NAMESPACE);
        let lp = ListParams::default().labels("app=igait-pipeline,managed-by=igait-backend");

        let job_list = jobs_api
            .list(&lp)
            .await
            .context("Failed to list K8s Jobs for cancellation")?;

        let dp = DeleteParams {
            propagation_policy: Some(PropagationPolicy::Foreground),
            ..Default::default()
        };

        let mut cancelled = 0;
        for k8s_job in job_list.items {
            let matches_job_id = k8s_job
                .metadata
                .annotations
                .as_ref()
                .and_then(|a| a.get("igait.niu.edu/job-id"))
                .map(|v| v == job_id)
                .unwrap_or(false);
            if !matches_job_id {
                continue;
            }

            let Some(name) = k8s_job.metadata.name.as_deref() else {
                continue;
            };

            match jobs_api.delete(name, &dp).await {
                Ok(_) => {
                    info!("Cancelled K8s Job {} for rerun of {}", name, job_id);
                    cancelled += 1;
                }
                Err(kube::Error::Api(e)) if e.code == 404 => {
                    // Already gone — race with TTL/finalizer; treat as success
                }
                Err(e) => {
                    warn!("Failed to cancel K8s Job {}: {:?}", name, e);
                }
            }
        }

        self.in_flight.write().await.remove(job_id);
        Ok(cancelled)
    }

    /// Checks for stale K8s Jobs that have failed/timed out without writing a result.
    #[instrument(skip(self))]
    async fn check_stale_jobs(&self) -> Result<()> {
        let jobs_api: Api<Job> = Api::namespaced(self.kube_client.clone(), NAMESPACE);
        let lp = ListParams::default().labels("app=igait-pipeline,managed-by=igait-backend");

        let job_list = jobs_api
            .list(&lp)
            .await
            .context("Failed to list K8s Jobs")?;

        for k8s_job in job_list.items {
            let job_name = k8s_job.metadata.name.as_deref().unwrap_or("unknown");
            let status = k8s_job.status.as_ref();

            let is_failed = status
                .and_then(|s| s.failed)
                .map(|f| f > 0)
                .unwrap_or(false);

            let conditions = status.and_then(|s| s.conditions.as_ref());
            let is_deadline_exceeded = conditions
                .map(|conds| {
                    conds.iter().any(|c| {
                        c.type_ == "Failed" && c.reason.as_deref() == Some("DeadlineExceeded")
                    })
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
                    let Some(stage) = StageId::try_new(stage_str) else {
                        warn!(
                            "K8s Job {} has unknown stage label {:?}, skipping cleanup",
                            job_name, stage_str
                        );
                        continue;
                    };

                    // Only write a failure result if one doesn't already exist
                    let result_path = job_result_path(job_id);
                    let existing: Option<serde_json::Value> =
                        self.rtdb.get(&result_path).await.unwrap_or(None);

                    if existing.is_none() {
                        let reason = if is_deadline_exceeded {
                            "timed out"
                        } else {
                            "crashed"
                        };
                        warn!(
                            "Writing synthetic failure result for job {} {} ({})",
                            job_id, stage, reason
                        );

                        let user_id_annotation = annotations
                            .and_then(|a| a.get("igait.niu.edu/user-id"))
                            .cloned()
                            .unwrap_or_default();

                        let job_epoch_annotation: u64 = annotations
                            .and_then(|a| a.get("igait.niu.edu/job-epoch"))
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0);

                        let error_text = format!("K8s Job {}: pod {}", reason, job_name);
                        let logs_text =
                            format!("{} pod {} without writing a result", stage, reason);

                        let failure_result = JobResult {
                            stage,
                            success: false,
                            output_keys: HashMap::new(),
                            error: Some(error_text.clone()),
                            logs: logs_text.clone(),
                            duration_ms: 0,
                            job_id: job_id.clone(),
                            user_id: user_id_annotation.clone(),
                            metadata: JobMetadata::default(),
                            input_keys: HashMap::new(),
                            requires_approval: false,
                            approved: false,
                            epoch: 0,
                            job_epoch: job_epoch_annotation,
                            taken_by: None,
                            taken_at: None,
                        };

                        let mut updates: HashMap<String, serde_json::Value> = HashMap::new();
                        updates.insert(result_path.clone(), serde_json::to_value(&failure_result)?);

                        if let Ok((parsed_user_id, job_key)) = QueueOps::parse_job_id(job_id) {
                            let uid = if user_id_annotation.is_empty() {
                                parsed_user_id
                            } else {
                                user_id_annotation
                            };
                            updates.insert(
                                stage_status_path(&uid, &job_key, stage),
                                serde_json::to_value(StageStatus::Error)?,
                            );
                            updates.insert(
                                stage_logs_path(&uid, &job_key, stage),
                                serde_json::Value::String(logs_text),
                            );
                        }

                        let rtdb = &self.rtdb;
                        let write_result = retry_transient(
                            &RetryPolicy::default(),
                            "check_stale_jobs/synthetic_failure",
                            || {
                                let updates = updates.clone();
                                async move { rtdb.multi_update(updates).await }
                            },
                        )
                        .await;
                        if let Err(e) = write_result {
                            error!(
                                "Failed to write synthetic failure result for {}: {}",
                                job_id, e
                            );
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

/// Main orchestration loop — polls every non-terminal stage queue and
/// dispatches K8s Jobs. The terminal (finalize) queue uses a different
/// item shape and is polled via its own path.
pub async fn orchestration_loop(orchestrator: Arc<Orchestrator>) {
    let stages: Vec<_> = igait_lib::microservice::STAGES
        .iter()
        .filter(|spec| !spec.terminal)
        .map(|spec| spec.id())
        .collect();

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
                    error!("Orchestrator error for {}: {:?}", stage, e);
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
