use super::*;
use crate::topology_manager::catalog::{self, CapacityPolicy, FactorChoice, RouterRole};
use crate::topology_manager::layout::NodeKind;
use crate::topology_manager::model::{SubscriberPolicy, SubscriberRecord};
use crate::topology_manager::ownership::NodeAlias;
use crate::topology_manager::rates::{NodeMinimum, RateBps};
use crate::topology_manager::store::InvalidPolicy;
use serde_json::json;

fn doc() -> RegistryDocument {
    RegistryDocument {
        catalog: Some(catalog::tests::catalog()),
        export_ownership: Some(Default::default()),
        ..Default::default()
    }
}

fn cfg() -> Config {
    Config::from_toml_str(
        "[[routers]]\nname='Core'\naddress='192.0.2.1'\nusername='u'\npassword_file='/x'\n\
         [[routers]]\nname='Edge'\naddress='192.0.2.2'\nusername='u'\npassword_file='/x'\n",
    )
    .unwrap()
}

fn discovery() -> Value {
    let candidate = |name: &str| {
        json!({"name": name, "type": "pppoe-out", "running": true, "disabled": false,
               "comment": null, "suggested": true, "source_id": null, "used_by": []})
    };
    json!({"collected_at": 0, "unconfigured": [], "routers": [
        {"router_id": "edge-derived", "name": "Edge", "state": "ok", "error": null,
         "candidates": [candidate("pppoe-out1")], "missing": []},
        {"router_id": "core-derived", "name": "Core", "state": "ok", "error": null,
         "candidates": [candidate("pppoe-out7"),
            {"name": "endpoint-1", "type": "pppoe-out", "running": true, "disabled": false,
             "comment": null, "suggested": true, "source_id": "wan-source-1", "used_by": ["wan1"]}],
         "missing": []}
    ]})
}

fn rule(e: anyhow::Error) -> String {
    e.chain()
        .find_map(|c| c.downcast_ref::<InvalidPolicy>())
        .map(|i| i.0.clone())
        .unwrap_or_else(|| format!("not a rule error: {e:#}"))
}

fn run(d: &RegistryDocument, edit: Value) -> anyhow::Result<Applied> {
    let edit: Edit = serde_json::from_value(edit).unwrap();
    apply(d, &edit, Some(&discovery()), &cfg())
}

fn refused(d: &RegistryDocument, edit: Value) -> String {
    rule(run(d, edit).map(|_| ()).unwrap_err())
}

fn node<'a>(d: &'a RegistryDocument, id: &str) -> &'a catalog::CatalogNode {
    d.catalog
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|n| n.id == id)
        .unwrap_or_else(|| panic!("no node {id}"))
}

fn new_node<'a>(
    before: &RegistryDocument,
    after: &'a RegistryDocument,
) -> &'a catalog::CatalogNode {
    let old: Vec<_> = before
        .catalog
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .map(|n| &n.id)
        .collect();
    after
        .catalog
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|n| !old.contains(&&n.id))
        .unwrap()
}

fn record(id: &str, assignment: Option<&str>, pin: Option<&str>) -> SubscriberRecord {
    let mut r = catalog::tests::subscriber("group1");
    r.id = id.into();
    r.identities[0].account_ref = format!("acct-{id}");
    r.policy = SubscriberPolicy {
        profile_id: None,
        assignment: assignment.map(Into::into),
        pin: pin.map(Into::into),
    };
    r
}

fn mbps(n: u64) -> Option<u64> {
    Some(n * 1_000_000)
}

#[test]
fn wan_create_adds_source_node_router_and_role() {
    let d = doc();
    let a = run(
        &d,
        json!({"kind": "wan.create", "router_id": "edge-derived", "endpoint_ref": "pppoe-out1",
               "name": "WAN3", "down_mbps": 500, "up_mbps": 100}),
    )
    .unwrap();
    assert!(a.affected.is_empty());
    let c = a.candidate.catalog.as_ref().unwrap();
    let router = c.routers.iter().find(|r| r.name == "Edge").unwrap();
    assert_eq!(router.roles, vec![RouterRole::Wan]);
    let n = new_node(&d, &a.candidate);
    assert_eq!(
        (n.kind, n.parent.as_deref(), n.name.as_str()),
        (NodeKind::Wan, None, "WAN3")
    );
    assert_eq!(
        n.capacity,
        CapacityPolicy::Fixed {
            base: RateBps {
                down: 500_000_000,
                up: 100_000_000
            },
            factors: FactorChoice::Inherit,
        }
    );
    let source = c
        .sources
        .iter()
        .find(|s| Some(&s.id) == n.source_id.as_ref())
        .unwrap();
    assert_eq!(
        source.origin,
        SourceOrigin::Wan {
            router_id: router.id.clone(),
            endpoint_ref: "pppoe-out1".into()
        }
    );
    // A router that the catalog knows keeps its catalog ID.
    let a = run(
        &d,
        json!({"kind": "wan.create", "router_id": "core-derived", "endpoint_ref": "pppoe-out7",
               "name": "WAN7", "down_mbps": 100, "up_mbps": 100, "factor_percent": [90, 80]}),
    )
    .unwrap();
    let n = new_node(&d, &a.candidate);
    let c = a.candidate.catalog.as_ref().unwrap();
    let source = c
        .sources
        .iter()
        .find(|s| Some(&s.id) == n.source_id.as_ref())
        .unwrap();
    assert!(matches!(&source.origin, SourceOrigin::Wan { router_id, .. } if router_id == "core"));
}

#[test]
fn wan_create_refuses_a_candidate_missing_from_discovery_or_already_used() {
    let d = doc();
    let wan = |endpoint: &str| {
        json!({"kind": "wan.create", "router_id": "core-derived", "endpoint_ref": endpoint,
               "name": "W", "down_mbps": 100, "up_mbps": 100})
    };
    assert_eq!(
        refused(&d, wan("ether9")),
        "the WAN candidate is not in the last discovery"
    );
    assert_eq!(
        refused(&d, wan("endpoint-1")),
        "the WAN candidate is already used"
    );
    let edit: Edit = serde_json::from_value(wan("pppoe-out7")).unwrap();
    assert_eq!(
        rule(apply(&d, &edit, None, &cfg()).map(|_| ()).unwrap_err()),
        "the WAN candidate is not in the last discovery"
    );
    // The same candidate twice: the second one is already used.
    let first = run(&d, wan("pppoe-out7")).unwrap().candidate;
    assert_eq!(
        refused(&first, wan("pppoe-out7")),
        "the WAN candidate is already used"
    );
}

#[test]
fn node_create_and_update_respect_capacity_modes_and_limits() {
    let d = doc();
    let create = |capacity: Value| json!({"kind": "node.create", "name": "Tower", "parent": "wan1", "capacity": capacity});
    let a = run(
        &d,
        create(json!({"mode": "wan_percent", "down_percent": 50, "up_percent": 40})),
    )
    .unwrap();
    let n = new_node(&d, &a.candidate);
    assert_eq!(n.kind, NodeKind::Intermediate);
    assert!(a.affected.is_empty());
    assert_eq!(
        refused(&d, create(json!({"mode": "automatic"}))),
        "an intermediate node uses inherit, fixed, or wan_percent"
    );
    assert_eq!(
        refused(
            &d,
            create(json!({"mode": "fixed", "down_mbps": 0, "up_mbps": 5}))
        ),
        "a rate must be a whole number of Mbps from 1 to 1000000"
    );
    assert_eq!(
        refused(
            &d,
            create(json!({"mode": "wan_percent", "down_percent": 101, "up_percent": 5}))
        ),
        "a percentage must be a whole number from 1 to 100"
    );
    assert_eq!(
        refused(
            &d,
            json!({"kind": "node.create", "name": " x", "parent": null, "capacity": {"mode": "inherit"}})
        ),
        "a name must have 1 to 64 characters without outer spaces"
    );
    let update = |id: &str, capacity: Value| json!({"kind": "node.update", "node_id": id, "capacity": capacity});
    assert_eq!(
        refused(&d, update("wan1", json!({"mode": "inherit"}))),
        "a WAN node uses fixed"
    );
    assert_eq!(
        refused(
            &d,
            update(
                "wan1",
                json!({"mode": "fixed", "down_mbps": 5, "up_mbps": 5, "factor_percent": [0, 50]})
            )
        ),
        "a percentage must be a whole number from 1 to 100"
    );
    assert_eq!(
        refused(&d, update("group1", json!({"mode": "inherit"}))),
        "a subscriber group uses automatic or fixed"
    );
    let a = run(
        &d,
        update(
            "wan1",
            json!({"mode": "fixed", "down_mbps": 200, "up_mbps": 50}),
        ),
    )
    .unwrap();
    assert_eq!(a.affected, vec!["profile-source".to_string()]);
}

#[test]
fn instance_add_accepts_only_a_selected_source_and_splits_the_minimum() {
    let mut d = doc();
    let c = d.catalog.as_mut().unwrap();
    // Equal WANs and one instance with a 30 Mbps minimum.
    c.nodes[1].capacity = c.nodes[0].capacity;
    c.nodes.retain(|n| n.id != "group2");
    c.sources[2].targets = vec!["group1".into()];
    c.nodes
        .iter_mut()
        .find(|n| n.id == "group1")
        .unwrap()
        .minimum = NodeMinimum {
        down: mbps(30),
        up: mbps(9),
    };
    let add = json!({"kind": "instance.add", "source_id": "profile-source", "parent": "wan2"});
    let a = run(&d, add.clone()).unwrap();
    assert_eq!(a.affected, vec!["profile-source".to_string()]);
    let n = new_node(&d, &a.candidate);
    assert_eq!(n.parent.as_deref(), Some("wan2"));
    assert_eq!(n.name, "One shared profile wan2");
    // Equal weights: the remainder goes to the first instance in catalog order.
    assert_eq!(
        n.minimum,
        NodeMinimum {
            down: mbps(15),
            up: mbps(4)
        }
    );
    assert_eq!(
        node(&a.candidate, "group1").minimum,
        NodeMinimum {
            down: mbps(15),
            up: mbps(5)
        }
    );
    let c = a.candidate.catalog.as_ref().unwrap();
    let source = c.sources.iter().find(|s| s.id == "profile-source").unwrap();
    assert!(source.targets.contains(&n.id));
    // A 1:2 capacity split.
    let mut d2 = doc();
    let c = d2.catalog.as_mut().unwrap();
    c.nodes.retain(|n| n.id != "group2");
    c.sources[2].targets = vec!["group1".into()];
    c.nodes
        .iter_mut()
        .find(|n| n.id == "group1")
        .unwrap()
        .minimum = NodeMinimum {
        down: mbps(30),
        up: None,
    };
    let a = run(&d2, add).unwrap();
    let n = new_node(&d2, &a.candidate);
    assert_eq!(
        n.minimum,
        NodeMinimum {
            down: mbps(20),
            up: None
        }
    );
    assert_eq!(
        node(&a.candidate, "group1").minimum,
        NodeMinimum {
            down: mbps(10),
            up: None
        }
    );
    // A source without an instance is not selected.
    let mut d3 = doc();
    let c = d3.catalog.as_mut().unwrap();
    c.nodes
        .retain(|n| n.source_id.as_deref() != Some("profile-source"));
    c.sources[2].targets.clear();
    assert_eq!(
        refused(
            &d3,
            json!({"kind": "instance.add", "source_id": "profile-source", "parent": "wan1"})
        ),
        "the source is not selected"
    );
}

#[test]
fn move_keeps_id_capacity_and_refuses_cycles_wan_children_and_duplicate_instances() {
    let mut d = doc();
    let tower = run(
        &d,
        json!({"kind": "node.create", "name": "Tower", "parent": "wan1", "capacity": {"mode": "inherit"}}),
    )
    .unwrap()
    .candidate;
    let tower_id = new_node(&d, &tower).id.clone();
    d = tower;
    let before = node(&d, "group1").clone();
    let a = run(
        &d,
        json!({"kind": "node.move", "node_id": "group1", "parent": tower_id}),
    )
    .unwrap();
    let after = node(&a.candidate, "group1");
    assert_eq!(
        (after.capacity, after.parent.as_deref()),
        (before.capacity, Some(tower_id.as_str()))
    );
    assert_eq!(a.affected, vec!["profile-source".to_string()]);
    assert_eq!(
        refused(
            &d,
            json!({"kind": "node.move", "node_id": "wan2", "parent": "wan1"})
        ),
        "a WAN node stays a root"
    );
    assert_eq!(
        refused(
            &d,
            json!({"kind": "node.move", "node_id": tower_id, "parent": tower_id})
        ),
        "a node cannot move below itself"
    );
    assert_eq!(
        refused(
            &d,
            json!({"kind": "node.move", "node_id": "group2", "parent": "wan1"})
        ),
        "duplicate source instance under the same parent"
    );
}

#[test]
fn remove_refuses_children_pins_last_instance_and_aliased_nodes() {
    let mut d = doc();
    assert_eq!(
        refused(&d, json!({"kind": "node.remove", "node_id": "wan1"})),
        "move or remove the children first"
    );
    d.subscribers = vec![record("s1", Some("group1"), Some("group1"))];
    assert_eq!(
        refused(&d, json!({"kind": "node.remove", "node_id": "group1"})),
        "unpin the subscribers first: s1"
    );
    let mut last = doc();
    let c = last.catalog.as_mut().unwrap();
    c.nodes.retain(|n| n.id != "group2");
    c.sources[2].targets = vec!["group1".into()];
    assert_eq!(
        refused(&last, json!({"kind": "node.remove", "node_id": "group1"})),
        "this is the last instance of the source"
    );
    let mut aliased = doc();
    aliased
        .export_ownership
        .as_mut()
        .unwrap()
        .nodes
        .push(NodeAlias {
            node_id: "group1".into(),
            id: "legacy-id".into(),
            name: "Legacy".into(),
        });
    assert_eq!(
        refused(
            &aliased,
            json!({"kind": "node.remove", "node_id": "group1"})
        ),
        "an enrolled node keeps its legacy export name"
    );
}

#[test]
fn remove_clears_offline_assignments_and_removes_an_unused_wan_source() {
    let mut d = doc();
    d.subscribers = vec![
        record("s1", Some("group1"), None),
        record("s2", Some("group2"), None),
    ];
    let a = run(&d, json!({"kind": "node.remove", "node_id": "group1"})).unwrap();
    assert_eq!(a.cleared, vec!["s1".to_string()]);
    assert_eq!(a.affected, vec!["profile-source".to_string()]);
    let s1 = a
        .candidate
        .subscribers
        .iter()
        .find(|s| s.id == "s1")
        .unwrap();
    assert_eq!(s1.policy.assignment, None);
    let c = a.candidate.catalog.as_ref().unwrap();
    let source = c.sources.iter().find(|s| s.id == "profile-source").unwrap();
    assert!(!source.targets.contains(&"group1".to_string()));
    // Remove the WAN after its child: the WAN source goes too.
    let a = run(
        &a.candidate,
        json!({"kind": "node.remove", "node_id": "wan1"}),
    )
    .unwrap();
    let c = a.candidate.catalog.as_ref().unwrap();
    assert!(c.sources.iter().all(|s| s.id != "wan-source-1"));
    assert!(
        c.routers
            .iter()
            .find(|r| r.id == "core")
            .unwrap()
            .roles
            .contains(&RouterRole::Wan)
    );
}

#[test]
fn rename_of_an_aliased_node_is_refused() {
    let mut d = doc();
    d.export_ownership.as_mut().unwrap().nodes.push(NodeAlias {
        node_id: "group1".into(),
        id: "legacy-id".into(),
        name: "Legacy".into(),
    });
    assert_eq!(
        refused(
            &d,
            json!({"kind": "node.update", "node_id": "group1", "name": "New"})
        ),
        "the export name comes from the legacy node"
    );
    let a = run(
        &d,
        json!({"kind": "node.update", "node_id": "group2", "name": "New"}),
    )
    .unwrap();
    assert_eq!(node(&a.candidate, "group2").name, "New");
    assert!(a.affected.is_empty());
}

#[test]
fn affected_sources_follow_the_spec() {
    let d = doc();
    let a = run(
        &d,
        json!({"kind": "node.create", "name": "Tower", "parent": null, "capacity": {"mode": "fixed", "down_mbps": 10, "up_mbps": 10}}),
    )
    .unwrap();
    assert!(a.affected.is_empty());
    let a = run(
        &d,
        json!({"kind": "node.update", "node_id": "wan1", "name": "Uplink"}),
    )
    .unwrap();
    assert!(a.affected.is_empty());
}

#[test]
fn edits_before_enrollment_are_refused() {
    let mut d = doc();
    d.export_ownership = None;
    assert_eq!(
        refused(
            &d,
            json!({"kind": "node.update", "node_id": "wan1", "name": "Uplink"})
        ),
        "enroll first: the registry has no export ownership"
    );
}

#[test]
fn unknown_fields_are_refused_at_parse() {
    let bad = json!({"kind": "node.remove", "node_id": "wan1", "extra": 1});
    assert!(serde_json::from_value::<Edit>(bad).is_err());
}

#[test]
fn tree_lists_every_catalog_node_with_capacity_and_alias() {
    let mut d = doc();
    d.revision = 4;
    d.export_ownership.as_mut().unwrap().nodes.push(NodeAlias {
        node_id: "group1".into(),
        id: "legacy-id".into(),
        name: "Legacy".into(),
    });
    let t = tree(&d);
    assert_eq!(t["revision"], json!(4));
    let nodes = t["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 4);
    let wan1 = nodes.iter().find(|n| n["id"] == "wan1").unwrap();
    assert_eq!(wan1["kind"], json!("wan"));
    assert_eq!(
        wan1["capacity"],
        json!({"mode": "fixed", "down_mbps": 310, "up_mbps": 100, "factor_percent": null})
    );
    let g1 = nodes.iter().find(|n| n["id"] == "group1").unwrap();
    assert_eq!(
        (&g1["alias"], &g1["source_id"]),
        (&json!(true), &json!("profile-source"))
    );
    assert_eq!(g1["capacity"], json!({"mode": "automatic"}));
    let sources = t["sources"].as_array().unwrap();
    assert!(
        sources
            .iter()
            .any(|s| s["id"] == "profile-source" && s["kind"] == "subscribers")
    );
}

#[test]
fn remove_keeps_the_group_minimum_total() {
    let mut d = doc();
    for (id, m) in [("group1", 15), ("group2", 15)] {
        d.catalog
            .as_mut()
            .unwrap()
            .nodes
            .iter_mut()
            .find(|n| n.id == id)
            .unwrap()
            .minimum = NodeMinimum {
            down: mbps(m),
            up: None,
        };
    }
    let a = run(&d, json!({"kind": "node.remove", "node_id": "group2"})).unwrap();
    assert_eq!(
        node(&a.candidate, "group1").minimum,
        NodeMinimum {
            down: mbps(30),
            up: None
        }
    );
}

fn identity(id: &str) -> Value {
    json!({"router_id": "distro", "source_id": "ppp-service", "kind": "ppp", "account_ref": format!("acct-{id}")})
}

fn with_records(records: Vec<SubscriberRecord>) -> RegistryDocument {
    let mut d = doc();
    d.subscribers = records;
    d
}

/// Three WANs with the group below WAN1 and WAN2 only, and a single-instance source.
fn single_and_double() -> RegistryDocument {
    let mut d = doc();
    let c = d.catalog.as_mut().unwrap();
    let mut other = c
        .sources
        .iter()
        .find(|s| s.id == "profile-source")
        .unwrap()
        .clone();
    other.id = "solo-source".into();
    other.targets = vec!["solo".into()];
    if let SourceOrigin::Subscribers { profile_ref, .. } = &mut other.origin {
        *profile_ref = Some("solo-profile".into());
    }
    c.sources.push(other);
    let mut solo = c.nodes.iter().find(|n| n.id == "group1").unwrap().clone();
    solo.id = "solo".into();
    solo.source_id = Some("solo-source".into());
    solo.parent = Some("wan2".into());
    c.nodes.push(solo);
    d
}

#[test]
fn rebalance_one_group_marks_it_and_refuses_a_single_instance() {
    let d = single_and_double();
    let a = run(
        &d,
        json!({"kind": "rebalance", "source_id": "profile-source"}),
    )
    .unwrap();
    assert_eq!(a.affected, vec!["profile-source".to_string()]);
    assert!(a.clear_offline);
    assert_eq!(
        a.candidate.catalog,
        d.catalog.as_ref().map(|c| {
            let mut c = c.clone();
            c.canonicalize();
            c
        })
    );
    assert_eq!(
        refused(&d, json!({"kind": "rebalance", "source_id": "solo-source"})),
        "the group has only one instance"
    );
}

#[test]
fn rebalance_all_skips_single_instance_groups_and_refuses_when_none_is_left() {
    let d = single_and_double();
    let a = run(&d, json!({"kind": "rebalance", "source_id": null})).unwrap();
    assert_eq!(a.affected, vec!["profile-source".to_string()]);
    let mut lone = d.clone();
    let c = lone.catalog.as_mut().unwrap();
    c.nodes.retain(|n| n.id != "group2");
    c.sources
        .iter_mut()
        .find(|s| s.id == "profile-source")
        .unwrap()
        .targets = vec!["group1".into()];
    assert_eq!(
        refused(&lone, json!({"kind": "rebalance", "source_id": null})),
        "no group has two or more instances"
    );
}

#[test]
fn pin_sets_pin_and_assignment_to_the_group_instance_below_the_wan() {
    let d = with_records(vec![record("s1", Some("group1"), None)]);
    let a = run(
        &d,
        json!({"kind": "subscriber.pin", "identity": identity("s1"), "wan_node_id": "wan2"}),
    )
    .unwrap();
    let s = &a.candidate.subscribers[0];
    assert_eq!(
        (s.policy.pin.as_deref(), s.policy.assignment.as_deref()),
        (Some("group2"), Some("group2"))
    );
    assert_eq!(a.require_online, vec!["s1".to_string()]);
    assert_eq!(a.affected, vec!["profile-source".to_string()]);
    assert!(!a.clear_offline);
}

#[test]
fn a_repin_moves_the_pin_to_the_other_wan() {
    let d = with_records(vec![record("s1", Some("group1"), Some("group1"))]);
    let a = run(
        &d,
        json!({"kind": "subscriber.pin", "identity": identity("s1"), "wan_node_id": "wan2"}),
    )
    .unwrap();
    assert_eq!(
        a.candidate.subscribers[0].policy.pin.as_deref(),
        Some("group2")
    );
}

#[test]
fn pin_refusals_follow_the_spec() {
    let pin = |id: &str, wan: &str| json!({"kind": "subscriber.pin", "identity": identity(id), "wan_node_id": wan});
    let d = with_records(vec![
        record("s1", Some("group1"), None),
        record("s2", None, None),
    ]);
    assert_eq!(
        refused(&d, pin("nobody", "wan2")),
        "confirm an edit or rebalance first, so the account gets a record"
    );
    assert_eq!(
        refused(&d, pin("s2", "wan2")),
        "the subscriber has no placement yet"
    );
    assert_eq!(refused(&d, pin("s1", "group2")), "the node is not a WAN");
    let mut no_instance = d.clone();
    let c = no_instance.catalog.as_mut().unwrap();
    c.nodes.retain(|n| n.id != "group2");
    c.sources
        .iter_mut()
        .find(|s| s.id == "profile-source")
        .unwrap()
        .targets = vec!["group1".into()];
    assert_eq!(
        refused(&no_instance, pin("s1", "wan2")),
        "the group has no instance below this WAN"
    );
    let mut two = d.clone();
    let tower = run(
        &two,
        json!({"kind": "node.create", "name": "T", "parent": "wan2", "capacity": {"mode": "inherit"}}),
    )
    .unwrap()
    .candidate;
    let tower_id = new_node(&two, &tower).id.clone();
    two = run(
        &tower,
        json!({"kind": "instance.add", "source_id": "profile-source", "parent": tower_id}),
    )
    .unwrap()
    .candidate;
    assert_eq!(
        refused(&two, pin("s1", "wan2")),
        "the group has more than one instance below this WAN"
    );
}

#[test]
fn unpin_by_identity_or_subscriber_id_keeps_the_assignment_and_marks_no_source() {
    let d = with_records(vec![record("s1", Some("group2"), Some("group2"))]);
    for edit in [
        json!({"kind": "subscriber.pin", "identity": identity("s1"), "wan_node_id": null}),
        json!({"kind": "subscriber.pin", "subscriber_id": "s1", "wan_node_id": null}),
    ] {
        let a = run(&d, edit).unwrap();
        let s = &a.candidate.subscribers[0];
        assert_eq!(
            (s.policy.pin.as_deref(), s.policy.assignment.as_deref()),
            (None, Some("group2"))
        );
        assert!(a.affected.is_empty() && a.require_online.is_empty());
    }
    assert_eq!(
        refused(&d, json!({"kind": "subscriber.pin", "wan_node_id": null})),
        "give identity or subscriber_id"
    );
}

#[test]
fn unpin_without_a_pin_is_refused() {
    let d = with_records(vec![record("s1", Some("group1"), None)]);
    assert_eq!(
        refused(
            &d,
            json!({"kind": "subscriber.pin", "subscriber_id": "s1", "wan_node_id": null})
        ),
        "the subscriber has no pin"
    );
}

#[test]
fn tree_lists_pins() {
    let d = with_records(vec![
        record("s1", Some("group1"), Some("group1")),
        record("s2", Some("group2"), None),
    ]);
    assert_eq!(
        tree(&d)["pins"],
        json!([{"subscriber_id": "s1", "node_id": "group1"}])
    );
}

fn speed(min: Value, max: Value) -> Value {
    json!({"kind": "speed.defaults", "down_min": min, "up_min": min, "down_max": max, "up_max": max})
}

fn legacy_profile_doc() -> RegistryDocument {
    use crate::topology_manager::rates::{Fraction, RateRule, RuleValue, SpeedOverrides};
    let mut d = doc();
    let rule = RuleValue::Override(RateRule::LegacyRoundedBase {
        factor: Fraction {
            numerator: 1,
            denominator: 10,
        },
        floor_bps: 1_000_000,
    });
    d.catalog.as_mut().unwrap().profiles[0].speed = SpeedOverrides {
        down_min: rule,
        up_min: rule,
        down_max: RuleValue::Inherit,
        up_max: RuleValue::Inherit,
    };
    d
}

#[test]
fn speed_defaults_set_the_defaults_and_reset_every_profile_override() {
    use crate::topology_manager::rates::{Fraction, RateRule, SpeedOverrides};
    let d = legacy_profile_doc();
    let a = run(&d, speed(json!({"percent": 10}), json!({"percent": 100}))).unwrap();
    let c = a.candidate.catalog.as_ref().unwrap();
    assert_eq!(
        c.defaults.down_min,
        RateRule::BaseFraction(Fraction {
            numerator: 100,
            denominator: 1000
        })
    );
    assert_eq!(
        c.defaults.up_max,
        RateRule::BaseFraction(Fraction {
            numerator: 1000,
            denominator: 1000
        })
    );
    assert!(
        c.profiles
            .iter()
            .all(|p| p.speed == SpeedOverrides::default())
    );
    assert_eq!(a.legacy_replaced, 1);
    assert!(a.check_only && a.affected.is_empty());
}

#[test]
fn speed_default_ranges_and_decimals() {
    let d = doc();
    let text = "a speed rule must be a minimum percent from 0.1 to 100, a maximum percent from 1 to 1000 with one decimal, or a whole number of Mbps from 1 to 1000000";
    for (min, max) in [
        (json!({"percent": 0.05}), json!({"percent": 100})),
        (json!({"percent": 100.5}), json!({"percent": 100})),
        (json!({"percent": 0.15}), json!({"percent": 100})),
        (json!({"percent": 10}), json!({"percent": 1001})),
        (json!({"mbps": 0}), json!({"percent": 100})),
    ] {
        assert_eq!(refused(&d, speed(min, max)), text);
    }
    for (min, max) in [
        (json!({"percent": 0.1}), json!({"percent": 1000})),
        (json!({"percent": 12.5}), json!({"mbps": 500})),
    ] {
        assert!(run(&d, speed(min, max)).is_ok());
    }
}

#[test]
fn a_minimum_above_the_maximum_for_a_local_plan_is_refused() {
    // The fixture plan is 50/10 Mbps: a 20 Mbps minimum is above a 100% upload maximum.
    let e = run(&doc(), speed(json!({"mbps": 20}), json!({"percent": 100})))
        .map(|_| ())
        .unwrap_err();
    assert!(e.chain().any(|c| c.is::<InvalidPolicy>()), "{e:#}");
}

#[test]
fn group_factor_round_trips_the_legacy_value_and_keeps_an_absent_direction() {
    use crate::topology_manager::rates::Fraction;
    let d = doc();
    let a = run(
        &d,
        json!({"kind": "group.factor", "source_id": "profile-source", "down": "0.215"}),
    )
    .unwrap();
    let c = a.candidate.catalog.as_ref().unwrap();
    let s = c.sources.iter().find(|s| s.id == "profile-source").unwrap();
    let SourceOrigin::Subscribers { group, .. } = &s.origin else {
        panic!()
    };
    assert_eq!(
        group.factors.down,
        Fraction {
            numerator: 215_000,
            denominator: 1_000_000
        }
    );
    // The fixture up factor is 1/2 and stays.
    assert_eq!(
        group.factors.up,
        Fraction {
            numerator: 1,
            denominator: 2
        }
    );
    assert_eq!(factor_text(group.factors.down), "0.215");
    assert_eq!(factor_text(group.factors.up), "0.5");
    assert_eq!(
        factor_text(Fraction {
            numerator: 1,
            denominator: 1
        }),
        "1"
    );
    assert!(a.check_only);
}

#[test]
fn group_factor_ranges_and_unknown_source() {
    let d = doc();
    let f = |v: &str| json!({"kind": "group.factor", "source_id": "profile-source", "up": v});
    for bad in ["0", "1.5", "0.0000001", "abc", "-0.5"] {
        assert_eq!(
            refused(&d, f(bad)),
            "a group factor must be a decimal from 0.001 to 1 with up to six decimals"
        );
    }
    assert_eq!(
        refused(
            &d,
            json!({"kind": "group.factor", "source_id": "nope", "up": "0.5"})
        ),
        "the source does not exist"
    );
}

#[test]
fn instance_factor_overrides_are_kept() {
    use crate::topology_manager::catalog::Factors;
    use crate::topology_manager::rates::Fraction;
    let mut d = doc();
    let half = Fraction {
        numerator: 1,
        denominator: 2,
    };
    let over = FactorChoice::Override(Factors {
        down: half,
        up: half,
    });
    d.catalog
        .as_mut()
        .unwrap()
        .nodes
        .iter_mut()
        .find(|n| n.id == "group1")
        .unwrap()
        .capacity = CapacityPolicy::Automatic { factors: over };
    let a = run(
        &d,
        json!({"kind": "group.factor", "source_id": "profile-source", "down": "0.9"}),
    )
    .unwrap();
    assert_eq!(
        node(&a.candidate, "group1").capacity,
        CapacityPolicy::Automatic { factors: over }
    );
}

#[test]
fn node_create_makes_a_group_without_a_source() {
    let d = doc();
    let a = run(
        &d,
        json!({"kind": "node.create", "name": "Fallback", "parent": "wan1", "node_kind": "group", "capacity": {"mode": "automatic"}}),
    )
    .unwrap();
    let n = new_node(&d, &a.candidate);
    assert_eq!(
        (n.kind, n.source_id.as_deref()),
        (NodeKind::SubscriberGroup, None)
    );
    assert_eq!(
        refused(
            &d,
            json!({"kind": "node.create", "name": "X", "parent": "wan1", "node_kind": "group", "capacity": {"mode": "inherit"}})
        ),
        "a subscriber group uses automatic or fixed"
    );
}

fn with_fallback_group() -> (RegistryDocument, String) {
    let d = doc();
    let a = run(
        &d,
        json!({"kind": "node.create", "name": "Fallback", "parent": "wan1", "node_kind": "group", "capacity": {"mode": "automatic"}}),
    )
    .unwrap();
    let id = new_node(&d, &a.candidate).id.clone();
    (a.candidate, id)
}

#[test]
fn fallback_set_and_clear_edits() {
    let (d, fb) = with_fallback_group();
    let a = run(&d, json!({"kind": "fallback.set", "node_ids": [fb], "default_down_mbps": 10, "default_up_mbps": 5})).unwrap();
    let f = a
        .candidate
        .catalog
        .as_ref()
        .unwrap()
        .fallback
        .clone()
        .unwrap();
    assert_eq!(f.node_ids, vec![fb.clone()]);
    assert_eq!(
        f.default_plan,
        RateBps {
            down: 10_000_000,
            up: 5_000_000
        }
    );
    assert_eq!(
        refused(
            &d,
            json!({"kind": "fallback.set", "node_ids": ["group1"], "default_down_mbps": 10, "default_up_mbps": 5})
        ),
        "a fallback node must be a group without a source"
    );
    let c = run(&a.candidate, json!({"kind": "fallback.clear"})).unwrap();
    assert!(c.candidate.catalog.as_ref().unwrap().fallback.is_none());
    assert!(c.clear_fallback);
}

#[test]
fn a_fallback_node_cannot_be_removed() {
    let (d, fb) = with_fallback_group();
    let d = run(&d, json!({"kind": "fallback.set", "node_ids": [fb], "default_down_mbps": 10, "default_up_mbps": 5}))
        .unwrap()
        .candidate;
    assert_eq!(
        refused(&d, json!({"kind": "node.remove", "node_id": fb})),
        "remove it from the fallback first"
    );
}

#[test]
fn tree_shows_the_fallback() {
    let (d, fb) = with_fallback_group();
    assert_eq!(tree(&d)["fallback"], Value::Null);
    let d = run(&d, json!({"kind": "fallback.set", "node_ids": [fb], "default_down_mbps": 10, "default_up_mbps": 5}))
        .unwrap()
        .candidate;
    assert_eq!(
        tree(&d)["fallback"],
        json!({"node_ids": [fb], "default_mbps": [10, 5]})
    );
}

mod fallback_fixes {
    use super::*;
    use crate::topology_manager::catalog::{CatalogRouter, FallbackPolicy};
    use crate::topology_manager::identity::{ReferenceAuthority, SourceSelection};
    use crate::topology_manager::model::{SourceKind, SourceScope};
    use crate::topology_manager::policy_draft::PolicyDraft;
    use crate::topology_manager::settings::SourceSettings;
    use crate::topology_manager::source_policy::{ProfileBinding, RateRead, SourcePolicy};

    fn ppp_scope() -> SourceScope {
        SourceScope {
            router_id: "distro".into(),
            source_id: "ppp-service".into(),
            kind: SourceKind::Ppp,
        }
    }

    fn selection(
        scope: SourceScope,
        bindings: &[(&str, &str)],
        authority: ReferenceAuthority,
    ) -> SourceSelection {
        SourceSelection {
            scope,
            service_names: vec![],
            authority,
            policy: SourcePolicy {
                profiles: bindings
                    .iter()
                    .map(|(r, n)| ProfileBinding {
                        source_ref: (*r).into(),
                        current_name: (*n).into(),
                        rate: RateRead::Disabled,
                    })
                    .collect(),
                ..Default::default()
            },
        }
    }

    fn settled() -> RegistryDocument {
        let mut d = doc();
        d.source_settings = Some(SourceSettings {
            selections: vec![selection(
                ppp_scope(),
                &[("provider-profile-id", "gold")],
                ReferenceAuthority::PppSecretName,
            )],
        });
        d.validate().unwrap();
        d
    }

    fn draft_of(d: &RegistryDocument) -> PolicyDraft {
        PolicyDraft {
            catalog: d.catalog.clone().unwrap(),
            source_settings: d.source_settings.clone().unwrap(),
        }
    }

    fn with_fallback(mut d: RegistryDocument) -> (RegistryDocument, String) {
        let a = run(
            &d,
            json!({"kind": "node.create", "name": "Fallback", "parent": "wan1", "node_kind": "group", "capacity": {"mode": "automatic"}}),
        )
        .unwrap();
        let fb = new_node(&d, &a.candidate).id.clone();
        d = a.candidate;
        d.catalog.as_mut().unwrap().fallback = Some(FallbackPolicy {
            node_ids: vec![fb.clone()],
            default_plan: RateBps {
                down: 10_000_000,
                up: 10_000_000,
            },
        });
        (d, fb)
    }

    #[test]
    fn fallback_clear_drops_assignments_to_the_fallback_nodes() {
        let (mut d, fb) = with_fallback(doc());
        d.subscribers = vec![record("s1", Some(&fb), None)];
        let a = run(&d, json!({"kind": "fallback.clear"})).unwrap();
        assert_eq!(a.cleared, vec!["s1".to_string()]);
        assert_eq!(a.candidate.subscribers[0].policy.assignment, None);
        let mut pinned = d.clone();
        pinned.subscribers = vec![record("s1", Some(&fb), Some(&fb))];
        assert!(
            refused(&pinned, json!({"kind": "fallback.clear"}))
                .starts_with("unpin the subscribers first")
        );
    }

    #[test]
    fn fallback_set_drops_assignments_to_a_node_that_leaves() {
        let (mut d, fb) = with_fallback(doc());
        let a = run(
            &d,
            json!({"kind": "node.create", "name": "Fallback2", "parent": "wan2", "node_kind": "group", "capacity": {"mode": "automatic"}}),
        )
        .unwrap();
        let fb2 = new_node(&d, &a.candidate).id.clone();
        d = a.candidate;
        d.subscribers = vec![record("s1", Some(&fb), None)];
        let a = run(&d, json!({"kind": "fallback.set", "node_ids": [fb2], "default_down_mbps": 10, "default_up_mbps": 10})).unwrap();
        assert_eq!(a.cleared, vec!["s1".to_string()]);
    }

    #[test]
    fn source_add_adds_the_router_and_its_role() {
        let mut d = settled();
        d.catalog
            .as_mut()
            .unwrap()
            .routers
            .retain(|r| r.id != "edge");
        let mut draft = draft_of(&d);
        let edge = SourceScope {
            router_id: "edge".into(),
            source_id: "lan".into(),
            kind: SourceKind::Dhcp,
        };
        draft.catalog.routers.push(CatalogRouter {
            id: "edge".into(),
            name: "Edge".into(),
            roles: vec![catalog::RouterRole::Dhcp],
        });
        let mut source = draft
            .catalog
            .sources
            .iter()
            .find(|s| s.id == "profile-source")
            .unwrap()
            .clone();
        source.id = "edge-lan".into();
        source.targets.clear();
        source.profile_id = Some("local-plan".into());
        if let SourceOrigin::Subscribers {
            scope, profile_ref, ..
        } = &mut source.origin
        {
            *scope = edge.clone();
            *profile_ref = None;
        }
        draft.catalog.sources.push(source);
        let mut node = draft
            .catalog
            .nodes
            .iter()
            .find(|n| n.id == "group1")
            .unwrap()
            .clone();
        node.id = "edge-lan-node".into();
        node.source_id = Some("edge-lan".into());
        draft.catalog.nodes.push(node);
        draft.source_settings.selections.push(selection(
            edge,
            &[],
            ReferenceAuthority::DhcpClientMac,
        ));
        let a = add_source(&d, &draft, "edge-lan", Some("wan2")).unwrap();
        let c = a.candidate.catalog.as_ref().unwrap();
        let router = c.routers.iter().find(|r| r.id == "edge").unwrap();
        assert!(router.roles.contains(&catalog::RouterRole::Dhcp));
    }

    #[test]
    fn frontend_draft_attaches_only_the_selected_discovered_source() {
        let discovered = draft_of(&settled());
        let empty = run(
            &RegistryDocument::default(),
            json!({"kind":"topology.initialize"}),
        )
        .unwrap()
        .candidate;
        let added = add_source(&empty, &discovered, "profile-source", None)
            .unwrap()
            .candidate;
        let c = added.catalog.as_ref().unwrap();
        assert_eq!(c.sources.len(), 1);
        assert_eq!(c.sources[0].id, "profile-source");
        assert_eq!(c.nodes.len(), 1);
        assert_eq!(c.nodes[0].source_id.as_deref(), Some("profile-source"));
        assert!(c.nodes[0].parent.is_none());
        assert!(added.subscribers.is_empty() && added.export_ownership.is_none());
        assert!(empty.catalog.as_ref().unwrap().nodes.is_empty());
        assert!(empty.catalog.as_ref().unwrap().sources.is_empty());
    }

    #[test]
    fn a_renamed_profile_is_a_rebind_candidate() {
        let d = settled();
        let mut draft = draft_of(&d);
        draft.source_settings.selections[0].policy.profiles[0].current_name = "gold-2026".into();
        let list = candidates(&d, &draft);
        let rebind = list
            .iter()
            .find(|c| c["rebind"] == true)
            .unwrap_or_else(|| panic!("{list:?}"));
        assert_eq!(rebind["key"], json!("profile-source"));
        assert!(
            rebind["name"]
                .as_str()
                .unwrap()
                .contains("gold -> gold-2026")
        );
        let a = add_source(&d, &draft, "profile-source", None).unwrap();
        let b = &a.candidate.source_settings.as_ref().unwrap().selections[0]
            .policy
            .profiles[0];
        assert_eq!(b.current_name, "gold-2026");
        assert_eq!(a.affected, vec!["profile-source".to_string()]);
    }
}

#[test]
fn every_web_builder_edit_is_a_service_edit() {
    use web_contract::topology::{BuilderEdit as B, Capacity as C};
    let fixed = C::Fixed {
        down_mbps: 100,
        up_mbps: 50,
        factor_percent: None,
    };
    let edits = [
        B::Initialize,
        B::NodeCreate {
            name: "Tower".into(),
            parent: Some("wan1".into()),
            capacity: C::Automatic,
            node_kind: Some("group".into()),
        },
        B::NodeCreate {
            name: "Core".into(),
            parent: None,
            capacity: fixed.clone(),
            node_kind: None,
        },
        B::InstanceAdd {
            source_id: "profile-source".into(),
            parent: None,
            name: None,
        },
        B::NodeMove {
            node_id: "group1".into(),
            parent: Some("wan2".into()),
        },
        B::NodeUpdate {
            node_id: "wan1".into(),
            name: Some("North".into()),
            capacity: Some(C::WanPercent {
                down_percent: 50,
                up_percent: 50,
            }),
        },
        B::NodeUpdate {
            node_id: "wan1".into(),
            name: None,
            capacity: Some(C::Inherit),
        },
        B::NodeRemove {
            node_id: "group1".into(),
        },
        B::WanCreate {
            router_id: "core".into(),
            endpoint_ref: "pppoe-out7".into(),
            name: "WAN 7".into(),
            down_mbps: 500,
            up_mbps: 100,
            factor_percent: None,
        },
        B::SourceAdd {
            key: "access-ppp".into(),
            parent: Some("wan1".into()),
        },
        B::PlanCreate {
            name: "Gold".into(),
            down_mbps: 100,
            up_mbps: 50,
        },
        B::PlanUpdate {
            plan_id: "plan-1".into(),
            name: None,
            down_mbps: Some(200),
            up_mbps: None,
        },
        B::PlanDelete {
            plan_id: "plan-1".into(),
        },
        B::SubscriberPlan {
            subscriber_id: "tm-sub-1".into(),
            plan_id: None,
        },
        B::SubscriberPin {
            subscriber_id: "tm-sub-1".into(),
            wan_node_id: Some("wan1".into()),
        },
        B::Rebalance { source_id: None },
        B::SpeedDefaults {
            down_min: web_contract::topology::RuleInput::Percent(20.5),
            up_min: web_contract::topology::RuleInput::Mbps(5),
            down_max: web_contract::topology::RuleInput::Percent(100.0),
            up_max: web_contract::topology::RuleInput::Percent(100.0),
        },
        B::GroupFactor {
            source_id: "profile-source".into(),
            down: Some("0.5".into()),
            up: None,
        },
        B::FallbackSet {
            node_ids: vec!["group1".into()],
            default_down_mbps: 10,
            default_up_mbps: 5,
        },
        B::FallbackClear,
    ];
    for edit in edits {
        let wire = serde_json::to_value(&edit).unwrap();
        let parsed: Result<Edit, _> = serde_json::from_value(wire.clone());
        assert!(parsed.is_ok(), "{wire}: {parsed:?}");
    }
}

#[test]
fn frontend_initialization_is_empty_reviewable_and_never_replaces_policy() {
    let original = RegistryDocument::default();
    let initialized = run(&original, json!({"kind":"topology.initialize"}))
        .unwrap()
        .candidate;
    assert!(original.catalog.is_none());
    let c = initialized.catalog.as_ref().unwrap();
    assert!(c.nodes.is_empty() && c.sources.is_empty());
    assert_eq!(c.routers.len(), cfg().routers.len());
    assert!(
        initialized
            .source_settings
            .as_ref()
            .unwrap()
            .selections
            .is_empty()
    );
    assert!(initialized.subscribers.is_empty());
    assert!(initialized.export_ownership.is_none());
    assert!(run(&initialized, json!({"kind":"topology.initialize"})).is_err());
    assert!(run(&doc(), json!({"kind":"topology.initialize"})).is_err());
    let added = run(
        &initialized,
        json!({"kind":"node.create","name":"Distribution","parent":null,
        "capacity":{"mode":"fixed","down_mbps":100,"up_mbps":50}}),
    );
    assert!(added.is_ok(), "{added:?}");
    assert!(added.unwrap().candidate.subscribers.is_empty());
}

#[test]
fn duplicate_source_parent_placement_is_refused() {
    let d = doc();
    let c = d.catalog.as_ref().unwrap();
    let n = c
        .nodes
        .iter()
        .find(|n| n.source_id.as_deref() == Some("profile-source"))
        .unwrap();
    let err = refused(
        &d,
        json!({"kind":"instance.add","source_id":"profile-source","parent":n.parent}),
    );
    assert_eq!(err, "the source already has an instance under this parent");
}

#[test]
fn frontend_initialization_rejects_any_existing_intent_and_preserves_old_encoding() {
    let original = RegistryDocument::default();
    assert!(
        !serde_json::to_value(&original)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("frontend_initialized")
    );
    for mut d in [
        RegistryDocument {
            catalog: Some(Default::default()),
            ..Default::default()
        },
        RegistryDocument {
            source_settings: Some(crate::topology_manager::settings::SourceSettings {
                selections: vec![],
            }),
            ..Default::default()
        },
        RegistryDocument {
            revision: 1,
            ..Default::default()
        },
        RegistryDocument {
            next_id: 2,
            ..Default::default()
        },
    ] {
        assert!(run(&d, json!({"kind":"topology.initialize"})).is_err());
        d.frontend_initialized = true;
        assert!(run(&d, json!({"kind":"topology.initialize"})).is_err());
    }
}

#[test]
fn node_location_sets_the_type_and_both_coordinates_in_one_edit() {
    let d = doc();
    let a = run(
        &d,
        json!({"kind": "node.location", "node_id": "group1", "geo_type": "ap",
        "location": {"latitude": "-12.236199459585354", "longitude": "122.02419129509887"}}),
    )
    .unwrap();
    let geo = node(&a.candidate, "group1").geo.as_ref().unwrap();
    assert_eq!(geo.geo_type, web_contract::topology::GeoType::Ap);
    let l = geo.location.as_ref().unwrap();
    assert_eq!(
        (l.latitude.as_str(), l.longitude.as_str()),
        ("-12.236199459585354", "122.02419129509887")
    );
    let typed = run(
        &a.candidate,
        json!({"kind": "node.location", "node_id": "group1",
        "geo_type": "site", "location": null}),
    )
    .unwrap();
    let geo = node(&typed.candidate, "group1").geo.as_ref().unwrap();
    assert_eq!(
        (geo.geo_type, geo.location.as_ref()),
        (web_contract::topology::GeoType::Site, None)
    );
    let cleared = run(
        &a.candidate,
        json!({"kind": "node.location", "node_id": "group1",
        "geo_type": null, "location": null}),
    )
    .unwrap();
    assert_eq!(node(&cleared.candidate, "group1").geo, None);
}

#[test]
fn node_location_refuses_bad_input_without_a_partial_change() {
    let d = doc();
    let edit = |node: &str, geo: Value, lat: &str, lon: &str| {
        json!({"kind": "node.location", "node_id": node, "geo_type": geo,
               "location": {"latitude": lat, "longitude": lon}})
    };
    assert!(refused(&d, edit("group1", json!("site"), "95", "1")).contains("latitude"));
    assert!(refused(&d, edit("group1", json!(null), "1", "1")).contains("Site or AP"));
    assert!(refused(&d, edit("nope", json!("site"), "1", "1")).contains("node"));
}

#[test]
fn group_allocation_sets_count_mode_and_refuses_unknown_sources_and_modes() {
    let d = doc();
    let a = run(
        &d,
        json!({"kind": "group.allocation", "source_id": "profile-source", "mode": "count"}),
    )
    .unwrap();
    let source = a
        .candidate
        .catalog
        .as_ref()
        .unwrap()
        .sources
        .iter()
        .find(|s| s.id == "profile-source")
        .unwrap();
    let SourceOrigin::Subscribers { group, .. } = &source.origin else {
        panic!("not a subscriber source")
    };
    assert_eq!(group.allocation, catalog::AllocationMode::Count);
    assert!(
        refused(
            &d,
            json!({"kind": "group.allocation", "source_id": "wan-source-1", "mode": "count"})
        )
        .contains("source")
    );
    assert!(
        serde_json::from_value::<Edit>(
            json!({"kind": "group.allocation", "source_id": "profile-source", "mode": "quality"})
        )
        .is_err()
    );
}

#[test]
fn a_fixed_capacity_on_a_shared_source_placement_is_refused() {
    let d = doc();
    let fixed = json!({"mode": "fixed", "down_mbps": 300, "up_mbps": 100, "factor_percent": null});
    let why = refused(
        &d,
        json!({"kind": "node.update", "node_id": "group1", "capacity": fixed}),
    );
    assert!(why.contains("shared limit"), "{why}");
    // One placement only: a fixed capacity is one real limit.
    let mut single = doc();
    single
        .catalog
        .as_mut()
        .unwrap()
        .nodes
        .retain(|n| n.id != "group2");
    if let Some(s) = single
        .catalog
        .as_mut()
        .unwrap()
        .sources
        .iter_mut()
        .find(|s| s.id == "profile-source")
    {
        s.targets.retain(|t| t != "group2");
    }
    assert!(
        run(
            &single,
            json!({"kind": "node.update", "node_id": "group1", "capacity": fixed})
        )
        .is_ok()
    );
}

/// `doc()` with one empty intermediate node `Tower` below `wan1`. Returns its ID.
fn with_tower(d: &mut RegistryDocument) -> String {
    let next = run(
        d,
        json!({"kind": "node.create", "name": "Tower", "parent": "wan1", "capacity": {"mode": "inherit"}}),
    )
    .unwrap()
    .candidate;
    let id = new_node(d, &next).id.clone();
    *d = next;
    id
}

#[test]
fn node_router_edit_sets_and_clears_the_router() {
    let mut d = doc();
    let tower = with_tower(&mut d);
    let set = run(
        &d,
        json!({"kind": "node.router", "node_id": tower, "router_id": "distro"}),
    )
    .unwrap()
    .candidate;
    assert_eq!(node(&set, &tower).router_id.as_deref(), Some("distro"));
    let clear = run(
        &set,
        json!({"kind": "node.router", "node_id": tower, "router_id": null}),
    )
    .unwrap()
    .candidate;
    assert_eq!(node(&clear, &tower).router_id, None);
    assert!(
        refused(
            &d,
            json!({"kind": "node.router", "node_id": "wan1", "router_id": "core"})
        )
        .contains("router node")
    );
}

#[test]
fn source_of_router_a_is_refused_in_the_branch_of_router_b() {
    let mut d = doc();
    let tower = with_tower(&mut d);
    let core = run(
        &d,
        json!({"kind": "node.router", "node_id": tower, "router_id": "core"}),
    )
    .unwrap()
    .candidate;
    // group1 holds `profile-source` of router `distro`.
    let error = refused(
        &core,
        json!({"kind": "node.move", "node_id": "group1", "parent": tower}),
    );
    assert!(error.contains("another router"), "{error}");
    // The same check refuses a router node above an existing source of another router.
    let moved = run(
        &d,
        json!({"kind": "node.move", "node_id": "group1", "parent": tower}),
    )
    .unwrap()
    .candidate;
    let error = refused(
        &moved,
        json!({"kind": "node.router", "node_id": tower, "router_id": "core"}),
    );
    assert!(error.contains("another router"), "{error}");
}

#[test]
fn router_node_of_an_absent_router_is_refused_on_edit_but_loads() {
    let mut d = doc();
    let tower = with_tower(&mut d);
    let mut d = run(
        &d,
        json!({"kind": "node.router", "node_id": tower, "router_id": "distro"}),
    )
    .unwrap()
    .candidate;
    let c = d.catalog.as_mut().unwrap();
    c.nodes
        .iter_mut()
        .find(|n| n.id == tower)
        .unwrap()
        .router_id = Some("gone".into());
    d.validate().unwrap();
    let error = refused(
        &d,
        json!({"kind": "node.update", "node_id": "wan1", "name": "WAN one"}),
    );
    assert!(error.contains("router node"), "{error}");
    // Clearing the router repairs the policy.
    run(
        &d,
        json!({"kind": "node.router", "node_id": tower, "router_id": null}),
    )
    .unwrap();
}

#[tokio::test(start_paused = true)]
async fn auto_read_adds_each_unconfigured_protocol_without_nodes() {
    use crate::topology_manager::policy_draft::{propose, read_catalog, tests as scenario};
    use crate::topology_manager::settings::SourceSettings;
    let (cfg, snap) = scenario::scenario();
    let draft = propose(&cfg, &[scenario::current(&cfg, &snap).await]).unwrap();
    let empty = RegistryDocument {
        catalog: Some(read_catalog(&cfg)),
        source_settings: Some(SourceSettings { selections: vec![] }),
        frontend_initialized: true,
        ..Default::default()
    };
    let next = auto_read(&empty, &draft)
        .unwrap()
        .expect("unconfigured protocols");
    next.validate().unwrap();
    let c = next.catalog.as_ref().unwrap();
    assert!(c.nodes.is_empty());
    assert!(c.sources.iter().all(|s| s.targets.is_empty()));
    let mut expected = draft.source_settings.selections.clone();
    expected.sort_by(|a, b| a.scope.cmp(&b.scope));
    assert_eq!(next.source_settings.as_ref().unwrap().selections, expected);
    assert_eq!(c.sources.len(), draft.catalog.sources.len());
    // Nothing is missing now.
    assert!(auto_read(&next, &draft).unwrap().is_none());
    // A protocol with one saved selection keeps the operator's choice.
    let mut partial = next.clone();
    let settings = partial.source_settings.as_mut().unwrap();
    let dhcp = settings
        .selections
        .iter()
        .position(|s| s.scope.kind == crate::topology_manager::model::SourceKind::Dhcp)
        .unwrap();
    let removed = settings.selections.remove(dhcp).scope;
    partial.catalog.as_mut().unwrap().sources.retain(|s| {
        !matches!(&s.origin, catalog::SourceOrigin::Subscribers { scope, .. } if scope == &removed)
    });
    assert!(auto_read(&partial, &draft).unwrap().is_none());
}
