//! Core types for microservice communication.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

// StageNumber was an enum with one variant per pipeline stage. It has
// been replaced by [`StageId`](super::registry::StageId) — a validated
// newtype over the stage's registry key. See igait-lib/src/microservice/registry.rs.

// ============================================================================
// VIDEO EDIT FLAGS
// ============================================================================

/// Describes a set of spatial/temporal transformations to apply to a single video.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoTransform {
    /// Rotation in degrees: 0, 90, 180, or 270.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<u16>,
    /// Trim start in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trim_start: Option<f64>,
    /// Trim end in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trim_end: Option<f64>,
    /// Crop region X offset (pixels).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_x: Option<u32>,
    /// Crop region Y offset (pixels).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_y: Option<u32>,
    /// Crop region width (pixels).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_width: Option<u32>,
    /// Crop region height (pixels).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_height: Option<u32>,
}

/// Video editing flags for front and side videos.
///
/// Stored on the Job record and forwarded to Stage 1 via `metadata.extra`
/// when re-processing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoEditFlags {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front: Option<VideoTransform>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<VideoTransform>,
}

// ============================================================================
// JOB METADATA
// ============================================================================

/// Metadata passed through the pipeline with each job.
/// 
/// This contains patient information and contact details needed for
/// email notifications and result tracking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobMetadata {
    /// Email address for sending notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    
    /// Patient age
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<i16>,
    
    /// Patient sex ('M', 'F', etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sex: Option<char>,
    
    /// Patient ethnicity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ethnicity: Option<String>,
    
    /// Patient height (as string, e.g., "5'10\"")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<String>,
    
    /// Patient weight in pounds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<i16>,
    
    /// Any additional key-value pairs
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

// ============================================================================
// FIRESTORE JOB DOCUMENT (for backend/frontend)
// ============================================================================

/// Firestore document representing a complete job.
/// This is the source of truth for job state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirestoreJob {
    /// Unique job identifier
    pub job_id: String,
    
    /// User ID who owns this job
    pub user_id: String,
    
    /// When the job was created
    pub created_at: DateTime<Utc>,
    
    /// When the job was last updated
    pub updated_at: DateTime<Utc>,
    
    /// Patient information
    pub patient: PatientInfo,
    
    /// Overall job status
    pub status: FirestoreJobStatus,
    
    /// Current stage id (None = still in upload / not yet claimed by a stage).
    pub current_stage: Option<super::registry::StageId>,
    
    /// Per-stage results
    pub stages: HashMap<String, FirestoreStageResult>,
    
    /// Final result (populated after stage 7)
    pub result: Option<FinalResult>,
    
    /// Email for notifications
    pub email: String,
    
    /// Whether completion email has been sent
    pub email_sent: bool,
}

/// Patient demographic information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatientInfo {
    pub age: i16,
    pub sex: char,
    pub height: String,
    pub weight: i16,
    pub ethnicity: String,
}

/// Overall job status in Firestore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirestoreJobStatus {
    Submitted,
    Processing,
    Completed,
    Failed,
}

/// Per-stage result stored in Firestore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirestoreStageResult {
    pub status: FirestoreStageStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub output_keys: Option<Vec<String>>,
    pub error: Option<String>,
}

/// Stage status in Firestore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirestoreStageStatus {
    Pending,
    Processing,
    Success,
    Failed,
    Skipped,
}

/// Final prediction result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalResult {
    /// ASD probability score (0.0 - 1.0)
    pub score: f64,
    
    /// Classification result
    pub classification: String,
    
    /// Storage key for the results archive
    pub archive_key: String,
}
