//! Queue-based worker infrastructure for stage microservices.
//!
//! This module provides the worker loop that polls Firebase Realtime Database
//! queues, claims jobs using transactions, and processes them independently.

use crate::microservice::{
    queue::{
        ClaimResult, FinalizeQueueItem, JobResult, ProcessingResult, QueueConfig, QueueItem,
        CLAIM_TIMEOUT_MS, HEARTBEAT_INTERVAL_SECS,
        generate_worker_id, job_result_path, next_stage, now_ms, queue_config_path,
        queue_item_path, queue_path,
    },
    backend_status::{JobStatus, StageStatus},
    StageNumber,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

// ============================================================================
// FIREBASE RTDB CLIENT
// ============================================================================

/// A simple Firebase Realtime Database client for queue operations.
/// 
/// This client supports the transaction-like pattern needed for safe job claiming.
#[derive(Clone)]
pub struct FirebaseRtdb {
    /// Base URL of the Firebase RTDB (e.g., "https://project-id.firebaseio.com")
    base_url: String,
    
    /// Auth token for database access
    auth_token: String,
    
    /// HTTP client
    client: Client,
}

impl FirebaseRtdb {
    /// Creates a new Firebase RTDB client.
    pub fn new(base_url: &str, auth_token: &str) -> Self {
        // Remove trailing slash if present
        let base_url = base_url.trim_end_matches('/').to_string();
        
        Self {
            base_url,
            auth_token: auth_token.to_string(),
            client: Client::new(),
        }
    }

    /// Creates a client from environment variables.
    /// 
    /// Expects:
    /// - `FIREBASE_RTDB_URL`: The database URL
    /// - `FIREBASE_ACCESS_KEY`: The auth token
    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("FIREBASE_RTDB_URL")
            .or_else(|_| Ok::<_, std::env::VarError>(
                "https://network-technology-project-default-rtdb.firebaseio.com".to_string()
            ))?;
        let auth_token = std::env::var("FIREBASE_ACCESS_KEY")
            .context("Missing FIREBASE_ACCESS_KEY environment variable")?;
        
        Ok(Self::new(&base_url, &auth_token))
    }

    /// Builds a URL for a given path.
    fn url(&self, path: &str) -> String {
        format!("{}/{}.json?auth={}", self.base_url, path, self.auth_token)
    }

    /// Gets data at a path.
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<Option<T>> {
        let url = self.url(path);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Firebase GET failed ({}): {}", status, body);
        }
        
        let value: Value = response.json().await?;
        if value.is_null() {
            return Ok(None);
        }
        
        let data: T = serde_json::from_value(value)?;
        Ok(Some(data))
    }

    /// Sets data at a path (overwrites).
    pub async fn set<T: Serialize>(&self, path: &str, data: &T) -> Result<()> {
        let url = self.url(path);
        let response = self.client.put(&url).json(data).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Firebase SET failed ({}): {}", status, body);
        }
        
        Ok(())
    }

    /// Updates specific fields at a path (PATCH).
    pub async fn update<T: Serialize>(&self, path: &str, data: &T) -> Result<()> {
        let url = self.url(path);
        let response = self.client.patch(&url).json(data).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Firebase UPDATE failed ({}): {}", status, body);
        }
        
        Ok(())
    }

    /// Deletes data at a path.
    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = self.url(path);
        let response = self.client.delete(&url).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Firebase DELETE failed ({}): {}", status, body);
        }
        
        Ok(())
    }

    /// Performs a multi-path update (atomic update to multiple paths).
    ///
    /// The updates map should have paths as keys (without leading slash)
    /// and the new values. Use `Value::Null` to delete a path.
    pub async fn multi_update(&self, updates: HashMap<String, Value>) -> Result<()> {
        let url = self.url("");
        let response = self.client.patch(&url).json(&updates).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Firebase MULTI_UPDATE failed ({}): {}", status, body);
        }

        Ok(())
    }

    /// Reads data at `path` alongside its current ETag.
    pub async fn get_with_etag<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<(Option<T>, String)> {
        let url = self.url(path);
        let response = self.client
            .get(&url)
            .header("X-Firebase-ETag", "true")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Firebase GET (with ETag) failed ({}): {}", status, body);
        }

        let etag = response.headers()
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let value: Value = response.json().await?;
        if value.is_null() {
            return Ok((None, etag));
        }

        let data: T = serde_json::from_value(value)?;
        Ok((Some(data), etag))
    }

    /// Conditional PUT — writes iff the current ETag matches.
    pub async fn put_if_match<T: Serialize>(&self, path: &str, data: &T, etag: &str) -> Result<CasResult> {
        let url = self.url(path);
        let response = self.client
            .put(&url)
            .header("if-match", etag)
            .json(data)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::PRECONDITION_FAILED {
            return Ok(CasResult::PreconditionFailed);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Firebase PUT (If-Match) failed ({}): {}", status, body);
        }

        Ok(CasResult::Ok)
    }

    /// Conditional PATCH — merges iff the current ETag matches.
    pub async fn patch_if_match<T: Serialize>(&self, path: &str, data: &T, etag: &str) -> Result<CasResult> {
        let url = self.url(path);
        let response = self.client
            .patch(&url)
            .header("if-match", etag)
            .json(data)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::PRECONDITION_FAILED {
            return Ok(CasResult::PreconditionFailed);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Firebase PATCH (If-Match) failed ({}): {}", status, body);
        }

        Ok(CasResult::Ok)
    }

    /// Conditional DELETE — removes iff the current ETag matches.
    pub async fn delete_if_match(&self, path: &str, etag: &str) -> Result<CasResult> {
        let url = self.url(path);
        let response = self.client
            .delete(&url)
            .header("if-match", etag)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::PRECONDITION_FAILED {
            return Ok(CasResult::PreconditionFailed);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Firebase DELETE (If-Match) failed ({}): {}", status, body);
        }

        Ok(CasResult::Ok)
    }

    /// Read-modify-write transaction with CAS retry.
    ///
    /// `transform` returns `None` to abort (nothing to do).
    pub async fn transaction<T, F>(&self, path: &str, mut transform: F) -> Result<TxOutcome<T>>
    where
        T: Serialize + for<'de> Deserialize<'de> + Clone,
        F: FnMut(Option<T>) -> Option<T>,
    {
        let mut backoff_ms: u64 = 25;

        for _ in 0..MAX_TRANSACTION_ATTEMPTS {
            let (current, etag) = self.get_with_etag::<T>(path).await?;

            let Some(new_value) = transform(current) else {
                return Ok(TxOutcome::Aborted);
            };

            match self.put_if_match(path, &new_value, &etag).await? {
                CasResult::Ok => return Ok(TxOutcome::Committed(new_value)),
                CasResult::PreconditionFailed => {
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(500);
                }
            }
        }

        Ok(TxOutcome::ExhaustedRetries)
    }
}

/// Firebase ETag sentinel meaning "this path is currently empty".
pub const NULL_ETAG: &str = "null_etag";

pub const MAX_TRANSACTION_ATTEMPTS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasResult {
    Ok,
    PreconditionFailed,
}

impl CasResult {
    pub fn is_ok(self) -> bool {
        matches!(self, CasResult::Ok)
    }

    pub fn is_precondition_failed(self) -> bool {
        matches!(self, CasResult::PreconditionFailed)
    }
}

#[derive(Debug, Clone)]
pub enum TxOutcome<T> {
    Committed(T),
    Aborted,
    ExhaustedRetries,
}

impl<T> TxOutcome<T> {
    pub fn committed(self) -> Option<T> {
        match self {
            TxOutcome::Committed(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_committed(&self) -> bool {
        matches!(self, TxOutcome::Committed(_))
    }
}

// ============================================================================
// QUEUE OPERATIONS
// ============================================================================

/// Operations for working with stage queues.
pub struct QueueOps {
    db: FirebaseRtdb,
    worker_id: String,
}

impl QueueOps {
    /// Creates a new QueueOps instance.
    pub fn new(db: FirebaseRtdb, worker_id: String) -> Self {
        Self { db, worker_id }
    }

    /// Attempts to claim an available job from the specified stage queue.
    ///
    /// Candidate items are filtered by approval status and claim/heartbeat
    /// state, then each is claimed via a per-path CAS transaction. If two
    /// workers see the same unclaimed item, at most one `put_if_match` wins;
    /// the loser's transform re-reads, sees it's now claimed, and aborts.
    pub async fn claim_job(&self, stage: StageNumber) -> ClaimResult<QueueItem> {
        let path = queue_path(stage);

        let config_path = queue_config_path(stage);
        let queue_config: QueueConfig = match self.db.get(&config_path).await {
            Ok(Some(cfg)) => cfg,
            Ok(None) => QueueConfig::default(),
            Err(e) => {
                eprintln!("Warning: failed to read queue config at {}: {}", config_path, e);
                QueueConfig::default()
            }
        };

        let items: Option<HashMap<String, QueueItem>> = match self.db.get(&path).await {
            Ok(items) => items,
            Err(e) => return ClaimResult::Error(format!("Failed to read queue: {}", e)),
        };

        let Some(items) = items else {
            return ClaimResult::QueueEmpty;
        };

        if items.is_empty() {
            return ClaimResult::QueueEmpty;
        }

        let now = now_ms();
        let requires_approval = queue_config.requires_approval;
        let mut candidates: Vec<String> = items
            .iter()
            .filter(|(_, item)| {
                let is_unclaimed = item.claimed_by.is_none();
                let is_stale = item
                    .claimed_at
                    .map(|t| now.saturating_sub(t) > CLAIM_TIMEOUT_MS)
                    .unwrap_or(false);
                (is_unclaimed || is_stale)
                    && item.is_approved_for_processing(requires_approval)
            })
            .map(|(k, _)| k.clone())
            .collect();

        if candidates.is_empty() {
            return ClaimResult::AllClaimed;
        }

        // Deterministic order prevents two orchestrators from deadlock-like
        // thrashing on the same key sequence.
        candidates.sort();

        for key in candidates {
            let item_path = format!("{}/{}", path, key);
            let worker_id = self.worker_id.clone();

            let outcome = self.db
                .transaction::<QueueItem, _>(&item_path, move |current| {
                    let current = current?;
                    let now = now_ms();
                    let is_unclaimed = current.claimed_by.is_none();
                    let is_stale = current
                        .claimed_at
                        .map(|t| now.saturating_sub(t) > CLAIM_TIMEOUT_MS)
                        .unwrap_or(false);

                    if !(is_unclaimed || is_stale) {
                        return None;
                    }

                    if !current.is_approved_for_processing(requires_approval) {
                        return None;
                    }

                    Some(current.claim(&worker_id))
                })
                .await;

            match outcome {
                Ok(TxOutcome::Committed(claimed)) => return ClaimResult::Claimed(claimed),
                Ok(TxOutcome::Aborted) => continue,
                Ok(TxOutcome::ExhaustedRetries) => {
                    return ClaimResult::Error(format!(
                        "Too many CAS retries claiming {}",
                        item_path
                    ));
                }
                Err(e) => return ClaimResult::Error(format!("Failed to claim job: {}", e)),
            }
        }

        ClaimResult::AllClaimed
    }

    /// Refreshes the heartbeat, but only if the caller's epoch still matches.
    ///
    /// Returns `Ok(false)` when the epoch has changed (the lease has been lost
    /// to another worker). Callers should stop processing and exit cleanly.
    pub async fn heartbeat(&self, stage: StageNumber, job_id: &str, epoch: u64) -> Result<bool> {
        let path = queue_item_path(stage, job_id);
        let outcome = self.db
            .transaction::<QueueItem, _>(&path, move |current| {
                let current = current?;
                if current.epoch != epoch {
                    return None;
                }
                Some(QueueItem {
                    claimed_at: Some(now_ms()),
                    ..current
                })
            })
            .await?;

        Ok(matches!(outcome, TxOutcome::Committed(_)))
    }

    /// Releases a claim back to the queue, iff the caller's epoch still matches.
    pub async fn release_job(&self, stage: StageNumber, job_id: &str, epoch: u64) -> Result<bool> {
        let path = queue_item_path(stage, job_id);
        let outcome = self.db
            .transaction::<QueueItem, _>(&path, move |current| {
                let current = current?;
                if current.epoch != epoch {
                    return None;
                }
                Some(QueueItem {
                    claimed_by: None,
                    claimed_at: None,
                    ..current
                })
            })
            .await?;

        Ok(matches!(outcome, TxOutcome::Committed(_)))
    }

    /// Moves a job to the next stage queue after successful processing.
    ///
    /// Returns `Ok(false)` if the lease was lost (epoch drift) — the move does
    /// not occur and the caller should exit. Verifies the epoch via a CAS
    /// read before performing the atomic multi-path move.
    pub async fn move_to_next_stage(
        &self,
        current_stage: StageNumber,
        job: &QueueItem,
        output_keys: HashMap<String, String>,
    ) -> Result<bool> {
        let current_path = queue_item_path(current_stage, &job.job_id);
        if !self.verify_epoch(&current_path, job.epoch).await? {
            return Ok(false);
        }

        let next = next_stage(current_stage);
        let mut next_item = QueueItem::new(
            job.job_id.clone(),
            job.user_id.clone(),
            output_keys,
            job.metadata.clone(),
            job.requires_approval,
        );
        next_item.approved = job.approved;

        let next_path = queue_item_path(next, &job.job_id);

        let mut updates = HashMap::new();
        updates.insert(current_path, Value::Null);
        updates.insert(next_path, serde_json::to_value(&next_item)?);

        self.db.multi_update(updates).await?;

        Ok(true)
    }

    /// Moves a job to the finalize queue after successful pipeline completion.
    pub async fn move_to_finalize_success(
        &self,
        current_stage: StageNumber,
        job: &QueueItem,
        output_keys: HashMap<String, String>,
    ) -> Result<bool> {
        let current_path = queue_item_path(current_stage, &job.job_id);
        if !self.verify_epoch(&current_path, job.epoch).await? {
            return Ok(false);
        }

        let finalize_item = FinalizeQueueItem::success(
            job.job_id.clone(),
            job.user_id.clone(),
            output_keys,
            job.metadata.clone(),
        );

        let finalize_path = queue_item_path(StageNumber::Stage7Finalize, &job.job_id);

        let mut updates = HashMap::new();
        updates.insert(current_path, Value::Null);
        updates.insert(finalize_path, serde_json::to_value(&finalize_item)?);

        self.db.multi_update(updates).await?;

        Ok(true)
    }

    /// Moves a job to the finalize queue after a stage failure.
    pub async fn move_to_finalize_failure(
        &self,
        current_stage: StageNumber,
        job: &QueueItem,
        error: String,
        error_logs: Option<String>,
    ) -> Result<bool> {
        let current_path = queue_item_path(current_stage, &job.job_id);
        if !self.verify_epoch(&current_path, job.epoch).await? {
            return Ok(false);
        }

        let finalize_item = FinalizeQueueItem::failure(
            job.job_id.clone(),
            job.user_id.clone(),
            current_stage.as_u8(),
            error,
            error_logs,
            job.metadata.clone(),
        );

        let finalize_path = queue_item_path(StageNumber::Stage7Finalize, &job.job_id);

        let mut updates = HashMap::new();
        updates.insert(current_path, Value::Null);
        updates.insert(finalize_path, serde_json::to_value(&finalize_item)?);

        self.db.multi_update(updates).await?;

        Ok(true)
    }

    /// Returns true iff the item at `path` still carries `expected_epoch`.
    /// Used before multi-path moves where per-path CAS is not available.
    async fn verify_epoch(&self, path: &str, expected_epoch: u64) -> Result<bool> {
        let (current, _etag) = self.db.get_with_etag::<QueueItem>(path).await?;
        Ok(matches!(current, Some(item) if item.epoch == expected_epoch))
    }

    /// Claims a job from the finalize queue via per-path CAS transaction.
    pub async fn claim_finalize_job(&self) -> ClaimResult<FinalizeQueueItem> {
        let path = queue_path(StageNumber::Stage7Finalize);

        let items: Option<HashMap<String, FinalizeQueueItem>> = match self.db.get(&path).await {
            Ok(items) => items,
            Err(e) => return ClaimResult::Error(format!("Failed to read finalize queue: {}", e)),
        };

        let Some(items) = items else {
            return ClaimResult::QueueEmpty;
        };

        if items.is_empty() {
            return ClaimResult::QueueEmpty;
        }

        let now = now_ms();
        let mut candidates: Vec<String> = items
            .iter()
            .filter(|(_, item)| {
                let is_unclaimed = item.claimed_by.is_none();
                let is_stale = item
                    .claimed_at
                    .map(|t| now.saturating_sub(t) > CLAIM_TIMEOUT_MS)
                    .unwrap_or(false);
                is_unclaimed || is_stale
            })
            .map(|(k, _)| k.clone())
            .collect();

        if candidates.is_empty() {
            return ClaimResult::AllClaimed;
        }

        candidates.sort();

        for key in candidates {
            let item_path = format!("{}/{}", path, key);
            let worker_id = self.worker_id.clone();

            let outcome = self.db
                .transaction::<FinalizeQueueItem, _>(&item_path, move |current| {
                    let current = current?;
                    let now = now_ms();
                    let is_unclaimed = current.claimed_by.is_none();
                    let is_stale = current
                        .claimed_at
                        .map(|t| now.saturating_sub(t) > CLAIM_TIMEOUT_MS)
                        .unwrap_or(false);

                    if !(is_unclaimed || is_stale) {
                        return None;
                    }

                    Some(current.claim(&worker_id))
                })
                .await;

            match outcome {
                Ok(TxOutcome::Committed(claimed)) => return ClaimResult::Claimed(claimed),
                Ok(TxOutcome::Aborted) => continue,
                Ok(TxOutcome::ExhaustedRetries) => {
                    return ClaimResult::Error(format!(
                        "Too many CAS retries claiming finalize {}",
                        item_path
                    ));
                }
                Err(e) => {
                    return ClaimResult::Error(format!("Failed to claim finalize job: {}", e))
                }
            }
        }

        ClaimResult::AllClaimed
    }

    /// Removes a completed job from the finalize queue.
    pub async fn complete_finalize(&self, job_id: &str) -> Result<()> {
        let path = queue_item_path(StageNumber::Stage7Finalize, job_id);
        self.db.delete(&path).await
    }

    /// Updates the job status directly in Firebase RTDB.
    /// 
    /// This writes to `users/{user_id}/jobs/{job_index}/status`
    pub async fn update_job_status(&self, user_id: &str, job_key: &str, status: &JobStatus) -> Result<()> {
        let path = format!("users/{}/jobs/{}/status", user_id, job_key);
        self.db.set(&path, status).await
    }

    /// Updates the per-stage status in Firebase RTDB.
    ///
    /// This writes to `users/{user_id}/jobs/{job_key}/stage_statuses/stage_{n}`
    pub async fn update_stage_status(&self, user_id: &str, job_key: &str, stage: u8, status: &StageStatus) -> Result<()> {
        let path = format!("users/{}/jobs/{}/stage_statuses/stage_{}", user_id, job_key, stage);
        self.db.set(&path, status).await
    }

    /// Uploads stage logs to Firebase RTDB.
    ///
    /// This writes to `users/{user_id}/jobs/{job_key}/stage_logs/stage_{n}`
    pub async fn update_stage_logs(&self, user_id: &str, job_key: &str, stage: u8, logs: &str) -> Result<()> {
        let path = format!("users/{}/jobs/{}/stage_logs/stage_{}", user_id, job_key, stage);
        self.db.set(&path, &logs).await
    }

    /// Parses a job_id string into (user_id, job_key).
    ///
    /// Job IDs are formatted as "{user_id}_{job_key}"
    pub fn parse_job_id(job_id: &str) -> Result<(String, String)> {
        let last_underscore = job_id.rfind('_')
            .ok_or_else(|| anyhow::anyhow!("Invalid job_id format: {}", job_id))?;

        let user_id = job_id[..last_underscore].to_string();
        let job_key = job_id[last_underscore + 1..].to_string();

        Ok((user_id, job_key))
    }
}

// ============================================================================
// STAGE WORKER TRAIT
// ============================================================================

/// Trait that stage workers must implement.
/// 
/// This is similar to the old `StageProcessor` but designed for queue-based operation.
#[async_trait]
pub trait StageWorker: Send + Sync + 'static {
    /// Which stage this worker handles.
    fn stage(&self) -> StageNumber;
    
    /// Human-readable service name.
    fn service_name(&self) -> &'static str;
    
    /// Process a job from the queue.
    /// 
    /// Returns `ProcessingResult::Success` with output keys on success,
    /// or `ProcessingResult::Failure` with error info on failure.
    async fn process(&self, job: &QueueItem) -> ProcessingResult;
}

// ============================================================================
// WORKER RUNNER
// ============================================================================

/// Configuration for the worker runner.
#[derive(Clone)]
pub struct WorkerConfig {
    /// How long to wait between queue polls when no jobs are available
    pub poll_interval: Duration,
    
    /// How long to wait after an error before retrying
    pub error_backoff: Duration,
    
    /// Whether to keep running after a fatal error (vs. crashing)
    pub resilient: bool,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(5),
            error_backoff: Duration::from_secs(10),
            resilient: true,
        }
    }
}

/// Runs a stage worker in a continuous loop.
pub struct WorkerRunner<W: StageWorker> {
    worker: Arc<W>,
    queue_ops: QueueOps,
    config: WorkerConfig,
    worker_id: String,
    shutdown_token: CancellationToken,
}

impl<W: StageWorker> WorkerRunner<W> {
    /// Creates a new worker runner.
    pub fn new(worker: W, db: FirebaseRtdb) -> Self {
        let worker_id = generate_worker_id(worker.service_name());
        let queue_ops = QueueOps::new(db, worker_id.clone());
        
        Self {
            worker: Arc::new(worker),
            queue_ops,
            config: WorkerConfig::default(),
            worker_id,
            shutdown_token: CancellationToken::new(),
        }
    }

    /// Sets custom configuration.
    pub fn with_config(mut self, config: WorkerConfig) -> Self {
        self.config = config;
        self
    }

    /// Returns a clone of the shutdown token for external cancellation.
    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown_token.clone()
    }

    /// Runs the worker loop.
    /// 
    /// This will continuously:
    /// 1. Poll the queue for available jobs
    /// 2. Claim and process any available job
    /// 3. Move the job to the next queue (or finalize queue on failure)
    /// 4. Sleep if no jobs are available
    /// 
    /// The loop will gracefully stop when shutdown is signaled.
    pub async fn run(&self) -> Result<()> {
        let stage = self.worker.stage();
        println!(
            "[{}] Starting worker {} for stage {} ({})",
            self.worker_id,
            self.worker.service_name(),
            stage.as_u8(),
            stage.name()
        );

        loop {
            // Check for shutdown signal
            if self.shutdown_token.is_cancelled() {
                println!("[{}] Shutdown signal received, stopping worker loop", self.worker_id);
                break;
            }

            match self.process_one_job().await {
                Ok(true) => {
                    // Processed a job, immediately check for more
                    continue;
                }
                Ok(false) => {
                    // No jobs available, wait before polling again (or until shutdown)
                    tokio::select! {
                        _ = tokio::time::sleep(self.config.poll_interval) => {},
                        _ = self.shutdown_token.cancelled() => {
                            println!("[{}] Shutdown signal received during sleep", self.worker_id);
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[{}] Error in worker loop: {:?}", self.worker_id, e);
                    
                    if self.config.resilient {
                        tokio::select! {
                            _ = tokio::time::sleep(self.config.error_backoff) => {},
                            _ = self.shutdown_token.cancelled() => {
                                println!("[{}] Shutdown signal received during error backoff", self.worker_id);
                                break;
                            }
                        }
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        println!("[{}] Worker stopped gracefully", self.worker_id);
        Ok(())
    }

    /// Attempts to process one job from the queue.
    /// 
    /// Returns `Ok(true)` if a job was processed, `Ok(false)` if no jobs were available.
    async fn process_one_job(&self) -> Result<bool> {
        let stage = self.worker.stage();

        // Check for shutdown before claiming
        if self.shutdown_token.is_cancelled() {
            return Ok(false);
        }

        // Try to claim a job
        let job = match self.queue_ops.claim_job(stage).await {
            ClaimResult::Claimed(job) => job,
            ClaimResult::QueueEmpty | ClaimResult::AllClaimed => {
                return Ok(false);
            }
            ClaimResult::Error(e) => {
                anyhow::bail!("Failed to claim job: {}", e);
            }
        };

        println!(
            "[{}] Claimed job {} for processing",
            self.worker_id, job.job_id
        );
        
        // Update job status to "Processing" and stage status to "Running" in RTDB
        let stage_num = stage.as_u8();
        self.update_job_status(&job.job_id, JobStatus::processing(stage_num)).await;
        self.update_stage_status(&job.job_id, stage_num, StageStatus::Running).await;

        let heartbeat_db = self.queue_ops.db.clone();
        let heartbeat_worker_id = self.worker_id.clone();
        let heartbeat_job_id = job.job_id.clone();
        let heartbeat_stage = stage;
        let heartbeat_epoch = job.epoch;
        let heartbeat_shutdown = self.shutdown_token.child_token();

        let heartbeat_handle = tokio::spawn(async move {
            let ops = QueueOps::new(heartbeat_db, heartbeat_worker_id);
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(HEARTBEAT_INTERVAL_SECS)) => {
                        match ops.heartbeat(heartbeat_stage, &heartbeat_job_id, heartbeat_epoch).await {
                            Ok(true) => {}
                            Ok(false) => {
                                eprintln!(
                                    "Heartbeat aborted: lease lost for job {} (epoch {})",
                                    heartbeat_job_id, heartbeat_epoch
                                );
                                break;
                            }
                            Err(e) => {
                                eprintln!("Heartbeat failed: {:?}", e);
                                break;
                            }
                        }
                    }
                    _ = heartbeat_shutdown.cancelled() => {
                        break;
                    }
                }
            }
        });

        let process_result = tokio::select! {
            result = self.worker.process(&job) => result,
            _ = self.shutdown_token.cancelled() => {
                println!(
                    "[{}] Job {} processing cancelled due to shutdown",
                    self.worker_id, job.job_id
                );
                heartbeat_handle.abort();
                let _ = self.queue_ops.release_job(stage, &job.job_id, job.epoch).await;
                return Ok(false);
            }
        };

        // Cancel heartbeat
        heartbeat_handle.abort();

        // Handle result
        match process_result {
            ProcessingResult::Success { output_keys, logs, duration_ms } => {
                println!(
                    "[{}] Job {} completed successfully in {}ms",
                    self.worker_id, job.job_id, duration_ms
                );

                // Upload stage logs to Firebase RTDB
                self.upload_stage_logs(&job.job_id, stage_num, &logs).await;

                // Mark this stage as complete
                self.update_stage_status(&job.job_id, stage_num, StageStatus::Complete).await;

                // Stage 7 is finalize, so stage 6 sends to finalize on success.
                let moved = if stage == StageNumber::Stage6Prediction {
                    self.queue_ops
                        .move_to_finalize_success(stage, &job, output_keys)
                        .await
                        .context("Failed to move job to finalize queue")?
                } else {
                    self.queue_ops
                        .move_to_next_stage(stage, &job, output_keys)
                        .await
                        .context("Failed to move job to next stage")?
                };
                if !moved {
                    eprintln!(
                        "[{}] Job {} (epoch {}): lease lost before move; successor will complete it",
                        self.worker_id, job.job_id, job.epoch
                    );
                }
            }
            ProcessingResult::Failure { error, logs, duration_ms } => {
                eprintln!(
                    "[{}] Job {} failed after {}ms: {}",
                    self.worker_id, job.job_id, duration_ms, error
                );

                self.upload_stage_logs(&job.job_id, stage_num, &logs).await;
                self.update_stage_status(&job.job_id, stage_num, StageStatus::Error).await;
                self.update_job_status(&job.job_id, JobStatus::error(logs.clone())).await;

                let moved = self.queue_ops
                    .move_to_finalize_failure(stage, &job, error, Some(logs))
                    .await
                    .context("Failed to move job to finalize queue")?;
                if !moved {
                    eprintln!(
                        "[{}] Job {} (epoch {}): lease lost before failure move; successor will handle",
                        self.worker_id, job.job_id, job.epoch
                    );
                }
            }
        }

        Ok(true)
    }
    
    /// Upload stage logs to Firebase RTDB
    async fn upload_stage_logs(&self, job_id: &str, stage: u8, logs: &str) {
        match QueueOps::parse_job_id(job_id) {
            Ok((user_id, job_index)) => {
                if let Err(e) = self.queue_ops.update_stage_logs(&user_id, &job_index, stage, logs).await {
                    eprintln!("Failed to upload stage {} logs to RTDB: {:?}", stage, e);
                }
            }
            Err(e) => {
                eprintln!("Failed to parse job_id for log upload: {:?}", e);
            }
        }
    }

    /// Update job status directly in RTDB
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

    /// Update per-stage status directly in RTDB
    async fn update_stage_status(&self, job_id: &str, stage: u8, status: StageStatus) {
        match QueueOps::parse_job_id(job_id) {
            Ok((user_id, job_index)) => {
                if let Err(e) = self.queue_ops.update_stage_status(&user_id, &job_index, stage, &status).await {
                    eprintln!("Failed to update stage {} status in RTDB: {:?}", stage, e);
                }
            }
            Err(e) => {
                eprintln!("Failed to parse job_id for stage status update: {:?}", e);
            }
        }
    }
}

// ============================================================================
// CONVENIENCE FUNCTION
// ============================================================================

/// Runs a stage worker with default configuration.
/// 
/// This is the main entry point for stage microservices.
/// Sets up signal handlers for graceful shutdown.
/// 
/// # Example
/// 
/// ```ignore
/// use igait_lib::microservice::{run_stage_worker, StageWorker, StageNumber, QueueItem, ProcessingResult};
/// 
/// struct MyStageWorker;
/// 
/// #[async_trait::async_trait]
/// impl StageWorker for MyStageWorker {
///     fn stage(&self) -> StageNumber { StageNumber::Stage2ValidityCheck }
///     fn service_name(&self) -> &'static str { "stage2-validity-check" }
///     
///     async fn process(&self, job: &QueueItem) -> ProcessingResult {
///         // ... do work ...
///         ProcessingResult::Success {
///             output_keys: job.input_keys.clone(),
///             logs: "Done!".to_string(),
///             duration_ms: 100,
///         }
///     }
/// }
/// 
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     run_stage_worker(MyStageWorker).await
/// }
/// ```
pub async fn run_stage_worker<W: StageWorker>(worker: W) -> Result<()> {
    let db = FirebaseRtdb::from_env()?;
    let runner = WorkerRunner::new(worker, db);
    let shutdown_token = runner.shutdown_token();
    
    // Spawn signal handler
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
        
        shutdown_token.cancel();
    });
    
    runner.run().await
}

/// Convenience macro for creating the main function of a stage worker.
#[macro_export]
macro_rules! stage_worker_main {
    ($worker:expr) => {
        #[tokio::main]
        async fn main() -> anyhow::Result<()> {
            $crate::microservice::run_stage_worker($worker).await
        }
    };
}

// ============================================================================
// K8S JOB MODE (RUN-ONCE)
// ============================================================================

/// Runs a stage worker in single-job mode for K8s Job execution.
///
/// Instead of polling a queue in a loop, this reads a pre-serialized `QueueItem`
/// from the `IGAIT_JOB_PAYLOAD` environment variable, processes it once, writes
/// the result to Firebase RTDB at `job_results/{job_id}`, and exits.
///
/// The backend orchestrator is responsible for:
/// - Claiming the job and spawning this K8s Job
/// - Reading the `JobResult` from RTDB after completion
/// - Moving the job to the next stage queue (or finalize on failure)
///
/// # Exit codes
/// - 0: Processing completed (check `JobResult.success` for outcome)
/// - 1: Fatal error (couldn't read env, connect to RTDB, etc.)
pub async fn run_stage_job<W: StageWorker>(worker: W) -> Result<()> {
    let stage = worker.stage();
    let stage_num = stage.as_u8();

    println!(
        "[job-mode] Starting {} for stage {} ({})",
        worker.service_name(),
        stage_num,
        stage.name()
    );

    // Read the job payload from environment
    let payload = std::env::var("IGAIT_JOB_PAYLOAD")
        .context("Missing IGAIT_JOB_PAYLOAD environment variable")?;
    let job: QueueItem = serde_json::from_str(&payload)
        .context("Failed to deserialize IGAIT_JOB_PAYLOAD as QueueItem")?;

    println!("[job-mode] Processing job {}", job.job_id);

    // Connect to Firebase RTDB for status updates and result reporting
    let db = FirebaseRtdb::from_env()?;
    let queue_ops = QueueOps::new(db.clone(), format!("job-{}", job.job_id));

    // Update job status to Processing
    if let Ok((user_id, job_index)) = QueueOps::parse_job_id(&job.job_id) {
        let _ = queue_ops.update_job_status(&user_id, &job_index, &JobStatus::processing(stage_num)).await;
        let _ = queue_ops.update_stage_status(&user_id, &job_index, stage_num, &StageStatus::Running).await;
    }

    // Process the job
    let process_result = worker.process(&job).await;

    // Build the JobResult
    let job_result = match &process_result {
        ProcessingResult::Success { output_keys, logs, duration_ms } => {
            println!("[job-mode] Job {} completed successfully in {}ms", job.job_id, duration_ms);

            // Update stage status
            if let Ok((user_id, job_index)) = QueueOps::parse_job_id(&job.job_id) {
                let _ = queue_ops.update_stage_status(&user_id, &job_index, stage_num, &StageStatus::Complete).await;
                let _ = queue_ops.update_stage_logs(&user_id, &job_index, stage_num, logs).await;
            }

            JobResult {
                stage: stage_num,
                success: true,
                output_keys: output_keys.clone(),
                error: None,
                logs: logs.clone(),
                duration_ms: *duration_ms,
                job_id: job.job_id.clone(),
                user_id: job.user_id.clone(),
                metadata: job.metadata.clone(),
                input_keys: job.input_keys.clone(),
                requires_approval: job.requires_approval,
                approved: job.approved,
                epoch: job.epoch,
            }
        }
        ProcessingResult::Failure { error, logs, duration_ms } => {
            eprintln!("[job-mode] Job {} failed after {}ms: {}", job.job_id, duration_ms, error);

            // Update stage status
            if let Ok((user_id, job_index)) = QueueOps::parse_job_id(&job.job_id) {
                let _ = queue_ops.update_stage_status(&user_id, &job_index, stage_num, &StageStatus::Error).await;
                let _ = queue_ops.update_job_status(&user_id, &job_index, &JobStatus::error(logs.clone())).await;
                let _ = queue_ops.update_stage_logs(&user_id, &job_index, stage_num, logs).await;
            }

            JobResult {
                stage: stage_num,
                success: false,
                output_keys: HashMap::new(),
                error: Some(error.clone()),
                logs: logs.clone(),
                duration_ms: *duration_ms,
                job_id: job.job_id.clone(),
                user_id: job.user_id.clone(),
                metadata: job.metadata.clone(),
                input_keys: job.input_keys.clone(),
                requires_approval: job.requires_approval,
                approved: job.approved,
                epoch: job.epoch,
            }
        }
    };

    // Write result to Firebase RTDB for the orchestrator to read
    let result_path = job_result_path(&job.job_id);
    db.set(&result_path, &job_result).await
        .context("Failed to write JobResult to Firebase RTDB")?;

    println!("[job-mode] Result written to RTDB at {}", result_path);

    // Exit with appropriate code
    if job_result.success {
        Ok(())
    } else {
        // Return error so the process exits with code 1
        anyhow::bail!("Job {} failed: {}", job.job_id, job_result.error.unwrap_or_default());
    }
}

