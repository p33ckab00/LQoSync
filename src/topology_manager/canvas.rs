//! Canvas card positions. A separate file with its own revision: a layout save never changes the
//! registry revision, so it cannot schedule a shaping write.

use super::store::RevisionConflict;
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use web_contract::topology::Direction;

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
    /// The current direction. Added in layout schema 2; schema 1 is top-down.
    #[serde(default, skip_serializing_if = "is_top_down")]
    pub direction: Direction,
    /// Card ID to `[x, y]` in the current direction.
    pub positions: BTreeMap<String, [f64; 2]>,
    /// The saved positions of the other directions. Added in layout schema 2.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub other: BTreeMap<Direction, BTreeMap<String, [f64; 2]>>,
}

fn is_top_down(d: &Direction) -> bool {
    *d == Direction::TopDown
}

/// The saved layout of `directory` (the `topology-manager` directory). Missing: revision 0.
pub fn load(directory: &Path) -> anyhow::Result<CanvasLayout> {
    let bytes = match std::fs::read(directory.join(FILE)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(CanvasLayout {
                schema_version: 2,
                ..Default::default()
            });
        }
        Err(e) => return Err(e).context("cannot read the canvas layout"),
    };
    let layout: CanvasLayout =
        serde_json::from_slice(&bytes).context("cannot decode the canvas layout")?;
    ensure!(
        matches!(layout.schema_version, 1 | 2),
        "unsupported canvas layout schema"
    );
    Ok(layout)
}

/// Replace the positions of `direction`. A new direction keeps the positions of the old one;
/// empty positions with a new direction restore its saved positions. `known` holds the current
/// node and card IDs. Returns the new revision.
pub fn save(
    directory: &Path,
    expected_revision: u64,
    direction: Direction,
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
    let mut current = load(directory)?;
    if current.revision != expected_revision {
        anyhow::bail!(RevisionConflict);
    }
    let positions = if direction == current.direction {
        positions
    } else {
        let old = std::mem::take(&mut current.positions);
        current.other.insert(current.direction, old);
        let saved = current.other.remove(&direction).unwrap_or_default();
        if positions.is_empty() {
            saved
        } else {
            positions
        }
    };
    // A top-down layout with no other direction stays schema 1, so an older build reads it.
    let schema_version = if direction == Direction::TopDown && current.other.is_empty() {
        1
    } else {
        2
    };
    let layout = CanvasLayout {
        schema_version,
        revision: current
            .revision
            .checked_add(1)
            .context("layout revision is exhausted")?,
        direction,
        positions,
        other: current.other,
    };
    crate::files::write_secret(&directory.join(FILE), &serde_json::to_vec(&layout)?)?;
    Ok(layout.revision)
}

#[cfg(test)]
#[path = "canvas_tests.rs"]
mod tests;
