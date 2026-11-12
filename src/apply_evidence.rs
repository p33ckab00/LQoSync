//! Read-only native acknowledgement observations. No generation is used as a fence.
mod monitor;
pub use monitor::Monitor;
mod transaction;
mod verify;
use anyhow::{Context, ensure};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;
pub use verify::{Acceptance, Prepared, Verified, prepare, verify_prepared, verify_receipt};

#[derive(Debug, Deserialize)]
enum State {
    Report(Report),
    Unavailable(String),
}
#[derive(Debug, Deserialize)]
struct Report {
    generation: String,
    report_schema: u32,
    transaction: Vec<u8>,
    #[serde(default)]
    content_key: String,
    #[serde(default)]
    desired_world_content_key: String,
    #[serde(default)]
    applied_circuit_capacity: Option<u32>,
}

#[derive(Debug, PartialEq)]
pub struct Applied {
    pub peer_pid: u32,
    pub generation: String,
    pub transaction: Value,
    pub content_key: String,
    pub desired_world_content_key: String,
    pub applied_circuit_capacity: Option<u32>,
}
impl Applied {
    fn from_report(report: Report, peer_pid: u32) -> anyhow::Result<Self> {
        let text = |s: &str| s.len() <= 128 && !s.chars().any(char::is_control);
        ensure!(
            report.report_schema == 1 && peer_pid > 0,
            "unsupported applied report"
        );
        ensure!(
            !report.generation.is_empty()
                && text(&report.generation)
                && text(&report.content_key)
                && text(&report.desired_world_content_key),
            "invalid applied identity"
        );
        ensure!(
            !report.transaction.is_empty() && report.transaction.len() <= 2 * 1024 * 1024,
            "invalid applied transaction size"
        );
        let mut bytes = report.transaction.as_slice();
        let transaction: Value =
            ciborium::from_reader(&mut bytes).context("invalid applied transaction CBOR")?;
        ensure!(
            bytes.is_empty() && transaction.is_object(),
            "invalid applied transaction envelope"
        );
        // This reader observes an opaque transaction; acceptance must validate its producer schema separately.
        ensure!(
            transaction["schema_version"]
                .as_u64()
                .is_some_and(|v| v > 0 && u32::try_from(v).is_ok()),
            "invalid transaction schema identifier"
        );
        ensure!(
            transaction["generation"].as_str() == Some(report.generation.as_str()),
            "applied transaction identity mismatch"
        );
        Ok(Self {
            peer_pid,
            generation: report.generation,
            transaction,
            content_key: report.content_key,
            desired_world_content_key: report.desired_world_content_key,
            applied_circuit_capacity: report.applied_circuit_capacity,
        })
    }
}

pub async fn read_applied(path: &Path) -> anyhow::Result<Applied> {
    let (state, peer_pid): (State, u32) = crate::worker_util::ipc::request_with_peer(
        path,
        "state",
        serde_json::json!({"kind":"state","limit":1}),
    )
    .await?;
    match state {
        State::Report(report) => Applied::from_report(report, peer_pid),
        State::Unavailable(reason) => {
            // Producer text can contain identifiers. Keep it out of public errors.
            let _ = reason;
            anyhow::bail!("native acknowledgement unavailable")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn report() -> Report {
        let mut transaction = Vec::new();
        ciborium::into_writer(
            &json!({"schema_version":1,"generation":"g1"}),
            &mut transaction,
        )
        .unwrap();
        Report {
            generation: "g1".into(),
            report_schema: 1,
            transaction,
            content_key: String::new(),
            desired_world_content_key: String::new(),
            applied_circuit_capacity: None,
        }
    }
    #[test]
    fn schema_six_transaction_is_observable_without_claiming_acceptance() {
        let mut r = report();
        let mut transaction = Vec::new();
        ciborium::into_writer(
            &json!({"schema_version":6,"generation":"g1"}),
            &mut transaction,
        )
        .unwrap();
        r.transaction = transaction;
        assert_eq!(
            Applied::from_report(r, 1).unwrap().transaction["schema_version"],
            6
        );
    }

    #[test]
    fn malformed_acknowledgements_fail_closed() {
        for mutate in [
            |r: &mut Report| r.report_schema = 2,
            |r: &mut Report| r.generation = "g2".into(),
            |r: &mut Report| r.transaction.push(0),
            |r: &mut Report| r.transaction.clear(),
            |r: &mut Report| r.content_key = "x".repeat(129),
        ] {
            let mut r = report();
            mutate(&mut r);
            assert!(Applied::from_report(r, 1).is_err());
        }
        assert!(Applied::from_report(report(), 0).is_err());
        assert!(Applied::from_report(report(), 1).is_ok());
    }
}
