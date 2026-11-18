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
        } => {
            let previous = previous.as_deref().map(read).transpose()?;
            let file = runner
                .propose_enrollment(previous.as_ref(), MAX_AGE)
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
        TopologyCommand::Enroll { mapping, yes } => {
            let outcome = runner
                .enroll_with_mapping(&read(mapping)?, MAX_AGE, *yes)
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
    };
    Ok((out, code))
}
