//! Allowlisted projection of published caches. No collector, config, or private snapshot access.
use serde_json::Value;
use std::{io::Read, path::Path};
use web_contract::{Component, Overview, Point};
fn read(root: &Path, name: &str) -> Option<Value> {
    let path = root.join(name);
    let canonical = path.canonicalize().ok()?;
    // The worker cache uses the existing root-owned RAM-cache symlink.
    if !canonical.starts_with(root)
        && !(name == "worker-utilization.json" && canonical.starts_with("/dev/shm/lqosync3-0"))
    {
        return None;
    }
    let mut f = std::fs::File::open(&canonical).ok()?;
    let before = f.metadata().ok()?;
    if !before.is_file() || before.len() > 2 * 1024 * 1024 {
        return None;
    }
    let mut bytes = Vec::new();
    f.by_ref()
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    let after = f.metadata().ok()?;
    if bytes.len() > 2 * 1024 * 1024
        || before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
    {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}
fn text(v: &Value, key: &str) -> String {
    v[key]
        .as_str()
        .filter(|s| s.len() < 96)
        .unwrap_or("unknown")
        .to_owned()
}
fn fresh(at: u64, now: u64, age: u64) -> bool {
    at > 0 && now >= at && now - at <= age
}
pub fn project(root: &Path, now: u64) -> Overview {
    let root = root.canonicalize().unwrap_or_else(|_| root.into());
    let status = read(&root, "status.json").unwrap_or_default();
    let collection = read(&root, "collection.json").unwrap_or_default();
    let health = read(&root, "system-health.json").unwrap_or_default();
    let workers = read(&root, "worker-utilization.json").unwrap_or_default();
    let updated = status["updated"].as_u64();
    let current = status["version"] == 3 && updated.is_some_and(|at| fresh(at, now, 120));
    let coherent = current
        && collection["version"] == 1
        && status["pid"].as_u64().is_some()
        && collection["pid"] == status["pid"]
        && collection["cycle_seq"]
            .as_u64()
            .zip(status["cycle_seq"].as_u64())
            .is_some_and(|(a, b)| a >= b)
        && fresh(
            collection["updated"].as_u64().unwrap_or(0),
            now,
            collection["fresh_for"].as_u64().unwrap_or(0).min(600),
        );
    let mut view = Overview {
        observed_at: updated,
        stale: !coherent,
        mode: text(&status, "mode"),
        target: text(&status, "target"),
        cycle_ms: status["cycle_ms"].as_u64(),
        health: if current && fresh(health["updated"].as_u64().unwrap_or(0), now, 120) {
            text(&health, "overall")
        } else {
            "UNKNOWN".into()
        },
        publication: if current && fresh(health["updated"].as_u64().unwrap_or(0), now, 120) {
            text(&health["update"], "phase")
        } else {
            "unknown".into()
        },
        traffic_stale: true,
        ..Default::default()
    };
    if coherent && let Some(routers) = collection["routers"].as_array() {
        view.routers = Some(routers.len() as u64);
        view.routers_online =
            Some(routers.iter().filter(|r| r["state"] == "online").count() as u64);
        let all_current = routers
            .iter()
            .all(|r| r["state"] == "online" && r["subscribers"].is_array());
        if all_current {
            view.subscribers = Some(
                routers
                    .iter()
                    .filter_map(|r| r["subscribers"].as_array())
                    .map(|s| s.len() as u64)
                    .sum(),
            );
            let sum = |key: &str| -> Option<u64> {
                if routers.is_empty() {
                    return None;
                }
                routers
                    .iter()
                    .map(|r| r["sources"]["ppp"][key].as_u64())
                    .try_fold(0u64, |sum, v| sum.checked_add(v?))
            };
            view.ppp_online = sum("online");
            view.ppp_offline = sum("offline");
        } else {
            view.stale = true;
        }
    }
    if let Some(items) = health["components"].as_array() {
        view.components = items
            .iter()
            .take(32)
            .map(|v| Component {
                name: text(v, "component"),
                state: if fresh(
                    v["observed_at"].as_u64().unwrap_or(0),
                    now,
                    v["max_age"].as_u64().unwrap_or(0),
                ) {
                    text(v, "state")
                } else {
                    "UNKNOWN".into()
                },
                observed_at: v["observed_at"].as_u64().unwrap_or(0),
            })
            .collect();
    }
    if workers["version"] == 1
        && let Some(items) = workers["traffic"].as_array()
    {
        view.traffic = items
            .iter()
            .rev()
            .take(300)
            .filter_map(|v| {
                let p: Point = serde_json::from_value(v.clone()).ok()?;
                (p.down_mbps.is_finite()
                    && p.up_mbps.is_finite()
                    && p.down_mbps >= 0.0
                    && p.up_mbps >= 0.0)
                    .then_some(p)
            })
            .collect();
        view.traffic.reverse();
        view.traffic_stale =
            workers["stale"] != false || !fresh(workers["updated"].as_u64().unwrap_or(0), now, 30);
    }
    view
}
