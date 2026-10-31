//! Manual reviewed recovery of one retained worker journal. It warms a fresh source, reviews the
//! journal, and runs only the named direction. See v3/RECOVERY_GUARD.md.
use anyhow::{Context, Result, bail, ensure};
use lqosync_v3::core_worker::{Recovery, RecoveryAction, RouterSource};
use std::{path::PathBuf, time::Duration};

const USAGE: &str = "Usage: core-worker-recover ROOT BATCH_REVISION forward|rollback [--execute]";
/// Recovery review lifetime. The library accepts 1–30 seconds.
const REVIEW_SECONDS: u64 = 30;
const MAX_ROUNDS: u32 = 12;

#[derive(Debug)]
struct Args {
    root: PathBuf,
    revision: String,
    action: RecoveryAction,
    execute: bool,
}
fn parse_args(args: &[String]) -> Result<Args> {
    let (mut positional, mut execute) = (vec![], false);
    for a in args {
        match a.as_str() {
            "--execute" => execute = true,
            s if !s.starts_with("--") => positional.push(s.to_string()),
            _ => bail!(USAGE),
        }
    }
    let [root, revision, action] = positional.as_slice() else {
        bail!(USAGE);
    };
    let id = uuid::Uuid::parse_str(revision).context("Batch revision must be a UUID")?;
    ensure!(
        id.to_string() == *revision,
        "Batch revision must be a canonical UUID"
    );
    let action = match action.as_str() {
        "forward" => RecoveryAction::Forward,
        "rollback" => RecoveryAction::Rollback,
        _ => bail!(USAGE),
    };
    Ok(Args {
        root: PathBuf::from(root),
        revision: revision.clone(),
        action,
        execute,
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args(&std::env::args().skip(1).collect::<Vec<_>>())?;
    let root = args.root.canonicalize()?;
    let interval = lqosync_v3::store::load_settings(&root)?
        .wan_health
        .map_or(10, |h| h.interval_seconds);
    // A first probe sample cannot establish health. Warm the source until every WAN is Healthy.
    let mut source = RouterSource::new(&root)?;
    let mut warm = false;
    for round in 1..=MAX_ROUNDS {
        match source.preview().await {
            Ok(snapshot) => {
                let health = &snapshot.runtime().health;
                let states = health
                    .iter()
                    .map(|(w, s)| format!("{w}={:?}", s.status))
                    .collect::<Vec<_>>();
                println!("Round {round}: health {states:?}");
                if !health.is_empty()
                    && health
                        .values()
                        .all(|s| s.status == lqosync_v3::wan_health::HealthStatus::Healthy)
                {
                    warm = true;
                    break;
                }
            }
            Err(e) => println!("Round {round}: {e:#}"),
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
    ensure!(
        warm,
        "WAN health did not become Healthy; recovery review not attempted"
    );
    let review =
        Recovery::review(&root, &args.revision, args.action, source, REVIEW_SECONDS).await?;
    println!("{}", serde_json::to_string_pretty(review.proposal())?);
    let approval = review.revision()?;
    if !args.execute {
        println!(
            "Dry run: no write sent. Add --execute to run the {:?} recovery.",
            args.action
        );
        return Ok(());
    }
    let mut worker = review.approve(&approval).await?;
    match args.action {
        RecoveryAction::Forward => worker.forward().await?,
        RecoveryAction::Rollback => worker.rollback().await?,
    }
    println!("{:?} recovery complete and verified", args.action);
    worker.finish()?;
    println!("Finished; recovery marker removed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn a(v: &[&str]) -> Result<Args> {
        parse_args(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }
    const REV: &str = "34b30f55-db4a-55a2-9c48-e3cba0357d5f";
    #[test]
    fn args_name_one_journal_and_one_direction() {
        let f = a(&["/root", REV, "forward"]).unwrap();
        assert_eq!(
            (f.revision.as_str(), f.action, f.execute),
            (REV, RecoveryAction::Forward, false)
        );
        let r = a(&["/root", REV, "rollback", "--execute"]).unwrap();
        assert_eq!((r.action, r.execute), (RecoveryAction::Rollback, true));
        assert!(a(&["/root", REV]).is_err(), "direction is mandatory");
        assert!(a(&["/root", "not-a-uuid", "forward"]).is_err());
        assert!(a(&["/root", REV, "both"]).is_err());
        assert!(a(&["/root", REV, "forward", "--force"]).is_err());
    }
}
