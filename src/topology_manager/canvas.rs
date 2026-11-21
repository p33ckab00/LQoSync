//! Canvas card positions. A separate file with its own revision: a layout save never changes the
//! registry revision, so it cannot schedule a shaping write.

use super::store::RevisionConflict;
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const FILE: &str = "canvas-layout.json";
/// More cards than any supported topology. Bounds the file and the web view.
pub const MAX_POSITIONS: usize = 4096;
/// Canvas coordinate limit in CSS pixels at zoom 1.
pub const MAX_COORDINATE: f64 = 1e7;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvasLayout {
    pub schema_version: u32,
    pub revision: u64,
    /// Node ID to `[x, y]`.
    pub positions: BTreeMap<String, [f64; 2]>,
}

/// The saved layout of `directory` (the `topology-manager` directory). Missing: revision 0.
pub fn load(directory: &Path) -> anyhow::Result<CanvasLayout> {
    let bytes = match std::fs::read(directory.join(FILE)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(CanvasLayout {
                schema_version: 1,
                ..Default::default()
            });
        }
        Err(e) => return Err(e).context("cannot read the canvas layout"),
    };
    let layout: CanvasLayout =
        serde_json::from_slice(&bytes).context("cannot decode the canvas layout")?;
    ensure!(
        layout.schema_version == 1,
        "unsupported canvas layout schema"
    );
    Ok(layout)
}

/// Replace all positions. `known` holds the current catalog node IDs. Returns the new revision.
pub fn save(
    directory: &Path,
    expected_revision: u64,
    positions: BTreeMap<String, [f64; 2]>,
    known: &BTreeSet<String>,
) -> anyhow::Result<u64> {
    ensure!(
        positions.len() <= MAX_POSITIONS,
        "too many canvas positions"
    );
    for (id, point) in &positions {
        ensure!(known.contains(id), "canvas position for an unknown node");
        ensure!(
            point
                .iter()
                .all(|v| v.is_finite() && v.abs() <= MAX_COORDINATE),
            "canvas position is out of range"
        );
    }
    let current = load(directory)?;
    if current.revision != expected_revision {
        anyhow::bail!(RevisionConflict);
    }
    let layout = CanvasLayout {
        schema_version: 1,
        revision: current
            .revision
            .checked_add(1)
            .context("layout revision is exhausted")?,
        positions,
    };
    crate::files::write_secret(&directory.join(FILE), &serde_json::to_vec(&layout)?)?;
    Ok(layout.revision)
}

#[cfg(test)]
#[path = "canvas_tests.rs"]
mod tests;
