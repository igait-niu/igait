//! Central registry of pipeline stages — the single source of truth
//! for stage identity, ordering, and UI metadata.
//!
//! Adding, removing, or reordering stages is expressed by editing the
//! [`STAGES`] constant below. The backend publishes this registry to
//! RTDB on launch; the frontend reads it from there to render stage
//! tabs generically.

use serde::Serialize;

/// Static description of a single pipeline stage.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct StageSpec {
    /// Kebab-case identifier — matches the folder name under
    /// `igait-stages/`, the Cargo crate/bin name, and the Docker image tag.
    pub key: &'static str,
    /// Human-readable name for logs and UI.
    pub display_name: &'static str,
    /// One-line description shown in the admin UI.
    pub description: &'static str,
    /// True for the terminal stage, which handles email + archiving and
    /// uses a distinct queue schema (`FinalizeQueueItem`).
    pub terminal: bool,
    /// Which custom UI panel (if any) the frontend should mount for
    /// this stage's tab. Everything else uses the default tab layout.
    pub panel: StagePanel,
}

/// Closed set of custom UI panels the frontend can mount inside a
/// generic `<StageTab>`. Adding a new panel variant is a deliberate
/// cross-language change: a new Rust variant here, a new Svelte
/// component keyed by the variant on the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StagePanel {
    /// No custom panel — the generic `<StageTab>` renders status, logs,
    /// and artifacts only.
    Default,
    /// Rotation / crop / trim flags applied on media-conversion rerun.
    VideoEdit,
    /// Admin override of the gait-cycle index array produced by
    /// cycle-detection.
    GaitCycles,
}

/// The ordered pipeline. Position determines execution order.
pub const STAGES: &[StageSpec] = &[
    StageSpec {
        key: "media-conversion",
        display_name: "Media Conversion",
        description: "Standardizes uploaded videos — resolution, frame rate, codec, and optional rotation/crop/trim.",
        terminal: false,
        panel: StagePanel::VideoEdit,
    },
    StageSpec {
        key: "validity-check",
        display_name: "Validity Check",
        description: "Verifies both front and side videos contain exactly one person walking, via YOLO + SlowFast + DeepSORT.",
        terminal: false,
        panel: StagePanel::Default,
    },
    StageSpec {
        key: "reframing",
        display_name: "Reframing",
        description: "Adjusts video framing and cropping based on detected person position (currently a pass-through placeholder).",
        terminal: false,
        panel: StagePanel::Default,
    },
    StageSpec {
        key: "pose-estimation",
        display_name: "Pose Estimation",
        description: "Extracts body keypoints using MediaPipe's Holistic model, producing pose overlay videos and landmark JSON.",
        terminal: false,
        panel: StagePanel::Default,
    },
    StageSpec {
        key: "cycle-detection",
        display_name: "Cycle Detection",
        description: "Analyzes pose landmarks to identify individual gait cycles via rhythmic template matching.",
        terminal: false,
        panel: StagePanel::GaitCycles,
    },
    StageSpec {
        key: "prediction",
        display_name: "Prediction",
        description: "Runs the ML ensemble to classify ASD from gait analysis data.",
        terminal: false,
        panel: StagePanel::Default,
    },
    StageSpec {
        key: "finalize",
        display_name: "Finalize",
        description: "Sends the result email, archives artifacts, and closes out the job.",
        terminal: true,
        panel: StagePanel::Default,
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
    fn descriptions_non_empty() {
        for s in STAGES {
            assert!(!s.description.is_empty(), "stage {:?} has empty description", s.key);
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
    fn panel_serializes_as_kebab_case() {
        assert_eq!(serde_json::to_string(&StagePanel::Default).unwrap(), "\"default\"");
        assert_eq!(serde_json::to_string(&StagePanel::VideoEdit).unwrap(), "\"video-edit\"");
        assert_eq!(serde_json::to_string(&StagePanel::GaitCycles).unwrap(), "\"gait-cycles\"");
    }

    #[test]
    fn stage_spec_serializes_to_expected_shape() {
        let spec = &STAGES[0];
        let json = serde_json::to_value(spec).unwrap();
        assert_eq!(json["key"], "media-conversion");
        assert_eq!(json["display_name"], "Media Conversion");
        assert!(json["description"].as_str().unwrap().len() > 0);
        assert_eq!(json["terminal"], false);
        assert_eq!(json["panel"], "video-edit");
    }
}
