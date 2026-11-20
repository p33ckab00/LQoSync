//! Allowlisted cache projection. Never opens a DPDK socket or runs a collector.
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashSet, path::Path};
use web_contract::dpdk::{Dpdk, Gauge, Port, Reading, Source, Worker};

#[derive(Default, Deserialize)]
pub(crate) struct Analytics {
    #[serde(default)]
    workers: Vec<Value>,
    #[serde(default)]
    pressure: Value,
    #[serde(default)]
    queue: Value,
    #[serde(default)]
    health: Value,
    #[serde(default)]
    classification: Classification,
}
#[derive(Default, Deserialize)]
struct Classification {
    #[serde(default)]
    mapped: Series,
    #[serde(default)]
    unmapped: Series,
}
#[derive(Default, Deserialize)]
struct Series {
    bytes: Option<Bytes>,
    #[serde(default)]
    history: Vec<Rate>,
}
#[derive(Deserialize)]
struct Bytes {
    download_bytes: u64,
    upload_bytes: u64,
}
#[derive(Deserialize)]
struct Rate {
    at: u64,
    down_mbps: Option<f64>,
    up_mbps: Option<f64>,
}

fn text(v: &Value, max: usize) -> Option<String> {
    v.as_str()
        .filter(|s| !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control))
        .map(str::to_owned)
}
fn reading(label: &str, value: impl ToString) -> Reading {
    Reading {
        label: label.into(),
        value: value.to_string(),
    }
}
fn counters(rows: &mut Vec<Reading>, v: &Value, keys: &[(&str, &str)]) {
    for (key, label) in keys {
        if let Some(n) = v[key].as_u64() {
            rows.push(reading(label, n));
        }
    }
}
fn source(updated: Option<u64>, stale: bool, now: u64, age: u64, present: bool) -> Source {
    let mut s = Source {
        updated: updated.filter(|at| *at > 0),
        stale,
        ..Default::default()
    };
    s.stale |= !present || !s.live(now, age);
    s.state = if !present {
        "UNAVAILABLE"
    } else if s.stale {
        "STALE"
    } else {
        "LIVE"
    }
    .into();
    s
}

pub(crate) fn project(
    a: &Analytics,
    version: Option<u64>,
    updated: Option<u64>,
    stale: Option<bool>,
    now: u64,
) -> Dpdk {
    let mut ids = HashSet::new();
    let valid = version == Some(1)
        && a.workers.len() <= 4096
        && a.workers.iter().all(|w| {
            w["queue_id"]
                .as_u64()
                .is_some_and(|id| id <= u16::MAX as u64 && ids.insert(id))
                && w["busy_ppm"].as_u64().is_some_and(|n| n <= 1_000_000)
        });
    let mut d = Dpdk {
        sampled_at: now,
        analytics: source(
            updated,
            stale != Some(false),
            now,
            6,
            valid && !a.workers.is_empty(),
        ),
        health: source(
            a.health["updated"].as_u64(),
            a.health["stale"] != false,
            now,
            15,
            version == Some(1) && a.health["summary"].is_object(),
        ),
        nic: source(None, true, now, 15, false),
        ..Default::default()
    };
    if valid && !a.workers.is_empty() {
        d.worker_count = a.workers.len() as u64;
        d.mean_busy_ppm = Some(
            a.workers
                .iter()
                .map(|w| w["busy_ppm"].as_u64().unwrap())
                .sum::<u64>()
                / d.worker_count,
        );
        let mut rows = a.workers.iter().collect::<Vec<_>>();
        rows.sort_by_key(|w| {
            (
                std::cmp::Reverse(w["busy_ppm"].as_u64().unwrap()),
                w["queue_id"].as_u64().unwrap(),
            )
        });
        d.workers = rows
            .into_iter()
            .take(16)
            .map(|w| {
                let busy = w["busy_ppm"].as_u64().unwrap();
                let mut details = Vec::new();
                if let Some(idle) = w["idle_ppm"]
                    .as_u64()
                    .filter(|idle| busy.checked_add(*idle) == Some(1_000_000))
                {
                    details.push(reading(
                        "Reported idle",
                        format!("{:.1}%", idle as f64 / 10000.0),
                    ));
                }
                counters(
                    &mut details,
                    w,
                    &[
                        ("laps_per_s", "Laps/s (reported)"),
                        ("busy_laps", "Busy laps"),
                        ("cycles_per_busy_lap", "Cycles/busy lap"),
                    ],
                );
                if let Some(phases) = w["phase_cycles"]
                    .as_array()
                    .filter(|p| p.len() == 10 && p.iter().all(|v| v.as_u64().is_some()))
                {
                    for (phase, n) in [
                        "RX",
                        "Dissect",
                        "Select",
                        "Charge",
                        "Leaf",
                        "TX",
                        "Begin cycle",
                        "Drop overdue",
                        "Handoff",
                        "Empty RX",
                    ]
                    .iter()
                    .zip(phases)
                    {
                        details.push(reading(
                            &format!("{phase} cycles (total)"),
                            n.as_u64().unwrap(),
                        ));
                    }
                }
                Worker {
                    id: w["queue_id"].as_u64().unwrap() as u16,
                    busy_ppm: busy,
                    backlog_mean: w["backlog_mean"].as_u64(),
                    details,
                }
            })
            .collect();
    }
    if version == Some(1) {
        project_pressure(a, &mut d);
        project_health(&a.health, now, &mut d.health);
        counters(
            &mut d.analytics.details,
            &a.queue["totals"],
            &[
                ("drop_pkts", "Queue drops (total)"),
                ("aqm_marks", "AQM marks (total)"),
                ("backlog_bytes", "Queue backlog (bytes)"),
            ],
        );
        for (key, total, label) in [
            ("drop_delta", "drop_pkts", "Queue drops/sample"),
            ("aqm_delta", "aqm_marks", "AQM marks/sample"),
        ] {
            if let Some(delta) = a.queue[key].as_u64().filter(|n| {
                a.queue["totals"][total]
                    .as_u64()
                    .is_some_and(|total| *n <= total)
            }) {
                d.analytics.details.push(reading(label, delta));
            }
        }
        for (name, series) in [
            ("Mapped", &a.classification.mapped),
            ("Unmapped", &a.classification.unmapped),
        ] {
            if let Some(b) = &series.bytes {
                d.analytics.details.push(reading(
                    &format!("{name} bytes D/U"),
                    format!("{} / {}", b.download_bytes, b.upload_bytes),
                ));
                if let Some(p) = series
                    .history
                    .last()
                    .filter(|p| p.at > 0 && now.checked_sub(p.at).is_some_and(|age| age <= 6))
                    && let Some((down, up)) = p
                        .down_mbps
                        .zip(p.up_mbps)
                        .filter(|(a, b)| a.is_finite() && b.is_finite() && *a >= 0.0 && *b >= 0.0)
                {
                    d.analytics.details.push(reading(
                        &format!("{name} enqueue D/U"),
                        format!("{down:.2} / {up:.2} Mbps"),
                    ));
                }
            }
        }
    }
    d
}
fn project_pressure(a: &Analytics, d: &mut Dpdk) {
    let p = &a.pressure;
    let diag = &p["diagnostics"];
    if [
        ("pool_free_mbufs", "pool_total_mbufs"),
        ("ring_used", "ring_capacity"),
        ("rings_over_75", "ring_count"),
    ]
    .iter()
    .any(|(used, total)| {
        diag[*used]
            .as_u64()
            .zip(diag[*total].as_u64())
            .is_some_and(|(u, t)| u > t)
    }) {
        d.analytics
            .details
            .push(reading("Buffer/ring pressure", "UNKNOWN"));
        return;
    }

    let pool = diag["pool_total_mbufs"]
        .as_u64()
        .zip(diag["pool_free_mbufs"].as_u64())
        .filter(|(t, f)| f <= t);
    if let Some((total, free)) = pool.filter(|(t, _)| *t > 0) {
        d.pressure.push(Gauge {
            label: "Pool in use".into(),
            used: total - free,
            total,
        });
    }
    if let Some((used, total)) = diag["ring_used"]
        .as_u64()
        .zip(diag["ring_capacity"].as_u64())
        .filter(|(u, t)| *t > 0 && u <= t)
    {
        d.pressure.push(Gauge {
            label: "Handoff ring use".into(),
            used,
            total,
        });
    }
    counters(
        &mut d.analytics.details,
        diag,
        &[
            ("pool_free_mbufs", "Free pool buffers"),
            ("pool_total_mbufs", "Total pool buffers"),
            ("ring_count", "Handoff rings"),
            ("rings_over_75", "Rings above 75%"),
            ("ring_used", "Ring slots used"),
            ("ring_capacity", "Ring capacity"),
        ],
    );
    counters(
        &mut d.analytics.details,
        p,
        &[
            ("rx_nombuf_delta", "RX buffer exhaustion/sample"),
            ("pool_drop_delta", "Pool drops/sample"),
        ],
    );
    let alert = p["rx_nombuf_delta"].as_u64().is_some_and(|n| n > 0)
        || p["pool_drop_delta"].as_u64().is_some_and(|n| n > 0)
        || pool.is_some_and(|(t, f)| t > 0 && u128::from(f) * 10 <= u128::from(t))
        || diag["rings_over_75"].as_u64().is_some_and(|n| n > 0)
        || d.pressure.iter().any(|g| {
            g.label == "Handoff ring use" && u128::from(g.used) * 4 >= u128::from(g.total) * 3
        });
    let ring_known = d.pressure.iter().any(|g| g.label == "Handoff ring use")
        || ["ring_count", "ring_capacity", "ring_used", "rings_over_75"]
            .iter()
            .all(|k| diag[*k].as_u64() == Some(0));
    let complete = pool.is_some_and(|(t, _)| t > 0)
        && ring_known
        && diag["rings_over_75"].as_u64().is_some()
        && p["rx_nombuf_delta"].as_u64().is_some()
        && p["pool_drop_delta"].as_u64().is_some();
    d.analytics.details.push(reading(
        "Buffer/ring pressure",
        if d.analytics.stale {
            "UNKNOWN"
        } else if alert {
            "ALERT"
        } else if complete {
            "CLEAR"
        } else {
            "UNKNOWN"
        },
    ));
}
fn project_health(v: &Value, now: u64, out: &mut Source) {
    let s = &v["summary"];
    let workers = s["worker_count"].as_u64().filter(|n| *n <= 4096);
    let valid = text(&s["status"], 32).is_some()
        && text(&s["version"], 128).is_some()
        && s["uptime_ms"].as_u64().is_some()
        && s["missing_rollups"].as_u64().is_some()
        && workers.is_some()
        && ["missing_rollups", "aqm_degraded_workers"]
            .iter()
            .all(|k| s[*k].is_null() || s[*k].as_u64().is_some_and(|n| n <= workers.unwrap()));
    if !valid {
        out.stale = true;
        out.state = "UNKNOWN".into();
        return;
    }
    out.state = if out.live(now, 15) {
        text(&s["status"], 32).unwrap()
    } else {
        "UNKNOWN".into()
    };
    for (key, label, max) in [
        ("status", "Last reported status", 32),
        ("version", "Bridge version", 128),
        ("topology_tier", "Topology tier", 64),
    ] {
        if let Some(t) = text(&s[key], max) {
            out.details.push(reading(label, t));
        }
    }
    counters(
        &mut out.details,
        s,
        &[
            ("uptime_ms", "Uptime (ms)"),
            ("worker_count", "Health workers"),
            ("missing_rollups", "Missing rollups"),
            ("max_rollup_age_ms", "Oldest rollup (ms)"),
            ("aqm_degraded_workers", "AQM degraded workers"),
        ],
    );
    if let Some(empty) = s["classification_empty"].as_bool() {
        out.details.push(reading("Classification empty", empty));
    }
    if let Some(subsystems) = s["degraded_subsystems"]
        .as_array()
        .filter(|s| s.len() <= 32)
        && let Some(names) = subsystems
            .iter()
            .map(|v| text(v, 64))
            .collect::<Option<Vec<_>>>()
    {
        for name in names {
            out.details.push(reading("Degraded subsystem", name));
        }
    }
}

pub(crate) fn project_nic(root: &Path, now: u64, d: &mut Dpdk) {
    let v: Value = crate::overview::read(root, "nic-telemetry.json").unwrap_or_default();
    let ports = v["ports"].as_array();
    let mut ids = HashSet::new();
    let valid = v["version"] == 1
        && ports.is_some_and(|p| {
            p.len() <= 8
                && p.iter().all(|v| {
                    v["id"]
                        .as_u64()
                        .is_some_and(|id| id <= u16::MAX as u64 && ids.insert(id))
                })
        });
    d.nic = source(v["updated"].as_u64(), v["stale"] != false, now, 15, valid);
    if !valid {
        return;
    }
    if let Some(t) = text(&v["dpdk_version"], 128) {
        d.nic.details.push(reading("DPDK version", t));
    }
    counters(
        &mut d.nic.details,
        &v,
        &[("interval_ms", "Counter interval (ms)")],
    );
    if v["ports_capped"] == true {
        d.nic
            .details
            .push(reading("Port list", "Capped by collector"));
        d.details_capped = true;
    }
    d.ports = ports
        .unwrap()
        .iter()
        .map(|p| {
            let mut details = Vec::new();
            if let Some(t) = text(&p["duplex"], 32) {
                details.push(reading("Duplex", t));
            }
            for key in [
                "ipackets",
                "opackets",
                "ibytes",
                "obytes",
                "imissed",
                "ierrors",
                "oerrors",
                "rx_nombuf",
            ] {
                if let Some(n) = p["stats"][key].as_u64() {
                    details.push(reading(&format!("{key} (total)"), n));
                }
                if p["reset"] != true
                    && let Some(n) = p["deltas"][key].as_u64().filter(|delta| {
                        p["stats"][key]
                            .as_u64()
                            .is_some_and(|total| *delta <= total)
                    })
                {
                    details.push(reading(&format!("{key}/sample"), n));
                }
            }
            if let Some(x) = p["xstats"].as_object() {
                if x.len() > 16 || p["xstats_capped"] == true {
                    d.details_capped = true;
                }
                for (key, n) in x
                    .iter()
                    .filter(|(k, v)| {
                        k.len() <= 96 && !k.chars().any(char::is_control) && v.as_u64().is_some()
                    })
                    .take(16)
                {
                    details.push(reading(&format!("NIC {key}"), n.as_u64().unwrap()));
                }
            }
            Port {
                id: p["id"].as_u64().unwrap() as u16,
                name: text(&p["name"], 128).unwrap_or("Unknown".into()),
                state: text(&p["status"], 32).unwrap_or("Unknown".into()),
                speed_mbps: p["speed_mbps"].as_u64().filter(|n| *n <= u32::MAX as u64),
                reset: p["reset"] == true,
                details,
            }
        })
        .collect();
}
pub(crate) fn bound(d: &mut Dpdk) {
    // Keep health and pressure verdicts. Trim optional per-row details in batches.
    while serde_json::to_vec(d).is_ok_and(|v| v.len() > 16 * 1024) {
        d.details_capped = true;
        let mut removed = 0;
        for rows in d
            .ports
            .iter_mut()
            .map(|p| &mut p.details)
            .chain(d.workers.iter_mut().map(|w| &mut w.details))
        {
            let keep = rows.len() / 2;
            removed += rows.len() - keep;
            rows.truncate(keep);
        }
        if removed == 0 {
            break;
        }
    }
}

/// Keep classification numeric. Human-readable diagnostic rows are not chart inputs.
pub(crate) fn classification(
    a: &Analytics,
    live: bool,
    now: u64,
) -> web_contract::ClassificationTraffic {
    let series = |s: &Series| {
        let mut samples = s
            .history
            .iter()
            .rev()
            .take(300)
            .filter(|p| p.at > 0 && p.at <= now)
            .collect::<Vec<_>>();
        samples.sort_by_key(|p| p.at);
        samples.dedup_by_key(|p| p.at);
        let mut gap = false;
        let points = samples
            .into_iter()
            .filter_map(|p| {
                let valid = |v: Option<f64>| v.filter(|v| v.is_finite() && *v >= 0.0);
                match (valid(p.down_mbps), valid(p.up_mbps)) {
                    (Some(down_mbps), Some(up_mbps)) => {
                        let point = web_contract::Point {
                            at: p.at,
                            down_mbps,
                            up_mbps,
                            gap_before: gap,
                        };
                        gap = false;
                        Some(point)
                    }
                    _ => {
                        gap = true;
                        None
                    }
                }
            })
            .collect::<Vec<_>>();
        // The last raw sample can be a gap even when an older numeric point exists.
        let stale = !live
            || s.bytes.is_none()
            || gap
            || !points.last().is_some_and(|p| now.saturating_sub(p.at) <= 6);
        web_contract::TrafficSeries {
            points,
            available: s.bytes.is_some(),
            stale,
        }
    };
    web_contract::ClassificationTraffic {
        mapped: series(&a.classification.mapped),
        unmapped: series(&a.classification.unmapped),
        history_capped: false,
    }
}
