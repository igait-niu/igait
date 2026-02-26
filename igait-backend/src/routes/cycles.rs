//! Cycle editing endpoint for updating gait cycle detection results.
//!
//! Allows administrators to view and modify the `gait_cycles` array in
//! the Stage 5 output JSON files (front/side gait analysis) stored in S3.
//! This is useful for manually correcting cycle boundaries before
//! re-running prediction from Stage 6.
//!
//! **Admin-only** — the caller must have `administrator: true`.

use axum::{extract::{Path, State}, Json};
use anyhow::{Context, anyhow};
use firebase_auth::FirebaseUser;
use serde::{Deserialize, Serialize};

use igait_lib::microservice::StoragePaths;

use crate::helper::lib::{AppError, AppStatePtr};

/// A single gait cycle with start/end frame numbers and side (L/R).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaitCycle {
    pub start: u64,
    pub end: u64,
    pub side: String,
}

/// Request body for updating cycles.
#[derive(Debug, Deserialize)]
pub struct UpdateCyclesRequest {
    /// The JSON filename to update, e.g. "front_gait_analysis.json" or "side_gait_analysis.json"
    pub file_name: String,
    /// The new gait cycles array to replace the existing one
    pub gait_cycles: Vec<GaitCycle>,
}

/// Response body after a successful update.
#[derive(Debug, Serialize)]
pub struct UpdateCyclesResponse {
    pub success: bool,
    pub message: String,
}

/// `PUT /api/v1/cycles/:job_id`
///
/// Updates the `gait_cycles` array in the specified Stage 5 JSON file.
///
/// # Authorization
/// - The caller must be an administrator.
///
/// # Workflow
/// 1. Verify admin privileges
/// 2. Validate the file name (must be front or side gait analysis)
/// 3. Download the existing JSON from S3
/// 4. Replace the `gait_cycles` field with the new data
/// 5. Re-upload to S3
pub async fn cycles_entrypoint(
    current_user: FirebaseUser,
    State(app): State<AppStatePtr>,
    Path(job_id): Path<String>,
    Json(request): Json<UpdateCyclesRequest>,
) -> Result<Json<UpdateCyclesResponse>, AppError> {
    let app = &app.state;
    let caller_uid = &current_user.user_id;

    // ── 1. Verify the caller is an administrator ────────────────────
    let caller = app
        .db
        .lock()
        .await
        .get_user(caller_uid)
        .await
        .context("Failed to look up caller in the database")?;

    if !caller.administrator {
        return Err(AppError(anyhow!(
            "Forbidden: only administrators may edit cycle data."
        )));
    }

    // ── 2. Validate file name ───────────────────────────────────────
    let valid_files = ["front_gait_analysis.json", "side_gait_analysis.json"];
    if !valid_files.contains(&request.file_name.as_str()) {
        return Err(AppError(anyhow!(
            "Invalid file name '{}'. Must be one of: {:?}",
            request.file_name,
            valid_files
        )));
    }

    // Validate cycle data
    for (i, cycle) in request.gait_cycles.iter().enumerate() {
        if cycle.start >= cycle.end {
            return Err(AppError(anyhow!(
                "Invalid cycle at index {}: start ({}) must be less than end ({})",
                i, cycle.start, cycle.end
            )));
        }
        if cycle.side != "L" && cycle.side != "R" {
            return Err(AppError(anyhow!(
                "Invalid cycle at index {}: side must be 'L' or 'R', got '{}'",
                i, cycle.side
            )));
        }
    }

    // ── 3. Build the S3 key and download existing JSON ──────────────
    let s3_key = format!(
        "{}{}",
        StoragePaths::stage_dir(&job_id, 5),
        request.file_name
    );

    println!(
        "Admin {} updating cycles in {} for job {}",
        caller_uid, request.file_name, job_id
    );

    let existing_bytes = app
        .storage
        .download(&s3_key)
        .await
        .context(format!("Failed to download {} from S3", s3_key))?;

    let mut json_value: serde_json::Value = serde_json::from_slice(&existing_bytes)
        .context("Failed to parse existing JSON file")?;

    // ── 4. Replace gait_cycles ──────────────────────────────────────
    let new_cycles = serde_json::to_value(&request.gait_cycles)
        .context("Failed to serialize new gait cycles")?;

    json_value["gait_cycles"] = new_cycles;

    // ── 5. Re-upload to S3 ──────────────────────────────────────────
    let updated_bytes = serde_json::to_vec(&json_value)
        .context("Failed to serialize updated JSON")?;

    app.storage
        .upload(&s3_key, updated_bytes, Some("application/json"))
        .await
        .context("Failed to upload updated JSON to S3")?;

    println!(
        "Successfully updated {} cycles in {} for job {}",
        request.gait_cycles.len(),
        request.file_name,
        job_id
    );

    Ok(Json(UpdateCyclesResponse {
        success: true,
        message: format!(
            "Updated {} with {} gait cycles.",
            request.file_name,
            request.gait_cycles.len()
        ),
    }))
}
