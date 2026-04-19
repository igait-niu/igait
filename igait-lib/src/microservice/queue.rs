//! Queue types and operations for the distributed job processing pipeline.
//!
//! This module defines the data structures used for Firebase Realtime Database
//! queue-based job processing with claim-based distributed locking.

use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

use crate::microservice::{JobMetadata, StageId};

/// Default timeout for claimed jobs (5 minutes in milliseconds).
/// If a worker claims a job but doesn't update the heartbeat within this time,
/// the job becomes available for other workers to claim.
///
/// The heartbeat interval (30s) refreshes `claimed_at` regularly, so a live
/// worker will never hit this timeout. If a worker is OOMKilled or otherwise
/// ungracefully terminated, the heartbeat stops and the job becomes
/// re-claimable after this duration.
pub const CLAIM_TIMEOUT_MS: u64 = 5 * 60 * 1000;

/// Interval for heartbeat updates during long-running jobs (30 seconds).
/// Must be well below `CLAIM_TIMEOUT_MS` to prevent false expirations.
pub const HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// A timestamp field that uses Firebase's server-side clock.
///
/// Writing `Sentinel` emits `{".sv":"timestamp"}`, which Firebase replaces with
/// its own wall-clock time at commit. Reads deserialize the resolved `u64`.
/// This replaces client-side `now_ms()` on write paths so that timestamps
/// from globally-distributed replicas do not race due to clock skew.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerTimestamp {
    Sentinel,
    Value(u64),
}

impl ServerTimestamp {
    pub fn as_ms(self) -> u64 {
        match self {
            Self::Value(v) => v,
            Self::Sentinel => 0,
        }
    }
}

impl Serialize for ServerTimestamp {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Sentinel => {
                let mut m = s.serialize_map(Some(1))?;
                m.serialize_entry(".sv", "timestamp")?;
                m.end()
            }
            Self::Value(v) => v.serialize(s),
        }
    }
}

impl<'de> Deserialize<'de> for ServerTimestamp {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        match v {
            serde_json::Value::Number(n) => n
                .as_u64()
                .map(Self::Value)
                .ok_or_else(|| serde::de::Error::custom("ServerTimestamp out of range")),
            serde_json::Value::Object(_) => Ok(Self::Sentinel),
            other => Err(serde::de::Error::custom(format!(
                "ServerTimestamp: expected number or sentinel object, got {other}"
            ))),
        }
    }
}

// ============================================================================
// QUEUE ITEM TYPES
// ============================================================================

/// Configuration for a processing queue.
///
/// Stored at `queue_config/stage_{n}` in Firebase RTDB.
/// If `requires_approval` is true, all jobs in this queue
/// must be explicitly approved before workers can claim them.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueueConfig {
    /// Whether this queue globally requires manual approval
    /// before workers can pick up jobs.
    #[serde(default)]
    pub requires_approval: bool,
}

/// An item in a stage processing queue.
///
/// This represents a job waiting to be processed by a specific stage.
/// Workers claim items using Firebase transactions to prevent duplicate processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    /// The job ID (format: "{user_id}_{job_index}")
    pub job_id: String,

    /// User ID who owns this job
    pub user_id: String,

    /// When the item was added to this queue (Unix timestamp ms)
    pub enqueued_at: u64,

    /// Worker ID that claimed this job (None if unclaimed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,

    /// When the job was claimed (Unix timestamp ms, for timeout detection)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_at: Option<u64>,

    /// Storage keys for input files from previous stage
    #[serde(default)]
    pub input_keys: HashMap<String, String>,

    /// Job metadata (age, sex, etc. - needed for later stages)
    pub metadata: JobMetadata,

    /// Whether this specific job requires manual approval before processing.
    /// Set by the user at submission time.
    #[serde(default)]
    pub requires_approval: bool,

    /// Whether this job has been approved for processing.
    /// If neither the job's nor the queue's `requires_approval` flag is set,
    /// this field is ignored and the job can be picked up freely.
    #[serde(default)]
    pub approved: bool,

    /// Monotonic fencing token. Incremented on each successful claim; writes
    /// from any worker holding a stale epoch must be rejected.
    #[serde(default)]
    pub epoch: u64,
}

impl QueueItem {
    /// Creates a new unclaimed queue item.
    pub fn new(
        job_id: String,
        user_id: String,
        input_keys: HashMap<String, String>,
        metadata: JobMetadata,
        requires_approval: bool,
    ) -> Self {
        Self {
            job_id,
            user_id,
            enqueued_at: now_ms(),
            claimed_by: None,
            claimed_at: None,
            input_keys,
            metadata,
            requires_approval,
            approved: false,
            epoch: 0,
        }
    }

    /// Checks if this item is available for claiming.
    ///
    /// An item is available if:
    /// - It has never been claimed, OR
    /// - It was claimed but the claim has timed out
    ///
    /// Note: This does NOT check approval status — that is checked
    /// separately by the worker via `is_approved_for_processing`.
    pub fn is_available(&self) -> bool {
        match self.claimed_at {
            None => true, // Never claimed
            Some(claimed_time) => {
                // Check if claim has timed out
                now_ms().saturating_sub(claimed_time) > CLAIM_TIMEOUT_MS
            }
        }
    }

    /// Checks whether this item is approved for processing.
    ///
    /// A job is approved if:
    /// - `approved` is true (explicitly approved by an admin), OR
    /// - The job's own `requires_approval` flag is false AND the
    ///   queue-level `queue_requires_approval` flag is also false.
    pub fn is_approved_for_processing(&self, queue_requires_approval: bool) -> bool {
        if self.approved {
            return true;
        }
        // Not explicitly approved — only allow if neither flag is set
        !self.requires_approval && !queue_requires_approval
    }

    /// Claims this item for a worker. Increments the fencing epoch.
    pub fn claim(&self, worker_id: &str) -> Self {
        Self {
            claimed_by: Some(worker_id.to_string()),
            claimed_at: Some(now_ms()),
            epoch: self.epoch + 1,
            ..self.clone()
        }
    }

    /// Updates the heartbeat timestamp to prevent timeout during long operations.
    pub fn heartbeat(&self) -> Self {
        Self {
            claimed_at: Some(now_ms()),
            ..self.clone()
        }
    }

    /// Gets the input storage key for the front video from the input_keys.
    /// Falls back to constructing a path from `job_id` / current stage key.
    pub fn input_front_video(&self, stage: StageId) -> String {
        self.input_keys
            .get("front_video")
            .cloned()
            .unwrap_or_else(|| format!("jobs/{}/{}/front.mp4", self.job_id, stage.key()))
    }

    /// Gets the input storage key for the side video from the input_keys.
    /// Falls back to constructing a path from `job_id` / current stage key.
    pub fn input_side_video(&self, stage: StageId) -> String {
        self.input_keys
            .get("side_video")
            .cloned()
            .unwrap_or_else(|| format!("jobs/{}/{}/side.mp4", self.job_id, stage.key()))
    }

    /// Gets the output storage key for the front video for a given stage.
    pub fn output_front_video(&self, stage: StageId) -> String {
        format!("jobs/{}/{}/front.mp4", self.job_id, stage.key())
    }

    /// Gets the output storage key for the side video for a given stage.
    pub fn output_side_video(&self, stage: StageId) -> String {
        format!("jobs/{}/{}/side.mp4", self.job_id, stage.key())
    }
}

/// An item in the finalize queue.
///
/// This queue receives jobs that have either:
/// - Successfully completed all stages (success = true)
/// - Failed at some stage (success = false)
///
/// The finalize worker sends appropriate emails and updates the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizeQueueItem {
    /// The job ID (format: "{user_id}_{job_index}")
    pub job_id: String,

    /// User ID who owns this job
    pub user_id: String,

    /// When the item was added to this queue (Unix timestamp ms)
    pub enqueued_at: u64,

    /// Worker ID that claimed this job (None if unclaimed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,

    /// When the job was claimed (Unix timestamp ms, for timeout detection)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_at: Option<u64>,

    /// Whether the pipeline completed successfully
    pub success: bool,

    /// If failed, which stage failed (1-6)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_at_stage: Option<StageId>,

    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Error logs if failed (for debugging)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_logs: Option<String>,

    /// Final output keys (if successful - includes prediction results)
    #[serde(default)]
    pub output_keys: HashMap<String, String>,

    /// Job metadata for email content
    pub metadata: JobMetadata,

    #[serde(default)]
    pub requires_approval: bool,

    #[serde(default)]
    pub approved: bool,

    /// Monotonic fencing token. Incremented on each successful claim; writes
    /// from any worker holding a stale epoch must be rejected.
    #[serde(default)]
    pub epoch: u64,
}

impl FinalizeQueueItem {
    /// Creates a success finalize item (pipeline completed successfully).
    pub fn success(
        job_id: String,
        user_id: String,
        output_keys: HashMap<String, String>,
        metadata: JobMetadata,
        requires_approval: bool,
        approved: bool,
    ) -> Self {
        Self {
            job_id,
            user_id,
            enqueued_at: now_ms(),
            claimed_by: None,
            claimed_at: None,
            success: true,
            failed_at_stage: None,
            error: None,
            error_logs: None,
            output_keys,
            metadata,
            requires_approval,
            approved,
            epoch: 0,
        }
    }

    /// Creates a failure finalize item (stage failed). Failures bypass approval.
    pub fn failure(
        job_id: String,
        user_id: String,
        failed_at_stage: StageId,
        error: String,
        error_logs: Option<String>,
        metadata: JobMetadata,
    ) -> Self {
        Self {
            job_id,
            user_id,
            enqueued_at: now_ms(),
            claimed_by: None,
            claimed_at: None,
            success: false,
            failed_at_stage: Some(failed_at_stage),
            error: Some(error),
            error_logs,
            output_keys: HashMap::new(),
            metadata,
            requires_approval: false,
            approved: true,
            epoch: 0,
        }
    }

    pub fn is_approved_for_processing(&self, queue_requires_approval: bool) -> bool {
        if self.approved {
            return true;
        }
        !self.requires_approval && !queue_requires_approval
    }

    /// Checks if this item is available for claiming.
    pub fn is_available(&self) -> bool {
        match self.claimed_at {
            None => true,
            Some(claimed_time) => now_ms().saturating_sub(claimed_time) > CLAIM_TIMEOUT_MS,
        }
    }

    /// Claims this item for a worker. Increments the fencing epoch.
    pub fn claim(&self, worker_id: &str) -> Self {
        Self {
            claimed_by: Some(worker_id.to_string()),
            claimed_at: Some(now_ms()),
            epoch: self.epoch + 1,
            ..self.clone()
        }
    }
}

// ============================================================================
// QUEUE PATH HELPERS
// ============================================================================

/// Returns the Firebase RTDB path for a stage's queue.
///
/// Queue paths are: `queues/stage_{n}` for stages 1-6, `queues/finalize` for stage 7.
pub fn queue_path(stage: StageId) -> String {
    format!("queues/{}", stage.key())
}

/// Returns the Firebase RTDB path for a queue's configuration:
/// `queue_config/{stage.key}`.
pub fn queue_config_path(stage: StageId) -> String {
    format!("queue_config/{}", stage.key())
}

/// Returns the Firebase RTDB path for a specific job in a queue.
pub fn queue_item_path(stage: StageId, job_id: &str) -> String {
    // Replace characters that Firebase doesn't allow in keys
    let safe_job_id = job_id.replace('.', "_").replace('/', "_");
    format!("{}/{}", queue_path(stage), safe_job_id)
}

/// Returns the Firebase RTDB path for a job's result (written by K8s Job, read by orchestrator).
///
/// Result paths are: `job_results/{safe_job_id}`
pub fn job_result_path(job_id: &str) -> String {
    let safe_job_id = job_id.replace('.', "_").replace('/', "_");
    format!("job_results/{}", safe_job_id)
}

/// Returns the path of the one-shot email-sent marker for a given job.
pub fn result_notification_path(user_id: &str, job_key: &str) -> String {
    format!("users/{}/jobs/{}/notifications/result", user_id, job_key)
}

pub fn stage_status_path(user_id: &str, job_key: &str, stage: StageId) -> String {
    format!(
        "users/{}/jobs/{}/stage_statuses/{}",
        user_id, job_key, stage.key()
    )
}

pub fn stage_logs_path(user_id: &str, job_key: &str, stage: StageId) -> String {
    format!(
        "users/{}/jobs/{}/stage_logs/{}",
        user_id, job_key, stage.key()
    )
}

pub fn job_status_path(user_id: &str, job_key: &str) -> String {
    format!("users/{}/jobs/{}/status", user_id, job_key)
}

/// Returns the path of a job's coordination record (lease + generation epoch).
pub fn job_coordination_path(user_id: &str, job_key: &str) -> String {
    format!("users/{}/jobs/{}/coordination", user_id, job_key)
}

/// Job-level coordination state used to serialize reruns and reject zombie
/// K8s Jobs whose work was invalidated by a rerun.
///
/// * `epoch` — monotonic "job generation". Bumped on every rerun. K8s Jobs
///   stamp their current epoch on every `JobResult`; the orchestrator
///   discards results whose epoch is behind the live value.
/// * `lease_holder` / `lease_expires_at` — short-lived advisory lease used
///   by the rerun endpoint to serialize itself against concurrent reruns
///   (and against future self-healing flows that need the same guarantee).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobCoordination {
    #[serde(default)]
    pub epoch: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_holder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_expires_at: Option<u64>,
}

/// Persisted marker written via CAS insert-if-not-exists right before an outbound
/// result email is sent. If the insert races with a sibling worker, the loser
/// sees `PreconditionFailed` and skips the send — preventing duplicate emails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailNotificationMarker {
    pub sent_at: u64,
    pub sent_by: String,
    pub outcome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dedup_id: Option<String>,
}

/// The result a K8s Job writes to Firebase RTDB after processing.
///
/// The backend orchestrator reads this to determine whether the stage
/// succeeded and what the output keys are, then handles stage transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    /// Which stage produced this result.
    pub stage: StageId,
    /// Whether processing succeeded.
    pub success: bool,
    /// Output storage keys (empty on failure).
    #[serde(default)]
    pub output_keys: HashMap<String, String>,
    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Processing logs.
    pub logs: String,
    /// Processing duration in milliseconds.
    pub duration_ms: u64,
    /// The original job ID (for correlation).
    pub job_id: String,
    /// The original user ID.
    pub user_id: String,
    /// The original job metadata (needed for stage transitions).
    #[serde(default)]
    pub metadata: JobMetadata,
    /// The original input keys (needed for reconstructing QueueItem).
    #[serde(default)]
    pub input_keys: HashMap<String, String>,
    /// Whether the original job required approval.
    #[serde(default)]
    pub requires_approval: bool,
    /// Whether the original job was approved.
    #[serde(default)]
    pub approved: bool,
    /// Epoch the K8s Job claimed under. Orchestrator rejects results whose
    /// epoch no longer matches the queue item — indicates a zombie completion.
    #[serde(default)]
    pub epoch: u64,
    /// Job-generation epoch stamped from JobCoordination at K8s Job creation
    /// time. Orchestrator rejects results whose job_epoch is less than the
    /// live value — a rerun bumped the generation while this K8s Job was
    /// running, so its output is no longer valid.
    #[serde(default)]
    pub job_epoch: u64,
    /// Orchestrator instance that took this result for processing.
    /// Set via CAS; a second replica seeing this field set will skip the entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taken_by: Option<String>,
    /// When the result was taken (ms). If older than the take timeout,
    /// another replica can reclaim it — recovers results whose taker crashed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taken_at: Option<u64>,
}

/// Timeout after which a taken-but-unprocessed JobResult can be reclaimed (30s).
/// Orchestrator's transition work should complete in milliseconds, so this is
/// orders of magnitude larger than normal — only trips on true crashes.
pub const JOB_RESULT_TAKE_TIMEOUT_MS: u64 = 30_000;

/// Returns the next stage in the pipeline. The terminal stage maps
/// to itself (sink behavior).
pub fn next_stage(current: StageId) -> StageId {
    match super::registry::stage_after(current.key()) {
        Some(next) => StageId::new(next.key),
        None => current,
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Returns the current Unix timestamp in milliseconds.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Generates a unique worker ID for this instance.
///
/// Format: `{service_name}_{hostname}_{pid}_{random}`
pub fn generate_worker_id(service_name: &str) -> String {
    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("POD_NAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let pid = std::process::id();
    let random: u32 = rand_u32();

    format!("{}_{}_{}_{:08x}", service_name, hostname, pid, random)
}

/// Simple pseudo-random u32 (not cryptographically secure, just for IDs).
fn rand_u32() -> u32 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    hasher.write_u64(now_ms());
    hasher.write_u32(std::process::id());
    hasher.finish() as u32
}

// ============================================================================
// QUEUE OPERATION RESULTS
// ============================================================================

/// Result of attempting to claim a job from a queue.
#[derive(Debug, Clone)]
pub enum ClaimResult<T> {
    /// Successfully claimed a job
    Claimed(T),
    /// No jobs available in the queue
    QueueEmpty,
    /// Jobs exist but all are claimed by other workers
    AllClaimed,
    /// Error occurred during claim operation
    Error(String),
}

/// Result of a stage processing operation.
#[derive(Debug, Clone)]
pub enum ProcessingResult {
    /// Stage completed successfully with output keys
    Success {
        output_keys: HashMap<String, String>,
        logs: String,
        duration_ms: u64,
    },
    /// Stage failed with an error
    Failure {
        error: String,
        logs: String,
        duration_ms: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_paths() {
        // Every queue lives at queues/{key}; no numbered detour or
        // finalize special-case. Guards against someone re-introducing
        // the legacy stage_N convention.
        assert_eq!(
            queue_path(StageId::new("media-conversion")),
            "queues/media-conversion"
        );
        assert_eq!(queue_path(StageId::new("prediction")), "queues/prediction");
        assert_eq!(queue_path(StageId::new("finalize")), "queues/finalize");
    }

    #[test]
    fn test_next_stage() {
        assert_eq!(
            next_stage(StageId::new("media-conversion")),
            StageId::new("pose-estimation")
        );
        assert_eq!(
            next_stage(StageId::new("prediction")),
            StageId::new("finalize")
        );
        assert_eq!(
            next_stage(StageId::new("finalize")),
            StageId::new("finalize")
        );
    }

    #[test]
    fn test_queue_item_availability() {
        let item = QueueItem::new(
            "test_job".to_string(),
            "test_user".to_string(),
            HashMap::new(),
            JobMetadata::default(),
            false,
        );

        assert!(item.is_available());

        let claimed = item.claim("worker_1");
        assert!(!claimed.is_available()); // Just claimed, not timed out yet
    }

    #[test]
    fn test_server_timestamp_serializes_sentinel() {
        let json = serde_json::to_string(&ServerTimestamp::Sentinel).unwrap();
        assert_eq!(json, r#"{".sv":"timestamp"}"#);
    }

    #[test]
    fn test_server_timestamp_serializes_value() {
        let json = serde_json::to_string(&ServerTimestamp::Value(1_700_000_000_000)).unwrap();
        assert_eq!(json, "1700000000000");
    }

    #[test]
    fn test_server_timestamp_deserializes_number() {
        let v: ServerTimestamp = serde_json::from_str("1700000000000").unwrap();
        assert_eq!(v, ServerTimestamp::Value(1_700_000_000_000));
        assert_eq!(v.as_ms(), 1_700_000_000_000);
    }

    #[test]
    fn test_server_timestamp_deserializes_sentinel_object() {
        let v: ServerTimestamp = serde_json::from_str(r#"{".sv":"timestamp"}"#).unwrap();
        assert_eq!(v, ServerTimestamp::Sentinel);
        assert_eq!(v.as_ms(), 0);
    }

    #[test]
    fn test_approval_logic() {
        // Job that doesn't require approval
        let item = QueueItem::new(
            "job_1".to_string(),
            "user_1".to_string(),
            HashMap::new(),
            JobMetadata::default(),
            false,
        );
        // No approval required anywhere → approved for processing
        assert!(item.is_approved_for_processing(false));
        // Queue requires approval but job doesn't request it and isn't approved → blocked
        assert!(!item.is_approved_for_processing(true));

        // Job that requires approval
        let item2 = QueueItem::new(
            "job_2".to_string(),
            "user_2".to_string(),
            HashMap::new(),
            JobMetadata::default(),
            true,
        );
        // Not approved → blocked regardless of queue config
        assert!(!item2.is_approved_for_processing(false));
        assert!(!item2.is_approved_for_processing(true));

        // Explicitly approved item always passes
        let mut item3 = item2.clone();
        item3.approved = true;
        assert!(item3.is_approved_for_processing(false));
        assert!(item3.is_approved_for_processing(true));
    }
}
