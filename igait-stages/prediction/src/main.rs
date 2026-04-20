//! Prediction Microservice
//!
//! Runs the ML model to predict ASD classification from gait data.
//! Downloads front/side gait analysis JSONs from stage 5, invokes the
//! Python prediction pipeline (`iGAIT_MODEL_IO/main.py`), and uploads
//! the resulting `prediction.json` for stage 7 to consume.

use anyhow::{Context, Result};
use async_trait::async_trait;
use igait_lib::microservice::{
    check_env, run_stage_job, run_stage_worker, ProcessingResult, QueueItem, StageId, StageWorker,
    StorageClient, STAGE_REQUIRED_ENV,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::fs;
use tokio::process::Command;

/// Path to the Python prediction entry point (in the Docker container).
///
/// Locally you can override this via the `PREDICT_SCRIPT_PATH` env var.
const PREDICT_SCRIPT_PATH: &str = "/app/iGAIT_MODEL_IO/main.py";

/// The prediction worker.
pub struct PredictionWorker;

#[async_trait]
impl StageWorker for PredictionWorker {
    fn stage(&self) -> StageId {
        StageId::new("prediction")
    }

    fn service_name(&self) -> &'static str {
        "prediction"
    }

    async fn process(&self, job: &QueueItem) -> ProcessingResult {
        let start_time = Instant::now();
        let mut logs = String::new();

        println!("Processing job {}: Prediction", job.job_id);
        logs.push_str(&format!("Starting prediction for job {}\n", job.job_id));

        match self.do_prediction(job, &mut logs).await {
            Ok(output_keys) => {
                let duration = start_time.elapsed();
                logs.push_str(&format!("Prediction completed in {:?}\n", duration));

                ProcessingResult::Success {
                    output_keys,
                    logs,
                    duration_ms: duration.as_millis() as u64,
                }
            }
            Err(e) => {
                let duration = start_time.elapsed();
                eprintln!("Prediction failed for job {}: {}", job.job_id, e);
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

impl PredictionWorker {
    async fn do_prediction(
        &self,
        job: &QueueItem,
        logs: &mut String,
    ) -> Result<HashMap<String, String>> {
        let stage = StageId::new("prediction");

        // Get input keys from stage 5
        let front_gait_input = job
            .input_keys
            .get("front_gait_analysis")
            .context("Missing front_gait_analysis in input_keys")?
            .clone();
        let side_gait_input = job
            .input_keys
            .get("side_gait_analysis")
            .context("Missing side_gait_analysis in input_keys")?
            .clone();

        logs.push_str(&format!(
            "Input front gait analysis: {}\n",
            front_gait_input
        ));
        logs.push_str(&format!(
            "Input side gait analysis: {}\n",
            side_gait_input
        ));

        // Initialize storage client
        logs.push_str("Initializing storage client...\n");
        let storage = StorageClient::new()
            .await
            .context("Failed to initialize storage client")?;

        // Create temporary directory for processing
        let temp_dir = PathBuf::from("/tmp").join(format!("predict_{}", job.job_id));
        fs::create_dir_all(&temp_dir)
            .await
            .context("Failed to create temp directory")?;
        logs.push_str(&format!("Created temp directory: {:?}\n", temp_dir));

        // Create output directory for prediction results
        let output_dir = temp_dir.join("output");
        fs::create_dir_all(&output_dir).await?;

        // Download front gait analysis JSON
        logs.push_str("Downloading front gait analysis from storage...\n");
        let front_gait_data = storage
            .download(&front_gait_input)
            .await
            .context("Failed to download front gait analysis")?;
        let front_gait_path = temp_dir.join("front_gait_analysis.json");
        fs::write(&front_gait_path, &front_gait_data)
            .await
            .context("Failed to write front gait analysis file")?;
        logs.push_str(&format!(
            "Downloaded front gait analysis ({} bytes)\n",
            front_gait_data.len()
        ));

        // Download side gait analysis JSON
        logs.push_str("Downloading side gait analysis from storage...\n");
        let side_gait_data = storage
            .download(&side_gait_input)
            .await
            .context("Failed to download side gait analysis")?;
        let side_gait_path = temp_dir.join("side_gait_analysis.json");
        fs::write(&side_gait_path, &side_gait_data)
            .await
            .context("Failed to write side gait analysis file")?;
        logs.push_str(&format!(
            "Downloaded side gait analysis ({} bytes)\n",
            side_gait_data.len()
        ));

        // Run prediction script
        logs.push_str("Running prediction script...\n");
        let output = Command::new("python3")
            .args([
                PREDICT_SCRIPT_PATH,
                "predict",
                "--model",
                "mediapipe",
                "--side",
                side_gait_path.to_str().context("Invalid side path")?,
                "--front",
                front_gait_path.to_str().context("Invalid front path")?,
                "--out_dir",
                output_dir.to_str().context("Invalid output path")?,
                "--env",
                "PROD",
            ])
            .output()
            .await
            .context("Failed to run prediction script")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stdout.is_empty() {
            logs.push_str(&format!("Prediction stdout:\n{}\n", stdout));
        }
        if !stderr.is_empty() {
            logs.push_str(&format!("Prediction stderr:\n{}\n", stderr));
        }

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Prediction script failed with status: {}",
                output.status
            ));
        }

        // The Python script writes <subject>.json in the output dir.
        // The subject name is the side file's stem: "side_gait_analysis"
        let result_path = output_dir.join("side_gait_analysis.json");
        logs.push_str(&format!(
            "Reading prediction result from {:?}...\n",
            result_path
        ));
        let result_data = fs::read(&result_path)
            .await
            .context("Failed to read prediction result JSON")?;

        // Validate the JSON is parseable before uploading
        let prediction_result: serde_json::Value =
            serde_json::from_slice(&result_data).context("Failed to parse prediction result")?;
        logs.push_str(&format!(
            "Prediction result: {}\n",
            serde_json::to_string_pretty(&prediction_result)?
        ));

        // Upload the raw prediction result as prediction.json for stage 7.
        // Stage 7 is responsible for interpreting the result (averaging
        // probabilities, checking status, etc.).
        let prediction_key = format!("jobs/{}/{}/prediction.json", job.job_id, stage.key());
        logs.push_str(&format!(
            "Uploading prediction.json to {}...\n",
            prediction_key
        ));
        storage
            .upload(&prediction_key, result_data, Some("application/json"))
            .await
            .context("Failed to upload prediction.json")?;

        // Clean up temporary files
        logs.push_str("Cleaning up temporary files...\n");
        fs::remove_dir_all(&temp_dir)
            .await
            .context("Failed to clean up temp directory")?;

        // Return output keys for stage 7
        let mut output_keys = HashMap::new();
        output_keys.insert("prediction".to_string(), prediction_key);

        logs.push_str("Prediction pipeline complete!\n");

        Ok(output_keys)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    check_env(STAGE_REQUIRED_ENV)?;
    if std::env::var("IGAIT_JOB_PAYLOAD").is_ok() {
        run_stage_job(PredictionWorker).await
    } else {
        println!("Starting Prediction worker...");
        run_stage_worker(PredictionWorker).await
    }
}
