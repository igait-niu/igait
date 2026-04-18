//! Cycle Detection Microservice
//!
//! Analyzes pose landmark data from stage 4 to identify individual gait cycles
//! using rhythmic template matching. Runs the `gait_analysis_mediapipe.py` Python
//! script on each side's landmarks JSON, producing gait cycle indices and
//! passing through the landmark data.

use anyhow::{Context, Result};
use async_trait::async_trait;
use igait_lib::microservice::{
    run_stage_job, run_stage_worker, ProcessingResult, QueueItem, StageId, StageWorker,
    StorageClient,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::fs;
use tokio::process::Command;

/// Path to the Python gait cycle detection script (in the Docker container).
const GAIT_SCRIPT_PATH: &str = "/app/gait_analysis_mediapipe.py";

/// The cycle detection worker.
pub struct CycleDetectionWorker;

#[async_trait]
impl StageWorker for CycleDetectionWorker {
    fn stage(&self) -> StageId {
        StageId::new("cycle-detection")
    }

    fn service_name(&self) -> &'static str {
        "cycle-detection"
    }

    async fn process(&self, job: &QueueItem) -> ProcessingResult {
        let start_time = Instant::now();
        let mut logs = String::new();

        println!("Processing job {}: Cycle Detection", job.job_id);
        logs.push_str(&format!(
            "Starting cycle detection for job {}\n",
            job.job_id
        ));

        match self.do_cycle_detection(job, &mut logs).await {
            Ok(output_keys) => {
                let duration = start_time.elapsed();
                logs.push_str(&format!("Cycle detection completed in {:?}\n", duration));

                ProcessingResult::Success {
                    output_keys,
                    logs,
                    duration_ms: duration.as_millis() as u64,
                }
            }
            Err(e) => {
                let duration = start_time.elapsed();
                eprintln!("Cycle detection failed for job {}: {}", job.job_id, e);
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

impl CycleDetectionWorker {
    async fn do_cycle_detection(
        &self,
        job: &QueueItem,
        logs: &mut String,
    ) -> Result<HashMap<String, String>> {
        let stage = StageId::new("cycle-detection");

        // Get input landmark keys from stage 4
        let front_landmarks_input = job
            .input_keys
            .get("front_landmarks")
            .context("Missing front_landmarks in input_keys")?
            .clone();
        let side_landmarks_input = job
            .input_keys
            .get("side_landmarks")
            .context("Missing side_landmarks in input_keys")?
            .clone();

        // Pass through videos from stage 4
        let front_video_input = job
            .input_keys
            .get("front_video")
            .context("Missing front_video in input_keys")?
            .clone();
        let side_video_input = job
            .input_keys
            .get("side_video")
            .context("Missing side_video in input_keys")?
            .clone();

        logs.push_str(&format!("Input front landmarks: {}\n", front_landmarks_input));
        logs.push_str(&format!("Input side landmarks: {}\n", side_landmarks_input));
        logs.push_str(&format!("Input front video: {}\n", front_video_input));
        logs.push_str(&format!("Input side video: {}\n", side_video_input));

        // Initialize storage client
        logs.push_str("Initializing storage client...\n");
        let storage = StorageClient::new()
            .await
            .context("Failed to initialize storage client")?;

        // Create temporary directory for processing
        let temp_dir = PathBuf::from("/tmp").join(format!("cycles_{}", job.job_id));
        fs::create_dir_all(&temp_dir)
            .await
            .context("Failed to create temp directory")?;
        logs.push_str(&format!("Created temp directory: {:?}\n", temp_dir));

        // Create output directories for each side
        let front_output_dir = temp_dir.join("output_front");
        let side_output_dir = temp_dir.join("output_side");
        fs::create_dir_all(&front_output_dir).await?;
        fs::create_dir_all(&side_output_dir).await?;

        // Download front landmarks JSON
        logs.push_str("Downloading front landmarks from storage...\n");
        let front_landmarks_data = storage
            .download(&front_landmarks_input)
            .await
            .context("Failed to download front landmarks")?;
        let front_landmarks_path = temp_dir.join("front_landmarks.json");
        fs::write(&front_landmarks_path, &front_landmarks_data)
            .await
            .context("Failed to write front landmarks file")?;
        logs.push_str(&format!(
            "Downloaded front landmarks ({} bytes)\n",
            front_landmarks_data.len()
        ));

        // Download side landmarks JSON
        logs.push_str("Downloading side landmarks from storage...\n");
        let side_landmarks_data = storage
            .download(&side_landmarks_input)
            .await
            .context("Failed to download side landmarks")?;
        let side_landmarks_path = temp_dir.join("side_landmarks.json");
        fs::write(&side_landmarks_path, &side_landmarks_data)
            .await
            .context("Failed to write side landmarks file")?;
        logs.push_str(&format!(
            "Downloaded side landmarks ({} bytes)\n",
            side_landmarks_data.len()
        ));

        // Run gait cycle detection on front landmarks
        logs.push_str("Running gait cycle detection on front landmarks...\n");
        run_gait_cycle_detection(
            &front_landmarks_path,
            &front_output_dir,
            &format!("{}_front", job.job_id),
            logs,
        )
        .await
        .context("Failed to run gait cycle detection on front landmarks")?;
        logs.push_str("Front gait cycle detection complete.\n");

        // Run gait cycle detection on side landmarks
        logs.push_str("Running gait cycle detection on side landmarks...\n");
        run_gait_cycle_detection(
            &side_landmarks_path,
            &side_output_dir,
            &format!("{}_side", job.job_id),
            logs,
        )
        .await
        .context("Failed to run gait cycle detection on side landmarks")?;
        logs.push_str("Side gait cycle detection complete.\n");

        // Construct output storage keys
        let stage_num = stage.position();
        let front_gait_key = format!(
            "jobs/{}/stage_{}/front_gait_analysis.json",
            job.job_id, stage_num
        );
        let side_gait_key = format!(
            "jobs/{}/stage_{}/side_gait_analysis.json",
            job.job_id, stage_num
        );

        // Upload front gait analysis JSON
        let front_gait_path = front_output_dir.join(format!(
            "{}_front_gait_analysis.json",
            job.job_id
        ));
        logs.push_str(&format!(
            "Uploading front gait analysis from {:?}...\n",
            front_gait_path
        ));
        let front_gait_data = fs::read(&front_gait_path)
            .await
            .context("Failed to read front gait analysis JSON")?;
        storage
            .upload(&front_gait_key, front_gait_data, Some("application/json"))
            .await
            .context("Failed to upload front gait analysis JSON")?;
        logs.push_str(&format!(
            "Uploaded front gait analysis to: {}\n",
            front_gait_key
        ));

        // Upload side gait analysis JSON
        let side_gait_path = side_output_dir.join(format!(
            "{}_side_gait_analysis.json",
            job.job_id
        ));
        logs.push_str(&format!(
            "Uploading side gait analysis from {:?}...\n",
            side_gait_path
        ));
        let side_gait_data = fs::read(&side_gait_path)
            .await
            .context("Failed to read side gait analysis JSON")?;
        storage
            .upload(&side_gait_key, side_gait_data, Some("application/json"))
            .await
            .context("Failed to upload side gait analysis JSON")?;
        logs.push_str(&format!(
            "Uploaded side gait analysis to: {}\n",
            side_gait_key
        ));

        // Clean up temporary files
        logs.push_str("Cleaning up temporary files...\n");
        fs::remove_dir_all(&temp_dir)
            .await
            .context("Failed to clean up temp directory")?;

        // Return output keys for next stage
        // Pass through video keys + add gait analysis keys
        let mut output_keys = HashMap::new();
        output_keys.insert("front_video".to_string(), front_video_input);
        output_keys.insert("side_video".to_string(), side_video_input);
        output_keys.insert("front_landmarks".to_string(), front_landmarks_input);
        output_keys.insert("side_landmarks".to_string(), side_landmarks_input);
        output_keys.insert("front_gait_analysis".to_string(), front_gait_key);
        output_keys.insert("side_gait_analysis".to_string(), side_gait_key);

        logs.push_str("Cycle detection pipeline complete!\n");

        Ok(output_keys)
    }
}

/// Runs the Python gait cycle detection script on a landmarks JSON file.
async fn run_gait_cycle_detection(
    landmarks_path: &PathBuf,
    output_dir: &PathBuf,
    subject_id: &str,
    logs: &mut String,
) -> Result<()> {
    let output = Command::new("python3")
        .args([
            GAIT_SCRIPT_PATH,
            "--landmarks-json",
            landmarks_path.to_str().context("Invalid landmarks path")?,
            "--output-dir",
            output_dir.to_str().context("Invalid output path")?,
            "--subject-id",
            subject_id,
        ])
        .output()
        .await
        .context("Failed to run gait cycle detection script")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        logs.push_str(&format!("Gait cycle detection stdout:\n{}\n", stdout));
    }
    if !stderr.is_empty() {
        logs.push_str(&format!("Gait cycle detection stderr:\n{}\n", stderr));
    }

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Gait cycle detection failed with status: {}",
            output.status
        ));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var("IGAIT_JOB_PAYLOAD").is_ok() {
        run_stage_job(CycleDetectionWorker).await
    } else {
        println!("Starting Cycle Detection worker...");
        run_stage_worker(CycleDetectionWorker).await
    }
}
