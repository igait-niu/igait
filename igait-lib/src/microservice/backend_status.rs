//! Backend status update functionality for microservices.
//! 
//! This module provides utilities for microservices to update job status
//! directly in Firebase RTDB.

use serde::{Deserialize, Serialize};

/// The total number of processing stages in the pipeline
pub const NUM_STAGES: u8 = 7;

/// Per-stage status for tracking individual stage progress.
///
/// Stored in Firebase RTDB at `users/{uid}/jobs/{index}/stage_statuses/stage_{n}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    /// Stage has not started yet
    NotStarted,
    /// Stage is currently running
    Running,
    /// Stage completed successfully
    Complete,
    /// Stage failed with an error
    Error,
}

/// Simplified job status that gets stored in Firebase RTDB.
/// 
/// This is written directly to `users/{uid}/jobs/{index}/status`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "PascalCase")]
pub enum JobStatus {
    /// Job has been submitted and is waiting to be processed
    Submitted {
        #[serde(default = "default_submitted_value")]
        value: String,
    },
    /// Job is currently being processed by a stage
    Processing {
        stage: u8,
        num_stages: u8,
        value: String,
    },
    /// Job completed successfully with prediction results
    Complete {
        /// The prediction value (0.0 - 1.0 probability)
        prediction: f32,
        /// Whether ASD was detected
        asd: bool,
        value: String,
    },
    /// Job failed at some point in the pipeline
    Error {
        /// Collected error logs
        logs: String,
        value: String,
    },
}

fn default_submitted_value() -> String {
    "Job submitted successfully".to_string()
}

impl JobStatus {
    /// Create a new Submitted status
    pub fn submitted() -> Self {
        Self::Submitted {
            value: "Job submitted successfully".to_string(),
        }
    }

    /// Create a new Processing status for a given stage
    pub fn processing(stage: u8) -> Self {
        let stage_name = match stage {
            1 => "Converting video format",
            2 => "Checking video validity",
            3 => "Reframing video",
            4 => "Estimating pose landmarks",
            5 => "Detecting gait cycles",
            6 => "Running ML prediction",
            7 => "Finalizing results",
            _ => "Processing",
        };
        
        Self::Processing {
            stage,
            num_stages: NUM_STAGES,
            value: format!("Stage {}/{}: {}...", stage, NUM_STAGES, stage_name),
        }
    }

    /// Create a new Complete status with prediction results
    pub fn complete(prediction: f32, asd: bool) -> Self {
        let value = if asd {
            format!("Analysis complete - ASD indicators detected ({:.1}% confidence)", prediction * 100.0)
        } else {
            format!("Analysis complete - No ASD indicators ({:.1}% confidence)", (1.0 - prediction) * 100.0)
        };
        
        Self::Complete {
            prediction,
            asd,
            value,
        }
    }

    /// Create a new Error status with logs
    pub fn error(logs: String) -> Self {
        Self::Error {
            value: "Analysis failed - see logs for details".to_string(),
            logs,
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &str {
        match self {
            Self::Submitted { value } => value,
            Self::Processing { value, .. } => value,
            Self::Complete { value, .. } => value,
            Self::Error { value, .. } => value,
        }
    }

    /// Check if this status represents a processing state
    pub fn is_processing(&self) -> bool {
        matches!(self, Self::Processing { .. })
    }

    /// Check if this status represents a completed state
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete { .. })
    }

    /// Check if this status represents an error state
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    /// Get the code/type as a string (for frontend compatibility)
    pub fn code(&self) -> &'static str {
        match self {
            Self::Submitted { .. } => "Submitted",
            Self::Processing { .. } => "Processing",
            Self::Complete { .. } => "Complete",
            Self::Error { .. } => "Error",
        }
    }
}
