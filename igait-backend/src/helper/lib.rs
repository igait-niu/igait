use std::{sync::Arc, time::SystemTime};

use anyhow::{ Result, Context };
use axum::{
    async_trait, body::Body, extract::FromRequestParts, http::{self, request::Parts}, response::{IntoResponse, Response}
};
use serde::{Deserialize, Serialize};
use async_openai::{
    config::OpenAIConfig, types::AssistantObject, Client
};
use tokio::sync::Mutex;
use firebase_auth::{FirebaseAuth, FirebaseUser};
use igait_lib::microservice::{EmailClient, StorageClient, VideoEditFlags, StageStatus};
use ts_rs::TS;

use super::database::Database;

/// Custom serialization module for SystemTime as Unix timestamp (seconds)
mod systemtime_as_secs {
    use std::time::{SystemTime, UNIX_EPOCH};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }
}

/// The user struct, which contains a user ID and a list of jobs.
/// 
/// # Fields
/// * `uid` - The user ID
/// * `jobs` - The list of jobs
/// * `administrator` - Whether the user has administrator privileges
#[derive( Serialize, Deserialize, Debug, TS )]
#[ts(export)]
pub struct User {
    pub uid: String,
    #[serde(default)]
    pub jobs: Vec<Job>,
    #[serde(default)]
    pub administrator: bool,
}

/// The job struct, which contains the job
/// 
/// # Fields
/// * `age` - The age of the patient
/// * `ethnicity` - The ethnicity of the patient
/// * `sex` - The assigned sex of the patient
/// * `height` - The height of the patient
/// * `status` - The status of the job
/// * `timestamp` - The timestamp of the job (Unix timestamp in seconds)
/// * `weight` - The weight of the patient
/// * `email` - The email of the person who submitted the job
/// * `requires_approval` - Whether the user requested manual approval for this job
/// * `approved` - Whether this job has been approved for processing
#[derive( Serialize, Deserialize, Clone, Debug, TS )]
#[ts(export)]
pub struct Job {
    pub age: i16,
    pub ethnicity: Ethnicity,
    pub sex: Sex,
    pub height: String,
    pub status: JobStatus,
    #[serde(with = "systemtime_as_secs")]
    #[ts(type = "number")]
    pub timestamp: SystemTime,
    pub weight: i16,
    pub email: String,
    /// Whether this job requires manual approval before processing
    #[serde(default)]
    pub requires_approval: bool,
    /// Whether this job has been approved for processing
    #[serde(default)]
    pub approved: bool,
    /// Per-stage logs collected during processing.
    /// Keys are "stage_1" through "stage_7", values are the log text.
    #[serde(default)]
    pub stage_logs: std::collections::HashMap<String, String>,
    /// Per-stage statuses tracking individual stage progress.
    /// Keys are "stage_1" through "stage_7", values are the stage status.
    #[serde(default)]
    #[ts(type = "Record<string, StageStatus>")]
    pub stage_statuses: std::collections::HashMap<String, StageStatus>,
    /// Video editing flags (rotation, trim, crop) to apply on the next Stage 1 run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "{ front?: { rotation?: number, trim_start?: number, trim_end?: number, crop_x?: number, crop_y?: number, crop_width?: number, crop_height?: number }, side?: { rotation?: number, trim_start?: number, trim_end?: number, crop_x?: number, crop_y?: number, crop_width?: number, crop_height?: number } }")]
    pub video_edit: Option<VideoEditFlags>,
}

/// The total number of processing stages in the pipeline
pub const NUM_STAGES: u8 = 7;

/// Simplified job status that gets stored in Firebase RTDB.
/// 
/// This is a tagged union (discriminated by `code`) with variant-specific fields.
/// 
/// # Variants
/// * `Submitted` - Job has been submitted and is waiting to be processed
/// * `Processing` - Job is currently being processed by a stage
/// * `Complete` - Job completed successfully with prediction results
/// * `Error` - Job failed at some point in the pipeline
#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export)]
#[serde(tag = "code")]
pub enum JobStatus {
    /// Job has been submitted and is waiting to be processed
    Submitted {
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

    /// Get the code/type as a string
    pub fn code(&self) -> &'static str {
        match self {
            Self::Submitted { .. } => "Submitted",
            Self::Processing { .. } => "Processing",
            Self::Complete { .. } => "Complete",
            Self::Error { .. } => "Error",
        }
    }
}

/// Sex options for job submission.
/// 
/// # Variants
/// * `M` - Male
/// * `F` - Female
/// * `O` - Other
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, TS)]
#[ts(export)]
pub enum Sex {
    M,
    F,
    O,
}

impl std::fmt::Display for Sex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sex::M => write!(f, "M"),
            Sex::F => write!(f, "F"),
            Sex::O => write!(f, "O"),
        }
    }
}

impl std::str::FromStr for Sex {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "M" | "MALE" => Ok(Sex::M),
            "F" | "FEMALE" => Ok(Sex::F),
            "O" | "OTHER" => Ok(Sex::O),
            _ => Err(anyhow::anyhow!("Invalid sex value: {}", s)),
        }
    }
}

/// Ethnicity options for job submission.
/// 
/// # Variants
/// * `AfricanAmerican` - African American/Black
/// * `NativeAmerican` - Native American/American Indian
/// * `Asian` - Asian
/// * `Hispanic` - Hispanic/Latino
/// * `Caucasian` - Caucasian/White
/// * `PacificIslander` - Pacific Islander
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Ethnicity {
    AfricanAmerican,
    NativeAmerican,
    Asian,
    Hispanic,
    Caucasian,
    PacificIslander,
}

impl std::fmt::Display for Ethnicity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ethnicity::AfricanAmerican => write!(f, "African American/Black"),
            Ethnicity::NativeAmerican => write!(f, "Native American/American Indian"),
            Ethnicity::Asian => write!(f, "Asian"),
            Ethnicity::Hispanic => write!(f, "Hispanic/Latino"),
            Ethnicity::Caucasian => write!(f, "Caucasian/White"),
            Ethnicity::PacificIslander => write!(f, "Pacific Islander"),
        }
    }
}

impl std::str::FromStr for Ethnicity {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "africanAmerican" => Ok(Ethnicity::AfricanAmerican),
            "nativeAmerican" => Ok(Ethnicity::NativeAmerican),
            "asian" => Ok(Ethnicity::Asian),
            "hispanic" => Ok(Ethnicity::Hispanic),
            "caucasian" => Ok(Ethnicity::Caucasian),
            "pacificIslander" => Ok(Ethnicity::PacificIslander),
            _ => Err(anyhow::anyhow!("Invalid ethnicity value: {}", s)),
        }
    }
}

/// User role options - who is completing the submission form.
/// 
/// # Variants
/// * `Parent` - Parent of the patient
/// * `Doctor` - Medical professional
/// * `SchoolOfficial` - School nurse or administrator
/// * `Sibling` - Sibling of the patient
/// * `Grandparent` - Grandparent of the patient
/// * `Self_` - Patient themselves
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum UserRole {
    Parent,
    Doctor,
    SchoolOfficial,
    Sibling,
    Grandparent,
    #[serde(rename = "self")]
    Self_,
}

impl std::str::FromStr for UserRole {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "parent" => Ok(UserRole::Parent),
            "doctor" => Ok(UserRole::Doctor),
            "schoolOfficial" => Ok(UserRole::SchoolOfficial),
            "sibling" => Ok(UserRole::Sibling),
            "grandparent" => Ok(UserRole::Grandparent),
            "self" => Ok(UserRole::Self_),
            _ => Err(anyhow::anyhow!("Invalid user role value: {}", s)),
        }
    }
}

/// The state of the entire backend application with handles to the database and storage.
/// 
/// # Fields
/// * `db` - The database handle (Firebase RTDB)
/// * `storage` - AWS S3 client (GCS-backed)
/// * `email_client` - Email client for sending notifications
/// * `openai_client` - OpenAI client for AI assistant
/// * `openai_assistant` - The loaded OpenAI assistant
/// * `firebase_auth` - Firebase Auth for user verification
/// 
/// # Notes
/// * This struct is typically wrapped in an `Arc<>` to allow for concurrent access.
impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("db", &self.db)
            .field("storage", &self.storage)
            .field("email_client", &self.email_client)
            .field("openai_client", &self.openai_client)
            .field("openai_assistant", &self.openai_assistant)
            .field("firebase_auth", &"<firebase_auth>")
            .finish()
    }
}
fn get_bearer_token(header: &str) -> Option<String> {
    let prefix_len = "Bearer ".len();

    match header.len() {
        l if l < prefix_len => None,
        _ => Some(header[prefix_len..].to_string()),
    }
}
#[derive(Debug, Clone)]
pub struct AppStatePtr {
    pub state: Arc<AppState>
}
#[async_trait]
impl FromRequestParts<AppStatePtr> for FirebaseUser {
    type Rejection = UnauthorizedResponse;

    async fn from_request_parts(parts: &mut Parts, app_state_ptr: &AppStatePtr) -> Result<Self, Self::Rejection> {
        let store = &app_state_ptr.state.firebase_auth;

        let auth_header = parts
            .headers
            .get(http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");

        let bearer = get_bearer_token(auth_header).map_or(
            Err(UnauthorizedResponse {
                msg: "Missing Bearer Token".to_string(),
            }),
            Ok,
        )?;

        match store.verify(&bearer) {
            Err(e) => {
                eprintln!("Failed to verify Token: {}", e);

                Err(UnauthorizedResponse {
                    msg: format!("Failed to verify Token: {}", e),
                })
            },
            Ok(current_user) => Ok(current_user),
        }
    }
}

pub struct UnauthorizedResponse {
    msg: String,
}

impl IntoResponse for UnauthorizedResponse {
    fn into_response(self) -> Response {
        (http::StatusCode::UNAUTHORIZED, self.msg).into_response()
    }
}
pub struct AppState {
    pub db: Mutex<Database>,
    pub storage: StorageClient,
    pub email_client: EmailClient,
    pub openai_client: Client<OpenAIConfig>,
    pub openai_assistant: Option<AssistantObject>,
    pub firebase_auth: FirebaseAuth
}
impl AppState {
    /// Initializes the application state with database, storage, and service clients.
    /// 
    /// # Returns
    /// * A successful result with the application state if successful
    /// 
    /// # Fails
    /// * If the database fails to initialize
    /// * If the storage client fails to initialize
    /// * If Firebase Auth fails to initialize
    /// * If the OpenAI assistant can't be loaded
    /// 
    /// # Notes
    /// * This function is typically called at the start of the application.
    /// * Required environment variables:
    ///   - `GOOGLE_APPLICATION_CREDENTIALS` - Path to GCP service account JSON
    ///   - `FIREBASE_ACCESS_KEY` - Firebase RTDB access key
    ///   - `OPENAI_ASSISTANT_ID` - OpenAI assistant ID
    ///   - AWS credentials for SES
    pub async fn new() -> Result<Self> {
        let client = Client::new();
        let firebase_auth = FirebaseAuth::new("network-technology-project")
            .await;

        // Try to initialize the assistant (optional for upload route)
        let assistant = match std::env::var("OPENAI_ASSISTANT_ID") {
            Ok(assistant_id) => {
                match client.assistants().retrieve(&assistant_id).await {
                    Ok(a) => {
                        println!("✅ OpenAI Assistant loaded successfully");
                        Some(a)
                    }
                    Err(e) => {
                        eprintln!("⚠️  Failed to load OpenAI Assistant: {}", e);
                        eprintln!("   Upload and processing will work, but AI assistant features will be disabled");
                        None
                    }
                }
            }
            Err(_) => {
                eprintln!("⚠️  OPENAI_ASSISTANT_ID not set");
                eprintln!("   Upload and processing will work, but AI assistant features will be disabled");
                None
            }
        };

        // Initialize AWS S3 client
        let storage = StorageClient::new()
            .await
            .context("Failed to initialize AWS S3 client")?;

        // Initialize email client
        let email_client = EmailClient::from_env()
            .await
            .context("Failed to initialize email client")?;

        Ok(Self {
            db: Mutex::new(Database::init().await.context("Failed to initialize database while setting up app state!")?),
            storage,
            email_client,
            openai_client: client,
            openai_assistant: assistant,
            firebase_auth
        })
    }
}


/// The error type for the application.
/// 
/// # Fields
/// * `AppError` - The error type for the application
/// 
/// # Notes
/// * This error type is used to handle errors in the application.
/// * The reason for its existence is to allow for a more detailed error message to be returned by `axum` routes.
#[derive(Debug)]
pub struct AppError(pub anyhow::Error);
impl IntoResponse for AppError {
    fn into_response(self) -> Response<Body> {
        let err = &self.0;

        eprintln!("Encountered an error: {err:#?}");
        for (ind, ctx) in err.chain().enumerate() {
            eprintln!("  [{ind}] {ctx:#?}");
        }

        eprintln!("Full backtrace...");
        eprintln!("{:#?}", err.backtrace());

        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}",err),
        )
            .into_response()
    }
}
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

