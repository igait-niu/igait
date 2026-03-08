//! This module contains the routes for the API.
//! 
//! To learn more about a route and how it works, click on the module name at the bottom of the page.

/// This module contains the job upload endpoint for the API.
///
/// Uploads are processed by pushing to a Firebase RTDB queue, which is then
/// picked up by the Stage 1 worker for processing.
/// 
/// # Arguments
/// * `uid`: The Google Firebase user ID. Use the Firebase JS SDK to obtain before submitting.
/// * `age`: The age of the user in `MM/DD/YYYY` format.
/// * `ethnicity`: The ethnicity of the user.
/// * `sex`: The sex assigned to the patient at birth.
/// * `height`: The height of the user in `ft'in"` format.
/// * `weight`: The weight of the user in pounds.
/// * `email`: The email address of the user of which to send emails to.
/// * `front_file`: The front video file. Needs to be a file compatible with OpenPose.
/// * `side_file`: The side video file. Needs to be a file compatible with OpenPose.
/// 
/// # Example cURL Request
/// ```sh
/// curl -v -F fileuploadfront=@test.mp4 -F fileuploadside=@test.mp4 /
///     -F uid=curlplaceholder -F age=18 -F ethnicity=Caucasian /
///     -F email=me@hiibolt.com -F sex=M -F height="5'10" -F weight=120 / 
///     http://api.igaitapp.com/api/v1/upload
/// ```
/// 
/// # Potential Reasons for Failure
/// See each individual function for potential reasons for failure.
/// 
/// # Notes
/// <div class="warning">This route takes a long time to process. Make users aware of this on the frontend.</div>
pub mod upload;

/// This module contains the assistant endpoint for the API.
/// 
/// Currently, this works by proxying requests to the iGait Assistant API.
/// 
/// This is done so due to the limitations of Firebase Auth expecting authentication via the `Bearer` header.
/// Unfortunately, the WSS standard does not support this, so we instead extract it from an initial request and re-proxy to a now-authenticaed secure websocket.
/// This approach is certainly not ideal, but it is indeed highly secure. 
/// In fact, it is significantly more secure than the WSS standard itself, which does not support authentication at all.
/// 
/// For more information on usage, see the iGAIT frontend codebase.
pub mod assistant;

/// This module contains the contribute endpoint for the API,
/// which is used to submit a video for our research purposes.
/// 
/// # Arguments
/// * `uid`: The Google Firebase user ID. Use the Firebase JS SDK to obtain before submitting.
/// * `email`: The email address of the user of which to send emails to.
/// * `fileuploadfront`: The front video file. Needs to be a file compatible with OpenPose.
/// * `fileuploadside`: The side video file. Needs to be a file compatible with OpenPose.
/// * `name`: The name of the user submitting the video.
/// 
/// # Example cURL Request
/// ```sh
/// curl -v -F fileuploadfront=@test.mp4 -F fileuploadside=test.mp4 /
///     -F uid=curlplaceholder -F email=me@hiibolt.com -F name="John Doe" /
///     http://api.igaitapp.com/api/v1/contribute
/// ```
/// 
/// # Potential Reasons for Failure
/// See each individual function for potential reasons for failure.
pub mod contribute;

/// This module contains the rerun endpoint for the API,
/// which allows admin users to re-process a job from a specific stage.
///
/// Accepts a JSON body with `user_id`, `job_index` (0-indexed), and `stage` (1–6).
/// Note: Stage 7 (finalize) cannot be rerun as it uses a different queue item type.
/// Cleans up S3 outputs from the target stage onward, then re-queues the job.
pub mod rerun;

/// Files endpoint for generating presigned S3 download URLs.
///
/// Returns all files for a job grouped by stage, with time-limited
/// presigned URLs so the frontend can display/download them securely.
pub mod files;

/// Cycle editing endpoint for updating gait cycle detection results.
///
/// Allows administrators to modify the `gait_cycles` array in Stage 5
/// output JSON files (front/side gait analysis) stored in S3.
/// Useful for manually correcting cycle boundaries before re-running
/// prediction from Stage 6.
pub mod cycles;

/// Video editing endpoint for applying rotation, trim, and crop to job videos.
///
/// Copies Stage 1 outputs to Stage 0, stores edit flags on the Job,
/// and re-queues for Stage 1 processing with the transform parameters.
pub mod video_edit;

/// Internal endpoints for microservice communication
/// 
/// These endpoints are NOT exposed publicly and should only be called
/// by internal microservices.
pub mod internal;