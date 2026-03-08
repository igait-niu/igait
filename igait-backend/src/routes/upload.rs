//! Upload endpoint for submitting new gait analysis jobs.
//!
//! This module handles video uploads and initiates the processing pipeline
//! by uploading to AWS S3 and pushing to the Stage 1 queue.

use std::{collections::HashMap, sync::Arc, time::SystemTime};

use axum::{body::Bytes, extract::{Multipart, State}};
use anyhow::{Result, Context, anyhow};
use firebase_auth::FirebaseUser;

use igait_lib::microservice::{StoragePaths, JobMetadata, QueueItem, StageNumber, FirebaseRtdb, queue_item_path};

use crate::helper::{
    email::send_welcome_email,
    lib::{AppError, AppState, AppStatePtr, Job, JobStatus, Sex, Ethnicity},
};

/// The required arguments for the upload request.
struct UploadRequestArguments {
    age:        i16,
    ethnicity:  Ethnicity,
    sex:        Sex,
    height:     String,
    weight:     i16,
    email:      String,
    front_file: UploadRequestFile,
    side_file:  UploadRequestFile,
    requires_approval: bool,
}

/// A representation of a file in a `Multipart` request.
#[derive(Debug)]
struct UploadRequestFile {
    name:  String,
    bytes: Bytes,
}

/// Takes in the `Multipart` request and unpacks the arguments into an `UploadRequestArguments` object.
///
/// # Fails
/// If any of the fields are missing or if the files are too large.
///
/// # Arguments
/// * `multipart` - The `Multipart` object to unpack.
async fn unpack_upload_arguments(multipart: &mut Multipart) -> Result<UploadRequestArguments> {
    // Initialize all of the fields as options
    let mut age_option:       Option<i16>       = None;
    let mut ethnicity_option: Option<Ethnicity> = None;
    let mut sex_option:       Option<Sex>       = None;
    let mut height_option:    Option<String> = None;
    let mut weight_option:    Option<i16>    = None;
    let mut email_option:     Option<String> = None;
    let mut requires_approval: bool = false;

    // Initialize the file fields as options
    let mut front_file_name_option:  Option<String> = None;
    let mut side_file_name_option:   Option<String> = None;
    let mut front_file_bytes_option: Option<Bytes>  = None;
    let mut side_file_bytes_option:  Option<Bytes>  = None;

    // Loop through the fields
    while let Some(field) = multipart
        .next_field()
        .await
        .context("Bad upload request! Is it possible you submitted a file over the size limit?")?
    {
        let name = field.name();
        let field_name = field.file_name();
        println!("Field Incoming: {name:?} - File Attached: {field_name:?}");

        match field.name() {
            Some("fileuploadfront") => {
                front_file_name_option = field.file_name().map(String::from);
                front_file_bytes_option = Some(
                    field
                        .bytes()
                        .await
                        .context("Could not unpack bytes from field 'fileuploadfront'!")?,
                );
            }
            Some("fileuploadside") => {
                side_file_name_option = field.file_name().map(String::from);
                side_file_bytes_option = Some(
                    field
                        .bytes()
                        .await
                        .context("Could not unpack bytes from field 'fileuploadside'!")?,
                );
            }
            Some("uid") => {
                // uid is now derived from the authenticated FirebaseUser token;
                // ignore any user-supplied value.
                let _ = field.text().await;
            }
            Some("age") => {
                age_option = Some(
                    field
                        .text()
                        .await
                        .context("Field 'age' wasn't readable as text!")?
                        .parse()
                        .context("Field 'age' wasn't parseable as a number!")?,
                );
            }
            Some("ethnicity") => {
                ethnicity_option = Some(
                    field
                        .text()
                        .await
                        .context("Field 'ethnicity' wasn't readable as text!")?
                        .parse()
                        .context("Field 'ethnicity' wasn't a valid ethnicity value!")?,
                );
            }
            Some("email") => {
                email_option = Some(
                    field
                        .text()
                        .await
                        .context("Field 'email' wasn't readable as text!")?,
                );
            }
            Some("sex") => {
                sex_option = Some(
                    field
                        .text()
                        .await
                        .context("Field 'sex' wasn't readable as text!")?
                        .parse()
                        .context("Field 'sex' wasn't a valid sex value!")?,
                );
            }
            Some("height") => {
                height_option = Some(
                    field
                        .text()
                        .await
                        .context("Field 'height' wasn't readable as text!")?,
                );
            }
            Some("weight") => {
                weight_option = Some(
                    field
                        .text()
                        .await
                        .context("Field 'weight' wasn't readable as text!")?
                        .parse()
                        .context("Field 'weight' wasn't parseable as a number!")?,
                );
            }
            Some("requires_approval") => {
                let text = field
                    .text()
                    .await
                    .context("Field 'requires_approval' wasn't readable as text!")?;
                requires_approval = text == "true" || text == "1";
            }
            _ => {
                println!("Skipping unknown field: {name:?}");
            }
        }
    }

    // Make sure all of the fields are present
    let age       = age_option.ok_or(anyhow!("Missing 'age' in request"))?;
    let ethnicity = ethnicity_option.ok_or(anyhow!("Missing 'ethnicity' in request"))?;
    let sex       = sex_option.ok_or(anyhow!("Missing 'sex' in request"))?;
    let height    = height_option.ok_or(anyhow!("Missing 'height' in request"))?;
    let weight    = weight_option.ok_or(anyhow!("Missing 'weight' in request"))?;
    let email     = email_option.ok_or(anyhow!("Missing 'email' in request"))?;

    // Make sure all of the file fields are present
    let front_file_name  = front_file_name_option.ok_or(anyhow!("Missing 'fileuploadfront' in request!"))?;
    let side_file_name   = side_file_name_option.ok_or(anyhow!("Missing 'fileuploadside' in request!"))?;
    let front_file_bytes = front_file_bytes_option.ok_or(anyhow!("Missing 'fileuploadfront' bytes!"))?;
    let side_file_bytes  = side_file_bytes_option.ok_or(anyhow!("Missing 'fileuploadside' bytes!"))?;

    Ok(UploadRequestArguments {
        age,
        ethnicity,
        sex,
        height,
        weight,
        email,
        requires_approval,
        front_file: UploadRequestFile {
            name: front_file_name,
            bytes: front_file_bytes,
        },
        side_file: UploadRequestFile {
            name: side_file_name,
            bytes: side_file_bytes,
        },
    })
}

/// The entrypoint for the upload request.
///
/// # Workflow
/// 1. Parse and validate the multipart form data
/// 2. Create a new job in the database
/// 3. Upload videos to AWS S3
/// 4. Dispatch to Stage 1 microservice
/// 5. Send welcome email
///
/// # Fails
/// * If the arguments are missing or invalid
/// * If the files fail to upload to AWS S3
/// * If the job fails to save to the database
/// * If the welcome email fails to send
pub async fn upload_entrypoint(
    current_user: FirebaseUser,
    State(app): State<AppStatePtr>,
    mut multipart: Multipart,
) -> Result<(), AppError> {
    let app = app.state;
    let uid = current_user.user_id;

    println!("Unpacking upload arguments...");
    let arguments = unpack_upload_arguments(&mut multipart)
        .await
        .context("Failed to unpack arguments!")?;

    // Build a new status object
    let mut status = JobStatus::submitted();

    // Generate the new job ID (0-indexed)
    let job_index = app
        .db
        .lock()
        .await
        .count_jobs(&uid)
        .await
        .context("Failed to count the number of jobs!")?;

    // Build job ID string (format: "{user_id}_{job_index}")
    let job_id = format!("{}_{}", uid, job_index);
    println!("Created job ID: {}", job_id);

    // Build the new job object
    let job = Job {
        age:       arguments.age,
        ethnicity: arguments.ethnicity.clone(),
        sex:       arguments.sex,
        height:    arguments.height.clone(),
        weight:    arguments.weight,
        status:    status.clone(),
        email:     arguments.email.clone(),
        timestamp: SystemTime::now(),
        requires_approval: arguments.requires_approval,
        // Start unapproved — the worker logic will allow pick-up
        // if neither the job nor the queue requires approval.
        approved: false,
        stage_logs: HashMap::new(),
        stage_statuses: HashMap::new(),
    };

    // Add the job to the database
    app.db
        .lock()
        .await
        .new_job(&uid, job.clone())
        .await
        .context("Failed to add the new job to the database!")?;

    // Upload files to AWS S3 and dispatch to Stage 1
    if let Err(err) = upload_and_dispatch(
        app.clone(),
        &job_id,
        &uid,
        arguments.front_file,
        arguments.side_file,
        &job,
    )
    .await
    {
        // Populate the status object with error
        status = JobStatus::error(format!("Upload failed: {}", err));

        // Update the status of the job
        app.db
            .lock()
            .await
            .update_status(&uid, job_index, status)
            .await
            .context("Failed to update the status of the job!")?;

        return Err(AppError(err.context("Failed to upload files or dispatch job!")));
    }

    // Update status - job has been submitted and is ready for Stage 1
    status = JobStatus::submitted();

    // Send the welcome email
    send_welcome_email(app.clone(), &job, &uid, job_index)
        .await
        .context("Failed to send welcome email!")?;

    // Update the status of the job
    app.db
        .lock()
        .await
        .update_status(&uid, job_index, status)
        .await
        .context("Failed to update the status of the job!")?;

    println!("Job {} submitted successfully!", job_id);
    Ok(())
}

/// Uploads files to AWS S3 and pushes the job to the Stage 1 queue.
///
/// # Arguments
/// * `app` - The application state
/// * `job_id` - The full job ID (format: "{user_id}_{job_index}")
/// * `user_id` - The user ID
/// * `front_file` - The front video file
/// * `side_file` - The side video file
/// * `job` - The job containing all metadata
async fn upload_and_dispatch(
    app: Arc<AppState>,
    job_id: &str,
    user_id: &str,
    front_file: UploadRequestFile,
    side_file: UploadRequestFile,
    job: &Job,
) -> Result<()> {
    // Extract file extensions
    let front_extension = front_file
        .name
        .rsplit('.')
        .next()
        .context("Front file must have an extension!")?;
    let side_extension = side_file
        .name
        .rsplit('.')
        .next()
        .context("Side file must have an extension!")?;

    // Build storage paths
    let front_key = StoragePaths::upload_front_video(job_id, front_extension);
    let side_key = StoragePaths::upload_side_video(job_id, side_extension);

    println!("Uploading front video to: {}", front_key);
    let _: () = app.storage
        .upload(&front_key, front_file.bytes.to_vec(), Some("video/mp4"))
        .await
        .context("Failed to upload front video to AWS S3!")?;

    println!("Uploading side video to: {}", side_key);
    let _: () = app.storage
        .upload(&side_key, side_file.bytes.to_vec(), Some("video/mp4"))
        .await
        .context("Failed to upload side video to AWS S3!")?;

    println!("Files uploaded successfully, pushing to Stage 1 queue...");

    // Build the queue item for Stage 1
    let mut input_keys = HashMap::new();
    input_keys.insert("front_video".to_string(), front_key);
    input_keys.insert("side_video".to_string(), side_key);

    // Include all job metadata so it's available in the finalize stage
    let metadata = JobMetadata {
        email: Some(job.email.clone()),
        age: Some(job.age),
        sex: Some(job.sex.to_string().chars().next().unwrap_or('O')),
        ethnicity: Some(job.ethnicity.to_string()),
        height: Some(job.height.clone()),
        weight: Some(job.weight),
        extra: HashMap::new(),
    };

    let queue_item = QueueItem::new(
        job_id.to_string(),
        user_id.to_string(),
        input_keys,
        metadata,
        job.requires_approval,
    );

    // Push to Stage 1 queue in Firebase RTDB
    let rtdb = FirebaseRtdb::from_env()
        .context("Failed to initialize Firebase RTDB client")?;
    
    let queue_path = queue_item_path(StageNumber::Stage1MediaConversion, job_id);
    rtdb.set(&queue_path, &queue_item)
        .await
        .context("Failed to push job to Stage 1 queue")?;

    println!("Job {} pushed to Stage 1 queue successfully!", job_id);
    Ok(())
}
