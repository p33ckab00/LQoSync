use super::*;
use crate::config::Config;
use crate::cycle::{CycleInput, CycleOutput, RouterResult, assemble};
use crate::model::{RosRow, RouterSnapshot, State};
use crate::routeros::{ReadError, RouterSession};
use crate::topology_manager::identity::authorize_configured;
use crate::topology_manager::normalize::{CurrentRouter, normalize};
use crate::topology_manager::observation::ObservationGate;
use crate::topology_manager::preflight::legacy_issues;
use crate::topology_manager::reader::{ReadGuard, ReadPlan, Table};
use std::collections::BTreeMap;
use std::time::Duration;

const CONFIG: &str = include_str!("../../tests/scenarios/v3_root_promotion/config.toml");
const SNAPSHOT: &str = include_str!("../../tests/scenarios/v3_root_promotion/snapshot.json");

struct Session(BTreeMap<Table, Vec<RosRow>>);
impl RouterSession for Session {
    async fn print(&mut self, path: &str, fields: &str) -> Result<Vec<RosRow>, ReadError> {
        let wanted: Vec<_> = fields.split(',').collect();
        Ok(self
            .0
            .iter()
            .find(|(t, _)| t.command().0 == path)
            .map(|(_, rows)| {
                rows.iter()
                    .map(|r| {
                        r.iter()
                            .filter(|(k, _)| wanted.contains(&k.as_str()))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    })
                    .collect()
            })
            .unwrap_or_default())
    }
}

pub(crate) fn scenario() -> (Config, RouterSnapshot) {
    // The manager reads only bound leases. Enrollment needs require_bound = true (a runbook step).
    let cfg = Config::from_toml_str(&CONFIG.replace(
        "[routers.dhcp]\nenabled = true",
        "[routers.dhcp]\nenabled = true\ndiscover = true\nrequire_bound = true",
    ))
    .unwrap();
    assert!(cfg.routers[0].dhcp.discover && cfg.routers[0].dhcp.require_bound);
    let snap: serde_json::Value = serde_json::from_str(SNAPSHOT).unwrap();
    let snap: RouterSnapshot = serde_json::from_value(snap["routers"]["Hub-1"].clone()).unwrap();
    (cfg, snap)
}

pub(crate) fn legacy(cfg: &Config, snap: &RouterSnapshot) -> CycleOutput {
    legacy_with(cfg, snap, &[])
}

pub(crate) fn legacy_with(
    cfg: &Config,
    snap: &RouterSnapshot,
    statics: &[crate::model::CsvRow],
) -> CycleOutput {
    let mut n = 0;
    let mut ids = || {
        n += 1;
        format!("N{n:04}")
    };
    let input = CycleInput {
        config: cfg,
        state: &State::default(),
        static_rows: statics,
        routers: vec![("Hub-1".into(), RouterResult::Online(snap.clone()))],
        current_network: None,
        v3: true,
        force: false,
        now: 1_000,
    };
    assemble(input, &mut ids).unwrap()
}

/// The same router rows in the manager reader form: RouterOS `.id` values and active lease fields.
pub(crate) fn tables(snap: &RouterSnapshot) -> BTreeMap<Table, Vec<RosRow>> {
    let with = |rows: &[RosRow], prefix: &str, extra: &dyn Fn(&mut RosRow)| {
        rows.iter()
            .enumerate()
            .map(|(i, r)| {
                let mut r = r.clone();
                r.insert(".id".into(), format!("*{prefix}{i}"));
                extra(&mut r);
                r
            })
            .collect::<Vec<_>>()
    };
    BTreeMap::from([
        (Table::PppSecrets, with(&snap.ppp_secret, "s", &|_| {})),
        (Table::PppProfiles, with(&snap.ppp_profile, "p", &|_| {})),
        (
            Table::PppActive,
            with(&snap.ppp_active, "a", &|r| {
                r.insert("service".into(), "pppoe".into());
            }),
        ),
        (
            Table::DhcpServers,
            with(&snap.dhcp_server, "d", &|r| {
                r.insert("disabled".into(), "false".into());
            }),
        ),
        (
            Table::DhcpLeases,
            with(&snap.dhcp_lease, "l", &|r| {
                for (from, to) in [
                    ("address", "active-address"),
                    ("mac-address", "active-mac-address"),
                ] {
                    if let Some(v) = r.get(from).cloned() {
                        r.insert(to.into(), v);
                    }
                }
            }),
        ),
    ])
}

pub(crate) async fn current(cfg: &Config, snap: &RouterSnapshot) -> CurrentRouter {
    current_tables(cfg, tables(snap)).await
}

/// As `current`, from router rows that the test sets itself.
pub(crate) async fn current_tables(
    cfg: &Config,
    rows: BTreeMap<Table, Vec<RosRow>>,
) -> CurrentRouter {
    let catalog = read_catalog(cfg);
    let plan = ReadPlan::for_router(&catalog, &catalog.routers[0].id).unwrap();
    let evidence = ReadGuard::new(catalog.routers[0].id.clone())
        .unwrap()
        .read(
            &mut Some(Session(rows)),
            &plan,
            1,
            Duration::from_secs(1),
            Duration::from_secs(10),
        )
        .await
        .unwrap();
    normalize(&evidence).unwrap()
}

#[tokio::test(start_paused = true)]
async fn proposed_policy_has_no_legacy_issues() {
    let (cfg, snap) = scenario();
    let draft = propose(&cfg, &[current(&cfg, &snap).await]).unwrap();
    let doc = document(&draft).unwrap();
    assert_eq!(legacy_issues(&cfg, &doc).unwrap(), vec![]);
}

pub(crate) async fn candidate(
    cfg: &Config,
    snap: &RouterSnapshot,
) -> crate::topology_manager::publication::PreparedPair {
    let draft = propose(cfg, &[current(cfg, snap).await]).unwrap();
    let doc = document(&draft).unwrap();
    let auth = authorize_configured(&doc, 0, 1, vec![current(cfg, snap).await]).unwrap();
    assert!(auth.conflicts().is_empty(), "{:?}", auth.conflicts());
    crate::topology_manager::publication::prepare(
        auth.reconcile(&mut ObservationGate::default()).unwrap(),
    )
    .unwrap()
}

/// Remove the " [id]" label suffix. Adoption node aliases restore the legacy labels.
pub(crate) fn unaliased(network: &serde_json::Value) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    for (k, v) in network.as_object().unwrap() {
        let id = v["id"].as_str().unwrap();
        out.insert(
            k.trim_end_matches(&format!(" [{id}]")).to_string(),
            v.clone(),
        );
    }
    serde_json::Value::Object(out)
}

#[tokio::test(start_paused = true)]
async fn proposed_policy_reproduces_root_promotion_network() {
    let (cfg, snap) = scenario();
    let legacy = legacy(&cfg, &snap);
    let pair = candidate(&cfg, &snap).await;
    let quantized = crate::topology_manager::parity::quantized_network(
        &legacy.network,
        pair.node_export_adjustments(),
        str::to_string,
    )
    .unwrap();
    assert_eq!(unaliased(pair.network()), quantized);
    assert_eq!(pair.rows().len(), legacy.rows.len());
    assert!(!pair.node_export_adjustments().is_empty());
}

#[tokio::test(start_paused = true)]
async fn changed_fractional_capacity_is_not_hidden_by_quantization() {
    let (cfg, snap) = scenario();
    let mut legacy = legacy(&cfg, &snap).network;
    legacy["Gold-Hub-1"]["downloadBandwidthMbps"] = serde_json::json!(53.5);
    let pair = candidate(&cfg, &snap).await;
    let quantized = crate::topology_manager::parity::quantized_network(
        &legacy,
        pair.node_export_adjustments(),
        str::to_string,
    )
    .unwrap();
    assert_eq!(
        quantized["Gold-Hub-1"]["downloadBandwidthMbps"],
        serde_json::json!(53.5)
    );
    assert_ne!(unaliased(pair.network()), quantized);
}

#[tokio::test(start_paused = true)]
async fn dhcp_client_overrides_block_the_first_proposal() {
    let (_, snap) = scenario();
    let mut text = CONFIG.replace(
        "[routers.dhcp]\nenabled = true",
        "[routers.dhcp]\nenabled = true\ndiscover = true\nrequire_bound = true",
    );
    text.push_str("\n[[routers.dhcp.clients]]\nmac = \"02:00:00:00:00:01\"\ndownload_mbps = 10\nupload_mbps = 10\n");
    let with_clients = Config::from_toml_str(&text).unwrap();
    assert!(!with_clients.routers[0].dhcp.clients.is_empty());
    assert!(propose(&with_clients, &[current(&with_clients, &snap).await]).is_err());
}

#[tokio::test(start_paused = true)]
async fn proposed_policy_round_trips_through_json() {
    let (cfg, snap) = scenario();
    let draft = propose(&cfg, &[current(&cfg, &snap).await]).unwrap();
    let text = serde_json::to_string_pretty(&draft).unwrap();
    assert_eq!(serde_json::from_str::<PolicyDraft>(&text).unwrap(), draft);
}

#[tokio::test(start_paused = true)]
async fn discovered_dhcp_server_is_selected_when_discover_is_on() {
    let (cfg, mut snap) = scenario();
    let mut server = snap.dhcp_server[0].clone();
    server.insert("name".into(), "GUEST".into());
    snap.dhcp_server.push(server);
    let mut lease = snap.dhcp_lease[0].clone();
    for (k, v) in [
        ("server", "GUEST"),
        ("address", "100.66.9.9"),
        ("mac-address", "02:00:00:00:09:09"),
        ("host-name", "guest-1"),
    ] {
        lease.insert(k.into(), v.into());
    }
    snap.dhcp_lease.push(lease);
    let old = legacy(&cfg, &snap);
    assert!(old.rows.iter().any(|r| r.mac == "02:00:00:00:09:09"));
    let pair = candidate(&cfg, &snap).await;
    let quantized = crate::topology_manager::parity::quantized_network(
        &old.network,
        pair.node_export_adjustments(),
        str::to_string,
    )
    .unwrap();
    assert_eq!(unaliased(pair.network()), quantized);
    assert_eq!(pair.rows().len(), old.rows.len());
}

#[tokio::test(start_paused = true)]
async fn a_ppp_profile_without_an_active_session_gets_no_group() {
    let (cfg, mut snap) = scenario();
    let groups = |d: &PolicyDraft| {
        let mut names: Vec<String> = d.catalog.nodes.iter().map(|n| n.name.clone()).collect();
        names.sort();
        names
    };
    let before = groups(&propose(&cfg, &[current(&cfg, &snap).await]).unwrap());
    let mut unused = snap.ppp_profile[0].clone();
    unused.insert("name".into(), "Unused".into());
    snap.ppp_profile.push(unused);
    // A secret uses the profile, but the secret has no active session.
    let mut secret = snap.ppp_secret[0].clone();
    secret.insert("name".into(), "offline-user".into());
    secret.insert("profile".into(), "Unused".into());
    snap.ppp_secret.push(secret);
    let draft = propose(&cfg, &[current(&cfg, &snap).await]).unwrap();
    assert_eq!(groups(&draft), before);
}

#[tokio::test(start_paused = true)]
async fn a_dhcp_server_without_a_lease_gets_no_group() {
    let (cfg, snap) = scenario();
    let draft = propose(&cfg, &[current(&cfg, &snap).await]).unwrap();
    assert!(
        !draft
            .catalog
            .nodes
            .iter()
            .any(|n| n.name == "DHCP-EMPTY-Hub-1"),
        "{:?}",
        draft
            .catalog
            .nodes
            .iter()
            .map(|n| &n.name)
            .collect::<Vec<_>>()
    );
    // The server stays selected, so a first lease on it is read.
    assert!(
        draft
            .source_settings
            .selections
            .iter()
            .any(|s| s.service_names == ["EMPTY"])
    );
}

#[tokio::test(start_paused = true)]
async fn a_secret_that_moves_to_a_profile_without_a_group_keeps_the_others() {
    let (cfg, mut snap) = scenario();
    let mut unused = snap.ppp_profile[0].clone();
    unused.insert("name".into(), "Unused".into());
    snap.ppp_profile.push(unused);
    let draft = propose(&cfg, &[current(&cfg, &snap).await]).unwrap();
    let doc = document(&draft).unwrap();
    // Later, the operator moves one secret to the profile without a group.
    let user = snap.ppp_active[0]["name"].clone();
    for s in snap.ppp_secret.iter_mut().filter(|s| s["name"] == user) {
        s.insert("profile".into(), "Unused".into());
    }
    let auth = authorize_configured(&doc, 0, 1, vec![current(&cfg, &snap).await]).unwrap();
    assert!(auth.conflicts().is_empty(), "{:?}", auth.conflicts());
    let pair = crate::topology_manager::publication::prepare(
        auth.reconcile(&mut ObservationGate::default()).unwrap(),
    )
    .unwrap();
    // The moved subscriber stays shaped without a group; the others keep their groups.
    assert_eq!(pair.rows().len(), legacy(&cfg, &snap).rows.len());
    let ungrouped = pair
        .rows()
        .iter()
        .filter(|r| r.parent_node.is_empty())
        .count();
    assert_eq!(ungrouped, 1);
}

#[tokio::test(start_paused = true)]
async fn a_profile_id_equal_to_a_leased_server_id_is_not_active() {
    // RouterOS `.id` values are per table: profile `*1` and DHCP server `*1` can both exist.
    let (cfg, mut snap) = scenario();
    let mut unused = snap.ppp_profile[0].clone();
    unused.insert("name".into(), "Unused".into());
    snap.ppp_profile.push(unused);
    let mut rows = tables(&snap);
    let server = rows[&Table::DhcpServers][0][".id"].clone();
    let leased = rows[&Table::DhcpServers][0]["name"].clone();
    assert!(snap.dhcp_lease.iter().any(|l| l["server"] == leased));
    for p in rows.get_mut(&Table::PppProfiles).unwrap() {
        if p["name"] == "Unused" {
            p.insert(".id".into(), server.clone());
        }
    }
    let draft = propose(&cfg, &[current_tables(&cfg, rows).await]).unwrap();
    assert!(
        !draft
            .catalog
            .nodes
            .iter()
            .any(|n| n.name.starts_with("Unused")),
        "{:?}",
        draft
            .catalog
            .nodes
            .iter()
            .map(|n| &n.name)
            .collect::<Vec<_>>()
    );
}
