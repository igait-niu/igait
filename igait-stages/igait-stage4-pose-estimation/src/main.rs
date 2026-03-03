//! Stage 4: Pose Estimation Microservice
//!
//! Extracts body keypoints from videos using MediaPipe's Holistic model.
//! Processes both front and side videos, generating:
//! - Pose overlay videos
//! - JSON landmark data for ML training

use anyhow::{Context, Result};
use async_trait::async_trait;
use igait_lib::microservice::{
    run_stage_worker, ProcessingResult, QueueItem, StageNumber, StageWorker, StorageClient,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::fs;
use tokio::process::Command;

/// Path to the Python pose estimation script (in the Docker container)
const POSE_SCRIPT_PATH: &str = "/app/3DPoseEstimation.py";

/// The pose estimation worker.
pub struct PoseEstimationWorker;

#[async_trait]
impl StageWorker for PoseEstimationWorker {
    fn stage(&self) -> StageNumber {
        StageNumber::Stage4PoseEstimation
    }

    fn service_name(&self) -> &'static str {
        "igait-stage4-pose-estimation"
    }

    async fn process(&self, job: &QueueItem) -> ProcessingResult {
        let start_time = Instant::now();
        let mut logs = String::new();

        println!("Processing job {}: Pose Estimation", job.job_id);
        logs.push_str(&format!(
            "Starting pose estimation for job {}\n",
            job.job_id
        ));

        match self.do_pose_estimation(job, &mut logs).await {
            Ok(output_keys) => {
                let duration = start_time.elapsed();
                logs.push_str(&format!("Pose estimation completed in {:?}\n", duration));

                ProcessingResult::Success {
                    output_keys,
                    logs,
                    duration_ms: duration.as_millis() as u64,
                }
            }
            Err(e) => {
                let duration = start_time.elapsed();
                eprintln!("Pose estimation failed for job {}: {}", job.job_id, e);
                logs.push_str(&format!("ERROR: {}\n", e));

                ProcessingResult::Failure {
                    error: e.to_string(),
                    logs,
                    duration_ms: duration.as_millis() as u64,
                }
            }
        }
    }
}

impl PoseEstimationWorker {
    async fn do_pose_estimation(
        &self,
        job: &QueueItem,
        logs: &mut String,
    ) -> Result<HashMap<String, String>> {
        let stage = StageNumber::Stage4PoseEstimation;

        // Get input paths from stage 1's keys
        let front_input = job
            .input_keys
            .get("front_video")
            .context("Missing front_video in input_keys")?
            .clone();
        let side_input = job
            .input_keys
            .get("side_video")
            .context("Missing side_video in input_keys")?
            .clone();

        logs.push_str(&format!("Input front video: {}\n", front_input));
        logs.push_str(&format!("Input side video: {}\n", side_input));

        // Initialize storage client
        logs.push_str("Initializing storage client...\n");
        let storage = StorageClient::new()
            .await
            .context("Failed to initialize storage client")?;

        // Create temporary directory for processing
        let temp_dir = PathBuf::from("/tmp").join(format!("pose_{}", job.job_id));
        fs::create_dir_all(&temp_dir)
            .await
            .context("Failed to create temp directory")?;
        logs.push_str(&format!("Created temp directory: {:?}\n", temp_dir));

        // Create output directory for pose estimation results
        let output_dir = temp_dir.join("output");
        fs::create_dir_all(&output_dir)
            .await
            .context("Failed to create output directory")?;

        // Download input videos
        logs.push_str("Downloading front video from storage...\n");
        let front_data = storage
            .download(&front_input)
            .await
            .context("Failed to download front video")?;
        let front_input_path = temp_dir.join("front.mp4");
        fs::write(&front_input_path, front_data)
            .await
            .context("Failed to write front input file")?;
        logs.push_str(&format!(
            "Downloaded front video ({} bytes)\n",
            fs::metadata(&front_input_path).await?.len()
        ));

        logs.push_str("Downloading side video from storage...\n");
        let side_data = storage
            .download(&side_input)
            .await
            .context("Failed to download side video")?;
        let side_input_path = temp_dir.join("side.mp4");
        fs::write(&side_input_path, side_data)
            .await
            .context("Failed to write side input file")?;
        logs.push_str(&format!(
            "Downloaded side video ({} bytes)\n",
            fs::metadata(&side_input_path).await?.len()
        ));

        // Run pose estimation on front video
        logs.push_str("Running pose estimation on front video...\n");
        run_pose_estimation(&front_input_path, &output_dir, logs)
            .await
            .context("Failed to run pose estimation on front video")?;
        logs.push_str("Front video pose estimation complete.\n");

        // Run pose estimation on side video
        logs.push_str("Running pose estimation on side video...\n");
        run_pose_estimation(&side_input_path, &output_dir, logs)
            .await
            .context("Failed to run pose estimation on side video")?;
        logs.push_str("Side video pose estimation complete.\n");

        // Re-encode pose videos from MPEG-4 Part 2 to H.264 for browser playback
        let front_pose_path = output_dir.join("front_pose.mp4");
        let side_pose_path = output_dir.join("side_pose.mp4");

        reencode_to_h264(&front_pose_path, logs)
            .await
            .context("Failed to re-encode front pose video to H.264")?;
        reencode_to_h264(&side_pose_path, logs)
            .await
            .context("Failed to re-encode side pose video to H.264")?;

        // Construct output storage keys
        let stage_num = stage.as_u8();
        let front_pose_key = format!("jobs/{}/stage_{}/front_pose.mp4", job.job_id, stage_num);
        let side_pose_key = format!("jobs/{}/stage_{}/side_pose.mp4", job.job_id, stage_num);
        let front_landmarks_key =
            format!("jobs/{}/stage_{}/front_landmarks.json", job.job_id, stage_num);
        let side_landmarks_key =
            format!("jobs/{}/stage_{}/side_landmarks.json", job.job_id, stage_num);

        // Upload front pose video
        logs.push_str(&format!(
            "Uploading front pose video from {:?}...\n",
            front_pose_path
        ));
        let front_pose_data = fs::read(&front_pose_path)
            .await
            .context("Failed to read front pose video")?;
        storage
            .upload(&front_pose_key, front_pose_data, Some("video/mp4"))
            .await
            .context("Failed to upload front pose video")?;
        logs.push_str(&format!("Uploaded front pose video to: {}\n", front_pose_key));

        // Upload side pose video
        logs.push_str(&format!(
            "Uploading side pose video from {:?}...\n",
            side_pose_path
        ));
        let side_pose_data = fs::read(&side_pose_path)
            .await
            .context("Failed to read side pose video")?;
        storage
            .upload(&side_pose_key, side_pose_data, Some("video/mp4"))
            .await
            .context("Failed to upload side pose video")?;
        logs.push_str(&format!("Uploaded side pose video to: {}\n", side_pose_key));

        // Upload front landmarks JSON
        let front_landmarks_path = output_dir.join("training_data").join("front_landmarks.json");
        logs.push_str(&format!(
            "Uploading front landmarks from {:?}...\n",
            front_landmarks_path
        ));
        let front_landmarks_data = fs::read(&front_landmarks_path)
            .await
            .context("Failed to read front landmarks JSON")?;
        storage
            .upload(
                &front_landmarks_key,
                front_landmarks_data,
                Some("application/json"),
            )
            .await
            .context("Failed to upload front landmarks JSON")?;
        logs.push_str(&format!(
            "Uploaded front landmarks to: {}\n",
            front_landmarks_key
        ));

        // Upload side landmarks JSON
        let side_landmarks_path = output_dir.join("training_data").join("side_landmarks.json");
        logs.push_str(&format!(
            "Uploading side landmarks from {:?}...\n",
            side_landmarks_path
        ));
        let side_landmarks_data = fs::read(&side_landmarks_path)
            .await
            .context("Failed to read side landmarks JSON")?;
        storage
            .upload(
                &side_landmarks_key,
                side_landmarks_data,
                Some("application/json"),
            )
            .await
            .context("Failed to upload side landmarks JSON")?;
        logs.push_str(&format!(
            "Uploaded side landmarks to: {}\n",
            side_landmarks_key
        ));

        // Clean up temporary files
        logs.push_str("Cleaning up temporary files...\n");
        fs::remove_dir_all(&temp_dir)
            .await
            .context("Failed to clean up temp directory")?;

        // Return output keys for next stage
        let mut output_keys = HashMap::new();
        output_keys.insert("front_video".to_string(), front_pose_key.clone());
        output_keys.insert("side_video".to_string(), side_pose_key.clone());
        output_keys.insert("front_landmarks".to_string(), front_landmarks_key);
        output_keys.insert("side_landmarks".to_string(), side_landmarks_key);

        logs.push_str("Pose estimation pipeline complete!\n");

        Ok(output_keys)
    }
}

/// Runs the Python pose estimation script on a video file.
async fn run_pose_estimation(
    input_path: &PathBuf,
    output_dir: &PathBuf,
    logs: &mut String,
) -> Result<()> {
    let output = Command::new("python3")
        .args([
            POSE_SCRIPT_PATH,
            input_path.to_str().context("Invalid input path")?,
            "-o",
            output_dir.to_str().context("Invalid output path")?,
            "--no-gpu",
            "--save-data",
        ])
        .output()
        .await
        .context("Failed to run pose estimation script")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        logs.push_str(&format!("Pose estimation stdout:\n{}\n", stdout));
    }
    if !stderr.is_empty() {
        logs.push_str(&format!("Pose estimation stderr:\n{}\n", stderr));
    }

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Pose estimation failed with status: {}",
            output.status
        ));
    }

    Ok(())
}

/// Re-encodes a video file from MPEG-4 Part 2 (mp4v) to H.264 (libx264)
/// so that it can be played in web browsers.
async fn reencode_to_h264(video_path: &PathBuf, logs: &mut String) -> Result<()> {
    let temp_output = video_path.with_extension("h264.mp4");

    logs.push_str(&format!(
        "Re-encoding {:?} to H.264 for browser compatibility...\n",
        video_path
    ));

    let output = Command::new("ffmpeg")
        .args([
            "-i",
            video_path.to_str().context("Invalid video path")?,
            "-c:v",
            "libx264",
            "-preset",
            "fast",
            "-crf",
            "23",
            "-movflags",
            "+faststart",
            "-an",
            "-y",
            temp_output.to_str().context("Invalid temp output path")?,
        ])
        .output()
        .await
        .context("Failed to run ffmpeg re-encode")?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        logs.push_str(&format!("ffmpeg stderr:\n{}\n", stderr));
    }

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "ffmpeg re-encode failed with status: {}",
            output.status
        ));
    }

    // Replace original file with the H.264 version
    fs::rename(&temp_output, video_path)
        .await
        .context("Failed to replace original video with H.264 version")?;

    logs.push_str("H.264 re-encode complete.\n");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting Stage 4 Pose Estimation worker...");
    run_stage_worker(PoseEstimationWorker).await
}
