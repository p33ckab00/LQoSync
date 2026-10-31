//! Topology: parent names, group nodes, root nodes, and the nested network.json.
//!
//! Each node gets a stable `id` that does not depend on its name (ADR-0005).

use crate::config::{BackstopCap, Config, DhcpMode, NetworkMode, NodeTemplate};
use crate::model::{CsvRow, StoredNode};
use crate::sources::GroupKey;
use crate::speed::{Rate, clamp};
use anyhow::bail;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap};

/// FNV-1a 64-bit hash. Stable across builds and platforms.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Stable node ID: `lq3-` and 16 hex characters.
pub fn node_id(kind: &str, router: &str, key: &str) -> String {
    format!(
        "lq3-{:016x}",
        fnv1a64(format!("{kind}|{router}|{key}").as_bytes())
    )
}

/// Node ID of the `client` node of one circuit (`flat_promotion`).
pub fn client_node_id(circuit_id: &str) -> String {
    format!("lq3-c-{circuit_id}")
}

fn kind_key(group: &GroupKey) -> (&'static str, &str) {
    match group {
        GroupKey::Ppp => ("ppp", ""),
        GroupKey::PppPlan(p) => ("ppp_plan", p),
        GroupKey::Hotspot => ("hotspot", ""),
        GroupKey::DhcpSite(s) => ("dhcp_site", s),
        GroupKey::DhcpPlan(l) => ("dhcp_plan", l),
    }
}

pub fn group_node_id(router: &str, group: &GroupKey) -> String {
    let (kind, key) = kind_key(group);
    node_id(kind, router, key)
}

/// The template and the v2 default type of a group node.
fn template<'a>(cfg: &'a Config, group: &GroupKey) -> (&'a NodeTemplate, &'static str) {
    let n = &cfg.nodes;
    match group {
        GroupKey::Ppp => (&n.ppp, "plan"),
        GroupKey::PppPlan(_) => (&n.ppp_plan, "plan"),
        GroupKey::Hotspot => (&n.hotspot, "site"),
        GroupKey::DhcpSite(_) => (&n.dhcp_site, "site"),
        GroupKey::DhcpPlan(_) => (&n.dhcp_plan, "plan"),
    }
}

/// Fill the `[nodes]` name template of a group node.
pub fn group_node_name(cfg: &Config, router: &str, group: &GroupKey) -> String {
    let (kind, key) = kind_key(group);
    let field = match kind {
        "ppp_plan" => "{profile}",
        "dhcp_site" => "{server}",
        "dhcp_plan" => "{plan}",
        _ => "{none}",
    };
    template(cfg, group)
        .0
        .name
        .replace("{router}", router)
        .replace(field, key)
}

fn is_grouped(mode: NetworkMode) -> bool {
    matches!(
        mode,
        NetworkMode::GroupedRouterRoot | NetworkMode::RootPromotion
    )
}

/// `(Parent Node, Parent Node ID)` of a circuit. `flat_promotion` is set by the cycle.
pub fn parent_for(cfg: &Config, router: &str, group: &GroupKey) -> (String, String) {
    match cfg.network.mode {
        m if is_grouped(m) => (
            group_node_name(cfg, router, group),
            group_node_id(router, group),
        ),
        NetworkMode::FlatRouterRoot => (router.to_string(), node_id("root", router, "")),
        NetworkMode::FlatGlobalRoot => (cfg.network.root_name.clone(), node_id("root", "", "")),
        _ => (String::new(), String::new()),
    }
}

/// Count and rate sums of one group, from the kept circuits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupTotal {
    pub count: usize,
    pub sum: Rate,
    /// The base rate of the first circuit in the group.
    pub unit: Rate,
}

/// The groups of one router that is online in this cycle.
#[derive(Debug, Clone, PartialEq)]
pub struct RouterGroups {
    pub router: String,
    pub ppp_enabled: bool,
    pub per_plan_node: bool,
    pub totals: BTreeMap<GroupKey, GroupTotal>,
}

fn group_rate(cfg: &Config, router: &str, group: &GroupKey, t: &GroupTotal) -> Rate {
    let bd = &cfg.bandwidth_defaults;
    let rcfg = cfg.routers.iter().find(|r| r.name == router);
    let dhcp_factors = |pick: &dyn Fn(&crate::config::DhcpServer) -> bool| {
        rcfg.and_then(|r| r.dhcp.servers.iter().find(|s| pick(s)))
            .map_or((bd.dhcp_download_factor, bd.dhcp_upload_factor), |s| {
                s.factors(bd)
            })
    };
    let (raw, (fd, fu), cap): (Rate, (f64, f64), &BackstopCap) = match group {
        GroupKey::Ppp | GroupKey::PppPlan(_) => (
            t.sum,
            (bd.pppoe_aggregate_factor, bd.pppoe_aggregate_factor),
            &cfg.backstop_caps.pppoe,
        ),
        GroupKey::Hotspot => {
            let f = rcfg.map_or(
                (bd.hotspot_download_factor, bd.hotspot_upload_factor),
                |r| r.hotspot.factors(bd),
            );
            (count_rate(t), f, &cfg.backstop_caps.hotspot)
        }
        GroupKey::DhcpSite(name) => (
            count_rate(t),
            dhcp_factors(&|s| &s.name == name),
            &cfg.backstop_caps.dhcp,
        ),
        // shortcut: one plan node can merge leases of several per_plan servers; it uses the factors of the first one.
        GroupKey::DhcpPlan(_) => (
            t.sum,
            dhcp_factors(&|s| s.mode == DhcpMode::PerPlan),
            &cfg.backstop_caps.dhcp,
        ),
    };
    let mut rate = Rate {
        down: raw.down * fd,
        up: raw.up * fu,
    };
    if let Some(c) = cap.limit() {
        rate = Rate {
            down: rate.down.min(c.down),
            up: rate.up.min(c.up),
        };
    }
    Rate {
        down: clamp(rate.down, bd.safe_min_mbps),
        up: clamp(rate.up, bd.safe_min_mbps),
    }
}

/// `count × unit`, from left to right.
fn count_rate(t: &GroupTotal) -> Rate {
    Rate {
        down: t.count as f64 * t.unit.down,
        up: t.count as f64 * t.unit.up,
    }
}

/// Insert a node that this cycle makes. Two rules that give the same node name are a config
/// error (for example a PPP profile "HS" and the Hotspot node "HS-<router>").
fn put(
    nodes: &mut BTreeMap<String, StoredNode>,
    made: &mut HashMap<String, (String, String)>,
    name: String,
    kind: &str,
    node: StoredNode,
) -> anyhow::Result<()> {
    if let Some((other, id)) = made.get(&name)
        && *id != node.id
    {
        bail!(
            "network.json: node name \"{name}\" comes from two rules, {other} and {kind}; \
             change a [nodes] template, or rename the PPP profile or DHCP server"
        );
    }
    made.insert(name.clone(), (kind.to_string(), node.id.clone()));
    nodes.insert(name, node);
    Ok(())
}

/// Make all nodes of the cycle. `carried` holds the old nodes of routers that are not online.
pub fn build_nodes(
    cfg: &Config,
    v3: bool,
    online: &[RouterGroups],
    carried: &BTreeMap<String, StoredNode>,
    rows: &[CsvRow],
) -> anyhow::Result<BTreeMap<String, StoredNode>> {
    let net = &cfg.network;
    let mut nodes = carried.clone();
    let mut made: HashMap<String, (String, String)> = HashMap::new();
    let root = |id: String, router: Option<String>| StoredNode {
        id,
        router,
        parent: None,
        r#type: net.root_type.clone(),
        down: net.root_download_mbps,
        up: net.root_upload_mbps,
        seeded: false,
    };
    match net.mode {
        NetworkMode::GroupedRouterRoot | NetworkMode::FlatRouterRoot => {
            for r in &cfg.routers {
                let n = root(node_id("root", &r.name, ""), Some(r.name.clone()));
                put(&mut nodes, &mut made, r.name.clone(), "root", n)?;
            }
        }
        NetworkMode::FlatGlobalRoot => {
            let n = root(node_id("root", "", ""), None);
            put(&mut nodes, &mut made, net.root_name.clone(), "root", n)?;
        }
        NetworkMode::FlatPromotion => {
            for row in rows {
                let max = |s: &str| {
                    s.trim()
                        .parse::<f64>()
                        .unwrap_or(cfg.bandwidth_defaults.max_floor_mbps)
                };
                put(
                    &mut nodes,
                    &mut made,
                    row.circuit_name.clone(),
                    "client",
                    StoredNode {
                        id: client_node_id(&row.circuit_id),
                        router: None,
                        parent: None,
                        r#type: cfg.nodes.client.r#type.clone(),
                        down: max(&row.download_max),
                        up: max(&row.upload_max),
                        seeded: false,
                    },
                )?;
            }
        }
        NetworkMode::NoParent | NetworkMode::RootPromotion => {}
    }
    if !is_grouped(net.mode) {
        return Ok(nodes);
    }
    let parent_of =
        |router: &str| (net.mode == NetworkMode::GroupedRouterRoot).then(|| router.to_string());
    for g in online {
        let mut groups: Vec<(GroupKey, GroupTotal)> =
            g.totals.iter().map(|(k, t)| (k.clone(), *t)).collect();
        if g.ppp_enabled && !g.per_plan_node && !g.totals.contains_key(&GroupKey::Ppp) {
            let zero = Rate { down: 0.0, up: 0.0 };
            groups.push((
                GroupKey::Ppp,
                GroupTotal {
                    count: 0,
                    sum: zero,
                    unit: zero,
                },
            ));
        }
        for (key, t) in groups {
            let rate = group_rate(cfg, &g.router, &key, &t);
            let (tpl, v2_type) = template(cfg, &key);
            put(
                &mut nodes,
                &mut made,
                group_node_name(cfg, &g.router, &key),
                kind_key(&key).0,
                StoredNode {
                    id: group_node_id(&g.router, &key),
                    router: Some(g.router.clone()),
                    parent: parent_of(&g.router),
                    r#type: tpl.type_for(v2_type, v3),
                    down: rate.down,
                    up: rate.up,
                    seeded: false,
                },
            )?;
        }
    }
    Ok(nodes)
}

fn node_json(n: &StoredNode, children: Map<String, Value>) -> Value {
    json!({
        "children": Value::Object(children),
        "downloadBandwidthMbps": n.down,
        "id": n.id,
        "type": n.r#type,
        "uploadBandwidthMbps": n.up,
    })
}

/// Remove the named nodes at all levels of a network.json tree.
fn remove_named(tree: &mut Map<String, Value>, names: &BTreeMap<String, StoredNode>) {
    tree.retain(|k, _| !names.contains_key(k));
    for v in tree.values_mut() {
        if let Some(Value::Object(kids)) = v.get_mut("children") {
            remove_named(kids, names);
        }
    }
}

/// Take the children of a node out of a map, if the node is there.
fn take_children(map: &mut Map<String, Value>, name: &str) -> Map<String, Value> {
    match map.remove(name) {
        Some(Value::Object(mut old)) => match old.remove("children") {
            Some(Value::Object(k)) => k,
            _ => Map::new(),
        },
        _ => Map::new(),
    }
}

/// Find a node by name at any level.
fn find_mut<'a>(tree: &'a mut Map<String, Value>, name: &str) -> Option<&'a mut Value> {
    if tree.contains_key(name) {
        return tree.get_mut(name);
    }
    tree.values_mut().find_map(|v| match v.get_mut("children") {
        Some(Value::Object(kids)) => find_mut(kids, name),
        _ => None,
    })
}

/// Make the nested network.json. With `preserved`, keep the operator nodes and
/// replace only the nodes that the tool made in the last cycle.
pub fn to_network_json(
    nodes: &BTreeMap<String, StoredNode>,
    preserved: Option<Value>,
    prev_generated: &BTreeMap<String, StoredNode>,
) -> Value {
    let mut tree = match preserved {
        Some(Value::Object(m)) => m,
        _ => Map::new(),
    };
    // Remove the old generated nodes that this cycle does not make again.
    // A node that stays is updated in place, so the operator children under it stay.
    let stale: BTreeMap<String, StoredNode> = prev_generated
        .iter()
        .filter(|(k, v)| !v.seeded && !nodes.contains_key(*k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    remove_named(&mut tree, &stale);
    // Roots first, so that each child finds its parent.
    for (name, n) in nodes.iter().filter(|(_, n)| n.parent.is_none()) {
        let kids = take_children(&mut tree, name);
        tree.insert(name.clone(), node_json(n, kids));
    }
    for (name, n) in nodes.iter().filter(|(_, n)| n.parent.is_some()) {
        let parent = n.parent.as_deref().unwrap_or_default();
        match find_mut(&mut tree, parent).and_then(|p| p.get_mut("children")) {
            Some(Value::Object(kids)) => {
                // A node that is already there keeps its operator children.
                let old = take_children(kids, name);
                kids.insert(name.clone(), node_json(n, old));
            }
            _ => {
                tree.insert(name.clone(), node_json(n, Map::new()));
            }
        }
    }
    Value::Object(tree)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::model::{CsvRow, StoredNode};
    use crate::sources::GroupKey;
    use crate::speed::Rate;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn cfg(mode: &str, extra: &str) -> Config {
        Config::from_toml_str(&format!(
            r#"[network]
mode = "{mode}"
[bandwidth_defaults]
pppoe_aggregate_factor = 0.35
{extra}
[[routers]]
name = "R1"
address = "192.0.2.1"
username = "u"
password_file = "/x"
[routers.pppoe]
enabled = true
per_plan_node = true
[routers.dhcp]
enabled = true
[[routers.dhcp.servers]]
name = "LAN"
download_factor = 1.0
upload_factor = 1.0
"#
        ))
        .unwrap()
    }
    fn r(v: f64) -> Rate {
        Rate { down: v, up: v }
    }
    fn groups() -> Vec<RouterGroups> {
        let mut totals = BTreeMap::new();
        totals.insert(
            GroupKey::PppPlan("Gold".into()),
            GroupTotal {
                count: 2,
                sum: r(100.0),
                unit: r(50.0),
            },
        );
        totals.insert(
            GroupKey::PppPlan("Silver".into()),
            GroupTotal {
                count: 1,
                sum: r(20.0),
                unit: r(20.0),
            },
        );
        totals.insert(
            GroupKey::DhcpSite("LAN".into()),
            GroupTotal {
                count: 4,
                sum: r(200.0),
                unit: r(50.0),
            },
        );
        vec![RouterGroups {
            router: "R1".into(),
            ppp_enabled: true,
            per_plan_node: true,
            totals,
        }]
    }
    fn id(kind: &str, key: &str) -> String {
        node_id(kind, "R1", key)
    }
    fn none() -> BTreeMap<String, StoredNode> {
        BTreeMap::new()
    }
    fn net(c: &Config, v3: bool, g: &[RouterGroups]) -> serde_json::Value {
        to_network_json(&build_nodes(c, v3, g, &none(), &[]).unwrap(), None, &none())
    }
    fn groups_json() -> serde_json::Value {
        json!({
          "DHCP-LAN-R1": {"children":{},"downloadBandwidthMbps":200.0,"id":id("dhcp_site","LAN"),"type":"site","uploadBandwidthMbps":200.0},
          "Gold-R1":     {"children":{},"downloadBandwidthMbps":35.0,"id":id("ppp_plan","Gold"),"type":"plan","uploadBandwidthMbps":35.0},
          "Silver-R1":   {"children":{},"downloadBandwidthMbps":7.0,"id":id("ppp_plan","Silver"),"type":"plan","uploadBandwidthMbps":7.0}})
    }

    #[test]
    fn fnv_and_id_format() {
        assert_eq!(fnv1a64(b""), 0xcbf29ce484222325);
        assert_eq!(fnv1a64(b"a"), 0xaf63dc4c8601ec8c);
        let i = node_id("ppp_plan", "R1", "Gold");
        assert!(
            i.len() == 20 && i.starts_with("lq3-") && i[4..].chars().all(|c| c.is_ascii_hexdigit())
        );
        assert_eq!(client_node_id("C1"), "lq3-c-C1");
    }

    #[test]
    fn id_does_not_change_with_the_name_template() {
        let c = cfg(
            "root_promotion",
            "[nodes]\nppp_plan = { name = \"{profile}@{router}\" }",
        );
        let v = net(&c, false, &groups());
        assert_eq!(v["Gold@R1"]["id"], json!(id("ppp_plan", "Gold")));
    }

    #[test]
    fn root_promotion_puts_groups_at_top() {
        assert_eq!(
            net(&cfg("root_promotion", ""), false, &groups()),
            groups_json()
        );
    }

    #[test]
    fn grouped_router_root_nests_groups_under_router() {
        let v = net(&cfg("grouped_router_root", ""), false, &groups());
        assert_eq!(
            v,
            json!({"R1": {"children": groups_json(), "downloadBandwidthMbps": 640.0, "id": id("root", ""),
                          "type": "site", "uploadBandwidthMbps": 640.0}})
        );
    }

    #[test]
    fn flat_router_root_and_flat_global_root_make_roots_only() {
        let v = net(&cfg("flat_router_root", ""), false, &groups());
        assert_eq!(
            v,
            json!({"R1": {"children": {}, "downloadBandwidthMbps": 640.0, "id": id("root", ""), "type": "site", "uploadBandwidthMbps": 640.0}})
        );
        let v = net(&cfg("flat_global_root", ""), false, &groups());
        assert_eq!(
            v,
            json!({"LibreQoS-Network": {"children": {}, "downloadBandwidthMbps": 640.0, "id": node_id("root", "", ""),
                                        "type": "site", "uploadBandwidthMbps": 640.0}})
        );
    }

    #[test]
    fn no_parent_gives_empty_object() {
        assert_eq!(net(&cfg("no_parent", ""), false, &groups()), json!({}));
    }

    #[test]
    fn flat_promotion_gives_one_client_node_per_row() {
        let c = cfg("flat_promotion", "");
        let rows = [CsvRow {
            circuit_id: "C1".into(),
            circuit_name: "ana".into(),
            download_max: "50".into(),
            upload_max: "10".into(),
            ..Default::default()
        }];
        let v = to_network_json(
            &build_nodes(&c, false, &groups(), &none(), &rows).unwrap(),
            None,
            &none(),
        );
        assert_eq!(
            v,
            json!({"ana": {"children": {}, "downloadBandwidthMbps": 50.0, "id": "lq3-c-C1", "type": "client", "uploadBandwidthMbps": 10.0}})
        );
    }

    #[test]
    fn ppp_node_exists_with_zero_users() {
        let c = cfg("root_promotion", "");
        let g = [RouterGroups {
            router: "R1".into(),
            ppp_enabled: true,
            per_plan_node: false,
            totals: BTreeMap::new(),
        }];
        assert_eq!(
            net(&c, false, &g),
            json!({"PPP-R1": {"children": {}, "downloadBandwidthMbps": 0.128, "id": id("ppp", ""), "type": "plan", "uploadBandwidthMbps": 0.128}})
        );
    }

    #[test]
    fn backstop_cap_limits_group() {
        let c = cfg(
            "root_promotion",
            "[backstop_caps.pppoe]\nenabled = true\ndownload_mbps = 30\nupload_mbps = 30",
        );
        assert_eq!(
            net(&c, false, &groups())["Gold-R1"]["downloadBandwidthMbps"],
            json!(30.0)
        );
    }

    #[test]
    fn v3_flag_makes_group_nodes_site() {
        let v = net(&cfg("root_promotion", ""), true, &groups());
        assert_eq!(v["Gold-R1"]["type"], json!("site"));
        assert_eq!(v["Silver-R1"]["type"], json!("site"));
    }

    #[test]
    fn carried_nodes_of_failed_router_stay() {
        let c = cfg("root_promotion", "");
        let mut carried = none();
        let old = StoredNode {
            id: "lq3-old".into(),
            router: Some("R2".into()),
            parent: None,
            r#type: "plan".into(),
            down: 9.0,
            up: 9.0,
            seeded: false,
        };
        carried.insert("Old-R2".into(), old.clone());
        let nodes = build_nodes(&c, false, &groups(), &carried, &[]).unwrap();
        assert_eq!(nodes["Old-R2"], old);
        assert_eq!(nodes.len(), 4);
    }

    #[test]
    fn preserve_keeps_manual_nodes_and_drops_old_generated() {
        let c = cfg("grouped_router_root", "");
        let preserved = json!({
            "Manual": {"children": {}, "downloadBandwidthMbps": 5.0, "type": "site", "uploadBandwidthMbps": 5.0},
            "R1": {"children": {"Keep": {"children": {}, "downloadBandwidthMbps": 1.0, "type": "ap", "uploadBandwidthMbps": 1.0},
                                "Old-R1": {"children": {}, "downloadBandwidthMbps": 2.0, "type": "plan", "uploadBandwidthMbps": 2.0}},
                   "downloadBandwidthMbps": 100.0, "type": "node", "uploadBandwidthMbps": 100.0}});
        let mut prev = none();
        prev.insert(
            "Old-R1".into(),
            StoredNode {
                id: "x".into(),
                router: Some("R1".into()),
                parent: Some("R1".into()),
                r#type: "plan".into(),
                down: 2.0,
                up: 2.0,
                seeded: false,
            },
        );
        // The router root was also made by the tool in the last cycle.
        prev.insert(
            "R1".into(),
            StoredNode {
                id: "y".into(),
                router: Some("R1".into()),
                parent: None,
                r#type: "site".into(),
                down: 640.0,
                up: 640.0,
                seeded: false,
            },
        );
        let v = to_network_json(
            &build_nodes(&c, false, &groups(), &none(), &[]).unwrap(),
            Some(preserved),
            &prev,
        );
        assert!(v["Manual"].is_object());
        let kids = v["R1"]["children"].as_object().unwrap();
        assert!(
            kids.contains_key("Keep")
                && kids.contains_key("Gold-R1")
                && !kids.contains_key("Old-R1")
        );
        assert_eq!(
            (
                v["R1"]["downloadBandwidthMbps"].clone(),
                v["R1"]["type"].clone()
            ),
            (json!(640.0), json!("site"))
        );
    }

    #[test]
    fn parent_for_each_mode() {
        let g = GroupKey::PppPlan("Gold".into());
        let p = |m: &str| parent_for(&cfg(m, ""), "R1", &g);
        assert_eq!(
            p("root_promotion"),
            ("Gold-R1".into(), id("ppp_plan", "Gold"))
        );
        assert_eq!(
            p("grouped_router_root"),
            ("Gold-R1".into(), id("ppp_plan", "Gold"))
        );
        assert_eq!(p("flat_router_root"), ("R1".into(), id("root", "")));
        assert_eq!(
            p("flat_global_root"),
            ("LibreQoS-Network".into(), node_id("root", "", ""))
        );
        assert_eq!(p("no_parent"), (String::new(), String::new()));
        assert_eq!(p("flat_promotion"), (String::new(), String::new()));
    }

    #[test]
    fn a_kept_child_keeps_its_operator_children() {
        let c = cfg("grouped_router_root", "");
        let preserved = json!({"R1": {"children": {"Mid": {"children": {"Leaf": {"children": {}, "type": "ap"}}, "type": "site"}}, "type": "site"}});
        let mut nodes = build_nodes(&c, false, &[], &none(), &[]).unwrap();
        nodes.insert(
            "Mid".into(),
            StoredNode {
                id: "lq3-mid".into(),
                router: None,
                parent: Some("R1".into()),
                r#type: "site".into(),
                down: 1.0,
                up: 1.0,
                seeded: true,
            },
        );
        let v = to_network_json(&nodes, Some(preserved), &none());
        assert!(
            v["R1"]["children"]["Mid"]["children"]["Leaf"].is_object(),
            "{v}"
        );
    }
}
