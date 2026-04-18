//! Reframing Microservice
//!
//! Adjusts video framing and cropping based on detected person position.
//!
//! NOTE: This is currently a placeholder that passes through immediately.

use anyhow::Result;
use async_trait::async_trait;
use igait_lib::microservice::{
    run_stage_job, run_stage_worker, ProcessingResult, QueueItem, StageNumber, StageWorker,
};
use std::collections::HashMap;
use std::time::Instant;

/// The reframing worker.
pub struct ReframingWorker;

#[async_trait]
impl StageWorker for ReframingWorker {
    fn stage(&self) -> StageNumber {
        StageNumber::Stage3Reframing
    }

    fn service_name(&self) -> &'static str {
        "reframing"
    }

    async fn process(&self, job: &QueueItem) -> ProcessingResult {
        let start_time = Instant::now();
        let mut logs = String::new();

        println!("Processing job {}: Reframing (pass-through)", job.job_id);
        logs.push_str(&format!("Starting reframing for job {}\n", job.job_id));

        let stage = StageNumber::Stage3Reframing;

        // Get input paths (from stage 2)
        let front_input = job.input_front_video(stage);
        let side_input = job.input_side_video(stage);

        logs.push_str(&format!("Input front video: {}\n", front_input));
        logs.push_str(&format!("Input side video: {}\n", side_input));

        // For now, just pass through - output the same paths as input
        // TODO: Implement actual reframing logic
        let mut output_keys = HashMap::new();
        output_keys.insert("front_video".to_string(), front_input);
        output_keys.insert("side_video".to_string(), side_input);

        logs.push_str("Reframing complete (placeholder - no actual reframing performed)\n");

        let duration = start_time.elapsed();
        logs.push_str(&format!("Completed in {:?}\n", duration));

        ProcessingResult::Success {
            output_keys,
            logs,
            duration_ms: duration.as_millis() as u64,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var("IGAIT_JOB_PAYLOAD").is_ok() {
        run_stage_job(ReframingWorker).await
    } else {
        println!("Starting Reframing worker...");
        run_stage_worker(ReframingWorker).await
    }
}
