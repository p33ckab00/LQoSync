//! Explicit read-only preview. Does not save inventory, health state, or settings.
use anyhow::{Context, Result, ensure};
use lqosync_v3::{
    model, routeros,
    wan_health::{self, State},
};
use std::{collections::BTreeMap, path::PathBuf};
#[tokio::main]
async fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    ensure!(
        (1..=2).contains(&args.len()),
        "Usage: wan-health-preview ROOT [ROUNDS:1..60]"
    );
    let rounds = args
        .get(1)
        .map(|s| s.parse::<u32>())
        .transpose()?
        .unwrap_or(1);
    ensure!((1..=60).contains(&rounds), "Rounds must be 1–60");
    let root = PathBuf::from(&args[0]).canonicalize()?;
    ensure!(
        !root.starts_with("/opt/libreqos") && !root.starts_with("/etc/libreqos"),
        "Use the LQoSync data directory"
    );
    let raw = std::fs::read_to_string(root.join("settings.toml"))
        .context("Existing settings.toml required; preview never creates defaults")?;
    let settings: model::Settings = toml::from_str(&raw)?;
    model::validate(&settings)?;
    let revision = wan_health::settings_revision(&settings)?;
    let Some(config) = settings
        .wan_health
        .as_ref()
        .filter(|c| !c.monitors.is_empty())
    else {
        println!(
            "{}",
            serde_json::json!({"configured":false,"router_writes":false,"wans":[]})
        );
        return Ok(());
    };
    let dynamic = settings
        .dynamic
        .as_ref()
        .context("Dynamic topology required")?;
    let mut previous: BTreeMap<String, State> = BTreeMap::new();
    for round in 0..rounds {
        ensure!(
            std::fs::read_to_string(root.join("settings.toml"))? == raw,
            "Settings changed; restart preview with fresh configuration"
        );
        let mut inventories = BTreeMap::new();
        for monitor in &config.monitors {
            let wan = dynamic
                .wans
                .iter()
                .find(|w| w.id == monitor.wan)
                .context("Unknown monitored WAN")?;
            if inventories.contains_key(&wan.router) {
                continue;
            }
            let router = settings
                .routers
                .iter()
                .find(|r| r.id == wan.router)
                .context("Unknown health router")?;
            let inventory = read_health(router, &root).await;
            inventories.insert(wan.router.clone(), inventory);
        }
        ensure!(
            std::fs::read_to_string(root.join("settings.toml"))? == raw,
            "Settings changed during preview read"
        );
        let now = lqosync_v3::orchestrator::now();
        let mut statuses = vec![];
        for monitor in &config.monitors {
            let wan = dynamic
                .wans
                .iter()
                .find(|w| w.id == monitor.wan)
                .context("Unknown monitored WAN")?;
            let evidence = match &inventories[&wan.router] {
                Ok(inv) => {
                    wan_health::from_inventory(config, monitor, inv, &wan.interface, &revision)
                        .map_err(|e| e.to_string())
                }
                Err(_) => Err(
                    "Health read failed; check connectivity, credentials, TLS and read permissions"
                        .into(),
                ),
            };
            let state = wan_health::evaluate(config, previous.get(&monitor.wan), evidence, now)?;
            statuses.push(serde_json::json!({"wan":monitor.wan,"status":state.status,"reason":state.reason,"checked_at":state.checked_at}));
            previous.insert(monitor.wan.clone(), state);
        }
        println!(
            "{}",
            serde_json::json!({"configured":true,"router_writes":false,"round":round+1,"wans":statuses})
        );
        if round + 1 < rounds {
            tokio::time::sleep(std::time::Duration::from_secs(config.interval_seconds)).await;
        }
    }
    Ok(())
}
async fn read_health(
    router: &model::Router,
    root: &std::path::Path,
) -> Result<routeros::Inventory> {
    let password = lqosync_v3::credentials::password(root, router).await?;
    let client = routeros::Client::connect_with_password(router, &password).await?;
    drop(password);
    let started = lqosync_v3::orchestrator::now();
    let mut tables = BTreeMap::new();
    for path in [
        "/system/resource/print",
        "/interface/print",
        "/ip/route/print",
        "/routing/rule/print",
        "/ip/firewall/mangle/print",
        wan_health::NETWATCH,
    ] {
        tables.insert(path.into(), client.read(path).await?);
    }
    Ok(routeros::Inventory {
        router_id: router.id.clone(),
        collected_at: started,
        tables,
        errors: BTreeMap::new(),
    })
}
