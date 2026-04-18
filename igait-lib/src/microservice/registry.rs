//! Central registry of pipeline stages — the single source of truth
//! for stage identity, ordering, and user-visible metadata.
//!
//! Adding, removing, or reordering stages is expressed by editing the
//! [`STAGES`] constant below. The backend publishes this list to RTDB
//! on startup; the frontend reads it from there and renders every stage
//! through a single uniform component.

use serde::Serialize;

/// Static description of a single pipeline stage.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct StageSpec {
    /// Kebab-case identifier — matches the folder name under
    /// `igait-stages/`, the Cargo crate/bin name, and the Docker image tag.
    pub key: &'static str,
    /// Human-readable name for logs and UI.
    pub display_name: &'static str,
    /// One-line summary for cards, tooltips, and list views.
    pub short_description: &'static str,
    /// Longer prose describing what this stage does and what it emits.
    /// Shown in admin detail panels.
    pub description: &'static str,
    /// True for the terminal stage, which handles email + archiving and
    /// uses a distinct queue schema (`FinalizeQueueItem`).
    pub terminal: bool,
}

/// The ordered pipeline. Position determines execution order.
pub const STAGES: &[StageSpec] = &[
    StageSpec {
        key: "media-conversion",
        display_name: "Media Conversion",
        short_description: "Standardize uploaded videos to 1920x1080 @ 60fps H.264.",
        description: "Converts front and side uploads to a single canonical format — 1920x1080 padded, 60fps, H.264/AAC — and applies any rotation, trim, or crop requested via the video-edit flags on the job. Downstream stages can assume uniform input.",
        terminal: false,
    },
    StageSpec {
        key: "validity-check",
        display_name: "Validity Check",
        short_description: "Confirm each video contains one walking person.",
        description: "Runs a YOLO + SlowFast + DeepSORT detection pipeline on both videos. A job proceeds only if a human is detected in both clips; otherwise the job errors out of the pipeline with a failure notification.",
        terminal: false,
    },
    StageSpec {
        key: "reframing",
        display_name: "Reframing",
        short_description: "Passthrough — reserved for automatic reframing.",
        description: "Placeholder stage that currently passes its inputs through unchanged. Reserved for future logic that reframes videos around the detected person's bounding box.",
        terminal: false,
    },
    StageSpec {
        key: "pose-estimation",
        display_name: "Pose Estimation",
        short_description: "Extract 3D body keypoints with MediaPipe.",
        description: "Runs MediaPipe Holistic on each video, producing per-frame 3D landmark JSON for downstream analysis plus pose-overlay preview videos re-encoded to browser-playable H.264.",
        terminal: false,
    },
    StageSpec {
        key: "cycle-detection",
        display_name: "Cycle Detection",
        short_description: "Identify individual gait cycles in pose data.",
        description: "Analyzes the landmark streams with rhythmic template matching to identify individual gait cycles. Emits a gait-analysis JSON per side, consumed by the prediction stage.",
        terminal: false,
    },
    StageSpec {
        key: "prediction",
        display_name: "Prediction",
        short_description: "Run ensemble ASD classification on gait features.",
        description: "Runs the ensemble ML model over the gait-cycle features and emits a prediction.json describing the classification outcome. The finalize stage interprets this payload.",
        terminal: false,
    },
    StageSpec {
        key: "finalize",
        display_name: "Finalize",
        short_description: "Send the result email and archive job outputs.",
        description: "Reads the prediction outcome from storage, sends the success or failure email to the submitter, and archives the job's artifacts. Terminal stage — does not enqueue further work.",
        terminal: true,
    },
];

/// Look up a stage by key.
pub fn stage_by_key(key: &str) -> Option<&'static StageSpec> {
    STAGES.iter().find(|s| s.key == key)
}

/// Look up a stage by its position (0-indexed).
pub fn stage_by_index(idx: usize) -> Option<&'static StageSpec> {
    STAGES.get(idx)
}

/// Position (0-indexed) of a stage by key.
pub fn stage_index_of(key: &str) -> Option<usize> {
    STAGES.iter().position(|s| s.key == key)
}

/// The stage that follows the one with the given key.
/// `None` if the key is unknown or the stage is terminal.
pub fn stage_after(key: &str) -> Option<&'static StageSpec> {
    let idx = stage_index_of(key)?;
    STAGES.get(idx + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn stages_non_empty() {
        assert!(!STAGES.is_empty());
    }

    #[test]
    fn keys_are_unique() {
        let mut seen = HashSet::new();
        for s in STAGES {
            assert!(seen.insert(s.key), "duplicate stage key: {}", s.key);
        }
    }

    #[test]
    fn keys_are_kebab_case_non_empty() {
        for s in STAGES {
            assert!(!s.key.is_empty(), "stage key is empty");
            assert!(
                s.key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "stage key {:?} must be kebab-case (a-z, 0-9, -)",
                s.key
            );
            assert!(!s.key.starts_with('-') && !s.key.ends_with('-'),
                "stage key {:?} must not start or end with -", s.key);
        }
    }

    #[test]
    fn exactly_one_terminal_and_it_is_last() {
        let terminal_indices: Vec<usize> = STAGES
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.terminal.then_some(i))
            .collect();
        assert_eq!(terminal_indices.len(), 1, "exactly one terminal stage expected");
        assert_eq!(terminal_indices[0], STAGES.len() - 1, "terminal stage must be last");
    }

    #[test]
    fn stage_after_terminal_is_none() {
        let last = STAGES.last().unwrap();
        assert!(stage_after(last.key).is_none());
    }

    #[test]
    fn stage_after_first_is_second() {
        assert_eq!(
            stage_after(STAGES[0].key).map(|s| s.key),
            Some(STAGES[1].key),
        );
    }

    #[test]
    fn stage_by_key_round_trips() {
        for s in STAGES {
            let found = stage_by_key(s.key).expect("known key must resolve");
            assert_eq!(found.key, s.key);
        }
    }

    #[test]
    fn stage_by_key_unknown_returns_none() {
        assert!(stage_by_key("no-such-stage").is_none());
    }

    #[test]
    fn every_stage_has_non_empty_metadata() {
        for s in STAGES {
            assert!(!s.display_name.is_empty(), "{} has empty display_name", s.key);
            assert!(!s.short_description.is_empty(), "{} has empty short_description", s.key);
            assert!(!s.description.is_empty(), "{} has empty description", s.key);
        }
    }
}
