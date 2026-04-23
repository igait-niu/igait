//! Internal endpoint for microservice status updates
//!
//! This endpoint is NOT exposed publicly and should only be called by
//! the stage microservices to update job status.

use anyhow::Context;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::helper::lib::{AppError, AppStatePtr, JobStatus};

/// Request body for status update
#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub user_id: String,
    pub job_key: String,
    pub status: JobStatus,
}

/// Response for status update
#[derive(Debug, Serialize)]
pub struct UpdateStatusResponse {
    pub success: bool,
}

/// Internal endpoint to update job status
///
/// This is called by stage microservices to update the status of a job
/// as it progresses through the pipeline.
pub async fn update_status(
    State(app): State<AppStatePtr>,
    Json(request): Json<UpdateStatusRequest>,
) -> Result<Json<UpdateStatusResponse>, AppError> {
    println!(
        "Received status update for user {} job {}: {:?}",
        request.user_id,
        request.job_key,
        request.status.code()
    );

    // Update the status in the database
    app.state
        .db
        .lock()
        .await
        .update_status(&request.user_id, &request.job_key, request.status)
        .await
        .context("Failed to update job status")?;

    Ok(Json(UpdateStatusResponse { success: true }))
}
