//! Optional worker measurements from the verified ten-phase analytics contract.
use super::WorkerRow;

pub const PHASES: [&str; 10] = [
    "rx",
    "dissect",
    "select",
    "charge",
    "leaf",
    "tx",
    "begin_cycle",
    "drop_overdue",
    "handoff",
    "empty_rx",
];

impl WorkerRow {
    pub(super) fn sanitize_extended(&mut self) {
        if self
            .idle_ppm
            .is_some_and(|idle| self.busy_ppm.checked_add(idle) != Some(1_000_000))
        {
            self.idle_ppm = None;
        }
        if self
            .phase_cycles
            .as_ref()
            .is_some_and(|p| p.len() != PHASES.len())
        {
            self.phase_cycles = None;
        }
    }

    pub(super) fn clear_extended(&mut self) {
        self.idle_ppm = None;
        self.laps_per_s = None;
        self.busy_laps = None;
        self.cycles_per_busy_lap = None;
        self.phase_cycles = None;
    }

    pub fn extended_measurements(&self, live: bool) -> Vec<String> {
        let number = |n: Option<u64>| n.map_or("unknown".into(), |n| n.to_string());
        let available = self.idle_ppm.is_some()
            || self.laps_per_s.is_some()
            || self.busy_laps.is_some()
            || self.cycles_per_busy_lap.is_some()
            || self
                .phase_cycles
                .as_ref()
                .is_some_and(|p| p.len() == PHASES.len());
        let mut lines = vec![
            format!(
                "Worker metrics {}",
                if !available {
                    "unavailable"
                } else if live {
                    "LIVE"
                } else {
                    "STALE (last known)"
                }
            ),
            format!(
                "Reported idle: {}",
                self.idle_ppm
                    .map_or("unknown".into(), |n| format!("{:.1}%", n as f64 / 10000.0))
            ),
            format!("Laps/s (reported): {}", number(self.laps_per_s)),
            format!("Busy laps (reported): {}", number(self.busy_laps)),
            format!("Cycles/busy lap: {}", number(self.cycles_per_busy_lap)),
            "Laps/s is producer-reported, not packets/s.".into(),
            "Idle bar = 100% - busy (derived).".into(),
        ];
        if let Some(phases) = self
            .phase_cycles
            .as_ref()
            .filter(|p| p.len() == PHASES.len())
        {
            lines.push("Phase TSC cycles (cumulative):".into());
            lines.extend(
                PHASES
                    .iter()
                    .zip(phases)
                    .map(|(name, n)| format!("  {name}: {n}")),
            );
        } else {
            lines.push("Phase TSC cycles: unknown".into());
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_worker_fields_keep_unknown_distinct_from_reported_zero() {
        let row = WorkerRow::default();
        let text = row.extended_measurements(true).join("\n");
        assert!(
            text.contains("metrics unavailable")
                && text.contains("Reported idle: unknown")
                && text.contains("Laps/s (reported): unknown")
                && text.contains("Phase TSC cycles: unknown")
        );
        let mut row = WorkerRow {
            busy_ppm: 1000000,
            idle_ppm: Some(0),
            laps_per_s: Some(0),
            busy_laps: Some(0),
            cycles_per_busy_lap: Some(0),
            phase_cycles: Some(vec![0; 10]),
            ..Default::default()
        };
        row.sanitize_extended();
        let text = row.extended_measurements(true).join("\n");
        assert!(
            text.contains("Reported idle: 0.0%")
                && text.contains("Laps/s (reported): 0")
                && text.contains("empty_rx: 0")
        );
        assert!(!text.contains("unknown"));
    }

    #[test]
    fn malformed_phase_lengths_and_idle_values_do_not_corrupt_other_fields() {
        for count in [0, 9, 11, 100] {
            let mut row = WorkerRow {
                busy_ppm: 125000,
                idle_ppm: Some(u64::MAX),
                laps_per_s: Some(10),
                phase_cycles: Some(vec![1; count]),
                ..Default::default()
            };
            row.sanitize_extended();
            assert!(row.idle_ppm.is_none() && row.phase_cycles.is_none());
            assert_eq!(row.laps_per_s, Some(10));
            assert_eq!(row.busy_ppm, 125000);
        }
        let mut row = WorkerRow {
            busy_ppm: 125000,
            idle_ppm: Some(875000),
            phase_cycles: Some(vec![u64::MAX; 10]),
            ..Default::default()
        };
        row.sanitize_extended();
        let text = row.extended_measurements(false).join("\n");
        assert!(text.contains("STALE (last known)") && text.contains(&u64::MAX.to_string()));
        assert_eq!(row.idle_ppm, Some(875000));
    }
}
