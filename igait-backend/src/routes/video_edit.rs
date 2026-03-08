//! Video editing endpoint for applying transformations to job videos.
//!
//! Allows administrators to set rotation, trim, and crop parameters for
//! front and/or side videos. On save the endpoint:
//! 1. Stores the edit flags on the Job record in Firebase RTDB
//! 2. Copies Stage 1 outputs → Stage 0 (so normalised MP4s are the new base)
//! 3. Clears outputs and logs from Stage 1 onward
//! 4. Re-queues the job for Stage 1 with the edit flags in metadata.extra
//!
//! **Admin-only** — the caller must have `administrator: true`.

use std::collections::HashMap;

use axum::{extract::{Path, State}, Json};
use anyhow::{Context, anyhow};
use firebase_auth::FirebaseUser;
use serde::{Deserialize, Serialize};

use igait_lib::microservice::{
    FirebaseRtdb, JobMetadata, QueueItem, StageNumber, StoragePaths, VideoEditFlags,
    queue_item_path,
};

use crate::helper::lib::{AppError, AppStatePtr, JobStatus, NUM_STAGES};

/// Request body for the video-edit endpoint.
#[derive(Debug, Deserialize)]
pub struct VideoEditRequest {
    /// Transformations to apply to the front video.
    pub front: Option<VideoTransformRequest>,
    /// Transformations to apply to the side video.
    pub side: Option<VideoTransformRequest>,
}

/// Per-video transformation parameters.
#[derive(Debug, Deserialize)]
pub struct VideoTransformRequest {
    /// Rotation in degrees: 0, 90, 180, or 270.
    pub rotation: Option<u16>,
    /// Trim start in seconds.
    pub trim_start: Option<f64>,
    /// Trim end in seconds.
    pub trim_end: Option<f64>,
    /// Crop region X offset (pixels).
    pub crop_x: Option<u32>,
    /// Crop region Y offset (pixels).
    pub crop_y: Option<u32>,
    /// Crop width (pixels).
    pub crop_width: Option<u32>,
    /// Crop height (pixels).
    pub crop_height: Option<u32>,
}

/// Response body for the video-edit endpoint.
#[derive(Debug, Serialize)]
pub struct VideoEditResponse {
    pub success: bool,
    pub message: String,
    pub objects_deleted: usize,
}

/// `POST /api/v1/video-edit/:job_id`
///
/// Stores video edit flags, copies Stage 1 → Stage 0, and re-runs from Stage 1.
pub async fn video_edit_entrypoint(
    current_user: FirebaseUser,
    State(app): State<AppStatePtr>,
    Path(job_id): Path<String>,
    Json(request): Json<VideoEditRequest>,
) -> Result<Json<VideoEditResponse>, AppError> {
    let app = &app.state;
    let caller_uid = &current_user.user_id;

    // ── 0. Verify admin ────────────────────────────────────────────
    let caller = app
        .db
        .lock()
        .await
        .get_user(caller_uid)
        .await
        .context("Failed to look up caller in the database")?;

    if !caller.administrator {
        return Err(AppError(anyhow!(
            "Forbidden: only administrators may edit videos."
        )));
    }

    // ── 1. Parse job_id → (user_id, job_index) ─────────────────────
    let last_underscore = job_id
        .rfind('_')
        .ok_or_else(|| anyhow!("Invalid job_id format. Expected userId_jobIndex"))?;
    let target_uid = &job_id[..last_underscore];
    let job_index: usize = job_id[last_underscore + 1..]
        .parse()
        .context("Invalid job index in job_id")?;

    // ── 2. Validate request ─────────────────────────────────────────
    if let Some(ref front) = request.front {
        validate_transform(front, "front")?;
    }
    if let Some(ref side) = request.side {
        validate_transform(side, "side")?;
    }

    // ── 3. Build VideoEditFlags ─────────────────────────────────────
    let video_edit = VideoEditFlags {
        front: request.front.as_ref().map(to_lib_transform),
        side: request.side.as_ref().map(to_lib_transform),
    };

    // ── 4. Store flags on the Job record ────────────────────────────
    let job = {
        let mut db = app.db.lock().await;
        let mut job = db
            .get_job(target_uid, job_index)
            .await
            .context("Failed to fetch job — does it exist?")?;
        job.video_edit = Some(video_edit.clone());
        // We need to write the whole user record back (existing pattern)
        let mut user = db.get_user(target_uid).await.context("Failed to get user")?;
        user.jobs[job_index] = job.clone();
        // Write user back via RTDB
        drop(db);
        let rtdb = FirebaseRtdb::from_env().context("Failed to init RTDB")?;
        rtdb.set(&format!("users/{}", target_uid), &user)
            .await
            .context("Failed to write updated user record")?;
        job
    };

    println!(
        "Video edit requested by admin {}: job={}, front={}, side={}",
        caller_uid,
        job_id,
        request.front.is_some(),
        request.side.is_some()
    );

    // ── 5. Copy Stage 1 outputs → Stage 0 ──────────────────────────
    let stage1_front = StoragePaths::stage_front_video(&job_id, 1, "mp4");
    let stage0_front = StoragePaths::stage_front_video(&job_id, 0, "mp4");
    let stage1_side = StoragePaths::stage_side_video(&job_id, 1, "mp4");
    let stage0_side = StoragePaths::stage_side_video(&job_id, 0, "mp4");

    // Download from stage 1, upload to stage 0
    let front_data = app
        .storage
        .download(&stage1_front)
        .await
        .context("Failed to download front video from Stage 1")?;
    app.storage
        .upload(&stage0_front, front_data, Some("video/mp4"))
        .await
        .context("Failed to upload front video to Stage 0")?;

    let side_data = app
        .storage
        .download(&stage1_side)
        .await
        .context("Failed to download side video from Stage 1")?;
    app.storage
        .upload(&stage0_side, side_data, Some("video/mp4"))
        .await
        .context("Failed to upload side video to Stage 0")?;

    println!("Copied Stage 1 outputs → Stage 0 for job {}", job_id);

    // ── 6. Delete S3 outputs for stages 1..=7 ──────────────────────
    let mut total_deleted: usize = 0;
    for s in 1..=NUM_STAGES {
        let prefix = StoragePaths::stage_dir(&job_id, s);
        let deleted = app
            .storage
            .delete_by_prefix(&prefix)
            .await
            .context(format!("Failed to delete S3 objects for stage {}", s))?;
        total_deleted += deleted;
    }

    // ── 6b. Clear stage logs for stages 1..=7 ──────────────────────
    let rtdb = FirebaseRtdb::from_env().context("Failed to init RTDB for log cleanup")?;
    for s in 1..=NUM_STAGES {
        let log_path = format!(
            "users/{}/jobs/{}/stage_logs/stage_{}",
            target_uid, job_index, s
        );
        rtdb.delete(&log_path)
            .await
            .context(format!("Failed to delete logs for stage {}", s))?;
    }

    // ── 7. Build QueueItem with video_edit in metadata.extra ────────
    let mut extra = HashMap::new();
    extra.insert(
        "video_edit".to_string(),
        serde_json::to_value(&video_edit)
            .context("Failed to serialise video_edit for metadata")?,
    );

    let metadata = JobMetadata {
        email: Some(job.email.clone()),
        age: Some(job.age),
        sex: Some(job.sex.to_string().chars().next().unwrap_or('O')),
        ethnicity: Some(job.ethnicity.to_string()),
        height: Some(job.height.clone()),
        weight: Some(job.weight),
        extra,
    };

    let input_keys = {
        let mut keys = HashMap::new();
        keys.insert("front_video".to_string(), stage0_front);
        keys.insert("side_video".to_string(), stage0_side);
        keys
    };

    let mut queue_item = QueueItem::new(
        job_id.clone(),
        target_uid.to_string(),
        input_keys,
        metadata,
        job.requires_approval,
    );
    queue_item.approved = true; // Admin-initiated

    // ── 8. Push into Stage 1 queue ──────────────────────────────────
    let target_stage = StageNumber::Stage1MediaConversion;
    let path = queue_item_path(target_stage, &job_id);
    rtdb.set(&path, &queue_item)
        .await
        .context("Failed to push job to Stage 1 queue")?;

    // ── 9. Update job status ────────────────────────────────────────
    let status = JobStatus::processing(1);
    app.db
        .lock()
        .await
        .update_status(target_uid, job_index, status)
        .await
        .context("Failed to update job status")?;

    println!("Job {} re-queued for Stage 1 with video edits", job_id);

    Ok(Json(VideoEditResponse {
        success: true,
        message: format!(
            "Video edits saved. Job {} is being re-processed from Stage 1.",
            job_id
        ),
        objects_deleted: total_deleted,
    }))
}

/// Validates a per-video transform request.
fn validate_transform(t: &VideoTransformRequest, label: &str) -> Result<(), AppError> {
    if let Some(r) = t.rotation {
        if r != 0 && r != 90 && r != 180 && r != 270 {
            return Err(AppError(anyhow!(
                "Invalid {} rotation: {}. Must be 0, 90, 180, or 270.",
                label,
                r
            )));
        }
    }
    if let Some(start) = t.trim_start {
        if start < 0.0 {
            return Err(AppError(anyhow!(
                "Invalid {} trim_start: must be >= 0",
                label
            )));
        }
    }
    if let (Some(start), Some(end)) = (t.trim_start, t.trim_end) {
        if end <= start {
            return Err(AppError(anyhow!(
                "Invalid {} trim: end ({}) must be greater than start ({})",
                label,
                end,
                start
            )));
        }
    }
    if let (Some(w), Some(h)) = (t.crop_width, t.crop_height) {
        if w == 0 || h == 0 {
            return Err(AppError(anyhow!(
                "Invalid {} crop: width and height must be > 0",
                label
            )));
        }
    }
    Ok(())
}

/// Converts our request-local transform into the igait-lib type.
fn to_lib_transform(t: &VideoTransformRequest) -> igait_lib::microservice::VideoTransform {
    igait_lib::microservice::VideoTransform {
        rotation: t.rotation,
        trim_start: t.trim_start,
        trim_end: t.trim_end,
        crop_x: t.crop_x,
        crop_y: t.crop_y,
        crop_width: t.crop_width,
        crop_height: t.crop_height,
    }
}
