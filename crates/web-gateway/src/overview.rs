//! Allowlisted projection of published caches. No collector, config, or private snapshot access.
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::{io::Read, path::Path};
use web_contract::{Component, Overview, Point, WanCapacity};
pub(crate) fn read<T: DeserializeOwned>(root: &Path, name: &str) -> Option<T> {
    read_within(root, name, 2 * 1024 * 1024)
}
/// `read` with another size limit, for a cache that grows with the subscriber count.
pub(crate) fn read_within<T: DeserializeOwned>(root: &Path, name: &str, max: u64) -> Option<T> {
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
    if !before.is_file() || before.len() > max {
        return None;
    }
    let mut bytes = Vec::new();
    f.by_ref().take(max + 1).read_to_end(&mut bytes).ok()?;
    let after = f.metadata().ok()?;
    if bytes.len() as u64 > max
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
    let status: Value = read(&root, "status.json").unwrap_or_default();
    let collection: Value = read(&root, "collection.json").unwrap_or_default();
    let health: Value = read(&root, "system-health.json").unwrap_or_default();
    let workers: WorkerTraffic = read(&root, "worker-utilization.json").unwrap_or_default();
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
    if current {
        view.wan_capacity = serde_json::from_value::<WanCapacity>(status["wan_capacity"].clone())
            .ok()
            .filter(|v| {
                v.down_bps > 0
                    && v.up_bps > 0
                    && v.wan_count > 0
                    && v.down_bps < (1u64 << 53)
                    && v.up_bps < (1u64 << 53)
            })
            .map(|mut v| {
                // Bound the per-WAN labels so they cannot crowd the chart or the overview budget.
                let rate = |r: u64| r > 0 && r < (1u64 << 53);
                v.wans.retain(|w| {
                    rate(w.down_bps) && rate(w.up_bps) && !w.name.is_empty() && w.name.len() <= 64
                });
                v.wans.truncate(16);
                v
            });
    }
    if coherent && let Some(routers) = collection["routers"].as_array() {
        // The collection lists only routers with a subscriber role; the inventory below lists all.
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
            view.ipoe_active = if routers.is_empty() {
                None
            } else {
                routers
                    .iter()
                    .map(
                        |router| match router["sources"]["dhcp"]["enabled"].as_bool()? {
                            false => Some(0u64),
                            true => {
                                let rows = router["subscribers"].as_array()?;
                                if rows.iter().any(|r| {
                                    r["source"] == "dhcp"
                                        && !matches!(
                                            r["state"].as_str(),
                                            Some(
                                                "bound"
                                                    | "unbound"
                                                    | "disabled"
                                                    | "unknown"
                                                    | "stale"
                                            )
                                        )
                                }) {
                                    return None;
                                }
                                Some(
                                    rows.iter()
                                        .filter(|r| r["source"] == "dhcp" && r["state"] == "bound")
                                        .count() as u64,
                                )
                            }
                        },
                    )
                    .try_fold(0u64, |total, n| total.checked_add(n?))
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
    project_traffic(&workers, now, &mut view, 300);
    view.dpdk = Some(crate::dpdk::project(
        &workers.dpdk_fields,
        workers.version,
        workers.updated,
        workers.stale,
        now,
    ));
    crate::dpdk::project_nic(&root, now, view.dpdk.as_mut().unwrap());
    crate::dpdk::bound(view.dpdk.as_mut().unwrap());
    bound_traffic(&mut view);
    // Managed routers: every configured router, including a WAN-only core that the cycle does not read.
    if coherent && let Some(inventory) = crate::routers::load(&root) {
        view.routers = Some(inventory.routers.len() as u64);
    }
    view
}

/// Every retained traffic sample of the three sources (Actual, Mapped, Unmapped), for the history seed.
/// The core keeps about one hour of one-second samples. The result is not bounded for IPC.
pub fn history_samples(root: &Path, now: u64) -> [Vec<Point>; 3] {
    let root = root.canonicalize().unwrap_or_else(|_| root.into());
    let workers: WorkerTraffic = read(&root, "worker-utilization.json").unwrap_or_default();
    let mut view = Overview::default();
    project_traffic(&workers, now, &mut view, usize::MAX);
    let (mapped, unmapped) = view
        .classification
        .map(|c| (c.mapped.points, c.unmapped.points))
        .unwrap_or_default();
    [view.traffic, mapped, unmapped]
}

/// Refresh traffic and worker analytics without decoding subscriber collections each second.
pub fn refresh_traffic(root: &Path, now: u64, view: &mut Overview) {
    let root = root.canonicalize().unwrap_or_else(|_| root.into());
    let workers: WorkerTraffic = read(&root, "worker-utilization.json").unwrap_or_default();
    project_traffic(&workers, now, view, 300);
    let mut dpdk = crate::dpdk::project(
        &workers.dpdk_fields,
        workers.version,
        workers.updated,
        workers.stale,
        now,
    );
    crate::dpdk::project_nic(&root, now, &mut dpdk);
    crate::dpdk::bound(&mut dpdk);
    view.dpdk = Some(dpdk);
    bound_traffic(view);
}

// Decode the selected telemetry fields. Unrelated cache fields stay outside the browser contract.
#[derive(Default, Deserialize)]
struct WorkerTraffic {
    version: Option<u64>,
    updated: Option<u64>,
    stale: Option<bool>,
    #[serde(default)]
    traffic: Vec<Value>,
    #[serde(flatten)]
    dpdk_fields: crate::dpdk::Analytics,
}

fn project_traffic(workers: &WorkerTraffic, now: u64, view: &mut Overview, limit: usize) {
    view.traffic_stale = true;
    if let Some(c) = &mut view.classification {
        c.mapped.stale = true;
        c.unmapped.stale = true;
    }
    if workers.version == Some(1) {
        view.classification = Some(crate::dpdk::classification(
            &workers.dpdk_fields,
            workers.stale == Some(false) && fresh(workers.updated.unwrap_or(0), now, 6),
            now,
            limit,
        ));
        let items = &workers.traffic;
        let mut samples = items
            .iter()
            .rev()
            .take(limit)
            .filter_map(|v| {
                let at = v["at"].as_u64().filter(|at| *at > 0 && *at <= now)?;
                let point = serde_json::from_value::<Point>(v.clone()).ok().filter(|p| {
                    p.down_mbps.is_finite()
                        && p.up_mbps.is_finite()
                        && p.down_mbps >= 0.0
                        && p.up_mbps >= 0.0
                });
                Some((at, point))
            })
            .collect::<Vec<_>>();
        samples.sort_by_key(|(at, _)| *at);
        samples.dedup_by_key(|(at, _)| *at);
        let mut gap = false;
        view.traffic = samples
            .into_iter()
            .filter_map(|(_, p)| match p {
                Some(mut p) => {
                    p.gap_before |= gap;
                    gap = false;
                    Some(p)
                }
                None => {
                    gap = true;
                    None
                }
            })
            .collect();
        view.traffic_stale = workers.stale != Some(false)
            || !fresh(workers.updated.unwrap_or(0), now, 30)
            || !view.traffic.last().is_some_and(|p| fresh(p.at, now, 30));
    }
}

/// Reserve space for the IPC reply envelope. Remove only the oldest observations.
fn bound_traffic(view: &mut Overview) {
    const BUDGET: usize = 60 * 1024;
    while serde_json::to_vec(view).map_or(usize::MAX, |v| v.len()) > BUDGET {
        let oldest = view
            .traffic
            .first()
            .map(|p| p.at)
            .into_iter()
            .chain(
                view.classification
                    .iter()
                    .flat_map(|c| [&c.mapped.points, &c.unmapped.points])
                    .filter_map(|p| p.first().map(|p| p.at)),
            )
            .min();
        let Some(oldest) = oldest else {
            break;
        };
        // Batch removal keeps projection work bounded for high precision histories.
        let cutoff = oldest.saturating_add(15);
        view.traffic.retain(|p| p.at > cutoff);
        if let Some(c) = &mut view.classification {
            c.mapped.points.retain(|p| p.at > cutoff);
            c.unmapped.points.retain(|p| p.at > cutoff);
            c.history_capped = true;
        }
    }
}
