//! Manual RouterOS acceptance, Phase 1: one test host on one core router.
//! See docs/superpowers/specs/2026-10-31-routeros-acceptance-phase1-design.md.
use anyhow::{Context, Result, bail, ensure};
use lqosync_v3::{
    core_migration::{Member, RequestNotSent},
    core_plan,
    core_writer::{Evidence, Operation, Transport},
    core_writer_transport::{RouterTransport, Verifier, members},
    credentials, orchestrator,
    routeros::{self, Row},
    store,
};
use serde_json::json;
use std::{collections::BTreeMap, path::PathBuf};

const TEST_ADDRESS: &str = "192.0.2.254";
/// Fixed test identity. Owned markers require a canonical UUID.
const TEST_SUBSCRIBER: &str = "acce5500-0000-4000-8000-000000000001";
const ROUTER: &str = "RB5k9Core";
const LIST_PATH: &str = "/ip/firewall/address-list/print";
const TABLES: [&str; 3] = [
    "/ip/firewall/mangle/print",
    "/ip/route/print",
    "/routing/table/print",
];

/// Fresh address-list read of the test router. The transport preflight adds its scope checks.
struct Fresh {
    client: routeros::Client,
}
impl Verifier for Fresh {
    async fn verify(&mut self) -> Result<Evidence> {
        let now = orchestrator::now();
        Ok(Evidence {
            context: "acceptance-phase1".into(),
            captured_at: now,
            valid_until: now + 30,
            rows: [(ROUTER.to_string(), list(&self.client).await?)].into(),
        })
    }
}
async fn list(client: &routeros::Client) -> Result<Vec<Member>> {
    members(&client.read(LIST_PATH).await?)
}
async fn tables(client: &routeros::Client) -> Result<BTreeMap<&'static str, Vec<Row>>> {
    let mut out = BTreeMap::new();
    for t in TABLES {
        out.insert(t, strip_counters(&client.read(t).await?));
    }
    Ok(out)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let execute = match args.as_slice() {
        [] => false,
        [a] if a == "--execute" => true,
        _ => bail!("Usage: core-acceptance [--execute]"),
    };
    let root = std::env::var_os("LQOSYNC3_DATA")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?.join("data"))
        .canonicalize()?;
    ensure!(
        !root.starts_with("/opt/libreqos") && !root.starts_with("/etc/libreqos"),
        "Use the LQoSync application data directory"
    );
    let settings = store::load_settings(&root)?;
    let router = settings
        .routers
        .iter()
        .find(|r| r.id == ROUTER)
        .context("Router RB5k9Core is not configured")?
        .clone();
    let list_for = |wan: &str| {
        settings
            .core_lists
            .bindings
            .iter()
            .find(|b| b.router == ROUTER && b.wan == wan)
            .map(|b| b.list.clone())
            .with_context(|| format!("No {wan} binding on {ROUTER}"))
    };
    let (list1, list2) = (list_for("wan1")?, list_for("wan2")?);
    let owner = &settings.core_lists.owner;
    let mark1 = core_plan::marker(owner, TEST_SUBSCRIBER, "wan1");
    let mark2 = core_plan::marker(owner, TEST_SUBSCRIBER, "wan2");

    let password = credentials::password(&root, &router).await?;
    let client = routeros::Client::connect_with_password(&router, &password).await?;
    let baseline = list(&client).await?;
    check_same(&baseline, &list(&client).await?)
        .context("Two address-list reads without a change differ")?;
    check_clear(&baseline)?;
    let start_tables = tables(&client).await?;
    println!("Router: {ROUTER}; test host: {TEST_ADDRESS}");
    println!("Step 1: add to {list1} with comment {mark1}");
    println!("Step 2: move to {list2} with comment {mark2}; the row ID must not change");
    println!("Step 3: remove by row ID; all rows must equal the baseline");
    println!("Baseline: {} address-list rows", baseline.len());
    for (t, rows) in &start_tables {
        println!("Baseline: {t}: {} rows", rows.len());
    }
    if !execute {
        drop(password);
        println!("Dry run: no write sent. Add --execute to send the three test writes.");
        return Ok(());
    }
    let verifier = Fresh {
        client: routeros::Client::connect_with_password(&router, &password).await?,
    };
    drop(password);
    let mut transport = RouterTransport::new(&root, settings.clone(), verifier)?;

    let mut report = vec![];
    let (mut sent, mut test_id) = (false, None::<String>);
    let result: Result<()> = async {
        let want = Member {
            id: String::new(),
            list: list1.clone(),
            address: TEST_ADDRESS.into(),
            comment: mark1.clone(),
            dynamic: false,
            disabled: false,
            timeout: None,
        };
        let guard = transport.observe().await?;
        let add = Operation::Add {
            router: ROUTER.into(),
            member: want.clone(),
        };
        let id = match transport.execute(&add, &guard).await {
            Err(e) if e.downcast_ref::<RequestNotSent>().is_some() => return Err(e),
            r => {
                sent = true;
                r?.context("Add returned no row ID")?
            }
        };
        let added = list(&client).await?;
        if confirmed_test_row(&baseline, &added, &id, &mark1) {
            test_id = Some(id.clone());
        }
        report.push(json!({"step": "add", "id": id, "row": find(&added, &id).ok()}));
        check_added(&baseline, &added, &id, &want)?;
        println!("Add passed: row {id}");

        let before = find(&added, &id)?.clone();
        let after = Member {
            list: list2.clone(),
            comment: mark2.clone(),
            ..before.clone()
        };
        let guard = transport.observe().await?;
        let mv = Operation::Move {
            router: ROUTER.into(),
            before: before.clone(),
            after,
        };
        transport.execute(&mv, &guard).await?;
        let moved = list(&client).await?;
        report.push(json!({"step": "move", "id": id, "row": find(&moved, &id).ok()}));
        check_moved(&added, &moved, &id, &list2, &mark2)?;
        println!("Move passed: row {id} kept its ID");

        let row = find(&moved, &id)?.clone();
        let guard = transport.observe().await?;
        transport
            .execute(
                &Operation::Remove {
                    router: ROUTER.into(),
                    before: row,
                },
                &guard,
            )
            .await?;
        let removed = list(&client).await?;
        report.push(json!({"step": "remove", "id": id, "present": find(&removed, &id).is_ok()}));
        check_same(&baseline, &removed)?;
        test_id = None;
        println!("Remove passed: address-list equals the baseline");

        let end_tables = tables(&client).await?;
        for (t, rows) in &start_tables {
            ensure!(
                end_tables.get(t) == Some(rows),
                "{t} changed during the test"
            );
        }
        println!("Baseline tables unchanged");
        Ok(())
    }
    .await;
    let now = orchestrator::now();
    let outcome = match &result {
        Ok(()) => "passed".to_string(),
        Err(e) => format!("{e:#}"),
    };
    report.push(json!({"outcome": outcome}));
    // Print cleanup guidance before the report write, so a write failure cannot hide it.
    if result.is_err() {
        if let Some(id) = &test_id {
            eprintln!("Test row {id} can remain. Remove only that row:");
            eprintln!("/ip firewall address-list remove {id}");
        } else if sent {
            eprintln!(
                "Check for a row with address {TEST_ADDRESS} and comment {mark1} or {mark2}."
            );
            eprintln!("Remove only that row.");
        }
    }
    let written = serde_json::to_string_pretty(&report)
        .map_err(anyhow::Error::from)
        .and_then(|text| {
            store::atomic_write(
                &root.join(format!("state/acceptance/phase1-{now}.json")),
                &text,
            )
        });
    result?;
    written?;
    println!("Phase 1 passed");
    Ok(())
}

fn by_id(rows: &[Member]) -> BTreeMap<&str, &Member> {
    rows.iter().map(|r| (r.id.as_str(), r)).collect()
}
fn others<'a>(rows: &'a [Member], id: &str) -> BTreeMap<&'a str, &'a Member> {
    let mut map = by_id(rows);
    map.remove(id);
    map
}
fn find<'a>(rows: &'a [Member], id: &str) -> Result<&'a Member> {
    rows.iter()
        .find(|r| r.id == id)
        .with_context(|| format!("Test row {id} is missing"))
}
/// True only for a new row with the test address and marker. Only such a row may be named for removal.
fn confirmed_test_row(baseline: &[Member], added: &[Member], id: &str, marker: &str) -> bool {
    !by_id(baseline).contains_key(id)
        && find(added, id).is_ok_and(|r| {
            r.address.strip_suffix("/32").unwrap_or(&r.address) == TEST_ADDRESS
                && r.comment == marker
        })
}
/// Stop before any write if the test host or its marker already exists.
fn check_clear(rows: &[Member]) -> Result<()> {
    let marker = format!(":{TEST_SUBSCRIBER}:");
    for r in rows {
        ensure!(
            r.address.strip_suffix("/32").unwrap_or(&r.address) != TEST_ADDRESS,
            "Test address already exists as {}",
            r.id
        );
        ensure!(
            !r.comment.contains(&marker),
            "Test marker already exists on {}",
            r.id
        );
    }
    Ok(())
}
fn check_added(before: &[Member], after: &[Member], id: &str, expected: &Member) -> Result<()> {
    ensure!(
        !by_id(before).contains_key(id),
        "Returned ID {id} existed before the add"
    );
    ensure!(
        others(after, id) == by_id(before),
        "Other address-list rows changed during the add"
    );
    let row = find(after, id)?;
    let want = Member {
        id: id.into(),
        ..expected.clone()
    };
    ensure!(*row == want, "New row fields differ: {row:?}");
    Ok(())
}
fn check_moved(
    before: &[Member],
    after: &[Member],
    id: &str,
    list: &str,
    comment: &str,
) -> Result<()> {
    ensure!(
        others(after, id) == others(before, id),
        "Other address-list rows changed during the move"
    );
    let old = find(before, id)?;
    let want = Member {
        list: list.into(),
        comment: comment.into(),
        ..old.clone()
    };
    ensure!(
        *find(after, id)? == want,
        "Move changed more than list and comment"
    );
    Ok(())
}
fn check_same(before: &[Member], after: &[Member]) -> Result<()> {
    ensure!(
        by_id(before) == by_id(after),
        "Address-list rows differ from the baseline"
    );
    Ok(())
}
fn strip_counters(rows: &[Row]) -> Vec<Row> {
    rows.iter()
        .map(|r| {
            r.iter()
                .filter(|(k, _)| !matches!(k.as_str(), "bytes" | "packets"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(id: &str, list: &str, address: &str, comment: &str) -> Member {
        Member {
            id: id.into(),
            list: list.into(),
            address: address.into(),
            comment: comment.into(),
            dynamic: false,
            disabled: false,
            timeout: None,
        }
    }
    fn base() -> Vec<Member> {
        vec![m("*A", "WAN1-CUSTOMERS", "10.15.0.0/24", "IPoE vlan10")]
    }
    const W1: &str = "lqosync3:o:acce5500-0000-4000-8000-000000000001:wan1";
    const W2: &str = "lqosync3:o:acce5500-0000-4000-8000-000000000001:wan2";

    #[test]
    fn cleanup_id_only_for_confirmed_new_test_row() {
        let new = m("*20", "WAN1-CUSTOMERS", TEST_ADDRESS, W1);
        let added = [base(), vec![new]].concat();
        assert!(confirmed_test_row(&base(), &added, "*20", W1));
        // A returned ID that names an existing production row must never become a removal target.
        assert!(!confirmed_test_row(&base(), &added, "*A", W1));
        // A returned ID that is absent from the read is not a confirmed test row.
        assert!(!confirmed_test_row(&base(), &added, "*21", W1));
        let wrong = [base(), vec![m("*20", "WAN1-CUSTOMERS", "10.0.0.9", W1)]].concat();
        assert!(!confirmed_test_row(&base(), &wrong, "*20", W1));
    }
    #[test]
    fn test_subscriber_is_canonical_uuid() {
        let id = uuid::Uuid::parse_str(TEST_SUBSCRIBER).expect("UUID");
        assert!(!id.is_nil());
        assert_eq!(id.to_string(), TEST_SUBSCRIBER);
    }
    #[test]
    fn clear_rejects_test_address() {
        assert!(check_clear(&[m("*1", "X", "192.0.2.254", "")]).is_err());
        assert!(check_clear(&[m("*1", "X", "192.0.2.254/32", "")]).is_err());
    }
    #[test]
    fn clear_rejects_test_marker() {
        assert!(check_clear(&[m("*1", "X", "10.0.0.9", W1)]).is_err());
    }
    #[test]
    fn clear_accepts_other_rows() {
        assert!(check_clear(&base()).is_ok());
    }
    #[test]
    fn added_accepts_one_new_row() {
        let new = m("*20", "WAN1-CUSTOMERS", TEST_ADDRESS, W1);
        let after = [base(), vec![new.clone()]].concat();
        assert!(
            check_added(
                &base(),
                &after,
                "*20",
                &m("", "WAN1-CUSTOMERS", TEST_ADDRESS, W1)
            )
            .is_ok()
        );
        assert!(check_added(&base(), &after, "*21", &new).is_err());
    }
    #[test]
    fn added_rejects_changed_other_row() {
        let mut after = base();
        after[0].comment = "changed".into();
        after.push(m("*20", "WAN1-CUSTOMERS", TEST_ADDRESS, W1));
        assert!(
            check_added(
                &base(),
                &after,
                "*20",
                &m("", "WAN1-CUSTOMERS", TEST_ADDRESS, W1)
            )
            .is_err()
        );
    }
    #[test]
    fn added_rejects_wrong_fields() {
        let after = [base(), vec![m("*20", "WAN2-CUSTOMERS", TEST_ADDRESS, W1)]].concat();
        assert!(
            check_added(
                &base(),
                &after,
                "*20",
                &m("", "WAN1-CUSTOMERS", TEST_ADDRESS, W1)
            )
            .is_err()
        );
    }
    #[test]
    fn moved_accepts_list_and_comment_change() {
        let before = [base(), vec![m("*20", "WAN1-CUSTOMERS", TEST_ADDRESS, W1)]].concat();
        let after = [base(), vec![m("*20", "WAN2-CUSTOMERS", TEST_ADDRESS, W2)]].concat();
        assert!(check_moved(&before, &after, "*20", "WAN2-CUSTOMERS", W2).is_ok());
    }
    #[test]
    fn moved_rejects_new_id() {
        let before = [base(), vec![m("*20", "WAN1-CUSTOMERS", TEST_ADDRESS, W1)]].concat();
        let after = [base(), vec![m("*21", "WAN2-CUSTOMERS", TEST_ADDRESS, W2)]].concat();
        assert!(check_moved(&before, &after, "*20", "WAN2-CUSTOMERS", W2).is_err());
    }
    #[test]
    fn moved_rejects_changed_address() {
        let before = [base(), vec![m("*20", "WAN1-CUSTOMERS", TEST_ADDRESS, W1)]].concat();
        let after = [base(), vec![m("*20", "WAN2-CUSTOMERS", "192.0.2.253", W2)]].concat();
        assert!(check_moved(&before, &after, "*20", "WAN2-CUSTOMERS", W2).is_err());
    }
    #[test]
    fn same_detects_any_change() {
        assert!(check_same(&base(), &base()).is_ok());
        let mut after = base();
        after[0].disabled = true;
        assert!(check_same(&base(), &after).is_err());
    }
    #[test]
    fn strip_counters_ignores_bytes_and_packets() {
        let row = |b: &str| -> Row {
            [("chain", "output"), ("bytes", b), ("packets", b)]
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect()
        };
        assert_eq!(strip_counters(&[row("1")]), strip_counters(&[row("9")]));
        assert_eq!(strip_counters(&[row("1")])[0].len(), 1);
    }
}
