//! Rerun endpoint for re-processing a job from a specific stage.
//!
//! This endpoint allows administrators to rerun any job starting from
//! a given stage. It cleans up S3 outputs from the target stage onward
//! and re-inserts the job into the target stage's queue.
//!
//! Only users with `administrator: true` in the database are authorised.

use std::collections::HashMap;

use axum::{extract::State, Json};
use anyhow::{Context, anyhow};
use firebase_auth::FirebaseUser;
use serde::{Deserialize, Serialize};

use igait_lib::microservice::{
    JobMetadata, QueueItem, QueueOps, StageId, StageStatus, StoragePaths,
    FirebaseRtdb, queue_item_path, stage_logs_path, stage_status_path, stages_from,
};
use tracing::{info, instrument, warn};

use crate::helper::lib::{AppError, AppStatePtr, JobStatus};

/// Lease TTL for the rerun mutation phase. This guards the *handler's*
/// work — cancel K8s Jobs, bump epoch, S3 cleanup, write new queue item —
/// NOT the subsequent multi-hour stage processing. Stage-processing races
/// are handled by the epoch comparison in the orchestrator, not the lease.
///
/// Five minutes is generous for even pathological S3 cleanup fan-out and
/// short enough that a crashed rerun process can't block future admins
/// for long.
const RERUN_LEASE_TTL_MS: u64 = 5 * 60 * 1000;

/// Request body for the rerun endpoint.
#[derive(Debug, Deserialize)]
pub struct RerunRequest {
    /// The UID of the user who owns the job.
    /// Admins must specify this to indicate whose job to rerun.
    pub user_id: String,
    /// The key of the job in the user's job list.
    pub job_key: String,
    /// The stage to restart from (the registry key, e.g. "pose-estimation").
    pub stage: StageId,
}

/// Response body for the rerun endpoint.
#[derive(Debug, Serialize)]
pub struct RerunResponse {
    /// Whether the rerun was successfully initiated.
    pub success: bool,
    /// Human-readable message.
    pub message: String,
    /// Number of S3 objects deleted during cleanup.
    pub objects_deleted: usize,
}

/// Authenticated endpoint to rerun a job from a specific stage.
/// **Admin-only** — the caller must have `administrator: true`.
///
/// # Workflow
/// 1. Verify the caller is an administrator
/// 2. Validate the stage number
/// 3. Fetch the target user's job
/// 4. Delete S3 outputs for stages `stage..=7`
/// 5. Reconstruct a `QueueItem` with the correct input keys
/// 6. Push the item into the target stage's queue in Firebase RTDB
/// 7. Update the job status to "Processing" for the target stage
///
/// # Arguments
/// * `current_user` – The Firebase-authenticated user (extracted from Bearer token).
/// * `app` – The shared application state.
/// * `request` – JSON body with `user_id`, `job_key`, and `stage`.
#[instrument(skip(current_user, app), fields(
    caller_uid = %current_user.user_id,
    target_uid = %request.user_id,
    job_key = %request.job_key,
    stage = %request.stage,
))]
pub async fn rerun_entrypoint(
    current_user: FirebaseUser,
    State(app): State<AppStatePtr>,
    Json(request): Json<RerunRequest>,
) -> Result<Json<RerunResponse>, AppError> {
    let app = app.state;
    let caller_uid = &current_user.user_id;
    let target_uid = &request.user_id;
    let target_stage = request.stage;
    let job_key = &request.job_key;

    // ── 0. Verify the caller is an administrator ────────────────────
    let is_admin = app
        .db
        .lock()
        .await
        .is_admin(caller_uid)
        .await
        .context("Failed to check admin status")?;

    if !is_admin {
        return Err(AppError(anyhow!(
            "Forbidden: only administrators may rerun jobs."
        )));
    }

    // ── 1. Validate the target stage ────────────────────────────────
    // Terminal stages use FinalizeQueueItem (not QueueItem) and aren't
    // reachable through this endpoint — the payload shape is different.
    if target_stage.terminal() {
        return Err(AppError(anyhow!(
            "Cannot rerun the {} stage — it uses a different queue type.",
            target_stage.name()
        )));
    }

    // ── 2. Fetch the job ────────────────────────────────────────────
    let job = app
        .db
        .lock()
        .await
        .get_job(target_uid, job_key)
        .await
        .context("Failed to fetch the job — does it exist?")?;

    let job_id = format!("{}_{}", target_uid, job_key);
    info!(%job_id, "rerun requested by admin");

    let rtdb = FirebaseRtdb::from_env()
        .context("Failed to initialise Firebase RTDB client")?;
    let queue_ops = QueueOps::new(rtdb.clone(), format!("rerun_{}", caller_uid));

    // ── 2a. Acquire job lease ───────────────────────────────────────
    let lease_id = queue_ops
        .try_acquire_job_lease(target_uid, job_key, RERUN_LEASE_TTL_MS)
        .await
        .context("Failed to probe for job lease")?
        .ok_or_else(|| anyhow!(
            "Another rerun is in progress for {}_{} — try again shortly",
            target_uid, job_key
        ))?;

    let result = run_rerun_under_lease(
        app.clone(),
        rtdb.clone(),
        &queue_ops,
        &lease_id,
        target_uid,
        job_key,
        &job_id,
        target_stage,
        job,
    )
    .await;

    if let Err(e) = queue_ops.release_job_lease(target_uid, job_key, &lease_id).await {
        warn!(
            lease_id = %lease_id,
            ttl_ms = RERUN_LEASE_TTL_MS,
            "failed to release rerun lease: {:?} — lease will TTL",
            e
        );
    }

    let total_deleted = result?;

    Ok(Json(RerunResponse {
        success: true,
        message: format!(
            "Job {} is being re-processed from {}.",
            job_id,
            target_stage.name(),
        ),
        objects_deleted: total_deleted,
    }))
}

/// Executes the mutating portion of a rerun while the caller holds the lease.
/// Factored out so the outer function can release the lease on every path.
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all, fields(job_id = %job_id, stage = %target_stage, lease_id = %lease_id))]
async fn run_rerun_under_lease(
    app: std::sync::Arc<crate::helper::lib::AppState>,
    rtdb: FirebaseRtdb,
    queue_ops: &QueueOps,
    lease_id: &str,
    target_uid: &str,
    job_key: &str,
    job_id: &str,
    target_stage: StageId,
    job: crate::helper::lib::Job,
) -> Result<usize, AppError> {
    // ── 3. Cancel any still-running K8s Jobs for this job_id ────────
    // The epoch bump below is the correctness guarantee; this is the
    // proactive best-effort cancel that avoids wasted compute.
    if let Some(orch) = app.orchestrator.read().await.clone() {
        match orch.cancel_in_flight_jobs(job_id).await {
            Ok(n) if n > 0 => info!(cancelled = n, "cancelled in-flight K8s Jobs"),
            Ok(_) => {}
            Err(e) => warn!("cancel_in_flight_jobs failed: {:?}", e),
        }
    }

    // ── 4. Bump job epoch — orchestrator now rejects any straggler
    //       JobResult stamped with the old epoch.
    let new_epoch = queue_ops
        .bump_job_epoch(target_uid, job_key, lease_id)
        .await
        .context("Failed to bump job epoch")?;
    info!(new_epoch, "bumped job epoch");

    // ── 5. Delete S3 outputs for the target stage onward ─────────
    let mut total_deleted: usize = 0;
    for id in stages_from(target_stage) {
        let prefix = StoragePaths::stage_dir(job_id, id);
        let deleted = app
            .storage
            .delete_by_prefix(&prefix)
            .await
            .context(format!("Failed to delete S3 objects for {}", id))?;
        info!(deleted, prefix = %prefix, "deleted S3 objects for stage");
        total_deleted += deleted;
    }

    // ── 6. Clear stage logs + reset statuses for the target stage onward ──
    for id in stages_from(target_stage) {
        let log_path = stage_logs_path(target_uid, job_key, id);
        rtdb.delete(&log_path)
            .await
            .context(format!("Failed to delete logs for {}", id))?;
        let status_path = stage_status_path(target_uid, job_key, id);
        rtdb.set(&status_path, &StageStatus::NotStarted)
            .await
            .context(format!("Failed to reset stage status for {}", id))?;
    }

    // ── 7. Build a fresh QueueItem and write it to the target queue ─
    let input_keys = target_stage.spec().build_input_keys(job_id);

    let mut extra = HashMap::new();
    // Video-edit flags only apply when re-running from the stage that
    // reads the raw upload folder (i.e. media-conversion).
    if target_stage == StageId::new("media-conversion") {
        if let Some(ref video_edit) = job.video_edit {
            if let Ok(val) = serde_json::to_value(video_edit) {
                extra.insert("video_edit".to_string(), val);
            }
        }
    }

    let metadata = JobMetadata {
        email: Some(job.email.clone()),
        age: Some(job.age),
        sex: Some(job.sex.to_string().chars().next().unwrap_or('O')),
        ethnicity: Some(job.ethnicity.to_string()),
        height: Some(job.height.clone()),
        weight: Some(job.weight),
        extra,
    };

    let mut queue_item = QueueItem::new(
        job_id.to_string(),
        target_uid.to_string(),
        input_keys,
        metadata,
        job.requires_approval,
    );
    queue_item.approved = true;

    let path = queue_item_path(target_stage, job_id);
    rtdb.set(&path, &queue_item)
        .await
        .context("Failed to push job to the target stage queue")?;
    info!("pushed job to target stage queue");

    // ── 8. Update user-visible job status to Processing for this stage
    let status = JobStatus::processing(target_stage);
    app.db
        .lock()
        .await
        .update_status(target_uid, job_key, status)
        .await
        .context("Failed to update job status")?;

    Ok(total_deleted)
}

