use lqosync_v3::{
    model::*,
    routeros::{Inventory, Row},
};
use std::{collections::BTreeMap, sync::Arc};
fn row(fields: &[(&str, &str)]) -> Row {
    fields
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
fn node(id: &str, kind: &str, source: &str) -> Node {
    Node {
        id: id.into(),
        name: id.into(),
        parent: if kind == "wan" { "" } else { "wan" }.into(),
        kind: kind.into(),
        router: if kind == "wan" { "" } else { "r1" }.into(),
        source: source.into(),
        mode: "manual".into(),
        download: 1000.,
        upload: 1000.,
        download_factor: 1.,
        upload_factor: 1.,
    }
}
pub fn fixture() -> (Settings, BTreeMap<String, Arc<Inventory>>) {
    let router:Router=toml::from_str("id='r1'\nname='Router'\nhost='127.0.0.1'\nport=1\nusername='reader'\npassword_env='UNSET_TEST'\nrole='distro'\ntls=false\n").unwrap();
    let s = Settings {
        routers: vec![router],
        nodes: vec![
            node("wan", "wan", ""),
            node("ppp", "pppoe", "Plan"),
            node("dhcp", "dhcp", "dhcp1"),
            node("hs", "hotspot", "HS"),
        ],
        ..Default::default()
    };
    let tables = [
        (
            "/ppp/active/print",
            vec![row(&[("name", "alice"), ("address", "192.0.2.1")])],
        ),
        (
            "/ppp/secret/print",
            vec![row(&[("name", "alice"), ("profile", "Plan")])],
        ),
        (
            "/ppp/profile/print",
            vec![row(&[("name", "Plan"), ("rate-limit", "10M/50M")])],
        ),
        ("/ip/dhcp-server/print", vec![row(&[("name", "dhcp1")])]),
        (
            "/ip/dhcp-server/lease/print",
            vec![row(&[
                ("server", "dhcp1"),
                ("mac-address", "02:00:00:00:00:02"),
                ("address", "192.0.2.2"),
                ("status", "bound"),
                ("comment", "Client|20M/5M"),
                ("host-name", "DHCP client"),
            ])],
        ),
        (
            "/ip/hotspot/active/print",
            vec![row(&[
                ("user", "bob"),
                ("address", "192.0.2.3"),
                ("mac-address", "02:00:00:00:00:03"),
            ])],
        ),
        (
            "/ip/hotspot/user/print",
            vec![row(&[("name", "bob"), ("profile", "HS")])],
        ),
        (
            "/ip/hotspot/user/profile/print",
            vec![row(&[("name", "HS"), ("rate-limit", "2M/10M")])],
        ),
    ]
    .into_iter()
    .map(|(p, v)| (p.into(), v))
    .collect();
    let inv = Inventory {
        router_id: "r1".into(),
        collected_at: 100,
        tables,
        errors: BTreeMap::new(),
    };
    (s, [("r1".into(), Arc::new(inv))].into_iter().collect())
}
