use lqosync_v3::{
    core_diagnostics::{self, Origin},
    policy_fixture,
};
use std::{collections::BTreeMap, sync::Arc};
mod common;
#[path = "common/core.rs"]
mod core_fixture;
#[test]
fn fixture_projection_removes_private_content_and_keeps_route_finding_codes() {
    let (mut settings, mut inventories, _, _) = core_fixture::fixture();
    settings.core_lists.bindings.clear();
    let inventory = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inventory
        .tables
        .get_mut("/ip/firewall/mangle/print")
        .unwrap()[0]
        .insert("comment".into(), "PRIVATE_CANARY_PASSWORD".into());
    inventory
        .tables
        .get_mut("/ip/firewall/mangle/print")
        .unwrap()[0]
        .insert("PRIVATE_CANARY_KEY".into(), "10.123.77.42".into());
    inventory.tables.get_mut("/interface/print").unwrap()[0]
        .insert("mac-address".into(), "02:AA:BB:CC:DD:EE".into());
    let before = core_diagnostics::review(
        &settings,
        &inventories,
        &BTreeMap::new(),
        101,
        Origin::Saved,
    );
    let projected = policy_fixture::project(&inventories["r1"]);
    let text = serde_json::to_string(&projected).unwrap();
    for secret in [
        "PRIVATE_CANARY_PASSWORD",
        "PRIVATE_CANARY_KEY",
        "10.123.77.42",
        "02:AA:BB:CC:DD:EE",
        "uplink",
        "lqos-wan",
        "to-wan",
    ] {
        assert!(!text.contains(secret), "private fixture content remained");
    }
    let fixture_settings = policy_fixture::settings();
    let after = core_diagnostics::review(
        &fixture_settings,
        &[(projected.router_id.clone(), Arc::new(projected))]
            .into_iter()
            .collect(),
        &BTreeMap::new(),
        101,
        Origin::Saved,
    );
    assert_eq!(
        policy_fixture::signature(&before),
        policy_fixture::signature(&after)
    );
}

#[test]
fn actual_routeros_policy_projection_replays_unsupported_findings_without_false_fib_conflicts() {
    let fixture: lqosync_v3::policy_fixture::Fixture =
        serde_json::from_str(include_str!("fixtures/core-policy.actual-sanitized.json")).unwrap();
    let settings = policy_fixture::settings();
    let report = core_diagnostics::review(
        &settings,
        &[(
            fixture.inventory.router_id.clone(),
            Arc::new(fixture.inventory),
        )]
        .into_iter()
        .collect(),
        &BTreeMap::new(),
        101,
        Origin::Saved,
    );
    assert_eq!(policy_fixture::signature(&report), fixture.expected_codes);
    let core = &report.cores[0];
    assert_eq!(core.wans.len(), 2);
    assert_eq!(core.connection_paths.len(), 2);
    assert!(core.connection_paths.iter().all(|p| p.direction
        == lqosync_v3::core_connections::Direction::RouterReply
        && p.status == core_diagnostics::Outcome::Pass));
    assert_eq!(
        core.connection_paths
            .iter()
            .map(|p| (p.direction, p.status))
            .collect::<Vec<_>>(),
        fixture.expected_paths
    );

    assert!(
        core.wans
            .iter()
            .all(|w| w.status == core_diagnostics::Outcome::Unsupported)
    );
    assert_eq!(core.counts.pcc_rules, 0);
    assert_eq!(core.counts.fasttrack_rules, 0);
    assert!(!core.findings.iter().any(|f| f.code == "table-fib"));
}
