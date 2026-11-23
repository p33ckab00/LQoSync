//! The router inventory for the web: configured routers, catalog WAN roles, and the latest collection.
//! No router I/O. No secret value and no file path leave this module: only "configured" flags.
use crate::collection;
use crate::config::{Config, DhcpMode, DhcpSource, RouterCfg};
use crate::model::State;
use crate::topology_manager::catalog::{RouterRole, SourceOrigin, TopologyCatalog};
use std::path::Path;
use web_contract::routers::{
    CACHE, DhcpClientOverride, DhcpServerSetting, RouterCollection, RouterInventory, RouterLists,
    RouterOptions, RouterSource, RouterSummary, ShapingPolicy, VERSION,
};

/// A configured file exists and is not empty. The content is never read.
pub(crate) fn configured(path: &str) -> bool {
    !path.is_empty() && std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() > 0)
}

fn summary(
    r: &RouterCfg,
    catalog: Option<&TopologyCatalog>,
    report: Option<&collection::Report>,
    state: Option<&State>,
) -> RouterSummary {
    let id = crate::topology_manager::policy_draft::router_id(&r.name);
    let entry = catalog.and_then(|c| c.routers.iter().find(|c| c.name == r.name));
    let wans: Vec<String> = match (catalog, entry) {
        (Some(c), Some(e)) => c
            .sources
            .iter()
            .filter_map(|s| match &s.origin {
                SourceOrigin::Wan {
                    router_id,
                    endpoint_ref,
                } if *router_id == e.id => Some(endpoint_ref.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    let wan = !wans.is_empty() || entry.is_some_and(|e| e.roles.contains(&RouterRole::Wan));
    let roles = [
        (wan, "wan"),
        (r.pppoe.enabled, "ppp"),
        (r.dhcp.enabled, "dhcp"),
        (r.hotspot.enabled, "hotspot"),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .map(|(_, role)| role.to_string())
    .collect();
    let seen = report.and_then(|c| c.routers.iter().find(|c| c.name == r.name));
    let sources = [
        ("ppp", r.pppoe.enabled),
        ("dhcp", r.dhcp.enabled),
        ("hotspot", r.hotspot.enabled),
    ]
    .into_iter()
    .map(|(kind, enabled)| {
        let s = seen.and_then(|c| c.sources.get(kind));
        RouterSource {
            kind: kind.into(),
            enabled,
            records: s.map(|s| s.records as u64),
            online: s.and_then(|s| s.online).map(|n| n as u64),
            offline: s.and_then(|s| s.offline).map(|n| n as u64),
        }
    })
    .collect();
    let ex = &r.exclude;
    RouterSummary {
        id,
        name: r.name.clone(),
        enabled: r.enabled,
        address: r.address.clone(),
        port: r.port,
        tls: r.tls,
        tls_verify: r.tls_verify,
        tls_ca: configured(&r.tls_ca_file),
        username: r.username.clone(),
        credential: configured(&r.password_file),
        roles,
        wans,
        collection: seen.map(|c| RouterCollection {
            state: c.state.clone(),
            observed_at: c.observed_at,
            subscribers: c.subscribers.len() as u64,
        }),
        sources,
        location: r.latitude.zip(r.longitude).map(|(a, b)| [a, b]),
        options: RouterOptions {
            ppp_per_plan_node: r.pppoe.per_plan_node,
            hotspot_include_mac: r.hotspot.include_mac,
            dhcp_require_bound: r.dhcp.require_bound,
            dhcp_discover: r.dhcp.discover,
            dhcp_shape_unassigned: r.dhcp.shape_unassigned,
            dhcp_servers: r.dhcp.servers.len() as u64,
            dhcp_clients: r.dhcp.clients.len() as u64,
            exclusions: (ex.dhcp_servers.len() + ex.ppp_profiles.len() + ex.ips.len()) as u64,
        },
        lists: Some(RouterLists {
            exclude_dhcp_servers: ex.dhcp_servers.clone(),
            exclude_ppp_profiles: ex.ppp_profiles.clone(),
            exclude_ips: ex.ips.clone(),
            dhcp_clients: r
                .dhcp
                .clients
                .iter()
                .map(|c| DhcpClientOverride {
                    mac: c.mac.clone(),
                    download_mbps: c.download_mbps,
                    upload_mbps: c.upload_mbps,
                    name: c.name.clone(),
                })
                .collect(),
            dhcp_server_mode: mode(r.dhcp.server_mode).into(),
            dhcp_servers: r
                .dhcp
                .servers
                .iter()
                .map(|s| DhcpServerSetting {
                    name: s.name.clone(),
                    mode: mode(s.mode).into(),
                    speed_sources: s.speed_sources.as_ref().map(|v| {
                        v.iter()
                            .map(|x| {
                                match x {
                                    DhcpSource::ServerComment => "server_comment",
                                    DhcpSource::LeaseComment => "lease_comment",
                                    DhcpSource::Settings => "settings",
                                }
                                .to_string()
                            })
                            .collect()
                    }),
                    default_plan_down_mbps: s.default_plan_down_mbps,
                    default_plan_up_mbps: s.default_plan_up_mbps,
                    download_factor: s.download_factor,
                    upload_factor: s.upload_factor,
                })
                .collect(),
            truncated: false,
        }),
        circuits: state.map(|s| {
            s.circuits
                .values()
                .filter(|c| c.router.as_deref() == Some(r.name.as_str()))
                .count() as u64
        }),
    }
}

fn mode(m: DhcpMode) -> &'static str {
    match m {
        DhcpMode::PerSite => "per_site",
        DhcpMode::PerPlan => "per_plan",
    }
}

/// One summary per configured router, in config order.
pub fn project(
    cfg: &Config,
    catalog: Option<&TopologyCatalog>,
    report: Option<&collection::Report>,
    state: Option<&State>,
    config_digest: &str,
    now: u64,
) -> RouterInventory {
    let s = &cfg.safety;
    RouterInventory {
        version: VERSION,
        updated: now,
        // The same freshness window as the collection: one cycle with its retries.
        fresh_for: report.map_or(600, |r| r.fresh_for),
        routers: cfg
            .routers
            .iter()
            .map(|r| summary(r, catalog, report, state))
            .collect(),
        config_digest: config_digest.into(),
        shaping: ShapingPolicy {
            total_circuits: state.map_or(0, |s| s.circuits.len() as u64),
            grace_secs: cfg.runtime.subscriber_grace,
            remove_inactive: cfg.runtime.remove_inactive_entries,
            guard_enabled: s.enabled,
            guard_max_count: s.max_removed_count as u64,
            guard_max_percent: s.max_removed_percent,
            guard_held: state.and_then(|st| st.guard).map(|g| g.count as u64),
        },
        truncated: false,
    }
}

/// Write `routers.json` with mode 0600 through a temporary file and a rename.
pub fn publish(directory: &Path, inventory: &RouterInventory) -> anyhow::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let temp = directory.join(format!(".{CACHE}.tmp-{}", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temp)?;
    file.write_all(&serde_json::to_vec(inventory)?)?;
    file.sync_all()?;
    std::fs::rename(temp, directory.join(CACHE))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology_manager::catalog::{CatalogRouter, CatalogSource};
    use std::collections::BTreeMap;
    use std::os::unix::fs::PermissionsExt;

    fn config(dir: &Path) -> Config {
        let secret = dir.join("access.password");
        std::fs::write(&secret, "not-a-real-secret").unwrap();
        Config::from_toml_str(&format!(
            "[[routers]]\nname=\"Core\"\naddress=\"192.0.2.1\"\nusername=\"api\"\npassword_file=\"{}\"\n\
             tls=true\ntls_ca_file=\"{}\"\n\n\
             [[routers]]\nname=\"Access\"\naddress=\"192.0.2.2\"\nusername=\"api\"\npassword_file=\"{}\"\n\
             latitude=14.5\nlongitude=121.0\n[routers.pppoe]\nenabled=true\nper_plan_node=true\n",
            dir.join("missing.password").display(),
            dir.join("missing-ca.pem").display(),
            secret.display(),
        ))
        .unwrap()
    }
    fn catalog() -> TopologyCatalog {
        TopologyCatalog {
            routers: vec![CatalogRouter {
                id: crate::topology_manager::policy_draft::router_id("Core"),
                name: "Core".into(),
                roles: vec![],
            }],
            sources: vec![CatalogSource {
                excluded: false,
                id: "wan-1".into(),
                name: "ISP A".into(),
                origin: SourceOrigin::Wan {
                    router_id: crate::topology_manager::policy_draft::router_id("Core"),
                    endpoint_ref: "ether1".into(),
                },
                profile_id: None,
                targets: vec![],
            }],
            ..TopologyCatalog::default()
        }
    }
    fn report() -> collection::Report {
        let mut sources = BTreeMap::new();
        sources.insert(
            "ppp".into(),
            collection::Source {
                enabled: true,
                records: 3,
                online: Some(2),
                offline: Some(1),
            },
        );
        collection::Report {
            version: 1,
            updated: 1_000,
            fresh_for: 90,
            routers: vec![collection::Router {
                name: "Access".into(),
                state: "online".into(),
                observed_at: 990,
                sources,
                subscribers: vec![],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn a_wan_only_core_is_listed_with_its_catalog_wan_and_no_collection() {
        let dir = tempfile::tempdir().unwrap();
        let inv = project(
            &config(dir.path()),
            Some(&catalog()),
            Some(&report()),
            None,
            "d",
            1_000,
        );
        assert_eq!(
            inv.routers.len(),
            2,
            "the inventory lists every configured router"
        );
        let core = &inv.routers[0];
        assert_eq!(core.roles, ["wan"]);
        assert_eq!(core.wans, ["ether1"]);
        assert_eq!(
            core.collection, None,
            "the cycle does not read a WAN-only router"
        );
        assert_eq!(
            core.id,
            crate::topology_manager::policy_draft::router_id("Core")
        );
        assert!(
            !core.credential && !core.tls_ca,
            "missing files are not configured"
        );
        assert_eq!(inv.fresh_for, 90);
    }

    #[test]
    fn an_access_router_carries_its_collection_sources_and_options() {
        let dir = tempfile::tempdir().unwrap();
        let inv = project(&config(dir.path()), None, Some(&report()), None, "d", 1_000);
        let access = &inv.routers[1];
        assert_eq!(access.roles, ["ppp"]);
        assert!(access.credential);
        assert_eq!(access.location, Some([14.5, 121.0]));
        assert!(access.options.ppp_per_plan_node);
        let c = access.collection.as_ref().unwrap();
        assert_eq!((c.state.as_str(), c.observed_at), ("online", 990));
        let ppp = &access.sources[0];
        assert_eq!(
            (ppp.kind.as_str(), ppp.enabled, ppp.records, ppp.online),
            ("ppp", true, Some(3), Some(2))
        );
        // A configured source without collection evidence stays unknown, not zero.
        assert_eq!(access.sources[1].records, None);
    }

    #[test]
    fn the_cache_has_no_secret_or_path_and_is_root_only() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = config(dir.path());
        publish(
            dir.path(),
            &project(&cfg, Some(&catalog()), None, None, "d", 1_000),
        )
        .unwrap();
        let path = dir.path().join(CACHE);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("not-a-real-secret"), "{text}");
        assert!(
            !text.contains(".password") && !text.contains(".pem"),
            "{text}"
        );
        assert!(!text.contains(&dir.path().display().to_string()), "{text}");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let back: RouterInventory = serde_json::from_str(&text).unwrap();
        assert_eq!(back.routers.len(), 2);
    }
}
