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
    JobMetadata, QueueItem, QueueOps, StageNumber, StageStatus, StoragePaths,
    FirebaseRtdb, queue_item_path,
};
use tracing::{info, instrument, warn};

use crate::helper::lib::{AppError, AppStatePtr, JobStatus, NUM_STAGES};

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
    /// The stage number to restart from (1–7).
    pub stage: u8,
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
    stage = request.stage,
))]
pub async fn rerun_entrypoint(
    current_user: FirebaseUser,
    State(app): State<AppStatePtr>,
    Json(request): Json<RerunRequest>,
) -> Result<Json<RerunResponse>, AppError> {
    let app = app.state;
    let caller_uid = &current_user.user_id;
    let target_uid = &request.user_id;
    let stage = request.stage;
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

    // ── 1. Validate stage number ────────────────────────────────────
    // Stage 7 is the finalize stage and uses FinalizeQueueItem, not QueueItem.
    // We don't support rerunning from stage 7 since it would require a different payload.
    if stage < 1 || stage > 6 {
        return Err(AppError(anyhow!(
            "Invalid stage number {}. Must be between 1 and 6. (Note: stage 7 uses a different queue type and cannot be rerun via this endpoint)",
            stage
        )));
    }

    let target_stage = StageNumber::from_u8(stage)
        .ok_or_else(|| anyhow!("Failed to convert stage number {} to StageNumber", stage))?;

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
        stage,
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
            "Job {} is being re-processed from stage {} ({}).",
            job_id,
            stage,
            target_stage.name()
        ),
        objects_deleted: total_deleted,
    }))
}

/// Executes the mutating portion of a rerun while the caller holds the lease.
/// Factored out so the outer function can release the lease on every path.
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all, fields(job_id = %job_id, stage = stage, lease_id = %lease_id))]
async fn run_rerun_under_lease(
    app: std::sync::Arc<crate::helper::lib::AppState>,
    rtdb: FirebaseRtdb,
    queue_ops: &QueueOps,
    lease_id: &str,
    target_uid: &str,
    job_key: &str,
    job_id: &str,
    stage: u8,
    target_stage: StageNumber,
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

    // ── 5. Delete S3 outputs for stages `stage..=7` ────────────────
    let mut total_deleted: usize = 0;
    for s in stage..=NUM_STAGES {
        let prefix = StoragePaths::stage_dir(job_id, s);
        let deleted = app
            .storage
            .delete_by_prefix(&prefix)
            .await
            .context(format!("Failed to delete S3 objects for stage {}", s))?;
        info!(deleted, prefix = %prefix, "deleted S3 objects for stage");
        total_deleted += deleted;
    }

    // ── 6. Clear stage logs + reset stage statuses for `stage..=7` ──
    for s in stage..=NUM_STAGES {
        let log_path = format!("users/{}/jobs/{}/stage_logs/stage_{}", target_uid, job_key, s);
        rtdb.delete(&log_path)
            .await
            .context(format!("Failed to delete logs for stage {}", s))?;
        let status_path = format!("users/{}/jobs/{}/stage_statuses/stage_{}", target_uid, job_key, s);
        rtdb.set(&status_path, &StageStatus::NotStarted)
            .await
            .context(format!("Failed to reset stage status for stage {}", s))?;
    }

    // ── 7. Build a fresh QueueItem and write it to the target queue ─
    let input_keys = build_input_keys(job_id, stage);

    let mut extra = HashMap::new();
    if stage == 1 {
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
    let status = JobStatus::processing(stage);
    app.db
        .lock()
        .await
        .update_status(target_uid, job_key, status)
        .await
        .context("Failed to update job status")?;

    Ok(total_deleted)
}

/// Builds the `input_keys` map for the target stage.
///
/// The expected keys are **stage-specific**:
/// - Stages 1–3: `front_video`, `side_video` (video-based processing)
/// - Stage 4:    `front_video`, `side_video` (from stage 2 — stage 3 is a passthrough)
/// - Stage 5:    `front_landmarks`, `side_landmarks`, `front_video`, `side_video` (from pose estimation)
/// - Stage 6:    `front_gait_analysis`, `side_gait_analysis` (from cycle detection)
///
/// All keys point at the previous stage's outputs for the given job.
fn build_input_keys(job_id: &str, stage: u8) -> HashMap<String, String> {
    let prev = stage.saturating_sub(1);
    let mut keys = HashMap::new();

    match stage {
        // Stages 1–3 consume video outputs from the previous stage.
        1..=3 => {
            keys.insert(
                "front_video".to_string(),
                StoragePaths::stage_front_video(job_id, prev, "mp4"),
            );
            keys.insert(
                "side_video".to_string(),
                StoragePaths::stage_side_video(job_id, prev, "mp4"),
            );
        }
        // Stage 4 reads from stage 2 because stage 3 is a passthrough
        // that does not produce its own files.
        4 => {
            keys.insert(
                "front_video".to_string(),
                StoragePaths::stage_front_video(job_id, 2, "mp4"),
            );
            keys.insert(
                "side_video".to_string(),
                StoragePaths::stage_side_video(job_id, 2, "mp4"),
            );
        }
        // Stage 5 consumes pose landmarks AND videos from stage 4.
        5 => {
            keys.insert(
                "front_video".to_string(),
                StoragePaths::stage_front_video(job_id, prev, "mp4"),
            );
            keys.insert(
                "side_video".to_string(),
                StoragePaths::stage_side_video(job_id, prev, "mp4"),
            );
            keys.insert(
                "front_landmarks".to_string(),
                format!("jobs/{}/stage_{}/front_landmarks.json", job_id, prev),
            );
            keys.insert(
                "side_landmarks".to_string(),
                format!("jobs/{}/stage_{}/side_landmarks.json", job_id, prev),
            );
        }
        // Stage 6 consumes gait analysis outputs from stage 5.
        6 => {
            keys.insert(
                "front_gait_analysis".to_string(),
                format!("jobs/{}/stage_{}/front_gait_analysis.json", job_id, prev),
            );
            keys.insert(
                "side_gait_analysis".to_string(),
                format!("jobs/{}/stage_{}/side_gait_analysis.json", job_id, prev),
            );
        }
        // Fallback: default to the legacy video keys.
        _ => {
            keys.insert(
                "front_video".to_string(),
                StoragePaths::stage_front_video(job_id, prev, "mp4"),
            );
            keys.insert(
                "side_video".to_string(),
                StoragePaths::stage_side_video(job_id, prev, "mp4"),
            );
        }
    }

    keys
}
