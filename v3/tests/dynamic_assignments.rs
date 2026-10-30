use lqosync_v3::{
    assignment::{self, AssignmentState},
    dynamic::{self, Target},
    model::Settings,
    normalizer,
    routeros::Inventory,
    sessions,
};
use std::{collections::BTreeMap, sync::Arc};
mod common;
pub fn fixture() -> (Settings, BTreeMap<String, Arc<Inventory>>) {
    let (mut s, mut i) = common::fixture();
    s.nodes[0].router = "r1".into();
    s.nodes[0].source = "wan-a".into();
    s = dynamic::migrate(&s).unwrap();
    s.nodes.truncate(1);
    let c = s.dynamic.as_mut().unwrap();
    let mut w = c.wans[0].clone();
    w.id = "wan2".into();
    w.interface = "wan-b".into();
    c.wans.push(w);
    for p in &mut c.policies {
        p.default = Target {
            node: "wan".into(),
            wan: "wan".into(),
        };
        p.targets = vec![
            p.default.clone(),
            Target {
                node: "wan".into(),
                wan: "wan2".into(),
            },
        ];
        p.balance = true;
    }
    let inv = Arc::make_mut(i.get_mut("r1").unwrap());
    inv.tables.insert(
        "/interface/print".into(),
        ["wan-a", "wan-b"]
            .map(|name| {
                [
                    ("name".into(), name.into()),
                    ("running".into(), "true".into()),
                    ("disabled".into(), "false".into()),
                ]
                .into_iter()
                .collect()
            })
            .to_vec(),
    );
    for path in ["/ppp/active/print", "/ip/hotspot/active/print"] {
        inv.tables.get_mut(path).unwrap()[0]
            .extend([(".id".into(), "*1".into()), ("uptime".into(), "1h".into())]);
    }
    (s, i)
}
#[test]
fn mixed_sources_share_a_flat_root_and_keep_independent_wan_assignments() {
    let (s, i) = fixture();
    let e = BTreeMap::new();
    let mut rows = normalizer::collect(&s, &i, &e, 101).unwrap();
    let first = sessions::plan(
        &s,
        &mut rows,
        &AssignmentState::default(),
        None,
        &i,
        &e,
        101,
    )
    .unwrap();
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|s| s.node == "wan"));
    assert_eq!(first.assignments.routes.len(), 3);
    assert_eq!(first.active.len(), 3);
    let a = first
        .assignments
        .routes
        .values()
        .filter(|r| r.wan == "wan")
        .count();
    assert!(a == 1 || a == 2);
    let mut next = normalizer::collect(&s, &i, &e, 101).unwrap();
    let retained = sessions::plan(
        &s,
        &mut next,
        &AssignmentState::default(),
        Some(&first),
        &i,
        &e,
        101,
    )
    .unwrap();
    assert_eq!(retained, first);
    let d = tempfile::tempdir().unwrap();
    sessions::save(d.path(), &first).unwrap();
    assert_eq!(sessions::load(d.path()).unwrap(), Some(first));
}
#[test]
fn default_and_pins_use_only_selected_pairs_without_an_original_service_node() {
    let (mut s, i) = fixture();
    let e = BTreeMap::new();
    for p in &mut s.dynamic.as_mut().unwrap().policies {
        p.balance = false;
        p.targets = vec![Target {
            node: "wan".into(),
            wan: "wan2".into(),
        }];
        p.default = p.targets[0].clone();
    }
    let mut rows = normalizer::collect(&s, &i, &e, 101).unwrap();
    let a = assignment::assign(&s, &mut rows, &AssignmentState::default(), &i, &e, 101).unwrap();
    assert_eq!(a.routes.len(), 3);
    assert!(a.routes.values().all(|r| r.wan == "wan2"));
}

#[test]
fn independent_roots_share_the_same_routing_wan_minimum_budget() {
    let (mut s, i) = fixture();
    let mut second = s.nodes[0].clone();
    second.id = "other-root".into();
    s.nodes.push(second);
    let c = s.dynamic.as_mut().unwrap();
    c.wans[0].download = 10.;
    c.wans[0].upload = 4.;
    for (index, p) in c.policies.iter_mut().enumerate() {
        p.balance = false;
        p.default = Target {
            node: if index == 0 { "wan" } else { "other-root" }.into(),
            wan: "wan".into(),
        };
        p.targets = vec![p.default.clone()];
    }
    let e = BTreeMap::new();
    let mut rows = normalizer::collect(&s, &i, &e, 101).unwrap();
    let a = assignment::assign(&s, &mut rows, &AssignmentState::default(), &i, &e, 101).unwrap();
    // Both roots consume one shared routing WAN budget.
    lqosync_v3::minimums::allocate_assigned(&s, &mut rows, &a).unwrap();
    assert!(
        rows.iter().map(|r| r.download_min).sum::<f64>() <= 10. + 1e-8,
        "WAN minimum budget must not be multiplied by root count"
    );
    assert!(rows.iter().map(|r| r.upload_min).sum::<f64>() <= 4. + 1e-8);
    assert_eq!(a.routes.len(), 3);
}
