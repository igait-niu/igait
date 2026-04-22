//! Files endpoint for generating presigned S3 URLs.
//!
//! Returns presigned download URLs for all files belonging to a job,
//! grouped by stage. The caller must own the job **or** be an admin.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, Context};
use axum::{
    extract::{Path, State},
    Json,
};
use firebase_auth::FirebaseUser;
use serde::Serialize;

use igait_lib::microservice::StoragePaths;

use crate::helper::lib::{AppError, AppStatePtr};

/// How long presigned URLs stay valid.
const PRESIGN_EXPIRY: Duration = Duration::from_secs(15 * 60); // 15 minutes

/// A single file entry returned to the frontend.
#[derive(Debug, Serialize)]
pub struct FileEntry {
    /// The filename (e.g. "front.mp4", "results.zip")
    pub name: String,
    /// The presigned download URL
    pub url: String,
}

/// Response body: stages → list of files.
#[derive(Debug, Serialize)]
pub struct JobFilesResponse {
    /// Map of "stage_N" → vec of file entries
    pub stages: HashMap<String, Vec<FileEntry>>,
}

/// `GET /api/v1/files/:job_id`
///
/// Generates presigned S3 URLs for every file belonging to the given job.
///
/// # Authorization
/// - The authenticated user must **own** the job (their UID is the prefix
///   of `job_id`) **or** be an administrator.
pub async fn files_entrypoint(
    current_user: FirebaseUser,
    State(app): State<AppStatePtr>,
    Path(job_id): Path<String>,
) -> Result<Json<JobFilesResponse>, AppError> {
    let app = &app.state;
    let caller_uid = &current_user.user_id;

    // ── 1. Authorization ────────────────────────────────────────────
    // job_id format: "{user_id}_{job_index}"
    let owner_uid = job_id
        .rsplit_once('_')
        .map(|(uid, _)| uid)
        .ok_or_else(|| anyhow!("Invalid job ID format: {}", job_id))?;

    if caller_uid != owner_uid {
        // Check if caller is admin (reads only the boolean field, not the full user)
        let is_admin = app
            .db
            .lock()
            .await
            .is_admin(caller_uid)
            .await
            .context("Failed to check admin status")?;

        if !is_admin {
            return Err(AppError(anyhow!("Forbidden: you do not own this job.")));
        }
    }

    // ── 2. List all objects for this job ─────────────────────────────
    let prefix = StoragePaths::job_base(&job_id);
    let files = app
        .storage
        .list_and_presign(&prefix, PRESIGN_EXPIRY)
        .await
        .context("Failed to list and presign job files")?;

    // ── 3. Group by stage ───────────────────────────────────────────
    let mut stages: HashMap<String, Vec<FileEntry>> = HashMap::new();

    for (key, url) in files {
        // key looks like "jobs/{job_id}/stage_N/filename.ext"
        // Strip the "jobs/{job_id}/" prefix to get "stage_N/filename.ext"
        let relative = key.strip_prefix(&prefix).unwrap_or(&key);

        // Split into ("stage_N", "filename.ext")
        let (stage_dir, filename) = match relative.split_once('/') {
            Some((s, f)) => (s.to_string(), f.to_string()),
            None => continue, // skip if structure doesn't match
        };

        stages.entry(stage_dir).or_default().push(FileEntry {
            name: filename,
            url,
        });
    }

    Ok(Json(JobFilesResponse { stages }))
}
