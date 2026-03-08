//! Core types for microservice communication.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Identifies which stage a microservice handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageNumber {
    Stage1MediaConversion,
    Stage2ValidityCheck,
    Stage3Reframing,
    Stage4PoseEstimation,
    Stage5CycleDetection,
    Stage6Prediction,
    Stage7Finalize,
}

impl StageNumber {
    /// Returns the numeric stage number (1-7).
    pub fn as_u8(&self) -> u8 {
        match self {
            Self::Stage1MediaConversion => 1,
            Self::Stage2ValidityCheck => 2,
            Self::Stage3Reframing => 3,
            Self::Stage4PoseEstimation => 4,
            Self::Stage5CycleDetection => 5,
            Self::Stage6Prediction => 6,
            Self::Stage7Finalize => 7,
        }
    }

    /// Creates a StageNumber from a u8 (1-7).
    pub fn from_u8(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::Stage1MediaConversion),
            2 => Some(Self::Stage2ValidityCheck),
            3 => Some(Self::Stage3Reframing),
            4 => Some(Self::Stage4PoseEstimation),
            5 => Some(Self::Stage5CycleDetection),
            6 => Some(Self::Stage6Prediction),
            7 => Some(Self::Stage7Finalize),
            _ => None,
        }
    }

    /// Returns the human-readable name for this stage.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Stage1MediaConversion => "Media Conversion",
            Self::Stage2ValidityCheck => "Validity Check",
            Self::Stage3Reframing => "Reframing",
            Self::Stage4PoseEstimation => "Pose Estimation",
            Self::Stage5CycleDetection => "Cycle Detection",
            Self::Stage6Prediction => "Prediction",
            Self::Stage7Finalize => "Finalize",
        }
    }

    /// Returns the storage path prefix for this stage's outputs.
    pub fn storage_prefix(&self) -> &'static str {
        match self {
            Self::Stage1MediaConversion => "stage_1",
            Self::Stage2ValidityCheck => "stage_2",
            Self::Stage3Reframing => "stage_3",
            Self::Stage4PoseEstimation => "stage_4",
            Self::Stage5CycleDetection => "stage_5",
            Self::Stage6Prediction => "stage_6",
            Self::Stage7Finalize => "stage_7",
        }
    }
}

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
    
    /// Current stage number (0 = uploaded, 1-7 = processing that stage)
    pub current_stage: u8,
    
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
