//! Stage 1: Media Conversion Microservice
//! 
//! Converts uploaded videos to a standardized format:
//! - Resolution: 1920x1080 (padded if needed)
//! - Frame rate: 60fps
//! - Codec: H.264 (libx264)
//! - Audio: AAC 192kbps

use anyhow::{Context, Result};
use async_trait::async_trait;
use igait_lib::microservice::{
    run_stage_worker, ProcessingResult, QueueItem, StageNumber, StageWorker, StorageClient,
    VideoEditFlags, VideoTransform,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::fs;
use tokio::process::Command;

/// The media conversion worker.
pub struct MediaConversionWorker;

#[async_trait]
impl StageWorker for MediaConversionWorker {
    fn stage(&self) -> StageNumber {
        StageNumber::Stage1MediaConversion
    }

    fn service_name(&self) -> &'static str {
        "igait-stage1-media-conversion"
    }

    async fn process(&self, job: &QueueItem) -> ProcessingResult {
        let start_time = Instant::now();
        let mut logs = String::new();

        println!("Processing job {}: Media Conversion", job.job_id);
        logs.push_str(&format!("Starting media conversion for job {}\n", job.job_id));

        match self.do_conversion(job, &mut logs).await {
            Ok(output_keys) => {
                let duration = start_time.elapsed();
                logs.push_str(&format!("Conversion completed in {:?}\n", duration));
                
                ProcessingResult::Success {
                    output_keys,
                    logs,
                    duration_ms: duration.as_millis() as u64,
                }
            }
            Err(e) => {
                let duration = start_time.elapsed();
                eprintln!("Conversion failed for job {}: {}", job.job_id, e);
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

impl MediaConversionWorker {
    async fn do_conversion(
        &self,
        job: &QueueItem,
        logs: &mut String,
    ) -> Result<HashMap<String, String>> {
        let stage = StageNumber::Stage1MediaConversion;

        // Check for video edit flags in metadata.extra
        let video_edit: Option<VideoEditFlags> = job
            .metadata
            .extra
            .get("video_edit")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        if video_edit.is_some() {
            logs.push_str("Video edit flags detected — transforms will be applied.\n");
        }

        // Construct input paths from the job
        let front_input = job.input_front_video(stage);
        let side_input = job.input_side_video(stage);

        logs.push_str(&format!("Input front: {}\n", front_input));
        logs.push_str(&format!("Input side: {}\n", side_input));

        // Initialize storage client
        logs.push_str("Initializing storage client...\n");
        let storage = StorageClient::new().await
            .context("Failed to initialize storage client")?;

        // Create temporary directory for processing
        let temp_dir = PathBuf::from("/tmp").join(&job.job_id);
        fs::create_dir_all(&temp_dir).await
            .context("Failed to create temp directory")?;
        logs.push_str(&format!("Created temp directory: {:?}\n", temp_dir));

        // Download input files
        logs.push_str("Downloading front video from storage...\n");
        let front_data = storage.download(&front_input).await
            .context("Failed to download front video")?;
        let front_input_path = temp_dir.join("front_input.mp4");
        fs::write(&front_input_path, front_data).await
            .context("Failed to write front input file")?;
        logs.push_str(&format!("Downloaded front video ({} bytes)\n", fs::metadata(&front_input_path).await?.len()));

        logs.push_str("Downloading side video from storage...\n");
        let side_data = storage.download(&side_input).await
            .context("Failed to download side video")?;
        let side_input_path = temp_dir.join("side_input.mp4");
        fs::write(&side_input_path, side_data).await
            .context("Failed to write side input file")?;
        logs.push_str(&format!("Downloaded side video ({} bytes)\n", fs::metadata(&side_input_path).await?.len()));

        // Process videos
        let front_output_path = temp_dir.join("front.mp4");
        let side_output_path = temp_dir.join("side.mp4");

        let front_transform = video_edit.as_ref().and_then(|ve| ve.front.clone());
        let side_transform = video_edit.as_ref().and_then(|ve| ve.side.clone());

        logs.push_str("Converting front video...\n");
        standardize_video(&front_input_path, &front_output_path, front_transform.as_ref(), logs).await
            .context("Failed to convert front video")?;
        logs.push_str("Front video conversion done.\n");

        logs.push_str("Converting side video...\n");
        standardize_video(&side_input_path, &side_output_path, side_transform.as_ref(), logs).await
            .context("Failed to convert side video")?;
        logs.push_str("Side video conversion done.\n");

        // Construct output paths using helper methods
        let front_output_key = job.output_front_video(stage);
        let side_output_key = job.output_side_video(stage);
        
        logs.push_str("Uploading converted front video...\n");
        let front_output_data = fs::read(&front_output_path).await
            .context("Failed to read converted front video")?;
        storage.upload(&front_output_key, front_output_data, Some("video/mp4")).await
            .context("Failed to upload front video")?;
        logs.push_str(&format!("Uploaded front video to: {}\n", front_output_key));

        logs.push_str("Uploading converted side video...\n");
        let side_output_data = fs::read(&side_output_path).await
            .context("Failed to read converted side video")?;
        storage.upload(&side_output_key, side_output_data, Some("video/mp4")).await
            .context("Failed to upload side video")?;
        logs.push_str(&format!("Uploaded side video to: {}\n", side_output_key));

        // Clean up temporary files
        logs.push_str("Cleaning up temporary files...\n");
        fs::remove_dir_all(&temp_dir).await
            .context("Failed to clean up temp directory")?;

        // Return output keys for next stage
        let mut output_keys = HashMap::new();
        output_keys.insert("front_video".to_string(), front_output_key);
        output_keys.insert("side_video".to_string(), side_output_key);

        logs.push_str("Conversion complete!\n");
        
        Ok(output_keys)
    }
}

/// Builds the `-vf` filter string.
///
/// Order: crop → rotate → scale+pad (so crop coordinates are in source space).
fn build_video_filter(transform: Option<&VideoTransform>) -> String {
    let mut filters: Vec<String> = Vec::new();

    if let Some(t) = transform {
        // Crop
        if let (Some(w), Some(h)) = (t.crop_width, t.crop_height) {
            let x = t.crop_x.unwrap_or(0);
            let y = t.crop_y.unwrap_or(0);
            filters.push(format!("crop={}:{}:{}:{}", w, h, x, y));
        }

        // Rotation
        if let Some(deg) = t.rotation {
            match deg {
                90 => filters.push("transpose=1".to_string()),
                180 => filters.push("transpose=1,transpose=1".to_string()),
                270 => filters.push("transpose=2".to_string()),
                _ => {} // 0 or unsupported → no rotation
            }
        }
    }

    // Always finish with scale + pad to 1920×1080
    filters.push(
        "scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2"
            .to_string(),
    );

    filters.join(",")
}

/// Converts a video to the standardized format using FFmpeg,
/// optionally applying spatial/temporal transforms.
async fn standardize_video(
    input_file_path: &PathBuf,
    output_file_path: &PathBuf,
    transform: Option<&VideoTransform>,
    logs: &mut String,
) -> Result<()> {
    let vf = build_video_filter(transform);
    logs.push_str(&format!("Filter chain: {}\n", vf));

    let mut args: Vec<String> = Vec::new();
    args.push("-y".to_string());

    // Trim: -ss / -to placed before -i for fast seek
    if let Some(t) = transform {
        if let Some(start) = t.trim_start {
            args.push("-ss".to_string());
            args.push(format!("{:.3}", start));
        }
        if let Some(end) = t.trim_end {
            args.push("-to".to_string());
            args.push(format!("{:.3}", end));
        }
    }

    args.push("-i".to_string());
    args.push(
        input_file_path
            .to_str()
            .context("Invalid input path")?
            .to_string(),
    );
    args.extend_from_slice(&[
        "-vf".to_string(),
        vf,
        "-r".to_string(),
        "60".to_string(),
        "-b:v".to_string(),
        "5000k".to_string(),
        "-maxrate".to_string(),
        "5000k".to_string(),
        "-bufsize".to_string(),
        "10000k".to_string(),
        "-preset".to_string(),
        "fast".to_string(),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-c:a".to_string(),
        "aac".to_string(),
        "-b:a".to_string(),
        "192k".to_string(),
        output_file_path
            .to_str()
            .context("Invalid output path")?
            .to_string(),
    ]);

    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .await
        .context("Failed to run ffmpeg")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // FFmpeg outputs to stderr by default, so we log it
    if !stderr.is_empty() {
        logs.push_str(&format!("ffmpeg output: {}\n", stderr));
    }
    if !stdout.is_empty() {
        logs.push_str(&format!("ffmpeg stdout: {}\n", stdout));
    }

    if !output.status.success() {
        return Err(anyhow::anyhow!("ffmpeg failed with status: {}", output.status));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting Stage 1 Media Conversion worker...");
    run_stage_worker(MediaConversionWorker).await
}
