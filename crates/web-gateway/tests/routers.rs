use web_contract::routers::{RouterInventory, RouterSummary, VERSION};
use web_gateway::routers::{BUDGET, MAX_ROUTERS, bound, load};

fn inventory(n: usize, name_len: usize) -> RouterInventory {
    RouterInventory {
        version: VERSION,
        updated: 990,
        fresh_for: 300,
        routers: (0..n)
            .map(|i| RouterSummary {
                id: format!("router-{i}"),
                name: format!("{i:04}{}", "x".repeat(name_len)),
                ..Default::default()
            })
            .collect(),
        truncated: false,
    }
}

#[test]
fn the_inventory_fits_the_broker_reply_and_keeps_config_order() {
    let small = bound(inventory(3, 4));
    assert!(!small.truncated);
    assert_eq!(small.routers.len(), 3);
    let big = bound(inventory(150, 600));
    assert!(big.truncated);
    assert!(serde_json::to_vec(&big).unwrap().len() <= BUDGET);
    assert_eq!(big.routers[0].id, "router-0", "drops from the end only");
    // The whole broker reply line, with its envelope and newline, fits the 64 KiB limit.
    let reply = web_gateway::broker::Reply {
        ok: true,
        account: None,
        overview: None,
        traffic: None,
        routers: Some(big),
    };
    assert!(serde_json::to_vec(&reply).unwrap().len() < 65536);
    let many = bound(inventory(MAX_ROUTERS + 20, 1));
    assert!(many.truncated && many.routers.len() <= MAX_ROUTERS);
}

#[test]
fn a_wrong_version_or_missing_cache_is_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    assert!(load(dir.path()).is_none());
    let mut inv = inventory(1, 1);
    inv.version = 2;
    std::fs::write(
        dir.path().join("routers.json"),
        serde_json::to_vec(&inv).unwrap(),
    )
    .unwrap();
    assert!(load(dir.path()).is_none());
    inv.version = VERSION;
    std::fs::write(
        dir.path().join("routers.json"),
        serde_json::to_vec(&inv).unwrap(),
    )
    .unwrap();
    assert_eq!(load(dir.path()).unwrap().routers.len(), 1);
}

#[test]
fn the_dashboard_counts_every_configured_router_only_for_a_current_service() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("status.json"),
        r#"{"version":3,"updated":990,"pid":5,"cycle_seq":3}"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("collection.json"),r#"{"version":1,"updated":990,"pid":5,"cycle_seq":3,"fresh_for":300,"routers":[{"state":"online","subscribers":[],"sources":{}}]}"#).unwrap();
    std::fs::write(
        dir.path().join("routers.json"),
        serde_json::to_vec(&inventory(2, 1)).unwrap(),
    )
    .unwrap();
    // The collection has one subscriber router; the inventory adds the WAN-only core.
    assert_eq!(
        web_gateway::overview::project(dir.path(), 1000).routers,
        Some(2)
    );
    // A collection from another service process is not current: no count at all.
    std::fs::write(
        dir.path().join("collection.json"),
        r#"{"version":1,"updated":990,"pid":4,"cycle_seq":3,"fresh_for":300,"routers":[]}"#,
    )
    .unwrap();
    assert_eq!(
        web_gateway::overview::project(dir.path(), 1000).routers,
        None
    );
}
