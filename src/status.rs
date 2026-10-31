//! `status.json`: what the last cycle did, for the TUI and the operator.

use crate::cycle::{Change, Report};
use crate::model::{CsvRow, State};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct RouterStatus {
    pub name: String,
    /// online | failed | unreachable | offline
    pub state: &'static str,
    pub error: Option<String>,
    /// Rows read from each table.
    pub rows: BTreeMap<String, usize>,
    pub read_ms: u64,
}

/// One circuit with the values that the operator reads.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CircuitInfo {
    pub name: String,
    pub ipv4: String,
    pub download_max: String,
    pub upload_max: String,
    pub parent: String,
}

impl CircuitInfo {
    fn of(name: &str, row: Option<&CsvRow>) -> Self {
        let r = row.cloned().unwrap_or_default();
        CircuitInfo {
            name: name.to_string(),
            ipv4: r.ipv4,
            download_max: r.download_max,
            upload_max: r.upload_max,
            parent: r.parent_node,
        }
    }
}

/// One updated circuit with each changed column.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct UpdatedInfo {
    pub name: String,
    pub ipv4: String,
    /// `(column, old value, new value)`
    pub changes: Vec<Change>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ReportSummary {
    pub counts: BTreeMap<&'static str, usize>,
    pub added: Vec<CircuitInfo>,
    pub updated: Vec<UpdatedInfo>,
    pub removed: Vec<CircuitInfo>,
    pub held: Vec<String>,
    /// `(ip, kept, dropped)`
    pub duplicates: Vec<(String, String, String)>,
    pub warnings: Vec<String>,
    pub alerts: Vec<String>,
}

impl ReportSummary {
    /// `rows` are the rows of this cycle. `before` is the state at the start of the cycle (for removed rows).
    pub fn from_report(r: &Report, rows: &[CsvRow], before: &State) -> Self {
        let row = |n: &str| rows.iter().find(|x| x.circuit_name == n);
        let counts = [
            ("added", r.added.len()),
            ("updated", r.updated.len()),
            ("removed", r.removed.len()),
            ("held", r.held.len()),
            ("duplicates", r.duplicates.len()),
            ("warnings", r.warnings.len()),
            ("alerts", r.alerts.len()),
        ]
        .into_iter()
        .collect();
        ReportSummary {
            counts,
            added: r.added.iter().map(|n| CircuitInfo::of(n, row(n))).collect(),
            updated: r
                .updated
                .iter()
                .map(|(n, changes)| UpdatedInfo {
                    name: n.clone(),
                    ipv4: row(n).map_or(String::new(), |x| x.ipv4.clone()),
                    changes: changes.clone(),
                })
                .collect(),
            removed: r
                .removed
                .iter()
                .map(|n| CircuitInfo::of(n, before.circuits.get(n).map(|c| &c.row)))
                .collect(),
            held: r.held.clone(),
            duplicates: r.duplicates.clone(),
            warnings: r.warnings.clone(),
            alerts: r.alerts.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub version: u32,
    pub updated: u64,
    pub mode: String,
    pub target: String,
    pub cycle_ms: u64,
    pub next_scan_seconds: u64,
    pub routers: Vec<RouterStatus>,
    pub report: ReportSummary,
    pub files_changed: bool,
    /// skipped | ok | the error text
    pub reload: String,
    pub error: Option<String>,
    /// The last config reload (SIGHUP) failed: the old config stays in use.
    pub config_error: Option<String>,
}

impl Status {
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("status serializes");
        s.push('\n');
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cycle::Report;

    #[test]
    fn status_json_has_the_spec_fields() {
        let report = Report {
            added: vec!["a".into()],
            removed: vec!["b".into()],
            ..Report::default()
        };
        let st = Status {
            version: 1,
            updated: 10,
            mode: "shadow".into(),
            target: "v2_pair".into(),
            cycle_ms: 5,
            next_scan_seconds: 25,
            routers: vec![RouterStatus {
                name: "R1".into(),
                state: "online",
                error: None,
                rows: [("/ppp/secret".to_string(), 3)].into(),
                read_ms: 2,
            }],
            report: ReportSummary::from_report(&report, &[], &State::default()),
            files_changed: true,
            reload: "skipped".into(),
            error: None,
            config_error: None,
        };
        let v: serde_json::Value = serde_json::from_str(&st.to_json()).unwrap();
        for k in [
            "version",
            "updated",
            "mode",
            "target",
            "cycle_ms",
            "next_scan_seconds",
            "routers",
            "report",
            "files_changed",
            "reload",
            "error",
            "config_error",
        ] {
            assert!(v.get(k).is_some(), "missing {k}");
        }
        assert_eq!(v["report"]["counts"]["added"], 1);
        assert_eq!(v["report"]["removed"][0]["name"], "b");
        assert_eq!(v["routers"][0]["state"], "online");
        assert_eq!(v["routers"][0]["rows"]["/ppp/secret"], 3);
    }

    #[test]
    fn summary_takes_values_from_rows_and_the_old_state() {
        use crate::model::{Source, StoredCircuit};
        let row = |n: &str, ip: &str| CsvRow {
            circuit_name: n.into(),
            ipv4: ip.into(),
            download_max: "25".into(),
            upload_max: "5".into(),
            parent_node: "PPP-R1".into(),
            ..CsvRow::default()
        };
        let mut before = State::default();
        before.circuits.insert(
            "gone".into(),
            StoredCircuit {
                router: None,
                source: Source::Ppp,
                last_seen: 0,
                row: row("gone", "100.64.0.9"),
            },
        );
        let report = Report {
            added: vec!["new".into()],
            removed: vec!["gone".into()],
            ..Report::default()
        };
        let s = ReportSummary::from_report(&report, &[row("new", "100.64.0.2")], &before);
        assert_eq!(
            (s.added[0].ipv4.as_str(), s.added[0].parent.as_str()),
            ("100.64.0.2", "PPP-R1")
        );
        assert_eq!(s.removed[0].ipv4, "100.64.0.9");
    }
}
