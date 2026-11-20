use serde_json::{Value, json};
use web_gateway::overview::{project, refresh_traffic};
fn save(root: &std::path::Path, name: &str, value: &Value) {
    std::fs::write(root.join(name), serde_json::to_vec(value).unwrap()).unwrap();
}
fn analytics() -> Value {
    json!({"version":1,"updated":1000,"stale":false,
      "workers":[{"queue_id":0,"busy_ppm":0,"backlog_mean":0},{"queue_id":1,"busy_ppm":750000,"idle_ppm":250000,"backlog_mean":3}],
      "pressure":{"diagnostics":{"pool_free_mbufs":50,"pool_total_mbufs":100,"ring_count":1,"ring_capacity":100,"ring_used":75,"rings_over_75":1},
        "rx_nombuf_delta":0,"pool_drop_delta":0},
      "queue":{"totals":{"drop_pkts":0,"aqm_marks":10,"backlog_bytes":2048},"drop_delta":0,"aqm_delta":2},
      "health":{"updated":990,"stale":false,"summary":{"status":"degraded","version":"fixture","uptime_ms":10000,"worker_count":2,"missing_rollups":0,"aqm_degraded_workers":1,"topology_tier":"pair","classification_empty":false,"degraded_subsystems":["aqm"]}},
      "traffic":[{"at":1000,"down_mbps":20.0,"up_mbps":2.0}]})
}
fn nic() -> Value {
    json!({"version":1,"updated":990,"stale":false,"interval_ms":10000,"dpdk_version":"fixture",
      "ports":[{"id":0,"name":"port0","status":"up","speed_mbps":10000,"reset":false,
        "stats":{"ipackets":0,"ierrors":18446744073709551615u64},"deltas":{"ipackets":0,"ierrors":1}}]})
}
#[test]
fn sources_match_tui_ages_and_keep_zero_separate_from_unknown() {
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), "worker-utilization.json", &analytics());
    save(dir.path(), "nic-telemetry.json", &nic());
    let d = project(dir.path(), 1000).dpdk.unwrap();
    assert_eq!(d.worker_count, 2);
    assert_eq!(d.mean_busy_ppm, Some(375000));
    assert_eq!(d.workers[0].id, 1);
    assert_eq!(d.workers[1].backlog_mean, Some(0));
    assert!(d.analytics.live(1006, 6));
    assert!(!d.analytics.live(1007, 6));
    assert!(d.health.live(1005, 15));
    assert!(!d.health.live(1006, 15));
    assert_eq!(d.health.state, "degraded");
    assert!(d.nic.live(1005, 15));
    assert!(
        d.analytics
            .details
            .iter()
            .any(|r| r.label == "Queue drops (total)" && r.value == "0")
    );
    assert!(
        d.analytics
            .details
            .iter()
            .any(|r| r.label == "Buffer/ring pressure" && r.value == "ALERT")
    );
    assert!(
        d.ports[0]
            .details
            .iter()
            .any(|r| r.value == u64::MAX.to_string())
    );
}
#[test]
fn missing_old_future_stale_and_invalid_versions_never_claim_live() {
    let dir = tempfile::tempdir().unwrap();
    let missing = project(dir.path(), 1000).dpdk.unwrap();
    assert!(missing.analytics.stale && missing.health.stale && missing.nic.stale);
    assert_eq!(missing.mean_busy_ppm, None);
    for (key, value) in [
        ("updated", json!(1001)),
        ("updated", json!(990)),
        ("stale", json!(true)),
        ("version", json!(2)),
    ] {
        let mut a = analytics();
        a[key] = value;
        save(dir.path(), "worker-utilization.json", &a);
        assert!(project(dir.path(), 1000).dpdk.unwrap().analytics.stale);
    }
    let mut n = nic();
    n["updated"] = json!(1001);
    save(dir.path(), "nic-telemetry.json", &n);
    assert!(project(dir.path(), 1000).dpdk.unwrap().nic.stale);
}
#[test]
fn duplicate_workers_out_of_range_busy_and_invalid_extended_values_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = analytics();
    a["workers"][1]["queue_id"] = json!(0);
    save(dir.path(), "worker-utilization.json", &a);
    assert!(project(dir.path(), 1000).dpdk.unwrap().workers.is_empty());
    a = analytics();
    a["workers"][0]["busy_ppm"] = json!(1000001);
    save(dir.path(), "worker-utilization.json", &a);
    assert!(project(dir.path(), 1000).dpdk.unwrap().workers.is_empty());
    a = analytics();
    a["workers"][0]["idle_ppm"] = json!(1);
    a["workers"][0]["phase_cycles"] = json!([1, 2]);
    save(dir.path(), "worker-utilization.json", &a);
    let d = project(dir.path(), 1000).dpdk.unwrap();
    assert!(
        d.workers
            .iter()
            .find(|w| w.id == 0)
            .unwrap()
            .details
            .is_empty()
    );
}
#[test]
fn worker_refresh_does_not_replace_independent_nic_evidence() {
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), "worker-utilization.json", &analytics());
    save(dir.path(), "nic-telemetry.json", &nic());
    let mut v = project(dir.path(), 1000);
    let mut a = analytics();
    a["updated"] = json!(1001);
    a["workers"][0]["busy_ppm"] = json!(1000000);
    save(dir.path(), "worker-utilization.json", &a);
    refresh_traffic(dir.path(), 1001, &mut v);
    let d = v.dpdk.unwrap();
    assert_eq!(d.analytics.updated, Some(1001));
    assert_eq!(d.mean_busy_ppm, Some(875000));
    assert_eq!(d.nic.updated, Some(990));
    assert_eq!(d.ports.len(), 1);
    let mut v = project(dir.path(), 1000);
    // Cap evidence belongs to the current source, not a previous projection.
    let mut capped = nic();
    capped["ports_capped"] = json!(true);
    save(dir.path(), "nic-telemetry.json", &capped);
    refresh_traffic(dir.path(), 1006, &mut v);
    let d = v.dpdk.unwrap();
    assert!(d.nic.stale);
    assert_eq!(d.nic.state, "STALE");
    assert!(d.details_capped);
}
#[test]
fn one_second_refresh_reads_new_nic_counters_with_traffic() {
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), "worker-utilization.json", &analytics());
    save(dir.path(), "nic-telemetry.json", &nic());
    let mut view = project(dir.path(), 1000);
    let mut n = nic();
    n["updated"] = json!(1001);
    n["interval_ms"] = json!(1000);
    n["ports"][0]["stats"]["ipackets"] = json!(42);
    n["ports"][0]["deltas"]["ipackets"] = json!(7);
    save(dir.path(), "nic-telemetry.json", &n);
    refresh_traffic(dir.path(), 1001, &mut view);
    let d = view.dpdk.unwrap();
    assert_eq!(d.sampled_at, 1001);
    assert_eq!(d.nic.updated, Some(1001));
    assert_eq!(d.ports[0].activity.rx_packets, Some(7));
    assert_eq!(d.ports[0].activity.interval_ms, Some(1000));
    assert!(d.nic.live(1001, 15));
}
#[test]
fn worker_and_detail_bounds_fit_the_broker_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = analytics();
    a["health"]["summary"]["degraded_subsystems"] =
        json!((0..32).map(|_| "s".repeat(64)).collect::<Vec<_>>());
    a["workers"] = json!(
        (0..4096)
            .map(
                |i| json!({"queue_id":i,"busy_ppm":i*200,"backlog_mean":18446744073709551615u64,
      "laps_per_s":18446744073709551615u64,"phase_cycles":vec![u64::MAX;10]})
            )
            .collect::<Vec<_>>()
    );
    a["traffic"] = json!(
        (701..=1000)
            .map(|at| json!({"at":at,"down_mbps":1.0e308,"up_mbps":1.0e308}))
            .collect::<Vec<_>>()
    );
    for key in ["mapped", "unmapped"] {
        a["classification"][key] =
            json!({"bytes":{"download_bytes":1,"upload_bytes":1},"history":a["traffic"]});
    }
    save(dir.path(), "worker-utilization.json", &a);
    let mut n = nic();
    n["ports"]=json!((0..8).map(|id|json!({"id":id,"name":"a".repeat(128),"status":"up","speed_mbps":10000,
      "stats":{"ipackets":18446744073709551615u64},"xstats":(0..32).map(|i|(format!("{}-{i}","x".repeat(80)),json!(18446744073709551615u64))).collect::<serde_json::Map<_,_>>()
    })).collect::<Vec<_>>());
    save(dir.path(), "nic-telemetry.json", &n);
    save(
        dir.path(),
        "system-health.json",
        &json!({"updated":1000,"components":(0..32).map(|_|json!({"component":"c".repeat(95),"state":"s".repeat(95),"observed_at":1000,"max_age":120})).collect::<Vec<_>>()}),
    );
    let view = project(dir.path(), 1000);
    let d = view.dpdk.as_ref().unwrap();
    assert_eq!(d.worker_count, 4096);
    assert_eq!(d.workers.len(), 16);
    assert!(d.details_capped);
    assert!(serde_json::to_vec(d).unwrap().len() <= 16 * 1024);
    assert!(serde_json::to_vec(&view).unwrap().len() <= 60 * 1024);
    assert!(
        !view
            .classification
            .as_ref()
            .unwrap()
            .mapped
            .points
            .is_empty()
    );
}
#[test]
fn reset_does_not_become_a_new_error_delta_and_unknown_keys_do_not_leak() {
    let dir = tempfile::tempdir().unwrap();
    let mut n = nic();
    n["ports"][0]["reset"] = json!(true);
    n["ports"][0]["stats"]["SECRET"] = json!(123);
    n["password"] = json!("SECRET");
    save(dir.path(), "nic-telemetry.json", &n);
    let d = project(dir.path(), 1000).dpdk.unwrap();
    assert!(d.ports[0].reset);
    let body = serde_json::to_string(&d).unwrap();
    assert!(!body.contains("SECRET"));
    assert!(
        !d.ports[0]
            .details
            .iter()
            .any(|r| r.label.ends_with("/sample"))
    );
}

#[test]
fn invalid_pressure_ratios_and_counter_resets_do_not_make_clear_or_new_events() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = analytics();
    a["pressure"]["diagnostics"]["pool_free_mbufs"] = json!(101);
    a["queue"]["drop_delta"] = json!(99);
    save(dir.path(), "worker-utilization.json", &a);
    let d = project(dir.path(), 1000).dpdk.unwrap();
    assert!(d.pressure.is_empty());
    assert!(
        d.analytics
            .details
            .iter()
            .any(|r| r.label == "Buffer/ring pressure" && r.value == "UNKNOWN")
    );
    assert!(
        !d.analytics
            .details
            .iter()
            .any(|r| r.label == "Queue drops/sample")
    );
}

#[test]
fn corrupt_worker_cache_does_not_hide_independent_nic_data() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("worker-utilization.json"), b"{broken").unwrap();
    save(dir.path(), "nic-telemetry.json", &nic());
    let d = project(dir.path(), 1000).dpdk.unwrap();
    assert!(d.analytics.stale);
    assert!(d.workers.is_empty());
    assert_eq!(d.mean_busy_ppm, None);
    assert!(d.nic.live(1000, 15));
    assert_eq!(d.ports.len(), 1);
}

#[test]
fn physical_identity_and_activity_do_not_guess_from_speed_or_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut n = nic();
    n["ports"][0]["name"] = json!("0000:01:00.0");
    n["ports"][0]["identity"] = json!({"role":"LAN","connector":"qsfp28","confidence":"confirmed","evidence":"fixture module identifier"});
    n["interval_ms"] = json!(10000);
    n["ports"][0]["stats"]["ipackets"] = json!(100);
    n["ports"][0]["deltas"]["ipackets"] = json!(9);
    save(dir.path(), "nic-telemetry.json", &n);
    let p = project(dir.path(), 1000).dpdk.unwrap().ports.remove(0);
    assert_eq!(p.physical.role.as_deref(), Some("LAN"));
    assert_eq!(p.physical.pci.as_deref(), Some("0000:01:00.0"));
    assert_eq!(p.physical.connector.as_deref(), Some("qsfp28"));
    assert_eq!(p.activity.rx_packets, Some(9));
    n["ports"][0]["identity"] =
        json!({"role":"internet-maybe","connector":"rj45","confidence":"certain"});
    n["ports"][0]["reset"] = json!(true);
    save(dir.path(), "nic-telemetry.json", &n);
    let p = project(dir.path(), 1000).dpdk.unwrap().ports.remove(0);
    assert!(
        p.physical.role.is_none()
            && p.physical.connector.is_none()
            && p.activity.rx_packets.is_none()
    );
    n["ports"][0]["reset"] = json!(false);
    n["interval_ms"] = json!(0);
    save(dir.path(), "nic-telemetry.json", &n);
    assert!(
        project(dir.path(), 1000).dpdk.unwrap().ports[0]
            .activity
            .rx_packets
            .is_none()
    );
}
#[test]
fn worker_phases_are_typed_in_producer_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = analytics();
    a["workers"][1]["phase_cycles"] = json!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    a["workers"][1]["laps_per_s"] = json!(12000);
    a["workers"][1]["cycles_per_busy_lap"] = json!(42);
    save(dir.path(), "worker-utilization.json", &a);
    let d = project(dir.path(), 1000).dpdk.unwrap();
    let w = d.workers.iter().find(|w| w.id == 1).unwrap();
    let names: Vec<_> = w.phases.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "RX",
            "Dissect",
            "Select",
            "Charge",
            "Leaf",
            "TX",
            "Begin cycle",
            "Drop overdue",
            "Handoff",
            "Empty RX"
        ]
    );
    assert_eq!(w.phases[7].cycles, 8);
    assert_eq!(
        (w.laps_per_s, w.cycles_per_busy_lap),
        (Some(12000), Some(42))
    );
    // A malformed phase list gives no phases, and a missing list stays empty.
    assert!(
        d.workers
            .iter()
            .find(|w| w.id == 0)
            .unwrap()
            .phases
            .is_empty()
    );
}
#[test]
fn the_console_carries_typed_buffer_queue_and_bridge_instruments() {
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), "worker-utilization.json", &analytics());
    let c = project(dir.path(), 1000).dpdk.unwrap().console;
    assert_eq!((c.pool_free, c.pool_total), (Some(50), Some(100)));
    assert_eq!(
        (c.ring_used, c.ring_capacity, c.ring_count, c.rings_over_75),
        (Some(75), Some(100), Some(1), Some(1))
    );
    assert_eq!(
        (c.rx_nombuf_per_sample, c.pool_drops_per_sample),
        (Some(0), Some(0))
    );
    // One ring above 75 % is an alert.
    assert_eq!(c.pressure.as_deref(), Some("ALERT"));
    assert_eq!(
        (c.queue_drops_total, c.aqm_marks_total, c.backlog_bytes),
        (Some(0), Some(10), Some(2048))
    );
    assert_eq!(
        (c.queue_drops_per_sample, c.aqm_marks_per_sample),
        (Some(0), Some(2))
    );
    assert_eq!(
        (c.status.as_deref(), c.version.as_deref()),
        (Some("degraded"), Some("fixture"))
    );
    assert_eq!(
        (
            c.uptime_ms,
            c.workers,
            c.missing_rollups,
            c.aqm_degraded_workers
        ),
        (Some(10000), Some(2), Some(0), Some(1))
    );
    assert_eq!(c.classification_empty, Some(false));
    assert_eq!(c.degraded, ["aqm"]);
    // Without a cache, every instrument stays unknown.
    let empty = tempfile::tempdir().unwrap();
    assert_eq!(
        project(empty.path(), 1000).dpdk.unwrap().console,
        Default::default()
    );
}
