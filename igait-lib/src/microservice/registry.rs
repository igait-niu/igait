//! Central registry of pipeline stages — the single source of truth
//! for stage identity, ordering, UI metadata, and inter-stage I/O.
//!
//! Adding, removing, or reordering stages is expressed by editing the
//! [`STAGES`] constant below. The backend publishes this registry to
//! RTDB on launch; the frontend reads it from there to render stage
//! tabs generically. The backend's rerun flow also uses each stage's
//! declared [`inputs`](StageSpec::inputs) to reconstruct
//! `input_keys` without hardcoding stage knowledge anywhere else.

use serde::Serialize;
use std::collections::HashMap;

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
    /// The input artifacts this stage reads. Each entry names a key
    /// placed in the `QueueItem.input_keys` map plus the stage whose
    /// output folder the file lives in. The backend's rerun flow
    /// iterates these to rebuild queue items without hardcoding any
    /// stage-specific knowledge.
    pub inputs: &'static [StageInput],
}

/// One input artifact declaration for a [`StageSpec`].
#[derive(Debug, Clone, Copy, Serialize)]
pub struct StageInput {
    /// Key name placed in `QueueItem.input_keys` (e.g. `"front_video"`).
    pub name: &'static str,
    /// File extension, without a leading dot (e.g. `"mp4"`, `"json"`).
    pub ext: &'static str,
    /// Source of this input: either another stage's registry key, or
    /// the literal `"upload"` for raw user uploads (which the backend
    /// writes to `stage_0/` during submission).
    pub from: &'static str,
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

/// Sentinel `StageInput.from` value indicating the source is the raw
/// user upload folder (`stage_0/`) rather than another stage.
pub const UPLOAD_SOURCE: &str = "upload";

/// The ordered pipeline. Position determines execution order.
pub const STAGES: &[StageSpec] = &[
    StageSpec {
        key: "media-conversion",
        display_name: "Media Conversion",
        description: "Standardizes uploaded videos — resolution, frame rate, codec, and optional rotation/crop/trim.",
        terminal: false,
        panel: StagePanel::VideoEdit,
        inputs: &[
            StageInput { name: "front_video", ext: "mp4", from: UPLOAD_SOURCE },
            StageInput { name: "side_video",  ext: "mp4", from: UPLOAD_SOURCE },
        ],
    },
    StageSpec {
        key: "validity-check",
        display_name: "Validity Check",
        description: "Verifies both front and side videos contain exactly one person walking, via YOLO + SlowFast + DeepSORT.",
        terminal: false,
        panel: StagePanel::Default,
        inputs: &[
            StageInput { name: "front_video", ext: "mp4", from: "media-conversion" },
            StageInput { name: "side_video",  ext: "mp4", from: "media-conversion" },
        ],
    },
    StageSpec {
        key: "reframing",
        display_name: "Reframing",
        description: "Adjusts video framing and cropping based on detected person position (currently a pass-through placeholder).",
        terminal: false,
        panel: StagePanel::Default,
        inputs: &[
            StageInput { name: "front_video", ext: "mp4", from: "validity-check" },
            StageInput { name: "side_video",  ext: "mp4", from: "validity-check" },
        ],
    },
    StageSpec {
        key: "pose-estimation",
        display_name: "Pose Estimation",
        description: "Extracts body keypoints using MediaPipe's Holistic model, producing pose overlay videos and landmark JSON.",
        terminal: false,
        panel: StagePanel::Default,
        // Reframing is a passthrough that doesn't write its own files,
        // so we read directly from validity-check's outputs.
        inputs: &[
            StageInput { name: "front_video", ext: "mp4", from: "validity-check" },
            StageInput { name: "side_video",  ext: "mp4", from: "validity-check" },
        ],
    },
    StageSpec {
        key: "cycle-detection",
        display_name: "Cycle Detection",
        description: "Analyzes pose landmarks to identify individual gait cycles via rhythmic template matching.",
        terminal: false,
        panel: StagePanel::GaitCycles,
        inputs: &[
            StageInput { name: "front_video",     ext: "mp4",  from: "pose-estimation" },
            StageInput { name: "side_video",      ext: "mp4",  from: "pose-estimation" },
            StageInput { name: "front_landmarks", ext: "json", from: "pose-estimation" },
            StageInput { name: "side_landmarks",  ext: "json", from: "pose-estimation" },
        ],
    },
    StageSpec {
        key: "prediction",
        display_name: "Prediction",
        description: "Runs the ML ensemble to classify ASD from gait analysis data.",
        terminal: false,
        panel: StagePanel::Default,
        inputs: &[
            StageInput { name: "front_gait_analysis", ext: "json", from: "cycle-detection" },
            StageInput { name: "side_gait_analysis",  ext: "json", from: "cycle-detection" },
        ],
    },
    StageSpec {
        key: "finalize",
        display_name: "Finalize",
        description: "Sends the result email, archives artifacts, and closes out the job.",
        terminal: true,
        panel: StagePanel::Default,
        // Finalize uses FinalizeQueueItem (not QueueItem) and doesn't
        // consume files through the standard input_keys mechanism.
        inputs: &[],
    },
];

// ── Lookup helpers ─────────────────────────────────────────────────

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

/// S3 storage prefix where outputs from the given source live. The
/// sentinel [`UPLOAD_SOURCE`] resolves to `stage_0`; any other key
/// must be present in [`STAGES`] and yields `stage_N` with N being
/// the 1-indexed position.
///
/// # Panics
///
/// Panics if the source key is neither `"upload"` nor a known stage
/// key — this indicates a malformed registry and is a programmer
/// error rather than a runtime condition.
pub fn source_storage_prefix(source_key: &str) -> String {
    if source_key == UPLOAD_SOURCE {
        return "stage_0".to_string();
    }
    match stage_index_of(source_key) {
        Some(idx) => format!("stage_{}", idx + 1),
        None => panic!("unknown source stage key: {:?}", source_key),
    }
}

impl StageSpec {
    /// Build the `input_keys` map for a rerun of this stage, resolving
    /// each declared [`inputs`](Self::inputs) entry to its S3 path.
    pub fn build_input_keys(&self, job_id: &str) -> HashMap<String, String> {
        self.inputs
            .iter()
            .map(|input| {
                let prefix = source_storage_prefix(input.from);
                let path = format!("jobs/{}/{}/{}.{}", job_id, prefix, input.name, input.ext);
                (input.name.to_string(), path)
            })
            .collect()
    }
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
        assert!(json["inputs"].is_array());
    }

    #[test]
    fn inputs_reference_known_sources() {
        for s in STAGES {
            for input in s.inputs {
                if input.from == UPLOAD_SOURCE {
                    continue;
                }
                assert!(
                    stage_by_key(input.from).is_some(),
                    "stage {:?} input {:?} references unknown source {:?}",
                    s.key,
                    input.name,
                    input.from
                );
            }
        }
    }

    #[test]
    fn inputs_reference_earlier_stages_only() {
        // A stage can only consume outputs produced before it runs.
        for (i, s) in STAGES.iter().enumerate() {
            for input in s.inputs {
                if input.from == UPLOAD_SOURCE {
                    continue;
                }
                let src_idx = stage_index_of(input.from)
                    .expect("earlier assertion covers unknown sources");
                assert!(
                    src_idx < i,
                    "stage {:?} at index {} cannot read from {:?} at index {} (later or same stage)",
                    s.key, i, input.from, src_idx
                );
            }
        }
    }

    #[test]
    fn terminal_stage_has_no_declared_inputs() {
        // Finalize uses FinalizeQueueItem and a different I/O model.
        let last = STAGES.last().unwrap();
        assert!(last.terminal);
        assert!(
            last.inputs.is_empty(),
            "terminal stage declares inputs via StageSpec.inputs; those are ignored — drop them"
        );
    }

    #[test]
    fn source_storage_prefix_upload() {
        assert_eq!(source_storage_prefix(UPLOAD_SOURCE), "stage_0");
    }

    #[test]
    fn source_storage_prefix_stage() {
        assert_eq!(source_storage_prefix("media-conversion"), "stage_1");
        assert_eq!(source_storage_prefix("finalize"), "stage_7");
    }

    #[test]
    fn build_input_keys_media_conversion_reads_upload() {
        let spec = stage_by_key("media-conversion").unwrap();
        let keys = spec.build_input_keys("user_0");
        assert_eq!(keys.len(), 2);
        assert_eq!(keys["front_video"], "jobs/user_0/stage_0/front_video.mp4");
        assert_eq!(keys["side_video"], "jobs/user_0/stage_0/side_video.mp4");
    }

    #[test]
    fn build_input_keys_cycle_detection_reads_pose_estimation() {
        let spec = stage_by_key("cycle-detection").unwrap();
        let keys = spec.build_input_keys("user_0");
        assert_eq!(keys.len(), 4);
        assert_eq!(keys["front_video"], "jobs/user_0/stage_4/front_video.mp4");
        assert_eq!(keys["front_landmarks"], "jobs/user_0/stage_4/front_landmarks.json");
        assert_eq!(keys["side_landmarks"], "jobs/user_0/stage_4/side_landmarks.json");
    }

    #[test]
    fn build_input_keys_pose_estimation_skips_passthrough() {
        // Pose estimation (stage 4) should read from validity-check (stage 2),
        // not from reframing (stage 3), because reframing is a passthrough.
        let spec = stage_by_key("pose-estimation").unwrap();
        let keys = spec.build_input_keys("user_0");
        assert_eq!(keys["front_video"], "jobs/user_0/stage_2/front_video.mp4");
        assert_eq!(keys["side_video"], "jobs/user_0/stage_2/side_video.mp4");
    }

    #[test]
    fn build_input_keys_terminal_is_empty() {
        let spec = stage_by_key("finalize").unwrap();
        let keys = spec.build_input_keys("user_0");
        assert!(keys.is_empty());
    }
}
