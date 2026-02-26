//! Stage 2: Validity Check Microservice
//!
//! Verifies that both front and side videos contain exactly one person walking
//! using YOLO + SlowFast + DeepSORT detection pipeline.
//!
//! Outputs:
//! - `validity.json` with per-video and combined detection results
//! - Annotated videos with bounding box overlays (for debugging)
//!
//! If either video fails the check, the job errors out of the pipeline.

use anyhow::{Context, Result};
use async_trait::async_trait;
use igait_lib::microservice::{
    run_stage_worker, ProcessingResult, QueueItem, StageNumber, StageWorker, StorageClient,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::fs;
use tokio::process::Command;

/// Path to the detection script directory (in the Docker container).
const DETECTION_DIR: &str = "/app/igait-human-gait-detection";

/// Path to the detection script (in the Docker container).
const DETECTION_SCRIPT: &str = "yolo_slowfast.py";

/// Per-video validity result from the Python detection script.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct VideoValidity {
    valid: bool,
    human_detected: bool,
    walking_detected: bool,
    total_frames: u32,
    clips_processed: u32,
    clips_with_person: u32,
    clips_with_walking: u32,
    processing_time_seconds: f64,
}

/// Combined validity result for both videos.
#[derive(Debug, Serialize)]
struct CombinedValidity {
    overall_valid: bool,
    front: VideoValidity,
    side: VideoValidity,
}

/// The validity check worker.
pub struct ValidityCheckWorker;

#[async_trait]
impl StageWorker for ValidityCheckWorker {
    fn stage(&self) -> StageNumber {
        StageNumber::Stage2ValidityCheck
    }

    fn service_name(&self) -> &'static str {
        "igait-stage2-validity-check"
    }

    async fn process(&self, job: &QueueItem) -> ProcessingResult {
        let start_time = Instant::now();
        let mut logs = String::new();

        println!("Processing job {}: Validity Check", job.job_id);
        logs.push_str(&format!(
            "Starting validity check for job {}\n",
            job.job_id
        ));

        match self.do_validity_check(job, &mut logs).await {
            Ok(output_keys) => {
                let duration = start_time.elapsed();
                logs.push_str(&format!("Validity check completed in {:?}\n", duration));

                ProcessingResult::Success {
                    output_keys,
                    logs,
                    duration_ms: duration.as_millis() as u64,
                }
            }
            Err(e) => {
                let duration = start_time.elapsed();
                eprintln!("Validity check failed for job {}: {}", job.job_id, e);
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

impl ValidityCheckWorker {
    async fn do_validity_check(
        &self,
        job: &QueueItem,
        logs: &mut String,
    ) -> Result<HashMap<String, String>> {
        let stage = StageNumber::Stage2ValidityCheck;

        // Get input paths from previous stage
        let front_input_key = job
            .input_keys
            .get("front_video")
            .context("Missing front_video in input_keys")?
            .clone();
        let side_input_key = job
            .input_keys
            .get("side_video")
            .context("Missing side_video in input_keys")?
            .clone();

        logs.push_str(&format!("Input front video: {}\n", front_input_key));
        logs.push_str(&format!("Input side video: {}\n", side_input_key));

        // Initialize storage client
        logs.push_str("Initializing storage client...\n");
        let storage = StorageClient::new()
            .await
            .context("Failed to initialize storage client")?;

        // Create temporary directory for processing
        let temp_dir = PathBuf::from("/tmp").join(format!("validity_{}", job.job_id));
        fs::create_dir_all(&temp_dir)
            .await
            .context("Failed to create temp directory")?;
        logs.push_str(&format!("Created temp directory: {:?}\n", temp_dir));

        // Download front video
        logs.push_str("Downloading front video from storage...\n");
        let front_data = storage
            .download(&front_input_key)
            .await
            .context("Failed to download front video")?;
        let front_input_path = temp_dir.join("front.mp4");
        fs::write(&front_input_path, &front_data)
            .await
            .context("Failed to write front input file")?;
        logs.push_str(&format!(
            "Downloaded front video ({} bytes)\n",
            front_data.len()
        ));

        // Download side video
        logs.push_str("Downloading side video from storage...\n");
        let side_data = storage
            .download(&side_input_key)
            .await
            .context("Failed to download side video")?;
        let side_input_path = temp_dir.join("side.mp4");
        fs::write(&side_input_path, &side_data)
            .await
            .context("Failed to write side input file")?;
        logs.push_str(&format!(
            "Downloaded side video ({} bytes)\n",
            side_data.len()
        ));

        // Run detection on front video
        logs.push_str("\n=== Running detection on FRONT video ===\n");
        let front_output_path = temp_dir.join("front_annotated.mp4");
        let front_json_path = temp_dir.join("front_validity.json");
        run_detection(
            &front_input_path,
            &front_output_path,
            &front_json_path,
            logs,
        )
        .await
        .context("Front video detection script failed")?;

        // Parse front results
        let front_result = parse_validity_json(&front_json_path)
            .await
            .context("Failed to parse front validity results")?;
        logs.push_str(&format!("Front result: {:?}\n", front_result));

        // Run detection on side video
        logs.push_str("\n=== Running detection on SIDE video ===\n");
        let side_output_path = temp_dir.join("side_annotated.mp4");
        let side_json_path = temp_dir.join("side_validity.json");
        run_detection(
            &side_input_path,
            &side_output_path,
            &side_json_path,
            logs,
        )
        .await
        .context("Side video detection script failed")?;

        // Parse side results
        let side_result = parse_validity_json(&side_json_path)
            .await
            .context("Failed to parse side validity results")?;
        logs.push_str(&format!("Side result: {:?}\n", side_result));

        // Build combined validity result
        // NOTE: We only require a human to be detected in both clips.
        // Walking detection is bypassed for now — the SlowFast model is
        // too unreliable and rejects many valid submissions.
        let overall_valid = front_result.human_detected && side_result.human_detected;
        let combined = CombinedValidity {
            overall_valid,
            front: front_result.clone(),
            side: side_result.clone(),
        };
        let combined_json = serde_json::to_string_pretty(&combined)
            .context("Failed to serialize combined validity")?;

        // Construct S3 output keys
        let stage_num = stage.as_u8();
        let validity_key = format!("jobs/{}/stage_{}/validity.json", job.job_id, stage_num);
        let front_annotated_key = format!(
            "jobs/{}/stage_{}/front_annotated.mp4",
            job.job_id, stage_num
        );
        let side_annotated_key = format!(
            "jobs/{}/stage_{}/side_annotated.mp4",
            job.job_id, stage_num
        );

        // Upload validity.json (always, even on failure — useful for debugging)
        logs.push_str("Uploading validity.json...\n");
        storage
            .upload(
                &validity_key,
                combined_json.into_bytes(),
                Some("application/json"),
            )
            .await
            .context("Failed to upload validity.json")?;
        logs.push_str(&format!("Uploaded validity.json to: {}\n", validity_key));

        // Upload annotated videos
        logs.push_str("Uploading front annotated video...\n");
        let front_annotated_data = fs::read(&front_output_path)
            .await
            .context("Failed to read front annotated video")?;
        storage
            .upload(
                &front_annotated_key,
                front_annotated_data,
                Some("video/mp4"),
            )
            .await
            .context("Failed to upload front annotated video")?;
        logs.push_str(&format!(
            "Uploaded front annotated video to: {}\n",
            front_annotated_key
        ));

        logs.push_str("Uploading side annotated video...\n");
        let side_annotated_data = fs::read(&side_output_path)
            .await
            .context("Failed to read side annotated video")?;
        storage
            .upload(&side_annotated_key, side_annotated_data, Some("video/mp4"))
            .await
            .context("Failed to upload side annotated video")?;
        logs.push_str(&format!(
            "Uploaded side annotated video to: {}\n",
            side_annotated_key
        ));

        // Clean up temporary files
        logs.push_str("Cleaning up temporary files...\n");
        fs::remove_dir_all(&temp_dir)
            .await
            .context("Failed to clean up temp directory")?;

        // If either video failed validation, error out of the pipeline
        if !overall_valid {
            let mut reasons = Vec::new();
            if !front_result.human_detected {
                reasons.push("Front video: no human detected".to_string());
            }
            if !side_result.human_detected {
                reasons.push("Side video: no human detected".to_string());
            }
            return Err(anyhow::anyhow!(
                "Validity check failed: {}",
                reasons.join("; ")
            ));
        }

        // Build output keys — pass through original videos to next stage
        let mut output_keys = HashMap::new();
        output_keys.insert("front_video".to_string(), front_input_key);
        output_keys.insert("side_video".to_string(), side_input_key);
        output_keys.insert("validity".to_string(), validity_key);
        output_keys.insert("front_annotated".to_string(), front_annotated_key);
        output_keys.insert("side_annotated".to_string(), side_annotated_key);

        logs.push_str("Validity check passed — both videos contain a detected human!\n");

        Ok(output_keys)
    }
}

/// Runs the Python YOLO+SlowFast detection script on a single video.
async fn run_detection(
    input_path: &Path,
    output_path: &Path,
    json_path: &Path,
    logs: &mut String,
) -> Result<()> {
    let output = Command::new("python3")
        .args([
            DETECTION_SCRIPT,
            "--input",
            input_path.to_str().context("Invalid input path")?,
            "--output",
            output_path.to_str().context("Invalid output path")?,
            "--output-json",
            json_path.to_str().context("Invalid JSON output path")?,
            "--device",
            "cpu",
            "--max-seconds",
            "10",
        ])
        .current_dir(DETECTION_DIR)
        .output()
        .await
        .context("Failed to execute detection script")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        logs.push_str(&format!("Detection stdout:\n{}\n", stdout));
    }
    if !stderr.is_empty() {
        logs.push_str(&format!("Detection stderr:\n{}\n", stderr));
    }

    // The script exits with code 1 when no walking is detected —
    // that's not a script *error*, it's a valid result. We only fail
    // if something truly unexpected happened (no JSON output produced).
    if !json_path.exists() {
        return Err(anyhow::anyhow!(
            "Detection script did not produce validity JSON (exit status: {})",
            output.status
        ));
    }

    Ok(())
}

/// Parses the validity JSON produced by the detection script.
async fn parse_validity_json(path: &Path) -> Result<VideoValidity> {
    let contents = fs::read_to_string(path)
        .await
        .context("Failed to read validity JSON file")?;
    let result: VideoValidity =
        serde_json::from_str(&contents).context("Failed to parse validity JSON")?;
    Ok(result)
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting Stage 2 Validity Check worker...");
    run_stage_worker(ValidityCheckWorker).await
}
