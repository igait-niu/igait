use std::{sync::Arc, time::SystemTime};

use anyhow::{anyhow, Context, Result};
use axum::{
    body::Bytes,
    extract::{Multipart, State},
};
use firebase_auth::FirebaseUser;

use crate::helper::{
    email::send_contribution_email,
    lib::{AppError, AppState, AppStatePtr},
};

/// A request to upload a video for the contribute endpoint.
pub struct ContributeRequestArguments {
    name: String,
    email: String,
    front_file: ContributeRequestFile,
    side_file: ContributeRequestFile,
}

/// A representation of a file in a `Multipart` request.
#[derive(Debug)]
struct ContributeRequestFile {
    name: String,
    bytes: Bytes,
}

/// Takes in the `Multipart` request and unpacks the arguments into a `ContributeRequestArguments` object.
///
/// # Fails
/// If any of the fields are missing or if the files are too large.
///
/// # Arguments
/// * `multipart` - The `Multipart` object to unpack.
async fn unpack_contribute_arguments(
    multipart: &mut Multipart,
) -> Result<ContributeRequestArguments> {
    // Initialize all of the fields as options
    let mut name_option: Option<String> = None;
    let mut email_option: Option<String> = None;

    // Initialize the file fields as options
    let mut front_file_name_option: Option<String> = None;
    let mut side_file_name_option: Option<String> = None;
    let mut front_file_bytes_option: Option<Bytes> = None;
    let mut side_file_bytes_option: Option<Bytes> = None;

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
                front_file_bytes_option = Some(field.bytes()
                    .await
                    .context("Could not unpack bytes from field 'fileuploadfront'! Was there no file attached?")?);
            }
            Some("fileuploadside") => {
                side_file_name_option = field.file_name().map(String::from);
                side_file_bytes_option = Some(field.bytes()
                    .await
                    .context("Could not unpack bytes from field 'fileuploadside'! Was there no file attached?")?);
            }
            Some("email") => {
                email_option = Some(
                    field
                        .text()
                        .await
                        .context("Field 'uid' wasn't readable as text!")?
                        .to_string(),
                );
            }
            Some("name") => {
                name_option = Some(
                    field
                        .text()
                        .await
                        .context("Field 'name' wasn't readable as text!")?
                        .to_string(),
                );
            }
            Some("uid") => {
                // uid is now derived from the authenticated FirebaseUser token;
                // ignore any user-supplied value.
                let _ = field.text().await;
            }
            _ => {
                println!("Which had an unknown/no field name...");
            }
        }
    }

    // Make sure all of the fields are present
    let name: String = name_option.ok_or(anyhow!("Missing 'name' in request"))?;
    let email: String = email_option.ok_or(anyhow!("Missing 'email' in request"))?;

    // Make sure all of the file fields are present
    let front_file_name: String =
        front_file_name_option.ok_or(anyhow!("Missing 'fileuploadfront' in request!"))?;
    let side_file_name: String =
        side_file_name_option.ok_or(anyhow!("Missing 'fileuploadside' in request!"))?;
    let front_file_bytes: Bytes =
        front_file_bytes_option.ok_or(anyhow!("Missing 'fileuploadfront' in request!"))?;
    let side_file_bytes: Bytes =
        side_file_bytes_option.ok_or(anyhow!("Missing 'fileuploadside' in request!"))?;

    Ok(ContributeRequestArguments {
        name,
        email,
        front_file: ContributeRequestFile {
            name: front_file_name,
            bytes: front_file_bytes,
        },
        side_file: ContributeRequestFile {
            name: side_file_name,
            bytes: side_file_bytes,
        },
    })
}

/// The entrypoint for the contribute request.
///
/// # Fails
/// * If the arguments are missing.
/// * If the files are too large.
/// * If the files fail to save to S3.
/// * If the job fails to save to the database.
/// * If the welcome email fails to send.
///
/// # Arguments
/// * `app` - The application state.
/// * `multipart` - The `Multipart` object to unpack.
pub async fn contribute_entrypoint(
    current_user: FirebaseUser,
    State(app): State<AppStatePtr>,
    mut multipart: Multipart,
) -> Result<(), AppError> {
    let app = app.state;
    let uid = current_user.user_id;

    println!("Unpacking arguments...");
    // Unpack the arguments
    let arguments: ContributeRequestArguments = unpack_contribute_arguments(&mut multipart)
        .await
        .context("Failed to unpack arguments!")?;

    // Try to save the files to S3
    if let Err(err) = save_upload_files(
        app.clone(),
        arguments.front_file,
        arguments.side_file,
        &uid,
        &arguments.email,
        &arguments.name,
    )
    .await
    {
        return Err(AppError(
            err.context("Failed to save locally or upload files to S3!"),
        ));
    }

    // Thank the user for their contribution
    send_contribution_email(app.clone(), &arguments.email, &arguments.name)
        .await
        .context("Failed to send contribution email!")?;
    println!("Successfully sent contribution email!");

    Ok(())
}

/// Saves the contributed video files to AWS S3 for research purposes.
///
/// # Fails
/// * If the files fail to upload to AWS S3
///
/// # Arguments
/// * `app` - The application state
/// * `front_file` - The front video file to save
/// * `side_file` - The side video file to save
/// * `user_id` - The user ID to save the files under
/// * `email` - The email to save the files under
/// * `_name` - The name of the contributor (unused but kept for API compatibility)
async fn save_upload_files(
    app: Arc<AppState>,
    front_file: ContributeRequestFile,
    side_file: ContributeRequestFile,
    user_id: &str,
    email: &str,
    _name: &str,
) -> Result<()> {
    // Unpack the extensions
    let front_extension = front_file
        .name
        .split('.')
        .next_back()
        .context("Must have a file extension!")?;
    let side_extension = side_file
        .name
        .split('.')
        .next_back()
        .context("Must have a file extension!")?;

    // Ensure a directory exists for this file ID
    let unix_timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .context("Failed to get the current time!")?
        .as_secs();
    let email_user_id = format!("{};{}", user_id, email.replace('@', "_at_"));

    // Build storage keys
    let front_key = format!(
        "research/{}/{}/front.{}",
        email_user_id, unix_timestamp, front_extension
    );
    let side_key = format!(
        "research/{}/{}/side.{}",
        email_user_id, unix_timestamp, side_extension
    );

    // Upload files to AWS S3
    app.storage
        .upload(&front_key, front_file.bytes.to_vec(), Some("video/mp4"))
        .await
        .context("Failed to upload front file to AWS S3!")?;
    println!("Successfully uploaded front file to AWS S3!");

    app.storage
        .upload(&side_key, side_file.bytes.to_vec(), Some("video/mp4"))
        .await
        .context("Failed to upload side file to AWS S3!")?;
    println!("Successfully uploaded side file to AWS S3!");

    // Return as successful
    Ok(())
}
