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
