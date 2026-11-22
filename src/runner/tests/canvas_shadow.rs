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
