//! Manual executor run for acceptance. It runs the reconciliation preview, then the guarded worker
//! for one exactly expected operation. See docs/superpowers/specs/2026-10-31-executor-acceptance-design.md.
use anyhow::{Context, Result, bail, ensure};
use lqosync_v3::{
    core_reconcile::Decision,
    core_worker::{Prepared, RouterSource},
    store,
};
use std::{path::PathBuf, time::Duration};

const USAGE: &str = "Usage: core-worker-run ROOT --expect KIND:SUBSCRIBER:LIST [--rollback-after SECONDS] [--no-finish] [--execute]";
/// The whole run, including a rollback hold, must finish inside this approval window.
const APPROVAL_SECONDS: u64 = 180;
const MAX_ROUNDS: u32 = 12;

struct Expect {
    kind: String,
    subscriber: String,
    list: String,
}
fn parse_expect(text: &str) -> Result<Expect> {
    let parts = text.split(':').collect::<Vec<_>>();
    let [kind, subscriber, list] = parts.as_slice() else {
        bail!("Expect needs KIND:SUBSCRIBER:LIST");
    };
    ensure!(
        matches!(*kind, "add" | "move" | "remove"),
        "Expect kind must be add, move, or remove"
    );
    let id = uuid::Uuid::parse_str(subscriber).context("Expect subscriber must be a UUID")?;
    ensure!(
        !id.is_nil() && id.to_string() == *subscriber,
        "Expect subscriber must be a canonical UUID"
    );
    ensure!(!list.is_empty(), "Expect list is empty");
    Ok(Expect {
        kind: kind.to_string(),
        subscriber: subscriber.to_string(),
        list: list.to_string(),
    })
}
fn describe(d: &Decision) -> String {
    match d {
        Decision::Keep {
            subscriber, member, ..
        } => format!("keep {subscriber} {}", member.list),
        Decision::Add {
            subscriber, member, ..
        } => {
            format!("add {subscriber} {} {}", member.list, member.address)
        }
        Decision::Move {
            subscriber,
            before,
            after,
            ..
        } => format!("move {subscriber} {} -> {}", before.list, after.list),
        Decision::Remove {
            subscriber, member, ..
        } => {
            format!("remove {subscriber} {}", member.list)
        }
        Decision::Blocked { subscriber, reason } => format!("blocked {subscriber}: {reason}"),
    }
}
/// The operator approves one exact operation. Any other mutation or a Blocked decision stops the run.
fn check_plan(decisions: &[Decision], e: &Expect) -> Result<()> {
    let ops = decisions
        .iter()
        .filter(|d| !matches!(d, Decision::Keep { .. }))
        .collect::<Vec<_>>();
    ensure!(
        ops.len() == 1,
        "Plan has {} operations; expected exactly one",
        ops.len()
    );
    let same = |kind: &str, subscriber: &str, list: &str| {
        e.kind == kind && e.subscriber == subscriber && e.list == list
    };
    let ok = match ops[0] {
        Decision::Add {
            subscriber, member, ..
        } => same("add", subscriber, &member.list),
        Decision::Move {
            subscriber, after, ..
        } => same("move", subscriber, &after.list),
        Decision::Remove {
            subscriber, member, ..
        } => same("remove", subscriber, &member.list),
        Decision::Keep { .. } | Decision::Blocked { .. } => false,
    };
    ensure!(
        ok,
        "Plan operation differs from the expected operation: {}",
        describe(ops[0])
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let (mut root, mut expect, mut rollback_after, mut execute, mut no_finish) =
        (None, None, None, false, false);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--execute" => execute = true,
            // Acceptance of reviewed recovery: leave the verified journal and its marker in place.
            "--no-finish" => no_finish = true,
            "--expect" => expect = Some(parse_expect(it.next().context(USAGE)?)?),
            "--rollback-after" => {
                let secs: u64 = it.next().context(USAGE)?.parse().context(USAGE)?;
                // The source continuity window is at most 30 s, and one capture takes several seconds.
                ensure!(
                    (1..=10).contains(&secs),
                    "Rollback hold must be 1–10 seconds"
                );
                rollback_after = Some(secs);
            }
            s if !s.starts_with("--") && root.is_none() => root = Some(PathBuf::from(s)),
            _ => bail!(USAGE),
        }
    }
    let root = root.context(USAGE)?.canonicalize()?;
    let expect = expect.context(USAGE)?;
    ensure!(
        !root.starts_with("/opt/libreqos") && !root.starts_with("/etc/libreqos"),
        "Use an LQoSync application data directory"
    );
    let interval = store::load_settings(&root)?
        .wan_health
        .map_or(10, |h| h.interval_seconds);
    let mut source = RouterSource::new(&root)?;
    let mut ready = None;
    for round in 1..=MAX_ROUNDS {
        match source.preview().await {
            Ok(snapshot) => {
                let rt = snapshot.runtime();
                let health = rt
                    .health
                    .iter()
                    .map(|(w, s)| format!("{w}={:?}", s.status))
                    .collect::<Vec<_>>();
                let decisions = rt.preview.as_ref().map_or(vec![], |p| {
                    p.decisions.iter().map(describe).collect::<Vec<_>>()
                });
                println!("Round {round}: health {health:?}; decisions {decisions:?}");
                if rt
                    .preview
                    .as_ref()
                    .is_some_and(|p| check_plan(&p.decisions, &expect).is_ok())
                {
                    ready = Some(snapshot);
                    break;
                }
            }
            Err(e) => println!("Round {round}: {e:#}"),
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
    let snapshot = ready.context("No round produced exactly the expected operation")?;
    let prepared = Prepared::new(snapshot, APPROVAL_SECONDS)?;
    let revision = prepared.revision()?;
    println!("Batch revision: {revision}");
    if !execute {
        println!("Dry run: no write sent. Add --execute to run the worker.");
        return Ok(());
    }
    let mut worker = prepared.open(&root, &revision, source)?;
    let marker = "The recovery marker remains. Do not delete it. Use reviewed recovery.";
    worker
        .forward()
        .await
        .with_context(|| format!("Forward failed. {marker}"))?;
    println!("Forward complete and verified");
    if let Some(secs) = rollback_after {
        tokio::time::sleep(Duration::from_secs(secs)).await;
        worker
            .rollback()
            .await
            .with_context(|| format!("Rollback failed. {marker}"))?;
        println!("Rollback complete and verified");
    }
    if no_finish {
        println!("Stopped before finish. The recovery marker remains for batch {revision}.");
        return Ok(());
    }
    worker
        .finish()
        .with_context(|| format!("Finish failed. {marker}"))?;
    println!("Finished; recovery marker removed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lqosync_v3::core_migration::Member;

    const SUB: &str = "6c607ca4-1da1-51e6-a402-802aa35bd43b";
    fn m(list: &str) -> Member {
        Member {
            id: String::new(),
            list: list.into(),
            address: "10.0.100.220/32".into(),
            comment: String::new(),
            dynamic: false,
            disabled: false,
            timeout: None,
        }
    }
    fn add(list: &str) -> Decision {
        Decision::Add {
            subscriber: SUB.into(),
            router: "core".into(),
            member: m(list),
        }
    }
    fn keep() -> Decision {
        Decision::Keep {
            subscriber: "other".into(),
            router: "core".into(),
            member: m("WAN1-CUSTOMERS"),
        }
    }

    #[test]
    fn expect_parses_kind_subscriber_and_list() {
        let e = parse_expect(&format!("add:{SUB}:WAN1-CUSTOMERS")).unwrap();
        assert_eq!(
            (e.kind.as_str(), e.subscriber.as_str(), e.list.as_str()),
            ("add", SUB, "WAN1-CUSTOMERS")
        );
        assert!(parse_expect("add:not-a-uuid:WAN1-CUSTOMERS").is_err());
        assert!(parse_expect(&format!("delete:{SUB}:WAN1-CUSTOMERS")).is_err());
        assert!(parse_expect(&format!("add:{SUB}")).is_err());
    }
    #[test]
    fn plan_must_contain_exactly_the_expected_operation() {
        let e = parse_expect(&format!("add:{SUB}:WAN1-CUSTOMERS")).unwrap();
        assert!(
            check_plan(&[add("WAN1-CUSTOMERS"), keep()], &e).is_ok(),
            "Keep rows are ignored"
        );
        assert!(
            check_plan(&[add("WAN2-CUSTOMERS")], &e).is_err(),
            "wrong list"
        );
        assert!(check_plan(&[], &e).is_err(), "no operation");
        assert!(
            check_plan(&[add("WAN1-CUSTOMERS"), add("WAN1-CUSTOMERS")], &e).is_err(),
            "extra operation"
        );
        let blocked = Decision::Blocked {
            subscriber: "x".into(),
            reason: "r".into(),
        };
        assert!(
            check_plan(&[add("WAN1-CUSTOMERS"), blocked], &e).is_err(),
            "blocked decision"
        );
        let mv = Decision::Move {
            subscriber: SUB.into(),
            router: "core".into(),
            before: m("WAN1-CUSTOMERS"),
            after: m("WAN2-CUSTOMERS"),
        };
        let em = parse_expect(&format!("move:{SUB}:WAN2-CUSTOMERS")).unwrap();
        assert!(check_plan(std::slice::from_ref(&mv), &em).is_ok());
        assert!(check_plan(&[mv], &e).is_err(), "kind differs");
    }
}
