//! The public registry/observation path across disconnect and process restart.

use lqosync3::topology_manager::model::{SourceKind, SourceScope, SubscriberPolicy};
use lqosync3::topology_manager::observation::{
    AccountObservation, Endpoint, ObservationGate, SourceRead, reconcile,
};
use lqosync3::topology_manager::store::RegistryStore;
use std::net::IpAddr;

fn scope() -> SourceScope {
    SourceScope {
        router_id: "router-id".into(),
        source_id: "source-id".into(),
        kind: SourceKind::Ppp,
    }
}

fn accounts(session: &str, ip: &str, mac: &str) -> Vec<AccountObservation> {
    vec![AccountObservation {
        account_ref: "authoritative-account-id".into(),
        policy: Default::default(),
        endpoints: vec![Endpoint {
            session_id: session.into(),
            ips: vec![ip.parse().unwrap()],
            mac: Some(mac.into()),
        }],
    }]
}

#[test]
fn restart_and_reconnect_restore_policy_but_never_restore_old_endpoints() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("topology-manager/registry.json");
    let mut store = RegistryStore::open(dir.path()).unwrap();
    let mut gate = ObservationGate::default();
    let fresh = gate
        .accept(
            1,
            &[scope()],
            vec![SourceRead::Complete {
                scope: scope(),
                accounts: accounts("first-session", "192.0.2.1", "aa-bb-cc-dd-ee-01"),
            }],
        )
        .unwrap();
    let mut first = reconcile(store.document().unwrap(), fresh).unwrap();
    let subscriber_id = first.active[0].subscriber_id.clone();
    first.registry.subscribers[0].policy = SubscriberPolicy {
        profile_id: Some("local-plan-id".into()),
        assignment: Some("wan-2-instance-id".into()),
        pin: Some("wan-2-instance-id".into()),
    };
    store.commit(0, first.registry).unwrap();
    let saved = std::fs::read(&path).unwrap();
    for forbidden in ["192.0.2.1", "AA:BB:CC:DD:EE:01", "first-session"] {
        assert!(!String::from_utf8_lossy(&saved).contains(forbidden));
    }
    drop(store);

    let mut store = RegistryStore::open(dir.path()).unwrap();
    let mut gate = ObservationGate::default();
    let absent = gate
        .accept(
            1,
            &[scope()],
            vec![SourceRead::Complete {
                scope: scope(),
                accounts: Vec::new(),
            }],
        )
        .unwrap();
    let offline = reconcile(store.document().unwrap(), absent).unwrap();
    assert!(offline.active.is_empty());
    assert_eq!(offline.registry.subscribers.len(), 1);
    assert!(!store.commit(1, offline.registry).unwrap().changed);

    let fresh = gate
        .accept(
            2,
            &[scope()],
            vec![SourceRead::Complete {
                scope: scope(),
                accounts: accounts("new-session", "192.0.2.99", "aa-bb-cc-dd-ee-99"),
            }],
        )
        .unwrap();
    let returned = reconcile(store.document().unwrap(), fresh).unwrap();
    assert_eq!(returned.active.len(), 1);
    assert_eq!(returned.active[0].subscriber_id, subscriber_id);
    assert_eq!(returned.active[0].endpoints.len(), 1);
    assert_eq!(
        returned.active[0].endpoints[0].endpoint.session_id,
        "new-session"
    );
    assert_eq!(
        returned.active[0].endpoints[0].endpoint.ips,
        vec!["192.0.2.99".parse::<IpAddr>().unwrap()]
    );
    assert_eq!(
        returned.active[0].endpoints[0].endpoint.mac.as_deref(),
        Some("AA:BB:CC:DD:EE:99")
    );
    assert_eq!(
        returned.registry.subscribers[0].policy.pin.as_deref(),
        Some("wan-2-instance-id")
    );
    assert!(!store.commit(1, returned.registry).unwrap().changed);
    assert_eq!(std::fs::read(&path).unwrap(), saved);
}
