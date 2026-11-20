use web_gateway::overview::project;
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
