//! Task 6 shadow acceptance: legacy and topology generators read the same frozen router rows.
//! The topology candidate stays in memory or in scratch space. Production paths never change.
use super::enrollment::cli_flow::{AGE, confirm, files, setup};
use super::*;
use crate::topology_manager::enroll_mapping::RowDecision;
use crate::topology_manager::parity::{ParityIssue, compare_legacy};
use crate::topology_manager::policy_draft::tests as scenario;

/// A protected static row below a static root node. No router source produces it.
const STATIC: &str = "S0001,fixed-ap,S0002,fixed-ap,Management,static-root,,02:00:00:00:ff:01,100.64.9.1,,5,5,10,10,static";

/// The live legacy pair (with one protected static row) and an enrolled, not activated, manager.
async fn enrolled(e: &Env) -> Runner<FakeConnector> {
    let config = setup(e, None);
    let csv = e.lqos.join("ShapedDevices.csv");
    std::fs::write(&csv, format!("{}{STATIC}\n", read(&csv))).unwrap();
    let path = e.lqos.join("network.json");
    let mut network: serde_json::Value = serde_json::from_str(&read(&path)).unwrap();
    network["Management"] = serde_json::json!({"id":"static-root","type":"site",
        "downloadBandwidthMbps":20,"uploadBandwidthMbps":10,"children":{}});
    std::fs::write(&path, render_network_json(&network)).unwrap();
    let mut r = runner(e, config);
    let draft = r.propose_policy(AGE).await.unwrap();
    r.install_topology_policy(&draft).unwrap();
    let mut file = r.propose_enrollment(None, AGE, false).await.unwrap();
    file.static_roots = vec!["static-root".into()];
    for row in &mut file.rows {
        if row.legacy_circuit_name == "fixed-ap" {
            row.decision = RowDecision::Protect;
        }
    }
    // The operator names the static root, then asks for the mapping again (the CLI flow).
    let mut file = r.propose_enrollment(Some(&file), AGE, false).await.unwrap();
    confirm(&mut file);
    r.enroll_with_mapping(&file, AGE, true, false)
        .await
        .unwrap();
    r
}

fn live(e: &Env) -> (Vec<crate::model::CsvRow>, serde_json::Value) {
    (
        crate::state::read_csv(&read(&e.lqos.join("ShapedDevices.csv"))).unwrap(),
        serde_json::from_str(&read(&e.lqos.join("network.json"))).unwrap(),
    )
}

#[tokio::test]
async fn shadow_candidate_matches_the_legacy_pair_by_identity() {
    let e = env();
    let mut r = enrolled(&e).await;
    let before = (live(&e), e.lqos.join("reloaded").exists());
    let pair = r.prepare_topology(AGE).await.unwrap();
    let (rows, network) = live(&e);
    assert_eq!(compare_legacy(&pair, &rows, &network).unwrap(), vec![]);
    assert_eq!(pair.rows().len(), 8);
    assert!(pair.rows().iter().any(|row| row.circuit_name == "fixed-ap"));
    // Neither generator reads an IPv6 source in this release.
    assert!(pair.rows().iter().all(|row| row.ipv6.is_empty()));
    assert!(rows.iter().all(|row| row.ipv6.is_empty()));
    assert_eq!((live(&e), e.lqos.join("reloaded").exists()), before);
    assert!(!crate::topology_manager::runtime::topology_owns(&e.state).unwrap());
    assert!(!e.state.join("candidate/ShapedDevices.csv").exists());
    // The comparison is not blind: a lost, an extra, or a changed record is reported.
    let mut tampered = rows.clone();
    let lost = tampered.remove(0);
    tampered[0].parent_node_id = "elsewhere".into();
    let mut extra = lost.clone();
    extra.circuit_id = "X1".into();
    extra.device_id = "X2".into();
    tampered.push(extra);
    let issues = compare_legacy(&pair, &tampered, &network).unwrap();
    assert!(issues.contains(&added(&lost)), "{issues:?}");
    assert!(issues.contains(&changed(&tampered[0])), "{issues:?}");
    assert!(issues.contains(&ParityIssue::MissingRow {
        circuit_id: "X1".into(),
        device_id: "X2".into()
    }));
    let mut other = network.clone();
    other["Gold-Hub-1"]["downloadBandwidthMbps"] = serde_json::json!(60);
    assert!(
        compare_legacy(&pair, &rows, &other)
            .unwrap()
            .contains(&ParityIssue::ChangedNetwork)
    );
}

fn added(row: &crate::model::CsvRow) -> ParityIssue {
    ParityIssue::AddedRow {
        circuit_id: row.circuit_id.clone(),
        device_id: row.device_id.clone(),
    }
}

fn changed(row: &crate::model::CsvRow) -> ParityIssue {
    ParityIssue::ChangedRow {
        circuit_id: row.circuit_id.clone(),
        device_id: row.device_id.clone(),
    }
}

fn by_name<'a>(rows: &'a [crate::model::CsvRow], name: &str) -> &'a crate::model::CsvRow {
    rows.iter().find(|row| row.circuit_name == name).unwrap()
}

#[tokio::test]
async fn a_reconnect_keeps_identities_and_changes_only_the_endpoint() {
    let e = env();
    let mut r = enrolled(&e).await;
    let (config, mut snap) = scenario::scenario();
    // The same account reconnects with a new session ID and address.
    snap.ppp_active[3].insert("address".into(), "100.64.0.99".into());
    {
        let mut rows = e.fake.rows.lock().unwrap();
        let active = rows
            .get_mut("Hub-1")
            .unwrap()
            .get_mut("/ppp/active")
            .unwrap();
        let s1 = active.iter_mut().find(|row| row["name"] == "s1").unwrap();
        s1.insert("address".into(), "100.64.0.99".into());
        s1.insert(".id".into(), "*a-new".into());
    }
    let pair = r.prepare_topology(AGE).await.unwrap();
    let (rows, network) = live(&e);
    let s1 = by_name(&rows, "s1");
    assert_eq!(
        compare_legacy(&pair, &rows, &network).unwrap(),
        vec![changed(s1)]
    );
    let moved = by_name(pair.rows(), "s1");
    assert_eq!((moved.ipv4.as_str(), &moved.mac), ("100.64.0.99", &s1.mac));
    // The legacy generator puts the same account on the same new address.
    assert_eq!(
        by_name(&scenario::legacy(&config, &snap).rows, "s1").ipv4,
        "100.64.0.99"
    );
}

#[tokio::test]
async fn an_exclusion_removes_the_same_subscriber_in_both_generators() {
    let e = env();
    let mut r = enrolled(&e).await;
    r.cfg.exclude.ips.push("100.64.0.13".into());
    let (mut config, snap) = scenario::scenario();
    config.exclude.ips.push("100.64.0.13".into());
    let legacy = scenario::legacy(&config, &snap);
    assert!(legacy.rows.iter().all(|row| row.circuit_name != "g3"));
    let pair = r.prepare_topology(AGE).await.unwrap();
    let (mut rows, mut network) = live(&e);
    rows.retain(|row| row.circuit_name != "g3");
    // The legacy plan node shrinks from three to two subscribers in both generators.
    network["Gold-Hub-1"] = legacy.network["Gold-Hub-1"].clone();
    assert_eq!(compare_legacy(&pair, &rows, &network).unwrap(), vec![]);
}

#[tokio::test]
async fn a_source_outage_keeps_saved_intent_and_removes_nothing() {
    let e = env();
    let mut r = enrolled(&e).await;
    let revision = r.api_document().unwrap().unwrap().revision;
    let before = live(&e);
    e.fake.failing.lock().unwrap().insert("Hub-1".into());
    assert!(r.prepare_topology(AGE).await.is_err());
    assert_eq!(r.api_document().unwrap().unwrap().revision, revision);
    assert_eq!(live(&e), before);
    e.fake.failing.lock().unwrap().clear();
    let pair = r.prepare_topology(AGE).await.unwrap();
    assert_eq!(compare_legacy(&pair, &before.0, &before.1).unwrap(), vec![]);
}

#[tokio::test]
async fn a_canvas_edit_is_the_only_difference_and_publishes_through_the_fixture() {
    use web_contract::topology::{BuilderEdit, GeoType, Location, WebCommand};
    let e = env();
    let mut r = enrolled(&e).await;
    // GET and the cached view never read a router or write a file.
    let reads = e.fake.reads.load(std::sync::atomic::Ordering::SeqCst);
    let state = files(&e.state);
    let revision = r.api_document().unwrap().unwrap().revision;
    assert_eq!(
        e.fake.reads.load(std::sync::atomic::Ordering::SeqCst),
        reads
    );
    assert_eq!(files(&e.state), state);
    let location = Location::parse_pair("12.236199459585354, 122.02419129509887").unwrap();
    let edit = WebCommand::Edit {
        expected_revision: revision,
        edit: BuilderEdit::NodeLocation {
            node_id: "lq3-3fac2a02b0faebac".into(),
            geo_type: Some(GeoType::Site),
            location: Some(location),
        },
    };
    let saved = r
        .api_web_mutate("automatic-edit-shadow", "operator", edit)
        .await
        .unwrap();
    assert_eq!(saved["state"], "saved", "{saved}");
    let pair = r.prepare_topology(AGE).await.unwrap();
    let (rows, network) = live(&e);
    // Expected difference: the new location on one node. No record is added, lost, or changed.
    assert_eq!(
        compare_legacy(&pair, &rows, &network).unwrap(),
        vec![ParityIssue::ChangedNetwork]
    );
    let gold = pair
        .network()
        .as_object()
        .unwrap()
        .values()
        .find(|node| node["id"] == "lq3-3fac2a02b0faebac")
        .unwrap();
    assert_eq!(
        (gold["latitude"].to_string(), gold["longitude"].to_string()),
        ("12.236199459585354".into(), "122.02419129509887".into())
    );
    let mut located = network.clone();
    for key in ["latitude", "longitude"] {
        located["Gold-Hub-1"][key] = gold[key].clone();
    }
    assert_eq!(compare_legacy(&pair, &rows, &located).unwrap(), vec![]);
    // The controlled compiler and publisher write the same pair in a scratch target only.
    let (_native, mut native_cfg, mut compiler) =
        crate::topology_manager::compiler::tests::fixture();
    native_cfg.target = Target::V3Pair;
    let (csv, text) = (pair.csv().to_string(), pair.network_text().to_string());
    let compiled = crate::topology_manager::compiler::tests::compiled(&mut compiler, pair).await;
    r.cfg.libreqos = native_cfg.clone();
    r.publish_compiled(compiled, 2_000).unwrap();
    let target = Path::new(&native_cfg.directory);
    assert_eq!(read(&target.join("ShapedDevices.csv")), csv);
    assert_eq!(read(&target.join("network.json")), text);
    assert_eq!(live(&e), (rows, network));
}

#[tokio::test]
async fn an_initialized_empty_tree_reads_every_protocol_and_places_everyone_flat() {
    use web_contract::topology::{BuilderEdit, WebCommand};
    let e = env();
    let config = setup(&e, None);
    let mut r = runner(&e, config);
    let init = WebCommand::Edit {
        expected_revision: 0,
        edit: BuilderEdit::Initialize,
    };
    r.api_web_mutate("automatic-edit-flat", "operator", init)
        .await
        .unwrap();
    let pair = r.prepare_topology(AGE).await.unwrap();
    // The 7 router subscribers of the scenario. No node, no parent.
    assert_eq!(pair.network(), &serde_json::json!({}));
    assert_eq!(pair.rows().len(), 7);
    assert!(pair.rows().iter().all(|row| row.parent_node.is_empty()));
    // The saved policy does not change until a publication commits it.
    let saved = r.api_document().unwrap().unwrap();
    assert!(saved.source_settings.unwrap().selections.is_empty());
}

#[tokio::test]
async fn a_flat_publication_saves_the_auto_read_selections() {
    use web_contract::topology::{BuilderEdit, WebCommand};
    let e = env();
    let config = setup(&e, None);
    let mut r = runner(&e, config.clone());
    let init = WebCommand::Edit {
        expected_revision: 0,
        edit: BuilderEdit::Initialize,
    };
    r.api_web_mutate("automatic-edit-flat-publish", "operator", init)
        .await
        .unwrap();
    let pair = r.prepare_topology(AGE).await.unwrap();
    let (_native, mut native_cfg, mut compiler) =
        crate::topology_manager::compiler::tests::fixture();
    native_cfg.target = Target::V3Pair;
    let csv = pair.csv().to_string();
    let compiled = crate::topology_manager::compiler::tests::compiled(&mut compiler, pair).await;
    r.cfg.libreqos = native_cfg.clone();
    r.publish_compiled(compiled, 2_000).unwrap();
    assert_eq!(
        read(&Path::new(&native_cfg.directory).join("ShapedDevices.csv")),
        csv
    );
    // The publication saved the selections, so the next cycle needs no proposal read.
    let saved = r.api_document().unwrap().unwrap();
    assert!(
        !saved
            .source_settings
            .as_ref()
            .unwrap()
            .selections
            .is_empty()
    );
    assert!(!crate::topology_manager::edits::unread_protocols(
        &saved, &config
    ));
    assert!(
        saved
            .subscribers
            .iter()
            .all(|s| s.policy.assignment.is_none())
    );
}

/// Two WAN roots `wanA` (ether1) and `wanB` (ether2) on the scenario router, committed.
fn add_wans(r: &mut Runner<FakeConnector>, auto_failback: bool) {
    use crate::topology_manager::catalog::{
        CapacityPolicy, CatalogNode, CatalogSource, FactorChoice, RouterRole, SourceOrigin,
    };
    let mut doc = r.api_document().unwrap().unwrap();
    let c = doc.catalog.as_mut().unwrap();
    c.auto_failback = auto_failback;
    let router = c.routers.iter_mut().find(|r| r.name == "Hub-1").unwrap();
    router.roles.push(RouterRole::Wan);
    let router_id = router.id.clone();
    for (node, source, iface) in [("wanA", "wan-a", "ether1"), ("wanB", "wan-b", "ether2")] {
        c.sources.push(CatalogSource {
            id: source.into(),
            name: iface.into(),
            origin: SourceOrigin::Wan {
                router_id: router_id.clone(),
                endpoint_ref: iface.into(),
            },
            profile_id: None,
            targets: Vec::new(),
        });
        c.nodes.push(CatalogNode {
            address_list: None,
            router_id: None,
            geo: None,
            minimum: Default::default(),
            id: node.into(),
            name: node.into(),
            parent: None,
            source_id: Some(source.into()),
            kind: crate::topology_manager::layout::NodeKind::Wan,
            capacity: CapacityPolicy::Fixed {
                base: crate::topology_manager::rates::RateBps {
                    down: 1_000_000_000,
                    up: 1_000_000_000,
                },
                factors: FactorChoice::Inherit,
            },
        });
    }
    r.test_commit(doc).unwrap();
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Make `wanA` unavailable: more than 2 minutes with its interface down, until now.
fn fail_wan_a(monitor: &crate::wan_monitor::Handle, now: u64) {
    use crate::topology_manager::wan_health::Sample;
    for t in (now - 140..=now).step_by(10) {
        monitor.observe("wanA", t, Sample::InterfaceDown, None);
    }
}

#[tokio::test]
async fn the_cycle_passes_the_wan_snapshot_to_placement() {
    use web_contract::topology::{BuilderEdit, WebCommand};
    let e = env();
    let config = setup(&e, None);
    let mut r = runner(&e, config.clone());
    let init = WebCommand::Edit {
        expected_revision: 0,
        edit: BuilderEdit::Initialize,
    };
    r.api_web_mutate("automatic-edit-wan", "operator", init)
        .await
        .unwrap();
    add_wans(&mut r, true);
    let monitor = crate::wan_monitor::Handle::new(&config);
    r.wan_monitor = Some(monitor.clone());
    let parents = |pair: &crate::topology_manager::publication::PreparedPair| {
        let mut p: Vec<_> = pair
            .rows()
            .iter()
            .map(|row| row.parent_node_id.clone())
            .collect();
        p.sort();
        p.dedup();
        p
    };
    // No probe data: both WANs take subscribers (count mode).
    let pair = r.prepare_topology(AGE).await.unwrap();
    assert_eq!(parents(&pair), vec!["wanA".to_string(), "wanB".to_string()]);
    fail_wan_a(&monitor, unix_now());
    let pair = r.prepare_topology(AGE).await.unwrap();
    assert_eq!(parents(&pair), vec!["wanB".to_string()]);
}

#[tokio::test]
async fn manual_failback_waits_for_a_rebalance() {
    use crate::topology_manager::wan_health::{Sample, WanState};
    let e = env();
    let mut r = enrolled(&e).await;
    add_wans(&mut r, false);
    let monitor = crate::wan_monitor::Handle::new(&r.cfg);
    r.wan_monitor = Some(monitor.clone());
    r.prepare_topology(AGE).await.unwrap();
    let now = unix_now();
    fail_wan_a(&monitor, now - 700);
    for t in (now - 690..=now).step_by(10) {
        monitor.observe("wanA", t, Sample::Reply { rtt_us: 10_000 }, None);
    }
    assert_eq!(
        monitor.snapshot(now).0["wanA"].state,
        WanState::WaitingForRebalance
    );
    // Placement keeps the subscribers away until the operator returns them.
    let revision = r.api_document().unwrap().unwrap().revision;
    let release = web_contract::topology::WebCommand::Edit {
        expected_revision: revision,
        edit: web_contract::topology::BuilderEdit::FailbackRelease,
    };
    let saved = r
        .api_web_mutate("automatic-edit-release", "operator", release)
        .await
        .unwrap();
    assert_eq!(saved["state"], "saved", "{saved}");
    assert_eq!(monitor.snapshot(now).0["wanA"].state, WanState::Up);
    assert_eq!(r.api_document().unwrap().unwrap().revision, revision);
}

#[tokio::test]
async fn a_wan_failover_moves_only_the_affected_subscribers() {
    use web_contract::topology::{BuilderEdit, WebCommand};
    let e = env();
    let config = setup(&e, None);
    let mut r = runner(&e, config.clone());
    let init = WebCommand::Edit {
        expected_revision: 0,
        edit: BuilderEdit::Initialize,
    };
    r.api_web_mutate("automatic-edit-failover", "operator", init)
        .await
        .unwrap();
    add_wans(&mut r, true);
    let monitor = crate::wan_monitor::Handle::new(&config);
    r.wan_monitor = Some(monitor.clone());
    let key = |row: &crate::model::CsvRow| (row.circuit_id.clone(), row.device_id.clone());
    let before = r.prepare_topology(AGE).await.unwrap();
    fail_wan_a(&monitor, unix_now());
    let after = r.prepare_topology(AGE).await.unwrap();
    // The same records: nobody is added or lost.
    let mut a: Vec<_> = before.rows().iter().map(key).collect();
    let mut b: Vec<_> = after.rows().iter().map(key).collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
    for row in after.rows() {
        let old = before.rows().iter().find(|o| key(o) == key(row)).unwrap();
        assert_eq!(row.parent_node_id, "wanB");
        // Only the parent of a subscriber on the failed WAN changes.
        let mut moved = old.clone();
        moved.parent_node_id = row.parent_node_id.clone();
        moved.parent_node = row.parent_node.clone();
        assert_eq!(&moved, row);
    }
    assert!(before.rows().iter().any(|o| o.parent_node_id == "wanA"));
}

/// A core router address-list table for the writer tests. `fail` refuses each add.
#[derive(Clone, Default)]
struct ListRouter {
    rows: Arc<std::sync::Mutex<Vec<crate::model::RosRow>>>,
    fail: Arc<std::sync::atomic::AtomicBool>,
}
impl crate::routeros::RouterSession for ListRouter {
    fn print(
        &mut self,
        _: &str,
        _: &str,
    ) -> impl Future<Output = Result<Vec<crate::model::RosRow>, ReadError>> + Send {
        let rows = self.rows.lock().unwrap().clone();
        async move { Ok(rows) }
    }
}
impl crate::routeros::ListWriter for ListRouter {
    fn add(
        &mut self,
        e: &crate::topology_manager::wan_lists::ListEntry,
    ) -> impl Future<Output = Result<(), ReadError>> + Send {
        let fail = self.fail.load(std::sync::atomic::Ordering::SeqCst);
        if !fail {
            let mut rows = self.rows.lock().unwrap();
            let id = format!("*{}", rows.len() + 1);
            rows.push(row(&[
                (".id", &id),
                ("list", &e.list),
                ("address", &e.address.to_string()),
                ("comment", &e.comment),
            ]));
        }
        async move {
            if fail {
                Err(ReadError::Trap("not enough permissions".into()))
            } else {
                Ok(())
            }
        }
    }
    fn remove(&mut self, id: &str) -> impl Future<Output = Result<(), ReadError>> + Send {
        self.rows.lock().unwrap().retain(|r| r[".id"] != id);
        async { Ok(()) }
    }
    fn set_address(
        &mut self,
        id: &str,
        address: std::net::Ipv4Addr,
    ) -> impl Future<Output = Result<(), ReadError>> + Send {
        for r in self.rows.lock().unwrap().iter_mut() {
            if r[".id"] == id {
                r.insert("address".into(), address.to_string());
            }
        }
        async { Ok(()) }
    }
}

/// An initialized tree with two WANs, the writer on, and a scratch publication target.
async fn writer_runner(
    e: &Env,
    router: &ListRouter,
) -> (
    Runner<FakeConnector>,
    tempfile::TempDir,
    crate::config::Libreqos,
    crate::topology_manager::compiler::ControlledCompiler,
) {
    use web_contract::topology::{BuilderEdit, WebCommand};
    let config = setup(e, None);
    let mut r = runner(e, config);
    let init = WebCommand::Edit {
        expected_revision: 0,
        edit: BuilderEdit::Initialize,
    };
    r.api_web_mutate("automatic-edit-writer", "operator", init)
        .await
        .unwrap();
    add_wans(&mut r, true);
    r.cfg.wan_writer.enabled = true;
    let session = router.clone();
    r.wan_writer = Some(crate::wan_writer::start(move || {
        let s = session.clone();
        async move { Ok::<_, ReadError>(s) }
    }));
    let (root, mut native_cfg, compiler) = crate::topology_manager::compiler::tests::fixture();
    native_cfg.target = Target::V3Pair;
    (r, root, native_cfg, compiler)
}

async fn publish_once(
    r: &mut Runner<FakeConnector>,
    native_cfg: &crate::config::Libreqos,
    compiler: &mut crate::topology_manager::compiler::ControlledCompiler,
    now: u64,
) -> anyhow::Result<crate::write_receipt::WriteResult> {
    let pair = r.prepare_topology(AGE).await.unwrap();
    let wanted = r.wanted_lists(&pair).unwrap();
    let compiled = crate::topology_manager::compiler::tests::compiled(compiler, pair).await;
    r.cfg.libreqos = native_cfg.clone();
    let result = r.publish_with_lists(compiled, wanted, now).await;
    r.cfg.libreqos = native_cfg.clone();
    result
}

#[tokio::test]
async fn a_successful_write_then_publishes_the_pair() {
    let e = env();
    let lists = ListRouter::default();
    let (mut r, _root, native, mut c) = writer_runner(&e, &lists).await;
    publish_once(&mut r, &native, &mut c, 2_000).await.unwrap();
    let rows = lists.rows.lock().unwrap().clone();
    assert_eq!(rows.len(), 7, "one entry for each subscriber address");
    assert!(
        rows.iter()
            .all(|row| row["list"] == "via-wanA" || row["list"] == "via-wanB")
    );
    assert!(
        Path::new(&native.directory)
            .join("ShapedDevices.csv")
            .exists()
    );
    assert_eq!(r.wan_lists.as_ref().unwrap()["state"], "written");
}

#[tokio::test]
async fn a_failed_write_holds_publication_and_retries() {
    let e = env();
    let lists = ListRouter::default();
    lists.fail.store(true, std::sync::atomic::Ordering::SeqCst);
    let (mut r, _root, native, mut c) = writer_runner(&e, &lists).await;
    let error = publish_once(&mut r, &native, &mut c, 2_000)
        .await
        .unwrap_err();
    assert!(
        format!("{error:#}").contains("previous shaping pair stays"),
        "{error:#}"
    );
    assert!(
        !Path::new(&native.directory)
            .join("ShapedDevices.csv")
            .exists()
    );
    assert_eq!(r.wan_lists.as_ref().unwrap()["state"], "failed");
    lists.fail.store(false, std::sync::atomic::Ordering::SeqCst);
    publish_once(&mut r, &native, &mut c, 2_100).await.unwrap();
    assert!(
        Path::new(&native.directory)
            .join("ShapedDevices.csv")
            .exists()
    );
}

#[tokio::test]
async fn a_disabled_writer_publishes_without_waiting() {
    let e = env();
    let lists = ListRouter::default();
    lists.fail.store(true, std::sync::atomic::Ordering::SeqCst);
    let (mut r, _root, native, mut c) = writer_runner(&e, &lists).await;
    r.cfg.wan_writer.enabled = false;
    publish_once(&mut r, &native, &mut c, 2_000).await.unwrap();
    assert!(lists.rows.lock().unwrap().is_empty());
    assert!(r.wan_lists.is_none());
}

#[tokio::test]
async fn a_held_write_reports_the_reason() {
    let e = env();
    let lists = ListRouter::default();
    *lists.rows.lock().unwrap() = (1..=100)
        .map(|i| {
            row(&[
                (".id", &format!("*o{i}")),
                ("list", "via-wanA"),
                ("address", &format!("10.250.0.{i}")),
                ("comment", "lqosync: gone"),
            ])
        })
        .collect();
    let (mut r, _root, native, mut c) = writer_runner(&e, &lists).await;
    let error = publish_once(&mut r, &native, &mut c, 2_000)
        .await
        .unwrap_err();
    assert!(format!("{error:#}").contains("held"), "{error:#}");
    assert_eq!(r.wan_lists.as_ref().unwrap()["state"], "held");
    assert_eq!(lists.rows.lock().unwrap().len(), 100);
}

fn range(first: &str, last: &str, node: &str) -> (u32, u32, String) {
    let ip = |t: &str| u32::from(t.parse::<std::net::Ipv4Addr>().unwrap());
    (ip(first), ip(last), node.to_string())
}

/// An initialized tree with two WANs and a WAN monitor that knows the operator range lists.
async fn adopt_runner(
    e: &Env,
    ranges: Vec<(u32, u32, String)>,
) -> (Runner<FakeConnector>, crate::wan_monitor::Handle) {
    use web_contract::topology::{BuilderEdit, WebCommand};
    let config = setup(e, None);
    let mut r = runner(e, config.clone());
    let init = WebCommand::Edit {
        expected_revision: 0,
        edit: BuilderEdit::Initialize,
    };
    r.api_web_mutate("automatic-edit-adopt", "operator", init)
        .await
        .unwrap();
    add_wans(&mut r, true);
    let monitor = crate::wan_monitor::Handle::new(&config);
    monitor.set_legacy(ranges);
    r.wan_monitor = Some(monitor.clone());
    (r, monitor)
}

async fn adopt(r: &mut Runner<FakeConnector>) -> serde_json::Value {
    let revision = r.api_document().unwrap().unwrap().revision;
    let proposal = r
        .api_edit_propose(revision, crate::topology_manager::edits::Edit::WanAdopt)
        .await
        .unwrap();
    r.api_edit_confirm(proposal["proposal_id"].as_str().unwrap())
        .unwrap();
    proposal
}

/// IPv4 address to parent node of the next pair.
async fn parents_by_ip(r: &mut Runner<FakeConnector>) -> BTreeMap<String, String> {
    r.prepare_topology(AGE)
        .await
        .unwrap()
        .rows()
        .iter()
        .map(|row| (row.ipv4.clone(), row.parent_node_id.clone()))
        .collect()
}

#[tokio::test]
async fn adoption_keeps_the_current_wan_of_each_client() {
    let e = env();
    let (mut r, _) = adopt_runner(
        &e,
        vec![
            range("100.64.0.0", "100.64.0.255", "wanB"),
            range("100.66.0.0", "100.66.255.255", "wanA"),
        ],
    )
    .await;
    adopt(&mut r).await;
    for (ip, parent) in parents_by_ip(&mut r).await {
        let want = if ip.starts_with("100.64.") {
            "wanB"
        } else {
            "wanA"
        };
        assert_eq!(parent, want, "{ip}");
    }
}

#[tokio::test]
async fn adoption_moves_only_listed_clients() {
    let e = env();
    let (mut r, _) = adopt_runner(&e, vec![range("100.66.0.0", "100.66.255.255", "wanB")]).await;
    let before = parents_by_ip(&mut r).await;
    let proposal = adopt(&mut r).await;
    let after = parents_by_ip(&mut r).await;
    for (ip, parent) in &after {
        if ip.starts_with("100.66.") {
            assert_eq!(parent, "wanB", "{ip}");
        } else {
            assert_eq!(
                Some(parent),
                before.get(ip),
                "an unlisted client keeps its node: {ip}"
            );
        }
    }
    assert!(proposal["moves"].as_array().unwrap().len() <= 3);
}

#[tokio::test]
async fn routing_lists_follow_placement_and_failover() {
    let e = env();
    let lists = ListRouter::default();
    // A manual entry in a via- list: the writer never touches it.
    lists.rows.lock().unwrap().push(row(&[
        (".id", "*manual"),
        ("list", "via-wanA"),
        ("address", "192.0.2.50"),
        ("comment", "operator entry"),
    ]));
    let (mut r, _root, native, mut c) = writer_runner(&e, &lists).await;
    let monitor = crate::wan_monitor::Handle::new(&r.cfg);
    r.wan_monitor = Some(monitor.clone());
    publish_once(&mut r, &native, &mut c, 2_000).await.unwrap();
    let used = |lists: &ListRouter| {
        let mut l: Vec<String> = lists
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row["comment"].starts_with("lqosync: "))
            .map(|row| row["list"].clone())
            .collect();
        l.sort();
        l.dedup();
        l
    };
    assert_eq!(used(&lists), ["via-wanA", "via-wanB"]);
    fail_wan_a(&monitor, unix_now());
    publish_once(&mut r, &native, &mut c, 2_100).await.unwrap();
    assert_eq!(used(&lists), ["via-wanB"], "the failover moves the entries");
    let rows = lists.rows.lock().unwrap();
    assert!(rows.iter().any(|row| row[".id"] == "*manual"));
    assert_eq!(rows.len(), 8, "7 client addresses and the manual entry");
}

// The writer status in topology mode (activation plan Task 3).

fn notices(e: &Env) -> Vec<String> {
    std::fs::read_dir(e.state.join("telegram-events"))
        .map(|d| d.flatten().map(|f| read(&f.path())).collect::<Vec<_>>())
        .unwrap_or_default()
}

#[tokio::test]
async fn a_manager_failure_notifies_once_and_the_next_good_cycle_clears_it() {
    let e = env();
    let mut r = enrolled(&e).await;
    let mut status = r.base_status(1_000);
    r.record_manager(1_000, Some("admission refused".into()), &mut status);
    assert_eq!(status.writer, "topology");
    assert!(status.manager_failed);
    assert_eq!(status.manager_error.as_deref(), Some("admission refused"));
    assert_eq!(status.manager_failed_since, Some(1_000));
    let mut status = r.base_status(1_060);
    r.record_manager(1_060, Some("admission refused".into()), &mut status);
    assert_eq!(
        status.manager_failed_since,
        Some(1_000),
        "the first failure time stays"
    );
    assert_eq!(notices(&e).len(), 1, "no notice on each failed cycle");
    let mut status = r.base_status(1_120);
    r.record_manager(1_120, None, &mut status);
    assert!(!status.manager_failed && status.manager_error.is_none());
    let texts = notices(&e).join("\n");
    assert_eq!(notices(&e).len(), 2);
    assert!(texts.contains("the Topology Manager failed; the last pair stays"));
    assert!(texts.contains("the Topology Manager works again"));
}

#[tokio::test]
async fn a_restart_during_a_failure_sends_no_second_notice() {
    let e = env();
    let mut r = enrolled(&e).await;
    let mut status = r.base_status(1_000);
    r.record_manager(1_000, Some("compile failed".into()), &mut status);
    std::fs::write(e.state.join("status.json"), status.to_json()).unwrap();
    drop(r);
    // A new process reads the last status.json.
    let mut r = runner(&e, setup(&e, None));
    let mut status = r.base_status(2_000);
    r.record_manager(2_000, Some("compile failed".into()), &mut status);
    assert_eq!(status.manager_failed_since, Some(1_000));
    assert_eq!(notices(&e).len(), 1);
}

#[tokio::test]
async fn an_owned_cycle_failure_is_a_manager_failure_and_a_router_failure_is_not() {
    let e = env();
    let mut r = enrolled(&e).await;
    crate::topology_manager::runtime::claim(&e.state).unwrap();
    // The fixture has no LibreQoS compiler: the publication fails.
    let summary = r.cycle(1_000, false).await;
    assert!(summary.error.is_some());
    assert_eq!(summary.status.writer, "topology");
    assert!(summary.status.manager_failed, "{:?}", summary.status.error);
    // A router read failure keeps the pair and is not a manager failure.
    let e2 = env();
    let mut r = enrolled(&e2).await;
    crate::topology_manager::runtime::claim(&e2.state).unwrap();
    let (_root, mut native_cfg, compiler) = crate::topology_manager::compiler::tests::fixture();
    native_cfg.target = Target::V3Pair;
    r.cfg.libreqos = native_cfg;
    r.topology_compiler = Some(compiler);
    e2.fake.failing.lock().unwrap().insert("Hub-1".into());
    let summary = r.cycle(1_000, false).await;
    assert!(summary.error.is_some());
    assert!(!summary.status.manager_failed, "{:?}", summary.status.error);
    assert_eq!(summary.status.writer, "topology");
    // A legacy cycle reports the legacy writer.
    let e3 = env();
    let mut r = runner(&e3, setup(&e3, None));
    assert_eq!(r.cycle(1_000, false).await.status.writer, "legacy");
}

#[tokio::test]
async fn api_status_reports_the_manager_failure() {
    let e = env();
    let mut r = enrolled(&e).await;
    let mut status = r.base_status(1_000);
    r.record_manager(1_000, Some("compile failed".into()), &mut status);
    let v = r.api_status().unwrap();
    assert_eq!(
        (v["manager_failed"].clone(), v["manager_error"].clone()),
        (serde_json::json!(true), serde_json::json!("compile failed"))
    );
    // A new process reads the failure from the last status.json.
    std::fs::write(e.state.join("status.json"), status.to_json()).unwrap();
    drop(r);
    let mut r = runner(&e, setup(&e, None));
    assert_eq!(r.api_status().unwrap()["manager_error"], "compile failed");
    let mut status = r.base_status(2_000);
    r.record_manager(2_000, None, &mut status);
    assert_eq!(r.api_status().unwrap()["manager_failed"], false);
}

#[tokio::test]
async fn a_router_failure_during_a_manager_failure_keeps_the_failure() {
    let e = env();
    let mut r = enrolled(&e).await;
    crate::topology_manager::runtime::claim(&e.state).unwrap();
    let mut status = r.base_status(1_000);
    r.record_manager(1_000, Some("compile failed".into()), &mut status);
    let (_root, mut native_cfg, compiler) = crate::topology_manager::compiler::tests::fixture();
    native_cfg.target = Target::V3Pair;
    r.cfg.libreqos = native_cfg;
    r.topology_compiler = Some(compiler);
    e.fake.failing.lock().unwrap().insert("Hub-1".into());
    let summary = r.cycle(1_100, false).await;
    assert!(summary.error.is_some());
    assert!(summary.status.manager_failed);
    assert_eq!(summary.status.manager_failed_since, Some(1_000));
    assert_eq!(notices(&e).len(), 1);
}

// Live source canvas, Task 2.

/// Move secret `g1` of the fake router to a new profile `Bronze`.
fn bronze_on_the_router(e: &Env) {
    let mut rows = e.fake.rows.lock().unwrap();
    let tables = rows.get_mut("Hub-1").unwrap();
    let profiles = tables.get_mut("/ppp/profile").unwrap();
    let mut profile = profiles[0].clone();
    profile.insert("name".into(), "Bronze".into());
    profile.insert(".id".into(), "*p9".into());
    profiles.push(profile);
    for s in tables
        .get_mut("/ppp/secret")
        .unwrap()
        .iter_mut()
        .filter(|s| s["name"] == "g1")
    {
        s.insert("profile".into(), "Bronze".into());
    }
}

fn bronze_id() -> String {
    crate::topology::group_node_id("Hub-1", &crate::sources::GroupKey::PppPlan("Bronze".into()))
}

#[tokio::test]
async fn a_cycle_adds_the_group_of_a_new_profile() {
    let e = env();
    let mut r = enrolled(&e).await;
    bronze_on_the_router(&e);
    r.cycle(1_000, false).await;
    let doc = r.api_document().unwrap().unwrap();
    assert!(
        doc.catalog
            .unwrap()
            .nodes
            .iter()
            .any(|n| n.id == bronze_id())
    );
}

#[tokio::test]
async fn an_automatic_commit_yields_to_a_newer_revision() {
    let e = env();
    let mut r = enrolled(&e).await;
    let base = r.api_document().unwrap().unwrap();
    bronze_on_the_router(&e);
    let draft = r.propose_policy(AGE).await.unwrap();
    // A web edit commits first.
    let mut newer = base.clone();
    newer.catalog.as_mut().unwrap().nodes[0]
        .name
        .push_str(" (edited)");
    r.test_commit(newer).unwrap();
    assert!(!r.commit_live_groups(&base, &draft).unwrap());
    let doc = r.api_document().unwrap().unwrap();
    assert!(
        !doc.catalog
            .unwrap()
            .nodes
            .iter()
            .any(|n| n.id == bronze_id())
    );
}

// Live source canvas, Task 3.

#[tokio::test]
async fn the_view_marks_an_inactive_group() {
    let e = env();
    let mut r = enrolled(&e).await;
    r.cycle(1_000, false).await;
    // The Silver session ends.
    e.fake
        .rows
        .lock()
        .unwrap()
        .get_mut("Hub-1")
        .unwrap()
        .get_mut("/ppp/active")
        .unwrap()
        .retain(|row| row["name"] != "s1");
    r.cycle(2_000, false).await;
    let view: web_contract::topology::TopologyView = serde_json::from_slice(
        &std::fs::read(e.state.join(web_contract::topology::CACHE)).unwrap(),
    )
    .unwrap();
    let active = |name: &str| {
        view.nodes
            .iter()
            .find(|n| n.name.starts_with(name))
            .unwrap()
            .active
    };
    assert!(active("Gold"));
    assert!(!active("Silver"));
}
