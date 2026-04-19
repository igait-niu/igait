//! Publishes the central `STAGES` registry from igait-lib to RTDB
//! at `/registry/stages` on backend startup. The frontend reads this
//! path to render stage tabs directly from the source of truth —
//! no hardcoded stage list in TypeScript.

use anyhow::Result;
use igait_lib::microservice::{FirebaseRtdb, StagePanel, STAGES};
use std::collections::BTreeMap;
use tracing::info;

/// The shape written per stage. A flat struct with `order` derived
/// from the array index, so the frontend has a deterministic sort
/// key regardless of RTDB map iteration order.
#[derive(serde::Serialize)]
struct PublishedStage {
    key: &'static str,
    display_name: &'static str,
    description: &'static str,
    terminal: bool,
    panel: StagePanel,
    order: usize,
}

/// Writes every entry in `STAGES` to `/registry/stages`, keyed by
/// stage key. Overwrites any existing value so adding/removing stages
/// via a backend redeploy is idempotent.
pub async fn publish(db: &FirebaseRtdb) -> Result<()> {
    let published: BTreeMap<&'static str, PublishedStage> = STAGES
        .iter()
        .enumerate()
        .map(|(i, s)| (
            s.key,
            PublishedStage {
                key: s.key,
                display_name: s.display_name,
                description: s.description,
                terminal: s.terminal,
                panel: s.panel,
                order: i,
            },
        ))
        .collect();

    db.set("registry/stages", &published).await?;
    info!("Published stage registry ({} entries) to /registry/stages", STAGES.len());
    Ok(())
}
