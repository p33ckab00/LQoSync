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
