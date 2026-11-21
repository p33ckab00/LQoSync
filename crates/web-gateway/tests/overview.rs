use web_gateway::overview::project;
#[test]
fn capacity_requires_current_status_and_valid_direction_rates() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("status.json");
    let mut status = serde_json::json!({"version":3,"updated":990,"wan_capacity":{
        "down_bps":1_500_000_000u64,"up_bps":300_000_000,"wan_count":2,"registry_revision":7}});
    std::fs::write(&path, serde_json::to_vec(&status).unwrap()).unwrap();
    let capacity = project(dir.path(), 1000).wan_capacity.unwrap();
    assert_eq!(capacity.down_bps, 1_500_000_000);
    assert_eq!(capacity.up_bps, 300_000_000);
    assert!(project(dir.path(), 2000).wan_capacity.is_none());
    status["wan_capacity"]["up_bps"] = serde_json::json!(0);
    std::fs::write(&path, serde_json::to_vec(&status).unwrap()).unwrap();
    assert!(project(dir.path(), 1000).wan_capacity.is_none());
    status["wan_capacity"]["up_bps"] = serde_json::json!(1u64 << 53);
    std::fs::write(&path, serde_json::to_vec(&status).unwrap()).unwrap();
    assert!(project(dir.path(), 1000).wan_capacity.is_none());
}
#[test]
fn missing_sources_are_unknown_not_zero() {
    let dir = tempfile::tempdir().unwrap();
    let view = project(dir.path(), 1000);
    assert!(view.subscribers.is_none());
    assert!(view.routers.is_none());
    assert!(view.stale);
    assert!(view.traffic.is_empty());
}
#[test]
fn stale_and_mismatched_collection_never_claims_current_counts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("status.json"),r#"{"version":3,"updated":990,"pid":5,"cycle_seq":3,"mode":"apply","target":"v3_pair","config_path":"SECRET"}"#).unwrap();
    std::fs::write(
        dir.path().join("collection.json"),
        r#"{"version":1,"updated":990,"pid":4,"cycle_seq":3,"fresh_for":300,"routers":[]}"#,
    )
    .unwrap();
    let view = project(dir.path(), 1000);
    assert!(view.stale);
    assert!(view.subscribers.is_none());
    assert!(!serde_json::to_string(&view).unwrap().contains("SECRET"));
}

#[test]
fn missing_ppp_and_failed_router_do_not_claim_zero_or_current_counts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("status.json"),
        r#"{"version":3,"updated":990,"pid":5,"cycle_seq":3}"#,
    )
    .unwrap();
    let path = dir.path().join("collection.json");
    std::fs::write(&path,r#"{"version":1,"updated":990,"pid":5,"cycle_seq":3,"fresh_for":300,"routers":[{"state":"online","subscribers":[],"sources":{}}]}"#).unwrap();
    let view = project(dir.path(), 1000);
    assert!(view.ppp_online.is_none());
    assert_eq!(view.subscribers, Some(0));
    std::fs::write(&path,r#"{"version":1,"updated":990,"pid":5,"cycle_seq":3,"fresh_for":300,"routers":[{"state":"failed","subscribers":[{}],"sources":{"ppp":{"online":1,"offline":0}}}]}"#).unwrap();
    let view = project(dir.path(), 1000);
    assert!(view.stale);
    assert!(view.subscribers.is_none());
    assert!(view.ppp_online.is_none());
}

#[test]
fn ipoe_counts_only_bound_dhcp_and_keeps_unknown_distinct_from_zero() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("status.json"),
        r#"{"version":3,"updated":990,"pid":5,"cycle_seq":3}"#,
    )
    .unwrap();
    let path = dir.path().join("collection.json");
    let mut collection = serde_json::json!({"version":1,"updated":990,"pid":5,"cycle_seq":3,"fresh_for":300,"routers":[{"state":"online","sources":{"dhcp":{"enabled":true}},"subscribers":[{"source":"dhcp","state":"bound"},{"source":"dhcp","state":"unbound"},{"source":"ppp","state":"online"},{"source":"hotspot","state":"online"}]}]});
    std::fs::write(&path, serde_json::to_vec(&collection).unwrap()).unwrap();
    assert_eq!(project(dir.path(), 1000).ipoe_active, Some(1));
    collection["routers"][0]["subscribers"] = serde_json::json!([]);
    std::fs::write(&path, serde_json::to_vec(&collection).unwrap()).unwrap();
    assert_eq!(project(dir.path(), 1000).ipoe_active, Some(0));
    collection["routers"][0]["sources"] = serde_json::json!({});
    std::fs::write(&path, serde_json::to_vec(&collection).unwrap()).unwrap();
    assert_eq!(project(dir.path(), 1000).ipoe_active, None);
    assert_eq!(project(dir.path(), 2000).ipoe_active, None);
}

#[test]
fn traffic_is_bounded_ordered_and_marks_old_samples_stale() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("worker-utilization.json");
    let points = (600..1000)
        .map(|at| serde_json::json!({"at":at,"down_mbps":1.0,"up_mbps":0.0}))
        .collect::<Vec<_>>();
    let mut workers =
        serde_json::json!({"version":1,"updated":1000,"stale":false,"traffic":points});
    std::fs::write(&path, serde_json::to_vec(&workers).unwrap()).unwrap();
    let view = project(dir.path(), 1000);
    assert_eq!(view.traffic.len(), 300);
    assert!(!view.traffic_stale);
    workers["traffic"] = serde_json::json!([{"at":990,"down_mbps":1.0,"up_mbps":0.0},{"at":980,"down_mbps":2.0,"up_mbps":0.0},{"at":0,"down_mbps":1.0,"up_mbps":0.0},{"at":1001,"down_mbps":1.0,"up_mbps":0.0}]);
    std::fs::write(&path, serde_json::to_vec(&workers).unwrap()).unwrap();
    let view = project(dir.path(), 1000);
    assert_eq!(
        view.traffic.iter().map(|p| p.at).collect::<Vec<_>>(),
        vec![980, 990]
    );
    workers["traffic"] = serde_json::json!([{"at":100,"down_mbps":1.0,"up_mbps":0.0}]);
    std::fs::write(&path, serde_json::to_vec(&workers).unwrap()).unwrap();
    assert!(project(dir.path(), 1000).traffic_stale);
}

#[test]
fn a_missing_rate_marks_the_next_point_as_a_gap() {
    let dir = tempfile::tempdir().unwrap();
    let workers = serde_json::json!({"version":1,"updated":1000,"stale":false,"traffic":[
        {"at":998,"down_mbps":1.0,"up_mbps":0.5},
        {"at":999,"down_mbps":null,"up_mbps":null},
        {"at":1000,"down_mbps":2.0,"up_mbps":1.0}]});
    std::fs::write(
        dir.path().join("worker-utilization.json"),
        serde_json::to_vec(&workers).unwrap(),
    )
    .unwrap();
    let view = project(dir.path(), 1000);
    assert_eq!(view.traffic.len(), 2);
    assert!(!view.traffic[0].gap_before);
    assert!(view.traffic[1].gap_before);
}

#[test]
fn full_hour_cache_keeps_the_live_payload_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let points = (6400..=10000)
        .map(|at| serde_json::json!({"at":at,"down_mbps":100.0,"up_mbps":20.0}))
        .collect::<Vec<_>>();
    let workers = serde_json::json!({"version":1,"updated":10000,"stale":false,
        "traffic":points,"classification":{"mapped":{"history":points},"unmapped":{"history":points}},
        "workers":(0..4096).map(|id|serde_json::json!({"queue_id":id,"busy_ppm":500000})).collect::<Vec<_>>()});
    std::fs::write(
        dir.path().join("worker-utilization.json"),
        serde_json::to_vec(&workers).unwrap(),
    )
    .unwrap();
    let started = std::time::Instant::now();
    let view = project(dir.path(), 10000);
    let bytes = serde_json::to_vec(&view).unwrap().len();
    println!(
        "One-hour source projection: {:?}; {bytes} browser bytes",
        started.elapsed()
    );
    assert_eq!(view.traffic.len(), 300);
    assert!(!view.traffic_stale);
    assert!(bytes < 65536);
}

#[test]
fn classification_preserves_zeros_gaps_and_independent_availability() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("worker-utilization.json");
    let mut workers = serde_json::json!({"version":1,"updated":1000,"stale":false,
        "classification":{"mapped":{"bytes":{"download_bytes":0,"upload_bytes":0},"history":[
            {"at":998,"down_mbps":0.0,"up_mbps":0.0},
            {"at":999,"down_mbps":null,"up_mbps":null},
            {"at":1000,"down_mbps":12.0,"up_mbps":3.0},
            {"at":1001,"down_mbps":90.0,"up_mbps":90.0}]}}});
    std::fs::write(&path, serde_json::to_vec(&workers).unwrap()).unwrap();
    let view = project(dir.path(), 1000);
    let c = view.classification.unwrap();
    assert!(c.mapped.available);
    assert!(!c.mapped.stale);
    assert_eq!(c.mapped.points.len(), 2);
    assert_eq!(c.mapped.points[0].down_mbps, 0.0);
    assert!(c.mapped.points[1].gap_before);
    assert!(!c.unmapped.available);
    assert!(c.unmapped.points.is_empty());
    workers["classification"]["mapped"]["history"][2]["down_mbps"] = serde_json::Value::Null;
    std::fs::write(&path, serde_json::to_vec(&workers).unwrap()).unwrap();
    assert!(
        project(dir.path(), 1000)
            .classification
            .unwrap()
            .mapped
            .stale
    );
    std::fs::remove_file(path).unwrap();
    let mut old = web_contract::Overview {
        classification: Some(c),
        ..Default::default()
    };
    web_gateway::overview::refresh_traffic(dir.path(), 1000, &mut old);
    let c = old.classification.unwrap();
    assert!(c.mapped.stale);
    assert_eq!(c.mapped.points.len(), 2);
}

#[test]
fn precision_histories_fit_the_complete_broker_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let points = (9701..=10000)
        .map(|at| {
            serde_json::json!({"at":at,
        "down_mbps":123.12345678912345,"up_mbps":12.12345678912345})
        })
        .collect::<Vec<_>>();
    let series =
        serde_json::json!({"bytes":{"download_bytes":1,"upload_bytes":1},"history":points});
    let workers = serde_json::json!({"version":1,"updated":10000,"stale":false,"traffic":points,
        "classification":{"mapped":series,"unmapped":series}});
    std::fs::write(
        dir.path().join("worker-utilization.json"),
        serde_json::to_vec(&workers).unwrap(),
    )
    .unwrap();
    let view = project(dir.path(), 10000);
    let c = view.classification.as_ref().unwrap();
    assert!(!c.mapped.points.is_empty());
    assert_eq!(c.mapped.points.last().unwrap().at, 10000);
    let reply = web_gateway::broker::Reply {
        ok: true,
        account: None,
        overview: Some(view),
        traffic: None,
        routers: None,
        edit: None,
        activity: None,
        test: None,
        router: None,
    };
    assert!(serde_json::to_vec(&reply).unwrap().len() + 1 < 65536);
}
