use lqosync_v3::{
    assignment::{self, AssignmentState},
    core_plan::{self, Binding, Policy},
    model::{Settings, Subscriber},
    normalizer,
    routeros::{Inventory, Row},
};
use std::{collections::BTreeMap, sync::Arc};
pub const OWNER: &str = "80dff65e-3400-4c10-a702-46b19a9d8666";
pub fn row(fields: &[(&str, &str)]) -> Row {
    fields
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
pub fn fixture() -> (
    Settings,
    BTreeMap<String, Arc<Inventory>>,
    Vec<Subscriber>,
    AssignmentState,
) {
    let (mut s, mut inv) = super::common::fixture();
    s.routers[0].role = "both".into();
    s.nodes[0].router = "r1".into();
    s.nodes[0].source = "uplink".into();
    s.core_lists = Policy {
        owner: OWNER.into(),
        bindings: vec![Binding {
            wan: "wan".into(),
            router: "r1".into(),
            list: "lqos-wan".into(),
            table: "to-wan".into(),
        }],
    };
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    i.tables.extend([
        (
            "/system/resource/print".into(),
            vec![row(&[("version", "7.20.1 (stable)")])],
        ),
        (
            "/interface/print".into(),
            vec![row(&[
                ("name", "uplink"),
                ("running", "true"),
                ("disabled", "false"),
            ])],
        ),
        (
            "/routing/table/print".into(),
            vec![row(&[("name", "to-wan"), ("fib", "true")])],
        ),
        ("/routing/rule/print".into(), vec![]),
        ("/ip/firewall/filter/print".into(), vec![]),
        ("/ip/firewall/address-list/print".into(), vec![]),
        (
            "/ip/firewall/mangle/print".into(),
            vec![row(&[
                (".id", "*1"),
                ("chain", "prerouting"),
                ("action", "mark-routing"),
                ("src-address-list", "lqos-wan"),
                ("new-routing-mark", "to-wan"),
                ("passthrough", "false"),
                ("disabled", "false"),
                ("dst-address-type", "!local"),
            ])],
        ),
        (
            "/ip/route/print".into(),
            vec![row(&[
                ("dst-address", "0.0.0.0/0"),
                ("routing-table", "to-wan"),
                ("immediate-gw", "203.0.113.1%uplink"),
                ("active", "true"),
                ("disabled", "false"),
            ])],
        ),
    ]);
    let mut subs = normalizer::normalize(&s, &inv, &BTreeMap::new(), 101).unwrap();
    let state = assignment::assign(
        &s,
        &mut subs,
        &AssignmentState::default(),
        &inv,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    (s, inv, subs, state)
}
#[allow(dead_code)]
pub fn owned(id: &str, address: &str, subscriber: &str, wan: &str) -> Row {
    row(&[
        (".id", id),
        ("list", "lqos-wan"),
        ("address", address),
        ("comment", &core_plan::marker(OWNER, subscriber, wan)),
        ("disabled", "false"),
        ("dynamic", "false"),
    ])
}
