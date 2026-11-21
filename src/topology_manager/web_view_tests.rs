use super::*;
use crate::collection::{self, Report, State};
use crate::topology_manager::catalog::{self, BaseRateAuthority, SubscriberProfile};
use crate::topology_manager::identity::{ReferenceAuthority, SourceSelection};
use crate::topology_manager::model::{
    IdentityRef, RegistryDocument, SourceKind, SourceScope, SubscriberPolicy, SubscriberRecord,
};
use crate::topology_manager::policy_draft::router_id;
use crate::topology_manager::rates::RateBps;
use crate::topology_manager::settings::SourceSettings;
use serde_json::json;

fn scope(router: &str, kind: SourceKind) -> SourceScope {
    SourceScope {
        router_id: router_id(router),
        source_id: format!("{router}-{kind:?}"),
        kind,
    }
}

fn identity(router: &str, kind: SourceKind, account_ref: &str) -> IdentityRef {
    IdentityRef {
        scope: scope(router, kind),
        account_ref: account_ref.into(),
    }
}

fn record(id: &str, identities: Vec<IdentityRef>, plan: Option<&str>) -> SubscriberRecord {
    SubscriberRecord {
        id: id.into(),
        identities,
        policy: SubscriberPolicy {
            profile_id: plan.map(str::to_owned),
            assignment: None,
            pin: None,
        },
    }
}

fn selection(router: &str, kind: SourceKind, authority: ReferenceAuthority) -> SourceSelection {
    SourceSelection {
        scope: scope(router, kind),
        service_names: Vec::new(),
        authority,
        policy: Default::default(),
    }
}

/// Two routers. `sub-1` has a PPP and a DHCP identity on Core; `sub-2` has a PPP identity on
/// Edge with the same account name as `sub-1`. `sub-3` is offline with a saved pin.
fn doc() -> RegistryDocument {
    let mut catalog = catalog::tests::catalog();
    catalog.profiles.push(SubscriberProfile {
        id: "plan-100".into(),
        name: "Access 100".into(),
        base: BaseRateAuthority::Local(RateBps {
            down: 100_000_000,
            up: 50_000_000,
        }),
        speed: Default::default(),
        eligible_targets: None,
    });
    let mut offline = record(
        "sub-3",
        vec![identity("Core", SourceKind::Ppp, "ppp-name/v1/carol")],
        None,
    );
    offline.policy.pin = Some("wan1".into());
    RegistryDocument {
        revision: 7,
        subscribers: vec![
            record(
                "sub-1",
                vec![
                    identity("Core", SourceKind::Ppp, "ppp-name/v1/alice"),
                    identity("Core", SourceKind::Dhcp, "dhcp-mac/v1/02:00:00:00:00:01"),
                ],
                Some("plan-100"),
            ),
            record(
                "sub-2",
                vec![identity("Edge", SourceKind::Ppp, "ppp-name/v1/alice")],
                Some("plan-100"),
            ),
            offline,
        ],
        catalog: Some(catalog),
        source_settings: Some(SourceSettings {
            selections: vec![
                selection("Core", SourceKind::Ppp, ReferenceAuthority::PppSecretName),
                selection("Core", SourceKind::Dhcp, ReferenceAuthority::DhcpClientMac),
                selection("Edge", SourceKind::Ppp, ReferenceAuthority::PppSecretName),
            ],
        }),
        ..Default::default()
    }
}

fn row(source: &str, name: &str, ip: &str, mac: &str, state: State) -> collection::Subscriber {
    collection::Subscriber {
        key: String::new(),
        source: source.into(),
        name: name.into(),
        circuit: String::new(),
        ipv4: ip.into(),
        mac: mac.into(),
        state,
    }
}

fn router(name: &str, state: &str, subscribers: Vec<collection::Subscriber>) -> collection::Router {
    collection::Router {
        name: name.into(),
        state: state.into(),
        observed_at: 90,
        sources: Default::default(),
        subscribers,
    }
}

fn report() -> Report {
    Report {
        version: 1,
        cycle_seq: 4,
        updated: 100,
        fresh_for: 60,
        routers: vec![
            router(
                "Core",
                "online",
                vec![
                    row("ppp", "alice", "192.0.2.10", "", State::Online),
                    row(
                        "dhcp",
                        "host",
                        "192.0.2.11",
                        "02-00-00-00-00-01",
                        State::Bound,
                    ),
                    // The IP of alice, reused by a new unknown client: no link by IP.
                    row(
                        "dhcp",
                        "new",
                        "192.0.2.10",
                        "02:00:00:00:00:09",
                        State::Bound,
                    ),
                    row("ppp", "carol", "", "", State::Offline),
                    row("hotspot", "guest", "", "", State::Online),
                ],
            ),
            router(
                "Edge",
                "unreachable",
                vec![row("ppp", "alice", "192.0.2.20", "", State::Online)],
            ),
        ],
        ..Default::default()
    }
}

fn preview(revision: u64) -> Value {
    json!({
        "collected_at": 80, "registry_revision": revision, "generation": 3, "state": "ok",
        "nodes": [{"id": "wan1", "label": "wan1", "parent": null, "kind": "wan", "source": null,
                   "calculated_mbps": [400.0, 200.0], "exported_mbps": [400.0, 200.0],
                   "subscribers": 1}],
        "subscribers": [{"id": "sub-1", "identities": [], "account_ref": "", "scope": "",
                         "ips": ["192.0.2.10"], "macs": [], "node": "wan1", "pin": null,
                         "profile": "plan-100", "min_mbps": [50.0, 25.0],
                         "max_mbps": [100.0, 50.0], "fallback": false}],
        "inventory": [], "conflicts": [], "rounded": []
    })
}

fn input<'a>(
    doc: Option<&'a RegistryDocument>,
    preview: Option<&'a Value>,
    report: Option<&'a Report>,
) -> Input<'a> {
    Input {
        doc,
        preview,
        collection: report,
        mode: "observe",
        writer: "legacy",
        cycle_seq: 4,
        now: 100,
        fresh_for: 60,
        publication: None,
        history: &[],
    }
}

fn obs<'a>(v: &'a TopologyView, router: &str, kind: &str, name: &str) -> &'a Observation {
    v.observations
        .iter()
        .find(|o| o.router == router && o.kind == kind && o.name == name)
        .unwrap_or_else(|| panic!("no observation {router}/{kind}/{name}"))
}

#[test]
fn no_registry_and_no_preview_stay_explicit() {
    let v = project(&input(None, None, None));
    assert_eq!(v.version, web_contract::topology::VERSION);
    assert_eq!(v.registry_revision, None);
    assert!(!v.enrolled);
    assert_eq!(v.observed, None);
    assert!(v.nodes.is_empty() && v.subscribers.is_empty() && v.observations.is_empty());
    assert!(!v.snapshot_id.is_empty());
}

#[test]
fn registry_tree_plans_and_saved_records_without_a_preview() {
    let d = doc();
    let v = project(&input(Some(&d), None, None));
    assert_eq!(v.registry_revision.as_deref(), Some("7"));
    assert_eq!(v.observed, None);
    assert!(v.nodes.iter().any(|n| n.id == "wan1" && n.kind == "wan"));
    assert!(v.nodes.iter().all(|n| n.calculated_mbps.is_none()));
    // Offline records stay visible with their saved pin and no endpoint.
    let carol = v.subscribers.iter().find(|s| s.id == "sub-3").unwrap();
    assert_eq!(carol.pin.as_deref(), Some("wan1"));
    assert_eq!(carol.active, None);
    // Only `plan-` profiles are plans. sub-1 counts once although it has two identities.
    assert_eq!(v.plans.len(), 1);
    let p = &v.plans[0];
    assert_eq!(
        (p.id.as_str(), p.down_mbps, p.up_mbps),
        ("plan-100", 100, 50)
    );
    assert_eq!(
        (p.ppp_identities, p.dhcp_identities, p.subscribers),
        (2, 1, 2)
    );
}

#[test]
fn preview_of_the_current_revision_adds_capacity_and_endpoints() {
    let d = doc();
    let p = preview(7);
    let v = project(&input(Some(&d), Some(&p), None));
    let o = v.observed.as_ref().unwrap();
    assert_eq!(
        (o.revision.as_str(), o.generation, o.stale),
        ("7", Some(3), false)
    );
    let core = v.nodes.iter().find(|n| n.id == "wan1").unwrap();
    assert_eq!(core.exported_mbps, Some([400.0, 200.0]));
    assert_eq!(core.members, Some(1));
    let alice = v.subscribers.iter().find(|s| s.id == "sub-1").unwrap();
    assert_eq!(alice.active.as_ref().unwrap().ips, vec!["192.0.2.10"]);
}

#[test]
fn an_older_preview_keeps_its_revision_and_is_stale() {
    let d = doc();
    let p = preview(6);
    let v = project(&input(Some(&d), Some(&p), None));
    let o = v.observed.as_ref().unwrap();
    assert_eq!((o.revision.as_str(), o.stale), ("6", true));
    assert_eq!(v.registry_revision.as_deref(), Some("7"));
    assert_ne!(
        v.snapshot_id,
        project(&input(Some(&d), Some(&preview(7)), None)).snapshot_id
    );
}

#[test]
fn a_conflict_preview_keeps_every_conflict_and_no_capacity() {
    let d = doc();
    let mut p = preview(7);
    p["state"] = json!("conflicts");
    p["nodes"] = json!([]);
    p["subscribers"] = json!([]);
    p["generation"] = json!(null);
    let many: Vec<Value> = (0..300)
        .map(|i| json!({"kind": "identity", "subject": format!("s{i}"), "detail": "x"}))
        .collect();
    p["conflicts"] = json!(many);
    let v = project(&input(Some(&d), Some(&p), None));
    assert_eq!(v.observed.as_ref().unwrap().state, "conflicts");
    assert_eq!(v.conflicts.len(), 300);
    assert!(v.nodes.iter().all(|n| n.members.is_none()));
}

#[test]
fn observations_link_only_through_the_reference_authority() {
    let d = doc();
    let r = report();
    let v = project(&input(Some(&d), None, Some(&r)));
    let a = obs(&v, "Core", "ppp", "alice");
    assert_eq!(
        (a.link.as_str(), a.subscriber_id.as_deref()),
        ("verified", Some("sub-1"))
    );
    // A linked row shows the policy of its saved record.
    assert_eq!(a.plan_id.as_deref(), Some("plan-100"));
    assert_eq!(obs(&v, "Core", "ppp", "carol").pin.as_deref(), Some("wan1"));
    // A different MAC spelling still gives the same normalized reference.
    let h = obs(&v, "Core", "dhcp", "host");
    assert_eq!(h.subscriber_id.as_deref(), Some("sub-1"));
    // A reused IP never inherits a policy.
    let n = obs(&v, "Core", "dhcp", "new");
    assert_eq!(
        (n.link.as_str(), n.subscriber_id.as_deref()),
        ("none", None)
    );
    // The same account name on another router is another identity.
    let e = obs(&v, "Edge", "ppp", "alice");
    assert_eq!(e.subscriber_id.as_deref(), Some("sub-2"));
    // Hotspot is not a PPPoE or IPoE view.
    assert!(v.observations.iter().all(|o| o.kind != "hotspot"));
}

#[test]
fn a_failed_router_makes_its_rows_stale() {
    let d = doc();
    let r = report();
    let v = project(&input(Some(&d), None, Some(&r)));
    assert_eq!(obs(&v, "Edge", "ppp", "alice").state, "stale");
    assert_eq!(obs(&v, "Core", "ppp", "carol").state, "offline");
    let edge = v.collection.iter().find(|c| c.name == "Edge").unwrap();
    assert_eq!(
        (edge.state.as_str(), edge.router_id.clone()),
        ("unreachable", router_id("Edge"))
    );
}

#[test]
fn undecidable_links_are_unknown() {
    let mut d = doc();
    let settings = d.source_settings.as_mut().unwrap();
    // A comment authority: the collection does not carry the comment.
    settings.selections[0].authority = ReferenceAuthority::OperatorManagedComment;
    // A PPP service filter: the collection does not carry the PPP service.
    settings.selections[2].service_names = vec!["pppoe-a".into()];
    let r = report();
    let v = project(&input(Some(&d), None, Some(&r)));
    for (router, name) in [("Core", "alice"), ("Edge", "alice")] {
        let o = obs(&v, router, "ppp", name);
        assert_eq!(
            (o.link.as_str(), o.subscriber_id.as_deref()),
            ("unknown", None)
        );
    }
    // A router/kind outside the topology sources has no link.
    d.source_settings = None;
    let v = project(&input(Some(&d), None, Some(&r)));
    assert_eq!(obs(&v, "Core", "ppp", "alice").link, "none");
}

#[test]
fn the_view_holds_no_secret_or_path() {
    let d = doc();
    let (p, r) = (preview(7), report());
    let text = serde_json::to_string(&project(&input(Some(&d), Some(&p), Some(&r)))).unwrap();
    for word in [
        "password",
        "credential",
        "/etc/",
        "/var/",
        "export_ownership",
    ] {
        assert!(!text.contains(word), "{word}");
    }
}

#[test]
fn publish_writes_a_private_file_that_reads_back() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let d = doc();
    let v = project(&input(Some(&d), None, None));
    publish(dir.path(), &v).unwrap();
    let path = dir.path().join(web_contract::topology::CACHE);
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let back: TopologyView = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(back, v);
}

#[test]
fn fifty_thousand_subscribers_project_with_every_link() {
    let mut d = doc();
    let mut rows = Vec::new();
    for i in 0..50_000 {
        let name = format!("user-{i}");
        let account_ref = crate::topology_manager::identity::ppp_name_reference(&name);
        d.subscribers.push(record(
            &format!("bulk-{i}"),
            vec![identity("Core", SourceKind::Ppp, &account_ref)],
            Some("plan-100"),
        ));
        rows.push(row("ppp", &name, "198.51.100.1", "", State::Online));
    }
    let mut r = report();
    r.routers[0].subscribers.extend(rows);
    let started = std::time::Instant::now();
    let v = project(&input(Some(&d), None, Some(&r)));
    let bytes = serde_json::to_vec(&v).unwrap().len();
    eprintln!("50k projection: {:?}, {bytes} bytes", started.elapsed());
    assert_eq!(v.subscribers.len(), 50_003);
    let linked = v
        .observations
        .iter()
        .filter(|o| {
            o.subscriber_id
                .as_deref()
                .is_some_and(|s| s.starts_with("bulk-"))
        })
        .count();
    assert_eq!(linked, 50_000);
    assert_eq!(v.plans[0].subscribers, 50_002);
    assert!(bytes < crate::collection::MAX_BYTES as usize);
}

#[test]
fn speed_rules_and_source_factors_are_exact_text() {
    use crate::topology_manager::rates::{Fraction, RateRule};
    let mut d = doc();
    let c = d.catalog.as_mut().unwrap();
    c.defaults.down_min = RateRule::BaseFraction(Fraction {
        numerator: 205,
        denominator: 1000,
    });
    c.defaults.up_min = RateRule::FixedBps(12_500_000);
    let v = project(&input(Some(&d), None, None));
    let speed = v.speed.unwrap();
    assert_eq!(
        (speed.down_min.unit.as_str(), speed.down_min.value.as_str()),
        ("percent", "20.5")
    );
    assert_eq!(
        (speed.up_min.unit.as_str(), speed.up_min.value.as_str()),
        ("mbps", "12.5")
    );
    let f = v
        .factors
        .iter()
        .find(|f| f.source_id == "profile-source")
        .unwrap();
    assert!(!f.down.is_empty() && !f.up.is_empty());
    assert!(
        v.factors.iter().all(|f| f.source_id != "wan-source-1"),
        "WAN sources have no factor"
    );
    assert_eq!(project(&input(None, None, None)).speed, None);
}

fn write(phase: &str, revision: Option<u64>) -> crate::write_receipt::Receipt {
    let topology = revision.map(|r| {
        json!({"policy_revision": r, "observation_generation": 3, "registry_digest": "0".repeat(64)})
    });
    serde_json::from_value(json!({
        "version": 1, "write_id": "w-9", "phase": phase, "started": 90, "updated": 95,
        "meta": {"topology": topology, "cycle_seq": 4, "config_digest": "c", "notice": null},
        "files": [
            {"path": "/opt/libreqos/src/ShapedDevices.csv", "sha256": "a", "outcome": "WRITTEN"},
            {"path": "/opt/libreqos/src/network.json", "sha256": "b", "outcome": "UNCHANGED"}],
        "error": "cannot write /opt/libreqos/src/x.tmp", "acceptance": null, "superseded_by": null
    }))
    .unwrap()
}

#[test]
fn the_latest_write_links_its_revision_and_hides_paths() {
    let d = doc();
    let receipt = write("IMPORT_CONFIRMED", Some(7));
    let mut i = input(Some(&d), None, None);
    i.publication = Some(&receipt);
    let v = project(&i);
    let p = v.publication.as_ref().unwrap();
    assert_eq!(
        (p.phase.as_str(), p.policy_revision.as_deref()),
        ("import_confirmed", Some("7"))
    );
    assert_eq!(p.files[0].name, "ShapedDevices.csv");
    assert_eq!(p.files[0].outcome, "written");
    assert!(p.failed && !p.accepted);
    let text = serde_json::to_string(&v).unwrap();
    assert!(
        !text.contains("/opt/libreqos"),
        "no file path or error text"
    );
    // A legacy writer write has no policy revision.
    let legacy = write("APPLIED", None);
    i.publication = Some(&legacy);
    assert_eq!(project(&i).publication.unwrap().policy_revision, None);
}

#[test]
fn web_history_is_newest_first_and_named() {
    use crate::topology_manager::web_receipts::{Record, State};
    use web_contract::topology::WebCommand;
    let record = |key: &str, at: u64, command: WebCommand, state: State| Record {
        key: key.into(),
        actor: "alice".into(),
        command,
        base_revision: 6,
        at,
        state,
    };
    let history = [
        record(
            "k1",
            10,
            WebCommand::PlanDelete {
                expected_revision: 6,
                plan_id: "plan-1".into(),
            },
            State::Failed {
                error: "conflict".into(),
            },
        ),
        record(
            "k2",
            20,
            WebCommand::EditConfirm {
                proposal_id: "edit-6-1".into(),
            },
            State::Saved {
                revision: 7,
                subject_id: None,
            },
        ),
    ];
    let d = doc();
    let mut i = input(Some(&d), None, None);
    i.history = &history;
    let v = project(&i);
    assert_eq!(v.history[0].change, "Reviewed change");
    assert_eq!(
        (
            v.history[0].state.as_str(),
            v.history[0].revision.as_deref()
        ),
        ("saved", Some("7"))
    );
    assert_eq!(v.history[1].change, "Delete plan plan-1");
    assert!(
        !serde_json::to_string(&v).unwrap().contains("edit-6-1"),
        "no service token"
    );
}
