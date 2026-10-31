//! One sync cycle: merge the router data with the state, remove, guard, validate, and make nodes.
//!
//! The cycle is a pure function. It does no file, network, clock, or random access.

use crate::config::{Config, NetworkMode, OfflineAction};
use crate::model::{CsvRow, GuardHold, RouterSnapshot, Source, State, StoredCircuit, StoredNode};
use crate::sources::{GroupKey, collect_router};
use crate::speed::{Rate, SpeedParser, circuit_rates, fmt_rate};
use crate::topology::{
    GroupTotal, RouterGroups, build_nodes, client_node_id, fnv1a64, parent_for, to_network_json,
};
use crate::validate;
use anyhow::bail;
use std::collections::{BTreeMap, HashMap, HashSet};

/// The result of the reads from one router in this cycle.
#[derive(Debug, Clone)]
pub enum RouterResult {
    Online(RouterSnapshot),
    Failed,
    Unreachable,
    Offline,
}

/// What changed in the cycle.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    pub added: Vec<String>,
    /// `(circuit, [(column, old value, new value)])`
    pub updated: Vec<(String, Vec<Change>)>,
    pub removed: Vec<String>,
    /// Rows that the removal guard kept.
    pub held: Vec<String>,
    /// `(ip, kept circuit, dropped circuit)`.
    pub duplicates: Vec<(String, String, String)>,
    pub warnings: Vec<String>,
    pub alerts: Vec<String>,
}

pub struct CycleInput<'a> {
    pub config: &'a Config,
    pub state: &'a State,
    pub static_rows: &'a [CsvRow],
    /// One result for each configured router, in config order.
    pub routers: Vec<(String, RouterResult)>,
    pub current_network: Option<serde_json::Value>,
    pub v3: bool,
    /// A manual "sync now": it releases a guard hold.
    pub force: bool,
    pub now: u64,
}

#[derive(Debug, Clone)]
pub struct CycleOutput {
    pub rows: Vec<CsvRow>,
    pub network: serde_json::Value,
    pub state: State,
    pub report: Report,
}

/// One row while the cycle works on it.
struct Entry {
    row: CsvRow,
    router: Option<String>,
    source: Source,
    last_seen: u64,
    existed_before: bool,
    collected_now: bool,
}

/// One changed column: `(column, old value, new value)`.
pub type Change = (String, String, String);

/// The changed fields of a row, in CSV column order. A rate is compared as a number ("25.0" = "25").
fn changed_columns(old: &CsvRow, new: &CsvRow) -> Vec<Change> {
    let pairs = [
        ("Device Name", &old.device_name, &new.device_name, false),
        ("Parent Node", &old.parent_node, &new.parent_node, false),
        (
            "Parent Node ID",
            &old.parent_node_id,
            &new.parent_node_id,
            false,
        ),
        ("MAC", &old.mac, &new.mac, false),
        ("IPv4", &old.ipv4, &new.ipv4, false),
        (
            "Download Min Mbps",
            &old.download_min,
            &new.download_min,
            true,
        ),
        ("Upload Min Mbps", &old.upload_min, &new.upload_min, true),
        (
            "Download Max Mbps",
            &old.download_max,
            &new.download_max,
            true,
        ),
        ("Upload Max Mbps", &old.upload_max, &new.upload_max, true),
        ("Comment", &old.comment, &new.comment, false),
    ];
    let same = |a: &str, b: &str, rate: bool| {
        a == b
            || (rate
                && matches!((a.trim().parse::<f64>(), b.trim().parse::<f64>()), (Ok(x), Ok(y)) if x == y))
    };
    pairs
        .iter()
        .filter(|(_, a, b, rate)| !same(a, b, *rate))
        .map(|(k, a, b, _)| (k.to_string(), a.to_string(), b.to_string()))
        .collect()
}

/// Run one cycle. `new_id` gives a random ID; the cycle asks again on a collision.
pub fn assemble(
    input: CycleInput,
    new_id: &mut dyn FnMut() -> String,
) -> anyhow::Result<CycleOutput> {
    let cfg = input.config;
    let mode = cfg.network.mode;
    if input.v3 && mode == NetworkMode::NoParent {
        bail!("network.mode \"no_parent\" does not work with LibreQoS v3; use \"root_promotion\"");
    }
    let bd = &cfg.bandwidth_defaults;
    let speed = SpeedParser::new(&cfg.speed)?;
    let mut report = Report::default();

    // Rule 1: state rows. Static rows stay a list: a static circuit can have several device rows.
    let mut statics: Vec<CsvRow> = input.static_rows.to_vec();
    let mut rows: BTreeMap<String, Entry> = BTreeMap::new();
    for (name, c) in &input.state.circuits {
        rows.insert(
            name.clone(),
            Entry {
                row: c.row.clone(),
                router: c.router.clone(),
                source: c.source,
                last_seen: c.last_seen,
                existed_before: true,
                collected_now: false,
            },
        );
    }
    // A static row replaces a state row with the same name.
    for s in &statics {
        rows.remove(&s.circuit_name);
    }

    // Rule 2: IP index and IDs in use.
    let mut ip_index: HashMap<String, String> = HashMap::new();
    let mut used_ids: HashSet<String> = HashSet::new();
    for r in statics.iter().chain(rows.values().map(|e| &e.row)) {
        if !r.ipv4.is_empty() {
            ip_index
                .entry(r.ipv4.clone())
                .or_insert_with(|| r.circuit_name.clone());
        }
        used_ids.insert(r.circuit_id.clone());
        used_ids.insert(r.device_id.clone());
    }
    let mut fresh_id = || loop {
        let id = new_id();
        if used_ids.insert(id.clone()) {
            break id;
        }
    };

    // Rules 3 and 4: collect from each online router, in config order.
    let state_of: HashMap<&str, &RouterResult> =
        input.routers.iter().map(|(n, r)| (n.as_str(), r)).collect();
    // Circuit name -> (router, group, base). A later router replaces an earlier one.
    let mut members: BTreeMap<String, (String, GroupKey, Rate)> = BTreeMap::new();
    for (rname, result) in &input.routers {
        let RouterResult::Online(snap) = result else {
            continue;
        };
        let Some(rcfg) = cfg.routers.iter().find(|r| &r.name == rname) else {
            continue;
        };
        let mut seen_here: HashSet<String> = HashSet::new();
        for c in collect_router(cfg, rcfg, snap, &speed, &mut report.warnings) {
            // Two rows of one router with the same Circuit Name: keep the first one.
            if !seen_here.insert(c.name.clone()) {
                report.warnings.push(format!(
                    "router {rname}: two rows give circuit {}; kept the first, skipped MAC {} IP {}",
                    c.name, c.mac, c.ipv4
                ));
                continue;
            }
            // A collected circuit replaces a static row with the same name and keeps its IDs.
            let replaced_static = statics.iter().find(|s| s.circuit_name == c.name).cloned();
            if replaced_static.is_some() {
                statics.retain(|s| s.circuit_name != c.name);
            }
            let old_row = rows.get(&c.name).map(|e| e.row.clone()).or(replaced_static);
            let old = old_row.as_ref();
            let is_new = old.is_none();
            // A new circuit on an IP that a static row or a circuit of this cycle holds is skipped.
            // An IP of an old row that was not collected goes to the new circuit (an IP pool reuses it).
            let holder_wins = |other: &String| {
                statics.iter().any(|s| s.ipv4 == c.ipv4)
                    || rows.get(other).is_some_and(|e| e.collected_now)
            };
            if is_new
                && !c.ipv4.is_empty()
                && let Some(other) = ip_index
                    .get(&c.ipv4)
                    .filter(|o| **o != c.name && holder_wins(o))
            {
                report.warnings.push(format!(
                    "duplicate IP {}: kept {other}, skipped new {}",
                    c.ipv4, c.name
                ));
                report
                    .duplicates
                    .push((c.ipv4.clone(), other.clone(), c.name.clone()));
                continue;
            }
            let (circuit_id, device_id) = match old {
                Some(r) => (r.circuit_id.clone(), r.device_id.clone()),
                None => (fresh_id(), fresh_id()),
            };
            let (parent_node, parent_node_id) = if mode == NetworkMode::FlatPromotion {
                (c.name.clone(), client_node_id(&circuit_id))
            } else {
                parent_for(cfg, rname, &c.group)
            };
            let r = circuit_rates(
                c.base,
                bd.min_rate_percentage,
                bd.safe_min_mbps,
                bd.max_floor_mbps,
            );
            let row = CsvRow {
                circuit_id,
                circuit_name: c.name.clone(),
                device_id,
                device_name: c.device_name.clone(),
                parent_node,
                parent_node_id,
                anchor_node_id: String::new(),
                mac: c.mac.clone(),
                ipv4: c.ipv4.clone(),
                ipv6: String::new(),
                download_min: fmt_rate(r.down_min),
                upload_min: fmt_rate(r.up_min),
                download_max: fmt_rate(r.down_max),
                upload_max: fmt_rate(r.up_max),
                comment: c.comment.clone(),
            };
            match old {
                None => report.added.push(c.name.clone()),
                Some(r) => {
                    let changes = changed_columns(r, &row);
                    if !changes.is_empty() {
                        report.updated.push((c.name.clone(), changes));
                    }
                }
            }
            if is_new && !c.ipv4.is_empty() {
                ip_index.insert(c.ipv4.clone(), c.name.clone());
            }
            let existed_before = old.is_some();
            members.insert(c.name.clone(), (rname.clone(), c.group.clone(), c.base));
            rows.insert(
                c.name.clone(),
                Entry {
                    row,
                    router: Some(rname.clone()),
                    source: c.source,
                    last_seen: input.now,
                    existed_before,
                    collected_now: true,
                },
            );
        }
    }

    // Rule 5: removal.
    let all_online = input
        .routers
        .iter()
        .all(|(_, r)| matches!(r, RouterResult::Online(_)));
    let grace = cfg.runtime.subscriber_grace;
    let mut to_remove: Vec<String> = Vec::new();
    if cfg.runtime.remove_inactive_entries {
        for (name, e) in &rows {
            if e.collected_now {
                continue;
            }
            let aged = input.now.saturating_sub(e.last_seen) >= grace;
            let remove = match e.router.as_deref() {
                None => all_online && aged,
                Some(r) => match state_of.get(r) {
                    None => aged,
                    Some(RouterResult::Online(_)) => aged,
                    Some(RouterResult::Offline) => {
                        cfg.router_access.offline_action == OfflineAction::Remove
                    }
                    Some(RouterResult::Failed | RouterResult::Unreachable) => false,
                },
            };
            if remove {
                to_remove.push(name.clone());
            }
        }
    }

    // Rule 6: removal guard.
    let mut guard = None;
    if !to_remove.is_empty() {
        let base = rows.len();
        let s = &cfg.safety;
        let too_many = to_remove.len() > s.max_removed_count
            || to_remove.len() as f64 * 100.0 > base as f64 * s.max_removed_percent;
        if s.enabled && too_many {
            let fingerprint = fnv1a64(to_remove.join("\n").as_bytes());
            let same = input
                .state
                .guard
                .as_ref()
                .is_some_and(|g| g.fingerprint == fingerprint);
            if !(same || input.force) {
                let since = input
                    .state
                    .guard
                    .as_ref()
                    .filter(|g| g.fingerprint == fingerprint)
                    .map_or(input.now, |g| g.since);
                report.alerts.push(format!(
                    "removal guard: {} of {base} circuits would go in one cycle; held until the next cycle or a sync now",
                    to_remove.len()
                ));
                guard = Some(GuardHold {
                    fingerprint,
                    since,
                    count: to_remove.len(),
                });
                report.held = std::mem::take(&mut to_remove);
            }
        }
    }
    for name in &to_remove {
        rows.remove(name);
    }
    report.removed = to_remove;

    // Duplicate IPs across all rows: a static row wins, then a row collected now, then a row
    // that existed before, then the lower name.
    let static_ip: HashMap<&str, &str> = statics
        .iter()
        .filter(|s| !s.ipv4.is_empty())
        .map(|s| (s.ipv4.as_str(), s.circuit_name.as_str()))
        .collect();
    let mut by_ip: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, e) in &rows {
        if !e.row.ipv4.is_empty() {
            by_ip
                .entry(e.row.ipv4.clone())
                .or_default()
                .push(name.clone());
        }
    }
    let mut dropped_names: HashSet<String> = HashSet::new();
    let rows_before_dedupe = rows.len();
    let mut stale_drops = 0usize;
    for (ip, mut names) in by_ip.into_iter() {
        let static_holder = static_ip.get(ip.as_str()).map(|s| s.to_string());
        if names.len() < 2 && static_holder.is_none() {
            continue;
        }
        names.sort_by_key(|n| {
            let e = &rows[n];
            (!e.collected_now, !e.existed_before, n.clone())
        });
        let (kept, losers) = match static_holder {
            Some(s) => (s, &names[..]),
            None => (names[0].clone(), &names[1..]),
        };
        for dropped in losers {
            if rows.get(dropped).is_some_and(|e| !e.collected_now) {
                stale_drops += 1;
            }
            dropped_names.insert(dropped.clone());
            rows.remove(dropped);
            members.remove(dropped);
            report
                .warnings
                .push(format!("duplicate IP {ip}: kept {kept}, dropped {dropped}"));
            report
                .duplicates
                .push((ip.clone(), kept.clone(), dropped.clone()));
        }
    }

    // The duplicate pass is not held (LibreQoS rejects a duplicate IP), but a large drop gets an alert.
    let s = &cfg.safety;
    if s.enabled
        && (stale_drops > s.max_removed_count
            || stale_drops as f64 * 100.0 > rows_before_dedupe as f64 * s.max_removed_percent)
    {
        report.alerts.push(format!(
            "duplicate IP pass dropped {stale_drops} old rows in one cycle; check the router address pools"
        ));
    }
    report.added.retain(|n| !dropped_names.contains(n));
    report.updated.retain(|(n, _)| !dropped_names.contains(n));

    // Rule 8: nodes.
    let mut online: Vec<RouterGroups> = Vec::new();
    for (rname, result) in &input.routers {
        if !matches!(result, RouterResult::Online(_)) {
            continue;
        }
        let Some(rcfg) = cfg.routers.iter().find(|r| &r.name == rname) else {
            continue;
        };
        let mut totals: BTreeMap<GroupKey, GroupTotal> = BTreeMap::new();
        for (name, (owner, group, base)) in &members {
            if owner != rname || !rows.contains_key(name) {
                continue;
            }
            let t = totals.entry(group.clone()).or_insert(GroupTotal {
                count: 0,
                sum: Rate { down: 0.0, up: 0.0 },
                unit: *base,
            });
            t.count += 1;
            t.sum.down += base.down;
            t.sum.up += base.up;
        }
        online.push(RouterGroups {
            router: rname.clone(),
            ppp_enabled: rcfg.pppoe.enabled,
            per_plan_node: rcfg.pppoe.per_plan_node,
            totals,
        });
    }
    let keeps_nodes = |r: &str| match state_of.get(r) {
        Some(RouterResult::Failed | RouterResult::Unreachable) => true,
        Some(RouterResult::Offline) => cfg.router_access.offline_action == OfflineAction::Keep,
        _ => false,
    };
    let carried: BTreeMap<String, StoredNode> = input
        .state
        .nodes
        .iter()
        .filter(|(_, n)| n.router.as_deref().is_some_and(keeps_nodes))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let plain_rows: Vec<CsvRow> = rows.values().map(|e| e.row.clone()).collect();
    let mut nodes = build_nodes(cfg, input.v3, &online, &carried, &plain_rows)?;
    // A row that stays (grace, seed, or a router that keeps its rows) keeps its old
    // parent node and the ancestors of that node, if this cycle does not make them.
    for e in rows.values().filter(|e| !e.collected_now) {
        let mut parent = Some(e.row.parent_node.clone()).filter(|p| !p.is_empty());
        while let Some(p) = parent.take() {
            if nodes.contains_key(&p) {
                break;
            }
            if let Some(old) = input.state.nodes.get(&p) {
                parent = old.parent.clone();
                nodes.insert(p, old.clone());
            }
        }
    }
    let preserved = if cfg.network.preserve_network_config {
        input.current_network
    } else {
        None
    };
    let network = to_network_json(&nodes, preserved, &input.state.nodes);

    // Rule 7: validation.
    let out_rows: Vec<CsvRow> = rows
        .values()
        .map(|e| e.row.clone())
        .chain(statics)
        .collect();
    validate::check(&out_rows, &network, mode)?;

    let circuits = rows
        .into_iter()
        .map(|(name, e)| {
            (
                name,
                StoredCircuit {
                    router: e.router,
                    source: e.source,
                    last_seen: e.last_seen,
                    row: e.row,
                },
            )
        })
        .collect();
    let state = State {
        version: 1,
        circuits,
        nodes,
        routers: input.state.routers.clone(),
        guard,
    };
    Ok(CycleOutput {
        rows: out_rows,
        network,
        state,
        report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::model::{CsvRow, RouterSnapshot, Source, State, StoredCircuit};
    use crate::topology::{client_node_id, node_id};

    const NOW: u64 = 10_000;

    fn cfg(mode: &str, extra: &str) -> Config {
        let router = |n: &str| {
            format!(
                "[[routers]]\nname=\"{n}\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n[routers.pppoe]\nenabled=true\n"
            )
        };
        Config::from_toml_str(&format!(
            "[network]\nmode=\"{mode}\"\n[safety]\nenabled=false\n{extra}\n{}{}",
            router("R1"),
            router("R2")
        ))
        .unwrap()
    }
    /// One PPP user per (name, ip). The comment gives 50/10 Mbps.
    fn ppp(users: &[(&str, &str)]) -> RouterResult {
        let mut s = RouterSnapshot::default();
        for (n, ip) in users {
            s.ppp_secret.push(
                [("name", *n), ("comment", "|50M/10M")]
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            );
            s.ppp_active.push(
                [("name", *n), ("address", *ip)]
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            );
        }
        RouterResult::Online(s)
    }
    fn stored(
        name: &str,
        router: Option<&str>,
        ip: &str,
        cid: &str,
        last_seen: u64,
    ) -> (String, StoredCircuit) {
        let row = CsvRow {
            circuit_id: cid.into(),
            circuit_name: name.into(),
            device_id: format!("D{cid}"),
            device_name: name.into(),
            parent_node: router.map_or(String::new(), |r| format!("PPP-{r}")),
            parent_node_id: router.map_or(String::new(), |r| node_id("ppp", r, "")),
            ipv4: ip.into(),
            download_min: "5".into(),
            upload_min: "1".into(),
            download_max: "50".into(),
            upload_max: "10".into(),
            comment: "PPP".into(),
            ..Default::default()
        };
        (
            name.into(),
            StoredCircuit {
                router: router.map(String::from),
                source: Source::Ppp,
                last_seen,
                row,
            },
        )
    }
    fn state(items: Vec<(String, StoredCircuit)>) -> State {
        State {
            circuits: items.into_iter().collect(),
            ..State::default()
        }
    }
    fn ids() -> impl FnMut() -> String {
        let mut n = 0;
        move || {
            n += 1;
            format!("N{n:04}")
        }
    }
    fn run_full(
        c: &Config,
        st: &State,
        statics: &[CsvRow],
        routers: Vec<(&str, RouterResult)>,
        force: bool,
    ) -> CycleOutput {
        let input = CycleInput {
            config: c,
            state: st,
            static_rows: statics,
            routers: routers
                .into_iter()
                .map(|(n, r)| (n.to_string(), r))
                .collect(),
            current_network: None,
            v3: false,
            force,
            now: NOW,
        };
        assemble(input, &mut ids()).unwrap()
    }
    fn run(c: &Config, st: &State, routers: Vec<(&str, RouterResult)>) -> CycleOutput {
        run_full(c, st, &[], routers, false)
    }
    fn names(out: &CycleOutput) -> Vec<&str> {
        let mut v: Vec<&str> = out.rows.iter().map(|r| r.circuit_name.as_str()).collect();
        v.sort();
        v
    }
    fn get<'a>(out: &'a CycleOutput, n: &str) -> &'a CsvRow {
        out.rows
            .iter()
            .find(|r| r.circuit_name == n)
            .unwrap_or_else(|| panic!("no row {n}"))
    }
    fn rp() -> Config {
        cfg("root_promotion", "")
    }

    #[test]
    fn new_circuit_gets_new_ids_and_existing_keeps_them() {
        let st = state(vec![stored("old", Some("R1"), "100.64.0.1", "K1", 1)]);
        let out = run(
            &rp(),
            &st,
            vec![
                ("R1", ppp(&[("old", "100.64.0.1"), ("new", "100.64.0.2")])),
                ("R2", ppp(&[])),
            ],
        );
        let new = get(&out, "new");
        assert_eq!(
            (new.circuit_id.as_str(), new.device_id.as_str()),
            ("N0001", "N0002")
        );
        assert_eq!(
            (new.parent_node.as_str(), new.parent_node_id.clone()),
            ("PPP-R1", node_id("ppp", "R1", ""))
        );
        assert_eq!(
            (
                new.download_min.as_str(),
                new.upload_min.as_str(),
                new.download_max.as_str(),
                new.upload_max.as_str()
            ),
            ("25", "5", "50", "10")
        );
        assert_eq!(get(&out, "old").circuit_id, "K1");
        assert_eq!(out.report.added, vec!["new".to_string()]);
        assert_eq!(out.state.circuits["new"].router.as_deref(), Some("R1"));
        assert_eq!(out.state.circuits["new"].last_seen, NOW);
    }

    #[test]
    fn id_collision_asks_for_another_id() {
        let st = state(vec![stored("old", Some("R1"), "100.64.0.1", "N0001", 1)]);
        let out = run(
            &rp(),
            &st,
            vec![
                ("R1", ppp(&[("old", "100.64.0.1"), ("new", "100.64.0.2")])),
                ("R2", ppp(&[])),
            ],
        );
        assert_eq!(get(&out, "new").circuit_id, "N0002");
    }

    #[test]
    fn failed_and_unreachable_router_keep_rows_and_nodes() {
        let c = rp();
        let st = state(vec![
            stored("a", Some("R1"), "100.64.0.1", "K1", 1),
            stored("b", Some("R2"), "100.64.0.2", "K2", 1),
        ]);
        let first = run(
            &c,
            &st,
            vec![
                ("R1", ppp(&[("a", "100.64.0.1")])),
                ("R2", ppp(&[("b", "100.64.0.2")])),
            ],
        );
        for bad in [RouterResult::Failed, RouterResult::Unreachable] {
            let out = run(
                &c,
                &first.state,
                vec![("R1", ppp(&[("a", "100.64.0.1")])), ("R2", bad)],
            );
            assert_eq!(get(&out, "b"), get(&first, "b"));
            assert_eq!(out.state.nodes["PPP-R2"], first.state.nodes["PPP-R2"]);
            assert_eq!(out.network["PPP-R2"], first.network["PPP-R2"]);
        }
    }

    #[test]
    fn online_router_removes_rows_not_seen() {
        let st = state(vec![
            stored("a", Some("R1"), "100.64.0.1", "K1", 1),
            stored("gone", Some("R1"), "100.64.0.9", "K9", 1),
        ]);
        let out = run(
            &rp(),
            &st,
            vec![("R1", ppp(&[("a", "100.64.0.1")])), ("R2", ppp(&[]))],
        );
        assert_eq!(names(&out), ["a"]);
        assert_eq!(out.report.removed, vec!["gone".to_string()]);
    }

    #[test]
    fn offline_remove_ignores_grace_and_keep_keeps() {
        let c = cfg("root_promotion", "[runtime]\nsubscriber_grace=600");
        let first = run(
            &c,
            &State::default(),
            vec![("R1", ppp(&[])), ("R2", ppp(&[("b", "100.64.0.2")]))],
        );
        let out = run(
            &c,
            &first.state,
            vec![("R1", ppp(&[])), ("R2", RouterResult::Offline)],
        );
        assert!(names(&out).is_empty());
        let c = cfg("root_promotion", "[router_access]\noffline_action=\"keep\"");
        let out = run(
            &c,
            &first.state,
            vec![("R1", ppp(&[])), ("R2", RouterResult::Offline)],
        );
        assert_eq!(names(&out), ["b"]);
        assert!(out.network["PPP-R2"].is_object());
    }

    #[test]
    fn a_row_that_stays_keeps_its_group_node() {
        // Grace keeps "solo", but its plan group has no member in this cycle.
        let c = Config::from_toml_str(
            "[network]\nmode=\"root_promotion\"\n[safety]\nenabled=false\n[runtime]\nsubscriber_grace=600\n\
             [[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n\
             [routers.pppoe]\nenabled=true\nper_plan_node=true\n",
        )
        .unwrap();
        let first = run(
            &c,
            &State::default(),
            vec![("R1", ppp(&[("solo", "100.64.0.4")]))],
        );
        let out = run_full(&c, &first.state, &[], vec![("R1", ppp(&[]))], false);
        assert_eq!(names(&out), ["solo"]);
        assert_eq!(out.network["default-R1"], first.network["default-R1"]);
    }

    #[test]
    fn seed_rows_go_only_when_all_routers_online() {
        let st = state(vec![stored("seed", None, "100.64.0.5", "K5", 1)]);
        let out = run(
            &rp(),
            &st,
            vec![("R1", ppp(&[])), ("R2", RouterResult::Failed)],
        );
        assert_eq!(names(&out), ["seed"]);
        let out = run(&rp(), &st, vec![("R1", ppp(&[])), ("R2", ppp(&[]))]);
        assert!(names(&out).is_empty());
    }

    #[test]
    fn rows_of_a_router_not_in_config_are_removed() {
        let st = state(vec![stored("x", Some("R9"), "100.64.0.7", "K7", NOW)]);
        let out = run(
            &rp(),
            &st,
            vec![("R1", ppp(&[])), ("R2", RouterResult::Failed)],
        );
        assert!(names(&out).is_empty());
    }

    #[test]
    fn grace_keeps_a_missing_subscriber() {
        let c = cfg("root_promotion", "[runtime]\nsubscriber_grace=600");
        let st = state(vec![
            stored("recent", Some("R1"), "100.64.0.1", "K1", NOW - 300),
            stored("old", Some("R1"), "100.64.0.2", "K2", NOW - 600),
        ]);
        let out = run(&c, &st, vec![("R1", ppp(&[])), ("R2", ppp(&[]))]);
        assert_eq!(names(&out), ["recent"]);
    }

    #[test]
    fn remove_inactive_entries_false_removes_nothing() {
        let c = cfg("root_promotion", "[runtime]\nremove_inactive_entries=false");
        let st = state(vec![stored("a", Some("R1"), "100.64.0.1", "K1", 1)]);
        let out = run(&c, &st, vec![("R1", ppp(&[])), ("R2", ppp(&[]))]);
        assert_eq!(names(&out), ["a"]);
    }

    #[test]
    fn static_row_passes_through_and_collected_replaces_it() {
        let s1 = CsvRow {
            circuit_id: "S1".into(),
            circuit_name: "fixed".into(),
            device_id: "SD1".into(),
            download_min: "5".into(),
            upload_min: "5".into(),
            download_max: "10".into(),
            upload_max: "10".into(),
            comment: "Static".into(),
            ..Default::default()
        };
        let s2 = CsvRow {
            circuit_id: "S2".into(),
            circuit_name: "taken".into(),
            device_id: "SD2".into(),
            ..s1.clone()
        };
        let out = run_full(
            &rp(),
            &State::default(),
            &[s1.clone(), s2],
            vec![("R1", ppp(&[("taken", "100.64.0.3")])), ("R2", ppp(&[]))],
            false,
        );
        assert_eq!(get(&out, "fixed"), &s1);
        let t = get(&out, "taken");
        assert_eq!((t.comment.as_str(), t.circuit_id.as_str()), ("PPP", "S2"));
        assert!(!out.state.circuits.contains_key("fixed"));
    }

    #[test]
    fn duplicate_ip_new_vs_existing_keeps_existing() {
        let st = state(vec![stored("old", Some("R1"), "100.64.0.1", "K1", 1)]);
        let out = run(
            &rp(),
            &st,
            vec![
                ("R1", ppp(&[("old", "100.64.0.1"), ("dup", "100.64.0.1")])),
                ("R2", ppp(&[])),
            ],
        );
        assert_eq!(names(&out), ["old"]);
        assert_eq!(
            out.report.duplicates,
            vec![("100.64.0.1".into(), "old".into(), "dup".into())]
        );
        assert!(out.report.added.is_empty(), "{:?}", out.report.added);
    }

    #[test]
    fn duplicate_ip_existing_vs_existing_keeps_collected() {
        let st = state(vec![
            stored("a", Some("R1"), "100.64.0.1", "K1", 1),
            stored("b", Some("R2"), "100.64.0.9", "K2", 1),
        ]);
        let out = run(
            &rp(),
            &st,
            vec![
                ("R1", ppp(&[("a", "100.64.0.9")])),
                ("R2", RouterResult::Failed),
            ],
        );
        assert_eq!(names(&out), ["a"]);
        assert_eq!(
            out.report.duplicates,
            vec![("100.64.0.9".into(), "a".into(), "b".into())]
        );
    }

    #[test]
    fn same_name_on_two_routers_later_router_wins() {
        let out = run(
            &rp(),
            &State::default(),
            vec![
                ("R1", ppp(&[("s", "100.64.0.1")])),
                ("R2", ppp(&[("s", "100.64.0.1")])),
            ],
        );
        assert_eq!(get(&out, "s").parent_node, "PPP-R2");
        assert_eq!(out.state.circuits["s"].router.as_deref(), Some("R2"));
    }

    #[test]
    fn empty_online_router_removes_its_rows() {
        let st = state(vec![stored("a", Some("R1"), "100.64.0.1", "K1", 1)]);
        let out = run(
            &rp(),
            &st,
            vec![
                ("R1", RouterResult::Online(RouterSnapshot::default())),
                ("R2", ppp(&[])),
            ],
        );
        assert!(names(&out).is_empty());
        assert_eq!(
            out.network["PPP-R1"]["downloadBandwidthMbps"],
            serde_json::json!(0.128)
        );
    }

    #[test]
    fn v3_with_no_parent_stops() {
        let c = cfg("no_parent", "");
        let input = CycleInput {
            config: &c,
            state: &State::default(),
            static_rows: &[],
            routers: vec![],
            current_network: None,
            v3: true,
            force: false,
            now: NOW,
        };
        let e = assemble(input, &mut ids()).err().unwrap();
        assert!(format!("{e:#}").contains("root_promotion"));
    }

    fn hundred() -> State {
        state(
            (0..100)
                .map(|i| {
                    stored(
                        &format!("u{i:03}"),
                        Some("R1"),
                        &format!("100.64.1.{i}"),
                        &format!("K{i}"),
                        1,
                    )
                })
                .collect(),
        )
    }
    fn first_forty() -> Vec<(String, String)> {
        (0..40)
            .map(|i| (format!("u{i:03}"), format!("100.64.1.{i}")))
            .collect()
    }
    fn ppp_owned(v: &[(String, String)]) -> RouterResult {
        let refs: Vec<(&str, &str)> = v.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        ppp(&refs)
    }
    fn guard_cfg() -> Config {
        Config::from_toml_str(&rp_toml_with_guard()).unwrap()
    }
    fn rp_toml_with_guard() -> String {
        "[network]\nmode=\"root_promotion\"\n[[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n[routers.pppoe]\nenabled=true\n".into()
    }

    #[test]
    fn guard_holds_a_mass_removal_and_keeps_other_changes() {
        let mut users = first_forty();
        users.push(("fresh".into(), "100.64.2.1".into()));
        let out = run(&guard_cfg(), &hundred(), vec![("R1", ppp_owned(&users))]);
        assert_eq!(out.rows.len(), 101);
        assert_eq!(out.report.held.len(), 60);
        assert_eq!(out.report.added, vec!["fresh".to_string()]);
        assert_eq!(out.report.alerts.len(), 1);
        assert!(out.state.guard.is_some());
    }

    #[test]
    fn guard_releases_on_same_fingerprint_next_cycle() {
        let c = guard_cfg();
        let held = run(&c, &hundred(), vec![("R1", ppp_owned(&first_forty()))]);
        let out = run(&c, &held.state, vec![("R1", ppp_owned(&first_forty()))]);
        assert_eq!(out.rows.len(), 40);
        assert_eq!(out.report.removed.len(), 60);
        assert!(out.state.guard.is_none());
    }

    #[test]
    fn guard_restarts_on_a_different_fingerprint() {
        let c = guard_cfg();
        let held = run(&c, &hundred(), vec![("R1", ppp_owned(&first_forty()))]);
        let fewer: Vec<_> = first_forty().into_iter().take(30).collect();
        let out = run(&c, &held.state, vec![("R1", ppp_owned(&fewer))]);
        assert_eq!(out.rows.len(), 100);
        assert_eq!(out.report.held.len(), 70);
        assert_ne!(
            out.state.guard.unwrap().fingerprint,
            held.state.guard.unwrap().fingerprint
        );
    }

    #[test]
    fn guard_releases_on_force() {
        let out = run_full(
            &guard_cfg(),
            &hundred(),
            &[],
            vec![("R1", ppp_owned(&first_forty()))],
            true,
        );
        assert_eq!(out.rows.len(), 40);
    }

    #[test]
    fn guard_off_removes_at_once() {
        let c = Config::from_toml_str(&format!(
            "[safety]\nenabled=false\n{}",
            rp_toml_with_guard()
        ))
        .unwrap();
        let out = run(&c, &hundred(), vec![("R1", ppp_owned(&first_forty()))]);
        assert_eq!(out.rows.len(), 40);
    }

    #[test]
    fn small_removal_is_not_held() {
        let most: Vec<_> = (0..97)
            .map(|i| (format!("u{i:03}"), format!("100.64.1.{i}")))
            .collect();
        let out = run(&guard_cfg(), &hundred(), vec![("R1", ppp_owned(&most))]);
        assert_eq!((out.rows.len(), out.report.held.len()), (97, 0));
    }

    #[test]
    fn flat_promotion_has_no_client_node_for_a_static_row() {
        let c = cfg("flat_promotion", "");
        let s = CsvRow {
            circuit_id: "S1".into(),
            circuit_name: "fixed".into(),
            device_id: "SD1".into(),
            download_min: "5".into(),
            upload_min: "5".into(),
            download_max: "10".into(),
            upload_max: "10".into(),
            comment: "static".into(),
            ..Default::default()
        };
        let out = run_full(
            &c,
            &State::default(),
            &[s],
            vec![("R1", ppp(&[("a", "100.64.0.1")])), ("R2", ppp(&[]))],
            false,
        );
        let keys: Vec<_> = out.network.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["a"]);
    }

    #[test]
    fn flat_promotion_parent_is_own_client_node() {
        let c = cfg("flat_promotion", "");
        let out = run(
            &c,
            &State::default(),
            vec![("R1", ppp(&[("a", "100.64.0.1")])), ("R2", ppp(&[]))],
        );
        let a = get(&out, "a");
        assert_eq!(
            (a.parent_node.as_str(), a.parent_node_id.clone()),
            ("a", client_node_id(&a.circuit_id))
        );
        assert_eq!(
            out.network["a"]["id"],
            serde_json::json!(client_node_id(&a.circuit_id))
        );
    }

    #[test]
    fn group_counts_skip_duplicate_ip_circuits() {
        let c = cfg(
            "root_promotion",
            "[bandwidth_defaults]\npppoe_aggregate_factor=1.0",
        );
        let st = state(vec![stored("old", Some("R1"), "100.64.0.1", "K1", 1)]);
        let out = run(
            &c,
            &st,
            vec![
                ("R1", ppp(&[("old", "100.64.0.1"), ("dup", "100.64.0.1")])),
                ("R2", ppp(&[])),
            ],
        );
        // Only "old" counts: 50 Mbps, not 100.
        assert_eq!(
            out.network["PPP-R1"]["downloadBandwidthMbps"],
            serde_json::json!(50.0)
        );
    }

    fn static_row(name: &str, cid: &str, did: &str, ip: &str) -> CsvRow {
        CsvRow {
            circuit_id: cid.into(),
            circuit_name: name.into(),
            device_id: did.into(),
            ipv4: ip.into(),
            download_min: "5".into(),
            upload_min: "5".into(),
            download_max: "10".into(),
            upload_max: "10".into(),
            comment: "static".into(),
            ..Default::default()
        }
    }

    #[test]
    fn static_row_wins_a_duplicate_ip() {
        let srv = static_row("srv", "S1", "SD1", "100.64.0.50");
        let first = run_full(
            &rp(),
            &State::default(),
            std::slice::from_ref(&srv),
            vec![("R1", ppp(&[("alice", "100.64.0.9")])), ("R2", ppp(&[]))],
            false,
        );
        let out = run_full(
            &rp(),
            &first.state,
            &[srv],
            vec![("R1", ppp(&[("alice", "100.64.0.50")])), ("R2", ppp(&[]))],
            false,
        );
        assert_eq!(names(&out), ["srv"]);
        assert_eq!(
            out.report.duplicates,
            vec![("100.64.0.50".into(), "srv".into(), "alice".into())]
        );
    }

    #[test]
    fn recycled_ip_goes_to_the_new_subscriber() {
        let c = cfg("root_promotion", "[runtime]\nsubscriber_grace=600");
        let st = state(vec![stored(
            "alice",
            Some("R1"),
            "100.64.0.5",
            "K1",
            NOW - 30,
        )]);
        let out = run(
            &c,
            &st,
            vec![("R1", ppp(&[("bob", "100.64.0.5")])), ("R2", ppp(&[]))],
        );
        assert_eq!(names(&out), ["bob"]);
        assert_eq!(out.report.added, vec!["bob".to_string()]);
        assert_eq!(
            out.report.duplicates,
            vec![("100.64.0.5".into(), "bob".into(), "alice".into())]
        );
    }

    #[test]
    fn same_name_twice_in_one_router_keeps_first_and_warns() {
        let c = Config::from_toml_str(
            "[network]\nmode=\"root_promotion\"\n[safety]\nenabled=false\n\
             [[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n\
             [routers.dhcp]\nenabled=true\n[[routers.dhcp.servers]]\nname=\"LAN\"\n",
        )
        .unwrap();
        let snap: RouterSnapshot = serde_json::from_str(
            r#"{"dhcp_lease":[
                {"server":"LAN","host-name":"iPhone","mac-address":"02:00:00:00:09:01","address":"100.66.9.1"},
                {"server":"LAN","host-name":"iPhone","mac-address":"02:00:00:00:09:02","address":"100.66.9.2"}]}"#,
        )
        .unwrap();
        let first = run(
            &c,
            &State::default(),
            vec![("R1", RouterResult::Online(snap.clone()))],
        );
        assert_eq!(names(&first), ["DHCP-iPhone"]);
        assert_eq!(get(&first, "DHCP-iPhone").mac, "02:00:00:00:09:01");
        assert!(
            first
                .report
                .warnings
                .iter()
                .any(|w| w.contains("DHCP-iPhone")),
            "{:?}",
            first.report.warnings
        );
        let again = run(&c, &first.state, vec![("R1", RouterResult::Online(snap))]);
        assert!(
            again.report.updated.is_empty(),
            "{:?}",
            again.report.updated
        );
    }

    #[test]
    fn node_name_collision_stops_with_clear_message() {
        let c = Config::from_toml_str(
            "[network]\nmode=\"root_promotion\"\n[safety]\nenabled=false\n\
             [[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n\
             [routers.pppoe]\nenabled=true\nper_plan_node=true\n[routers.hotspot]\nenabled=true\n",
        )
        .unwrap();
        let snap: RouterSnapshot = serde_json::from_str(
            r#"{"ppp_secret":[{"name":"alice","profile":"HS"}],"ppp_active":[{"name":"alice","address":"100.64.0.2"}],
                "ppp_profile":[{"name":"HS","rate-limit":"10M/10M"}],
                "hotspot_active":[{"user":"g","mac-address":"02:00:00:00:08:01","address":"100.65.0.9"}]}"#,
        )
        .unwrap();
        let input = CycleInput {
            config: &c,
            state: &State::default(),
            static_rows: &[],
            routers: vec![("R1".into(), RouterResult::Online(snap))],
            current_network: None,
            v3: false,
            force: false,
            now: NOW,
        };
        let e = format!("{:#}", assemble(input, &mut ids()).err().unwrap());
        assert!(
            e.contains("HS-R1") && e.contains("hotspot") && e.contains("ppp_plan"),
            "{e}"
        );
    }

    #[test]
    fn static_multi_device_circuit_keeps_all_rows() {
        let a = CsvRow {
            circuit_name: "Biz".into(),
            ..static_row("Biz", "C9", "D1", "100.64.9.1")
        };
        let b = CsvRow {
            circuit_name: "Biz".into(),
            ..static_row("Biz", "C9", "D2", "100.64.9.2")
        };
        let out = run_full(
            &rp(),
            &State::default(),
            &[a, b],
            vec![("R1", ppp(&[])), ("R2", ppp(&[]))],
            false,
        );
        assert_eq!(
            out.rows.iter().filter(|r| r.circuit_name == "Biz").count(),
            2
        );
    }

    #[test]
    fn seeded_nodes_survive_preserve_mode() {
        let c = Config::from_toml_str(
            "[network]\nmode=\"root_promotion\"\npreserve_network_config=true\n[safety]\nenabled=false\n\
             [[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n\
             [routers.pppoe]\nenabled=true\n",
        )
        .unwrap();
        let net = serde_json::json!({
            "Core": {"children": {"Tower": {"children": {}, "downloadBandwidthMbps": 5.0, "type": "ap", "uploadBandwidthMbps": 5.0}},
                     "downloadBandwidthMbps": 100.0, "type": "site", "uploadBandwidthMbps": 100.0},
            "PPP-R1": {"children": {}, "downloadBandwidthMbps": 10.0, "type": "plan", "uploadBandwidthMbps": 10.0}});
        let (_, st0) = stored("alice", None, "100.64.0.1", "K1", 1);
        let mut st = state(vec![("alice".into(), st0)]);
        st.nodes = crate::state::seed_nodes(&net);
        let input = CycleInput {
            config: &c,
            state: &st,
            static_rows: &[],
            routers: vec![("R1".into(), ppp(&[("alice", "100.64.0.1")]))],
            current_network: Some(net),
            v3: false,
            force: false,
            now: NOW,
        };
        let out = assemble(input, &mut ids()).unwrap();
        assert!(
            out.network["Core"]["children"]["Tower"].is_object(),
            "{}",
            out.network
        );
        assert!(out.network["PPP-R1"].is_object());
    }

    #[test]
    fn grace_also_applies_to_a_router_not_in_the_config() {
        // Cycle 1: R9 is in the config. Cycle 2: R9 is gone from the config.
        let with_r9 = Config::from_toml_str(&format!(
            "[runtime]\nsubscriber_grace=600\n[safety]\nenabled=false\n[router_defaults.pppoe]\nenabled=true\n{}",
            ["R1", "R9"].map(|n| format!("[[routers]]\nname=\"{n}\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n")).concat()
        ))
        .unwrap();
        let mut first = run(
            &with_r9,
            &State::default(),
            vec![
                ("R1", ppp(&[])),
                (
                    "R9",
                    ppp(&[("recent", "100.64.0.1"), ("old", "100.64.0.2")]),
                ),
            ],
        );
        first.state.circuits.get_mut("recent").unwrap().last_seen = NOW - 300;
        first.state.circuits.get_mut("old").unwrap().last_seen = NOW - 600;
        let c = cfg("root_promotion", "[runtime]\nsubscriber_grace=600");
        let out = run(&c, &first.state, vec![("R1", ppp(&[])), ("R2", ppp(&[]))]);
        assert_eq!(names(&out), ["recent"]);
    }

    #[test]
    fn many_duplicate_ip_drops_raise_an_alert() {
        let c = Config::from_toml_str(&format!(
            "[runtime]\nsubscriber_grace=600\n{}",
            rp_toml_with_guard()
        ))
        .unwrap();
        let st = state(
            (0..100)
                .map(|i| {
                    stored(
                        &format!("u{i:03}"),
                        Some("R1"),
                        &format!("100.64.1.{i}"),
                        &format!("K{i}"),
                        NOW - 30,
                    )
                })
                .collect(),
        );
        // 60 new names get the IPs of 60 old rows that grace keeps: the duplicate pass drops 60 rows.
        let fresh: Vec<_> = (0..60)
            .map(|i| (format!("n{i:03}"), format!("100.64.1.{i}")))
            .collect();
        let out = run(&c, &st, vec![("R1", ppp_owned(&fresh))]);
        assert_eq!(out.report.duplicates.len(), 60);
        assert!(
            out.report.alerts.iter().any(|a| a.contains("duplicate")),
            "{:?}",
            out.report.alerts
        );
    }

    #[test]
    fn changed_columns_compare_rates_as_numbers_and_keep_values() {
        let old = CsvRow {
            download_max: "25.0".into(),
            upload_max: "25.0".into(),
            ipv4: "10.0.0.1".into(),
            ..CsvRow::default()
        };
        let new = CsvRow {
            download_max: "25".into(),
            upload_max: "35".into(),
            ipv4: "10.0.0.2".into(),
            ..CsvRow::default()
        };
        assert_eq!(
            changed_columns(&old, &new),
            [
                (
                    "IPv4".to_string(),
                    "10.0.0.1".to_string(),
                    "10.0.0.2".to_string()
                ),
                (
                    "Upload Max Mbps".to_string(),
                    "25.0".to_string(),
                    "35".to_string()
                ),
            ]
        );
    }
}
