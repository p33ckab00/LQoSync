//! Which group nodes have active sources now (live source canvas). Runtime state, not policy:
//! a change here makes no registry revision.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const FILE: &str = "source-activity.json";

#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SourceActivity {
    /// The first good read after the file was made. `None`: no read yet.
    first_observed: Option<u64>,
    /// The last good read in which each source had a member. The keys are source IDs.
    last_active: BTreeMap<String, u64>,
}

impl SourceActivity {
    /// The saved state of `dir`. A missing or damaged file gives an empty state: then every
    /// source is active until the first good read.
    pub fn load(dir: &Path) -> SourceActivity {
        std::fs::read(dir.join(FILE))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Record one good read. Only a successful read may call this.
    pub fn observe(&mut self, now: u64, active: &BTreeSet<String>) {
        self.first_observed.get_or_insert(now);
        for id in active {
            self.last_active.insert(id.clone(), now);
        }
    }

    /// A source is active while its last active read is not older than `grace`. Before the first
    /// good read every source counts as active; after it, a source never seen active is not.
    pub fn is_active(&self, source_id: &str, now: u64, grace: u64) -> bool {
        match self.last_active.get(source_id) {
            Some(t) => now.saturating_sub(*t) <= grace,
            None => self.first_observed.is_none(),
        }
    }

    pub fn save(&self, dir: &Path) -> anyhow::Result<()> {
        crate::files::write_secret(&dir.join(FILE), &serde_json::to_vec(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_node_becomes_inactive_only_after_the_grace() {
        let mut a = SourceActivity::default();
        a.observe(1_000, &set(&["g"]));
        a.observe(1_500, &set(&[]));
        assert!(a.is_active("g", 1_599, 600));
        assert!(!a.is_active("g", 1_601, 600));
    }

    #[test]
    fn a_short_outage_hides_no_card() {
        let mut a = SourceActivity::default();
        a.observe(1_000, &set(&["g"]));
        for t in [1_120, 1_240] {
            a.observe(t, &set(&[]));
            assert!(a.is_active("g", t, 600));
        }
        a.observe(1_300, &set(&["g"]));
        assert!(a.is_active("g", 1_850, 600));
    }

    #[test]
    fn a_failed_read_changes_nothing() {
        let mut a = SourceActivity::default();
        a.observe(1_000, &set(&["g"]));
        let before = a.clone();
        // A failed read does not call observe; the state and the answer stay.
        assert_eq!(a, before);
        assert!(a.is_active("g", 1_500, 600));
    }

    #[test]
    fn a_missing_activity_file_keeps_every_node_visible() {
        let dir = tempfile::tempdir().unwrap();
        assert!(SourceActivity::load(dir.path()).is_active("g", 9_999, 600));
        std::fs::write(dir.path().join(FILE), b"not json").unwrap();
        let a = SourceActivity::load(dir.path());
        assert!(a.is_active("g", 9_999, 600));
    }

    #[test]
    fn a_source_never_seen_active_is_hidden_after_the_first_read() {
        // Owner, 2026-11-23: a profile without an active session, or a server without a lease,
        // does not belong on the canvas. Only the canvas changes; shaping does not use this.
        let mut a = SourceActivity::default();
        a.observe(10_000, &set(&["other"]));
        assert!(!a.is_active("g", 10_000, 600));
        assert!(a.is_active("other", 10_000, 600));
    }

    #[test]
    fn the_state_survives_a_save_and_a_load() {
        let dir = tempfile::tempdir().unwrap();
        let mut a = SourceActivity::default();
        a.observe(1_000, &set(&["g"]));
        a.save(dir.path()).unwrap();
        assert_eq!(SourceActivity::load(dir.path()), a);
    }
}
