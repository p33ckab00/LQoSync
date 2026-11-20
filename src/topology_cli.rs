//! The `lqosync3 topology ...` commands. Each command returns its report text and exit status.
//! Status 2 means a critical readiness issue, or an enrollment dry run that installed nothing.

use crate::cli::TopologyCommand;
use crate::routeros::Connector;
use crate::runner::Runner;
use anyhow::Context;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;
use std::time::Duration;

/// Source age limit for each prepare in these commands.
const MAX_AGE: Duration = Duration::from_secs(120);

/// Write a new operator file with mode 0600. An existing file is never replaced.
fn write_new(path: &Path, value: &impl serde::Serialize) -> anyhow::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .with_context(|| {
            format!(
                "cannot create {} (an existing file is not replaced)",
                path.display()
            )
        })?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn read<T: serde::de::DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let bytes = std::fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("cannot decode {}", path.display()))
}

/// Add the summary to `out`, then write the report. Return the exit status.
/// The summary comes first, so a failed report write cannot hide a "RESTORE FAILED".
fn live_test_output(
    out: &mut String,
    result: &crate::runner::LiveTestReport,
    report: &Path,
) -> anyhow::Result<i32> {
    out.push_str(&result.summary());
    let diffs = [&result.baseline, &result.manager, &result.restore];
    let diffs = diffs.into_iter().flatten();
    let differences = result.admission.len()
        + result.parity.len()
        + result.legacy_to_manager.len()
        + diffs.clone().map(|d| d.differences.len()).sum::<usize>();
    let warnings = result.warnings.len() + diffs.map(|d| d.warnings.len()).sum::<usize>();
    let _ = writeln!(out, "differences: {differences}, warnings: {warnings}");
    // Write the report also when the test failed.
    let mut json = serde_json::to_vec_pretty(result)?;
    json.push(b'\n');
    if let Err(e) = crate::files::write_if_changed(report, &json) {
        let _ = writeln!(out, "error: cannot write {}: {e:#}", report.display());
        return Ok(1);
    }
    let _ = writeln!(out, "report: {}", report.display());
    Ok(i32::from(!result.passed))
}

/// Name the service when the instance lock is held by another process.
fn locked(e: anyhow::Error) -> anyhow::Error {
    if format!("{e:#}").contains("another lqosync3 is running") {
        e.context("another lqosync3 process holds the instance lock; stop lqosync3.service before this command")
    } else {
        e
    }
}

fn rounded(out: &mut String, nodes: &[crate::topology_manager::publication::NodeExportAdjustment]) {
    for n in nodes {
        let _ = writeln!(
            out,
            "rounded node {}: {}/{} bps -> {}/{} bps",
            n.node_id,
            n.calculated_ceiling.down,
            n.calculated_ceiling.up,
            n.exported_ceiling.down,
            n.exported_ceiling.up
        );
    }
}

pub async fn run<C: Connector + 'static>(
    runner: &mut Runner<C>,
    command: &TopologyCommand,
) -> anyhow::Result<(String, i32)>
where
    C::Session: 'static,
{
    let mut out = String::new();
    let code = match command {
        TopologyCommand::PolicyPropose { out: path } => {
            let draft = runner.propose_policy(MAX_AGE).await?;
            write_new(path, &draft)?;
            let _ = writeln!(out, "wrote draft policy {}", path.display());
            0
        }
        TopologyCommand::Check { policy } => {
            let check = runner.check_topology(&read(policy)?, MAX_AGE).await?;
            for issue in &check.policy_issues {
                let _ = writeln!(out, "CRITICAL policy: {issue:?}");
            }
            if let Some(e) = &check.prepare_error {
                let _ = writeln!(out, "CRITICAL sources: {e}");
            }
            for (scope, count) in &check.candidate_only_by_scope {
                let _ = writeln!(
                    out,
                    "CRITICAL {scope}: {count} current account(s) have no legacy row; they enter after activation"
                );
            }
            if check.unmatched_legacy > 0 {
                let _ = writeln!(
                    out,
                    "CRITICAL {} legacy row(s) have no suggested mapping",
                    check.unmatched_legacy
                );
            }
            for mac in &check.random_macs {
                let _ = writeln!(out, "WARN locally administered DHCP client MAC {mac}");
            }
            rounded(&mut out, &check.rounded);
            let _ = writeln!(out, "suggested mappings: {}", check.suggested);
            let critical = !check.policy_issues.is_empty()
                || check.prepare_error.is_some()
                || !check.candidate_only_by_scope.is_empty()
                || check.unmatched_legacy > 0;
            i32::from(critical) * 2
        }
        TopologyCommand::PolicyInstall { policy } => {
            let revision = runner
                .install_topology_policy(&read(policy)?)
                .map_err(locked)?;
            let _ = writeln!(out, "installed policy revision {revision}");
            0
        }
        TopologyCommand::Propose {
            out: path,
            previous,
            replace,
        } => {
            let previous = previous.as_deref().map(read).transpose()?;
            let file = runner
                .propose_enrollment(previous.as_ref(), MAX_AGE, *replace)
                .await
                .map_err(locked)?;
            write_new(path, &file)?;
            let _ = writeln!(
                out,
                "wrote mapping {} with {} legacy row(s) and {} node(s); confirm each entry before enroll",
                path.display(),
                file.rows.len(),
                file.nodes.len()
            );
            0
        }
        TopologyCommand::Enroll {
            mapping,
            yes,
            replace,
        } => {
            let outcome = runner
                .enroll_with_mapping(&read(mapping)?, MAX_AGE, *yes, *replace)
                .await
                .map_err(locked)?;
            let _ = writeln!(
                out,
                "mapped {} row(s), protected {}, dropped {}",
                outcome.mapped,
                outcome.protected,
                outcome.dropped.len()
            );
            for row in &outcome.dropped {
                let _ = writeln!(
                    out,
                    "dropped legacy row {} / {}",
                    row.circuit_id, row.device_id
                );
            }
            rounded(&mut out, &outcome.rounded);
            match outcome.revision {
                Some(revision) => {
                    let _ = writeln!(
                        out,
                        "installed registry revision {revision}; shaping stays with the legacy loop until activation"
                    );
                    0
                }
                None => {
                    let _ = writeln!(
                        out,
                        "dry run: nothing installed; run again with --yes to install"
                    );
                    2
                }
            }
        }
        TopologyCommand::LiveTest {
            report,
            import_timeout,
        } => {
            let result = runner
                .live_test(Duration::from_secs(*import_timeout))
                .await
                .map_err(locked)?;
            live_test_output(&mut out, &result, report)?
        }
        TopologyCommand::LiveTestRestore { backup } => {
            let state = Path::new(&runner.cfg.runtime.state_dir);
            // Hold the instance lock, so this cannot race a running service.
            let _lock = crate::files::InstanceLock::acquire(state).map_err(locked)?;
            let timeout = Duration::from_secs(crate::cli::DEFAULT_IMPORT_TIMEOUT);
            crate::runner::restore_backup(
                &runner.cfg,
                &mut crate::runner::NativeIo,
                backup,
                timeout,
            )
            .await?;
            let _ = writeln!(
                out,
                "restored the pair from {} to {}; LibreQoS imported it and is ready",
                backup.display(),
                runner.cfg.libreqos.directory
            );
            0
        }
    };
    Ok((out, code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_report_write_still_shows_the_summary() {
        let dir = tempfile::tempdir().unwrap();
        // A file in place of the folder makes the write fail.
        std::fs::write(dir.path().join("file"), "").unwrap();
        let report = dir.path().join("file/report.json");
        let result = crate::runner::LiveTestReport {
            error: Some("RESTORE FAILED: test".into()),
            ..Default::default()
        };
        let mut out = String::new();
        let code = live_test_output(&mut out, &result, &report).unwrap();
        assert_ne!(code, 0);
        let summary = out.find("RESTORE FAILED").expect(&out);
        let error = out.find("cannot write").expect(&out);
        assert!(summary < error, "{out}");
    }
}
