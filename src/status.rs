//! `status.json`: what the last cycle did, for the TUI and the operator.

use crate::cycle::Report;
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

#[derive(Debug, Clone, Default, Serialize)]
pub struct ReportSummary {
    pub counts: BTreeMap<&'static str, usize>,
    pub added: Vec<String>,
    /// `(circuit, changed columns)`
    pub updated: Vec<(String, Vec<String>)>,
    pub removed: Vec<String>,
    pub held: Vec<String>,
    /// `(ip, kept, dropped)`
    pub duplicates: Vec<(String, String, String)>,
    pub warnings: Vec<String>,
    pub alerts: Vec<String>,
}

impl ReportSummary {
    pub fn from_report(r: &Report) -> Self {
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
            added: r.added.clone(),
            updated: r.updated.clone(),
            removed: r.removed.clone(),
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
            report: ReportSummary::from_report(&report),
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
        assert_eq!(v["report"]["removed"][0], "b");
        assert_eq!(v["routers"][0]["state"], "online");
        assert_eq!(v["routers"][0]["rows"]["/ppp/secret"], 3);
    }
}
