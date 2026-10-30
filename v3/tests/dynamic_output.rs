use lqosync_v3::{
    assignment::{self, AssignmentState},
    core_plan::{self, Binding},
    dynamic::{self, Target},
    minimums, model, normalizer,
};
use std::{collections::BTreeMap, sync::Arc};
mod common;
#[path = "common/core.rs"]
mod core_fixture;
#[test]
fn a_flat_root_exports_one_tree_but_routes_each_client_to_its_assigned_core_list() {
    let (mut s, mut inventory, _, _) = core_fixture::fixture();
    s = dynamic::migrate(&s).unwrap();
    s.nodes.truncate(1);
    let c = s.dynamic.as_mut().unwrap();
    let mut second = c.wans[0].clone();
    second.id = "wan2".into();
    second.interface = "uplink2".into();
    c.wans.push(second);
    for p in &mut c.policies {
        p.targets = vec![
            Target {
                node: "wan".into(),
                wan: "wan".into(),
            },
            Target {
                node: "wan".into(),
                wan: "wan2".into(),
            },
        ];
        p.default = p.targets[0].clone();
        p.balance = true;
    }
    s.core_lists.bindings.push(Binding {
        wan: "wan2".into(),
        router: "r1".into(),
        list: "lqos-wan2".into(),
        table: "to-wan2".into(),
    });
    let inv = Arc::make_mut(inventory.get_mut("r1").unwrap());
    for (path, key, value) in [
        ("/interface/print", "name", "uplink2"),
        ("/routing/table/print", "name", "to-wan2"),
    ] {
        let mut r = inv.tables[path][0].clone();
        r.insert(key.into(), value.into());
        inv.tables.get_mut(path).unwrap().push(r);
    }
    let mut rule = inv.tables["/ip/firewall/mangle/print"][0].clone();
    rule.insert(".id".into(), "*2".into());
    rule.insert("src-address-list".into(), "lqos-wan2".into());
    rule.insert("new-routing-mark".into(), "to-wan2".into());
    inv.tables
        .get_mut("/ip/firewall/mangle/print")
        .unwrap()
        .push(rule);
    let mut route = inv.tables["/ip/route/print"][0].clone();
    route.insert("routing-table".into(), "to-wan2".into());
    route.insert("immediate-gw".into(), "203.0.113.2%uplink2".into());
    inv.tables.get_mut("/ip/route/print").unwrap().push(route);
    for path in ["/ppp/active/print", "/ip/hotspot/active/print"] {
        inv.tables.get_mut(path).unwrap()[0].insert(".id".into(), "*1".into());
    }
    let errors = BTreeMap::new();
    let mut rows = normalizer::collect(&s, &inventory, &errors, 101).unwrap();
    let assignments = assignment::assign(
        &s,
        &mut rows,
        &AssignmentState::default(),
        &inventory,
        &errors,
        101,
    )
    .unwrap();
    minimums::allocate_assigned(&s, &mut rows, &assignments).unwrap();
    let output = model::compile_assigned(&s, &rows, &assignments).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output.network).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 1);
    assert_eq!(json["wan"]["children"], serde_json::json!({}));
    for row in csv::Reader::from_reader(output.csv.as_bytes()).records() {
        assert_eq!(&row.unwrap()[4], "wan");
    }
    let plan = core_plan::build(&s, &rows, &assignments, &inventory, &errors, 101).unwrap();
    assert_eq!(plan.entries.len(), 3);
    for e in &plan.entries {
        assert_eq!(e.node, "wan");
        assert_eq!(e.wan, assignments.routes[&e.subscriber].wan);
        assert_eq!(
            e.list,
            if e.wan == "wan" {
                "lqos-wan"
            } else {
                "lqos-wan2"
            }
        );
    }
    assert!(plan.entries.iter().any(|e| e.wan == "wan2"));
    let d = tempfile::tempdir().unwrap();
    let review = lqosync_v3::validation::review(
        &s,
        &inventory,
        &errors,
        d.path(),
        101,
        lqosync_v3::core_diagnostics::Origin::Live,
    );
    for code in [
        "inventory-health",
        "subscriber-assignment",
        "subscriber-minimums",
        "shaping-capacity",
        "core-membership",
    ] {
        let check = review.checks.iter().find(|c| c.code == code).unwrap();
        assert_eq!(
            format!("{:?}", check.status),
            "Pass",
            "{code}: {}",
            check.detail
        );
    }
    let entry = &plan.entries[0];
    Arc::make_mut(inventory.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/firewall/address-list/print")
        .unwrap()
        .push(core_fixture::row(&[
            (".id", "*A"),
            ("list", &entry.list),
            ("address", &entry.address),
            ("comment", "manual"),
        ]));
    assert!(
        core_plan::build(&s, &rows, &assignments, &inventory, &errors, 101)
            .unwrap_err()
            .to_string()
            .contains("protected")
    );
}
