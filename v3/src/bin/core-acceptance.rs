//! Manual RouterOS acceptance on one core router.
//! Phase 1: one test host. Phase 2: one live subscriber moves between WANs.
//! See docs/superpowers/specs/2026-10-31-routeros-acceptance-phase1-design.md
//! and docs/superpowers/specs/2026-10-31-routeros-acceptance-phase2-design.md.
use anyhow::{Context, Result, bail, ensure};
use lqosync_v3::{
    core_migration::{Member, RequestNotSent},
    core_plan,
    core_writer::{Evidence, Operation, Transport},
    core_writer_transport::{RouterTransport, Verifier, members},
    credentials,
    model::Settings,
    orchestrator,
    routeros::{self, Row},
    store,
};
use serde_json::json;
use std::{collections::BTreeMap, net::Ipv4Addr, path::Path, path::PathBuf, time::Duration};

const TEST_ADDRESS: &str = "192.0.2.254";
/// Fixed test identity. Owned markers require a canonical UUID.
const TEST_SUBSCRIBER: &str = "acce5500-0000-4000-8000-000000000001";
const ROUTER: &str = "RB5k9Core";
const LIST_PATH: &str = "/ip/firewall/address-list/print";
const CONNECTIONS: &str = "/ip/firewall/connection/print";
const PPP: &str = "/ppp/active/print";
const HOLD_SAMPLES: u32 = 4;
const SAMPLE_SECONDS: u64 = 15;
const TABLES: [&str; 3] = [
    "/ip/firewall/mangle/print",
    "/ip/route/print",
    "/routing/table/print",
];
const USAGE: &str = "Usage: core-acceptance [--address IPV4 --subscriber UUID] [--execute]";

#[derive(Debug)]
struct Args {
    address: String,
    subscriber: String,
    /// True for a live subscriber (Phase 2). False for the reserved test host (Phase 1).
    live: bool,
    execute: bool,
}
fn parse_args(args: &[String]) -> Result<Args> {
    let (mut address, mut subscriber, mut execute) = (None, None, false);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--execute" => execute = true,
            "--address" => address = Some(it.next().context(USAGE)?.clone()),
            "--subscriber" => subscriber = Some(it.next().context(USAGE)?.clone()),
            _ => bail!(USAGE),
        }
    }
    match (address, subscriber) {
        (None, None) => Ok(Args {
            address: TEST_ADDRESS.into(),
            subscriber: TEST_SUBSCRIBER.into(),
            live: false,
            execute,
        }),
        (Some(address), Some(subscriber)) => {
            address
                .parse::<Ipv4Addr>()
                .context("Address must be one IPv4 host")?;
            ensure!(
                address != TEST_ADDRESS,
                "Use the default mode for the test host"
            );
            let id = uuid::Uuid::parse_str(&subscriber).context("Subscriber must be a UUID")?;
            ensure!(
                !id.is_nil() && id.to_string() == subscriber && subscriber != TEST_SUBSCRIBER,
                "Subscriber must be the canonical UUID of a real subscriber"
            );
            Ok(Args {
                address,
                subscriber,
                live: true,
                execute,
            })
        }
        _ => bail!(USAGE),
    }
}

/// Fresh address-list read of the test router. The transport preflight adds its scope checks.
struct Fresh {
    client: routeros::Client,
}
impl Verifier for Fresh {
    async fn verify(&mut self) -> Result<Evidence> {
        let now = orchestrator::now();
        Ok(Evidence {
            context: "acceptance".into(),
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
/// Active PPP sessions on every configured router except the core, tagged with the router ID.
async fn sessions(root: &Path, settings: &Settings) -> Result<Vec<Row>> {
    let mut out = vec![];
    for r in settings.routers.iter().filter(|r| r.id != ROUTER) {
        let password = credentials::password(root, r).await?;
        let client = routeros::Client::connect_with_password(r, &password).await?;
        drop(password);
        for mut row in client.read(PPP).await? {
            row.insert("router".into(), r.id.clone());
            out.push(row);
        }
    }
    Ok(out)
}
/// A failed connection sample is recorded, not fatal. A stop here would leave the subscriber on wan2.
async fn sample(client: &routeros::Client, ip: &str) -> serde_json::Value {
    match client.read(CONNECTIONS).await {
        Ok(rows) => json!(wan_paths(&rows, ip)),
        Err(e) => json!({"sample_error": format!("{e:#}")}),
    }
}

enum Step {
    Add(String, String),
    Move(String, String),
    Hold,
    Remove,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args(&std::env::args().skip(1).collect::<Vec<_>>())?;
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
    let mark1 = core_plan::marker(owner, &args.subscriber, "wan1");
    let mark2 = core_plan::marker(owner, &args.subscriber, "wan2");
    let steps = if args.live {
        vec![
            Step::Add(list1.clone(), mark1.clone()),
            Step::Move(list2.clone(), mark2.clone()),
            Step::Hold,
            Step::Move(list1.clone(), mark1.clone()),
            Step::Remove,
        ]
    } else {
        vec![
            Step::Add(list1.clone(), mark1.clone()),
            Step::Move(list2.clone(), mark2.clone()),
            Step::Remove,
        ]
    };

    let password = credentials::password(&root, &router).await?;
    let client = routeros::Client::connect_with_password(&router, &password).await?;
    let baseline = list(&client).await?;
    check_same(&baseline, &list(&client).await?)
        .context("Two address-list reads without a change differ")?;
    check_clear(&baseline, &args.address, &args.subscriber)?;
    let start_tables = tables(&client).await?;
    let session = if args.live {
        let rows = sessions(&root, &settings).await?;
        let s = session_for(&rows, &args.address)?.clone();
        println!(
            "Session: {} on {} ({})",
            s.get("name").map_or("?", String::as_str),
            s.get("router").map_or("?", String::as_str),
            s.get(".id").map_or("?", String::as_str)
        );
        Some(s)
    } else {
        None
    };
    println!("Router: {ROUTER}; host: {}", args.address);
    for (n, step) in steps.iter().enumerate() {
        match step {
            Step::Add(l, m) => println!("Step {}: add to {l} with comment {m}", n + 1),
            Step::Move(l, m) => println!(
                "Step {}: move to {l} with comment {m}; the row ID must not change",
                n + 1
            ),
            Step::Hold => println!(
                "Step {}: hold {}s and sample connections",
                n + 1,
                u64::from(HOLD_SAMPLES) * SAMPLE_SECONDS
            ),
            Step::Remove => println!(
                "Step {}: remove by row ID; all rows must equal the baseline",
                n + 1
            ),
        }
    }
    println!("Baseline: {} address-list rows", baseline.len());
    for (t, rows) in &start_tables {
        println!("Baseline: {t}: {} static rows", rows.len());
    }
    let mut report = vec![];
    if args.live {
        let p = sample(&client, &args.address).await;
        println!("Before: connections by public address: {p}");
        report.push(json!({"step": "before", "paths": p}));
    }
    if !args.execute {
        drop(password);
        println!("Dry run: no write sent. Add --execute to send the writes.");
        return Ok(());
    }
    let verifier = Fresh {
        client: routeros::Client::connect_with_password(&router, &password).await?,
    };
    drop(password);
    let mut transport = RouterTransport::new(&root, settings.clone(), verifier)?;

    let (mut sent, mut test_id) = (false, None::<String>);
    let result: Result<()> = async {
        let mut current = baseline.clone();
        for (n, step) in steps.iter().enumerate() {
            if let Some(start) = &session
                && needs_session(step, &list2)
            {
                same_session(start, &sessions(&root, &settings).await?, &args.address)?;
            }
            match step {
                Step::Add(l, m) => {
                    let want = Member {
                        id: String::new(),
                        list: l.clone(),
                        address: args.address.clone(),
                        comment: m.clone(),
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
                    if confirmed_test_row(&current, &added, &id, &args.address, m) {
                        test_id = Some(id.clone());
                    }
                    report.push(json!({"step": "add", "id": id, "row": find(&added, &id).ok()}));
                    check_added(&current, &added, &id, &want)?;
                    println!("Add passed: row {id}");
                    current = added;
                }
                Step::Move(l, m) => {
                    let id = test_id.clone().context("No confirmed row to move")?;
                    let before = find(&current, &id)?.clone();
                    let after = Member {
                        list: l.clone(),
                        comment: m.clone(),
                        ..before.clone()
                    };
                    let guard = transport.observe().await?;
                    let mv = Operation::Move {
                        router: ROUTER.into(),
                        before,
                        after,
                    };
                    transport.execute(&mv, &guard).await?;
                    let moved = list(&client).await?;
                    report.push(json!({"step": "move", "id": id, "row": find(&moved, &id).ok()}));
                    check_moved(&current, &moved, &id, l, m)?;
                    println!("Move passed: row {id} kept its ID in {l}");
                    current = moved;
                }
                Step::Hold => {
                    for n in 1..=HOLD_SAMPLES {
                        tokio::time::sleep(Duration::from_secs(SAMPLE_SECONDS)).await;
                        let p = sample(&client, &args.address).await;
                        println!("Hold sample {n}: connections by public address: {p}");
                        report.push(json!({"step": "hold", "sample": n, "paths": p}));
                    }
                }
                Step::Remove => {
                    let id = test_id.clone().context("No confirmed row to remove")?;
                    let row = find(&current, &id)?.clone();
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
                    let present = find(&removed, &id).is_ok();
                    report.push(json!({"step": "remove", "id": id, "present": present}));
                    if !present {
                        (test_id, sent) = (None, false);
                    }
                    check_same(&baseline, &removed)?;
                    println!("Remove passed: address-list equals the baseline");
                    current = removed;
                }
            }
            if args.live && !matches!(step, Step::Hold) {
                let p = sample(&client, &args.address).await;
                println!("After step {}: connections by public address: {p}", n + 1);
                report.push(json!({"after_step": n + 1, "paths": p}));
            }
        }
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
            eprintln!("Row {id} can remain. Remove only that row:");
            eprintln!("/ip firewall address-list remove {id}");
        } else if sent {
            eprintln!(
                "Check for a row with address {} and comment {mark1} or {mark2}.",
                args.address
            );
            eprintln!("Remove only that row.");
        }
        if args.live && (test_id.is_some() || sent) {
            eprintln!("The subscriber may use wan2 until this row is removed.");
        }
    }
    let phase = if args.live { "phase2" } else { "phase1" };
    let written = serde_json::to_string_pretty(&report)
        .map_err(anyhow::Error::from)
        .and_then(|text| {
            store::atomic_write(
                &root.join(format!("state/acceptance/{phase}-{now}.json")),
                &text,
            )
        });
    result?;
    written?;
    println!("Acceptance passed");
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
fn bare(address: &str) -> &str {
    address.strip_suffix("/32").unwrap_or(address)
}
/// True only for a new row with the target address and marker. Only such a row may be named for removal.
fn confirmed_test_row(
    baseline: &[Member],
    added: &[Member],
    id: &str,
    address: &str,
    marker: &str,
) -> bool {
    !by_id(baseline).contains_key(id)
        && find(added, id).is_ok_and(|r| bare(&r.address) == address && r.comment == marker)
}
/// Stop before any write if the target host or its subscriber marker already exists.
fn check_clear(rows: &[Member], address: &str, subscriber: &str) -> Result<()> {
    let marker = format!(":{subscriber}:");
    for r in rows {
        ensure!(
            bare(&r.address) != address,
            "Address already exists as {}",
            r.id
        );
        ensure!(
            !r.comment.contains(&marker),
            "Subscriber marker already exists on {}",
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
/// Removes counters and dynamic rows. Dynamic OSPF routes for PPP clients can change during a test.
fn strip_counters(rows: &[Row]) -> Vec<Row> {
    rows.iter()
        .filter(|r| r.get("dynamic").is_none_or(|d| d != "true"))
        .map(|r| {
            r.iter()
                .filter(|(k, _)| !matches!(k.as_str(), "bytes" | "packets"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .collect()
}
/// Only steps that route the subscriber onto a new path need the same session.
/// Reverting or removing our own row is always safe.
fn needs_session(step: &Step, list2: &str) -> bool {
    match step {
        Step::Add(..) => true,
        Step::Move(list, _) => list == list2,
        Step::Hold | Step::Remove => false,
    }
}
fn session_for<'a>(rows: &'a [Row], ip: &str) -> Result<&'a Row> {
    let hits = rows
        .iter()
        .filter(|r| r.get("address").is_some_and(|a| a == ip))
        .collect::<Vec<_>>();
    ensure!(
        hits.len() == 1,
        "Expected one active PPP session for {ip}, found {}",
        hits.len()
    );
    Ok(hits[0])
}
/// A reconnect gives a new session ID. Then the address can belong to another subscriber.
fn same_session(start: &Row, rows: &[Row], ip: &str) -> Result<()> {
    let now = session_for(rows, ip)?;
    for key in [".id", "name", "router"] {
        ensure!(
            now.get(key) == start.get(key),
            "PPP session for {ip} changed; stop"
        );
    }
    Ok(())
}
fn host(address: &str) -> &str {
    address.split_once(':').map_or(address, |(h, _)| h)
}
/// Connection count per reply destination address. For source NAT that is the public WAN address.
fn wan_paths(rows: &[Row], ip: &str) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for r in rows {
        if r.get("src-address").is_some_and(|s| host(s) == ip)
            && let Some(d) = r.get("reply-dst-address")
        {
            *out.entry(host(d).to_string()).or_insert(0) += 1;
        }
    }
    out
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

    fn r(fields: &[(&str, &str)]) -> Row {
        fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }
    #[test]
    fn args_select_phase_and_validate_inputs() {
        let a = |v: &[&str]| parse_args(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        let p1 = a(&[]).unwrap();
        assert_eq!(
            (p1.address.as_str(), p1.live, p1.execute),
            (TEST_ADDRESS, false, false)
        );
        let p2 = a(&[
            "--address",
            "10.0.100.220",
            "--subscriber",
            "6c607ca4-1da1-51e6-a402-802aa35bd43b",
            "--execute",
        ])
        .unwrap();
        assert_eq!(
            (p2.address.as_str(), p2.live, p2.execute),
            ("10.0.100.220", true, true)
        );
        assert_eq!(p2.subscriber, "6c607ca4-1da1-51e6-a402-802aa35bd43b");
        assert!(
            a(&["--address", "10.0.100.220"]).is_err(),
            "address without subscriber"
        );
        assert!(
            a(&["--subscriber", "6c607ca4-1da1-51e6-a402-802aa35bd43b"]).is_err(),
            "subscriber without address"
        );
        assert!(
            a(&[
                "--address",
                "10.0.100.0/24",
                "--subscriber",
                "6c607ca4-1da1-51e6-a402-802aa35bd43b"
            ])
            .is_err()
        );
        assert!(
            a(&[
                "--address",
                "10.0.100.220",
                "--subscriber",
                "RowenaEstilloso"
            ])
            .is_err()
        );
        assert!(
            a(&["--address", "10.0.100.220", "--subscriber", TEST_SUBSCRIBER]).is_err(),
            "test identity on a live address"
        );
        assert!(a(&["--bogus"]).is_err());
    }
    #[test]
    fn session_check_only_guards_steps_that_route_the_subscriber_away() {
        let (l1, l2) = ("WAN1-CUSTOMERS".to_string(), "WAN2-CUSTOMERS".to_string());
        assert!(needs_session(&Step::Add(l1.clone(), W1.into()), &l2));
        assert!(needs_session(&Step::Move(l2.clone(), W2.into()), &l2));
        // Reverting our own row is always safe, so a session change must not block it.
        assert!(!needs_session(&Step::Move(l1, W1.into()), &l2));
        assert!(!needs_session(&Step::Remove, &l2));
        assert!(!needs_session(&Step::Hold, &l2));
    }
    #[test]
    fn session_requires_exactly_one_active_match() {
        let s1 = r(&[(".id", "*5"), ("name", "u1"), ("address", "10.0.100.220")]);
        let s2 = r(&[(".id", "*6"), ("name", "u2"), ("address", "10.0.100.221")]);
        assert_eq!(
            session_for(&[s1.clone(), s2.clone()], "10.0.100.220")
                .unwrap()
                .get(".id")
                .unwrap(),
            "*5"
        );
        assert!(session_for(std::slice::from_ref(&s2), "10.0.100.220").is_err());
        assert!(session_for(&[s1.clone(), s1.clone()], "10.0.100.220").is_err());
        let moved = r(&[(".id", "*7"), ("name", "u1"), ("address", "10.0.100.220")]);
        assert!(
            same_session(&s1, &[moved], "10.0.100.220").is_err(),
            "reconnect gives a new session ID"
        );
        assert!(same_session(&s1, std::slice::from_ref(&s1), "10.0.100.220").is_ok());
    }
    #[test]
    fn wan_paths_counts_reply_destination_per_subscriber() {
        let rows = [
            r(&[
                ("src-address", "10.0.100.220:5000"),
                ("reply-dst-address", "203.0.113.5:6000"),
            ]),
            r(&[
                ("src-address", "10.0.100.220:5001"),
                ("reply-dst-address", "203.0.113.5:6001"),
            ]),
            r(&[
                ("src-address", "10.0.100.220"),
                ("reply-dst-address", "198.51.100.9"),
            ]),
            r(&[
                ("src-address", "10.0.100.221:5000"),
                ("reply-dst-address", "203.0.113.5:7000"),
            ]),
            r(&[
                ("src-address", "10.0.100.2200:1"),
                ("reply-dst-address", "203.0.113.5:1"),
            ]),
        ];
        let paths = wan_paths(&rows, "10.0.100.220");
        assert_eq!(paths.get("203.0.113.5"), Some(&2));
        assert_eq!(paths.get("198.51.100.9"), Some(&1));
        assert_eq!(paths.len(), 2);
    }
    #[test]
    fn strip_counters_drops_dynamic_rows() {
        let rows = [
            r(&[("dst-address", "10.0.100.220/32"), ("dynamic", "true")]),
            r(&[("dst-address", "0.0.0.0/0"), ("dynamic", "false")]),
        ];
        assert_eq!(strip_counters(&rows).len(), 1);
    }
    #[test]
    fn cleanup_id_only_for_confirmed_new_test_row() {
        let new = m("*20", "WAN1-CUSTOMERS", TEST_ADDRESS, W1);
        let added = [base(), vec![new]].concat();
        assert!(confirmed_test_row(&base(), &added, "*20", TEST_ADDRESS, W1));
        // A returned ID that names an existing production row must never become a removal target.
        assert!(!confirmed_test_row(&base(), &added, "*A", TEST_ADDRESS, W1));
        // A returned ID that is absent from the read is not a confirmed test row.
        assert!(!confirmed_test_row(
            &base(),
            &added,
            "*21",
            TEST_ADDRESS,
            W1
        ));
        let wrong = [base(), vec![m("*20", "WAN1-CUSTOMERS", "10.0.0.9", W1)]].concat();
        assert!(!confirmed_test_row(
            &base(),
            &wrong,
            "*20",
            TEST_ADDRESS,
            W1
        ));
    }
    #[test]
    fn test_subscriber_is_canonical_uuid() {
        let id = uuid::Uuid::parse_str(TEST_SUBSCRIBER).expect("UUID");
        assert!(!id.is_nil());
        assert_eq!(id.to_string(), TEST_SUBSCRIBER);
    }
    #[test]
    fn clear_rejects_test_address() {
        assert!(
            check_clear(
                &[m("*1", "X", "192.0.2.254", "")],
                TEST_ADDRESS,
                TEST_SUBSCRIBER
            )
            .is_err()
        );
        assert!(
            check_clear(
                &[m("*1", "X", "192.0.2.254/32", "")],
                TEST_ADDRESS,
                TEST_SUBSCRIBER
            )
            .is_err()
        );
    }
    #[test]
    fn clear_rejects_test_marker() {
        assert!(
            check_clear(
                &[m("*1", "X", "10.0.0.9", W1)],
                TEST_ADDRESS,
                TEST_SUBSCRIBER
            )
            .is_err()
        );
    }
    #[test]
    fn clear_accepts_other_rows() {
        assert!(check_clear(&base(), TEST_ADDRESS, TEST_SUBSCRIBER).is_ok());
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
