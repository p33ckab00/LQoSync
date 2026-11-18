use super::super::catalog::{CatalogRouter, RouterRole, TopologyCatalog};
use super::super::model::{IdentityRef, SourceKind, SubscriberPolicy, SubscriberRecord};
use super::super::normalize::normalize;
use super::super::reader::{ReadGuard, ReadPlan, Table};
use super::super::source_policy::{ProfileBinding, RateRead, SourcePolicy};
use super::*;
use crate::model::RosRow;
use crate::routeros::{ReadError, RouterSession};
use std::collections::BTreeMap;
use std::time::Duration;

type Rows = BTreeMap<Table, Vec<RosRow>>;
struct Session(Rows);
impl RouterSession for Session {
    async fn print(&mut self, path: &str, fields: &str) -> Result<Vec<RosRow>, ReadError> {
        let table = [
            Table::Interfaces,
            Table::PppSecrets,
            Table::PppProfiles,
            Table::PppActive,
            Table::DhcpServers,
            Table::DhcpLeases,
            Table::HotspotUsers,
            Table::HotspotServers,
            Table::HotspotProfiles,
            Table::HotspotActive,
        ]
        .into_iter()
        .find(|t| t.command() == (path, fields))
        .unwrap();
        Ok(self.0.get(&table).cloned().unwrap_or_default())
    }
}
fn row(fields: &[(&str, &str)]) -> RosRow {
    fields.iter().map(|&(k, v)| (k.into(), v.into())).collect()
}
fn ppp(reference: Option<&str>) -> Rows {
    let mut secret = row(&[
        (".id", "*s"),
        ("name", "alice"),
        ("profile", "gold"),
        ("disabled", "false"),
    ]);
    if let Some(reference) = reference {
        secret.insert(
            "comment".into(),
            format!("Account note [lqosync-id:{reference}]"),
        );
    }
    BTreeMap::from([
        (
            Table::PppProfiles,
            vec![row(&[
                (".id", "*p"),
                ("name", "gold"),
                ("rate-limit", "2M/10M"),
            ])],
        ),
        (Table::PppSecrets, vec![secret]),
        (
            Table::PppActive,
            vec![row(&[
                (".id", "*a"),
                ("name", "alice"),
                ("address", "192.0.2.1"),
                ("caller-id", "aa:bb:cc:dd:ee:01"),
                ("service", "pppoe"),
            ])],
        ),
    ])
}
fn dhcp(reference: Option<&str>) -> Rows {
    let mut lease = row(&[
        (".id", "*l"),
        ("server", "lan"),
        ("status", "bound"),
        ("active-address", "192.0.2.2"),
        ("active-mac-address", "aa:bb:cc:dd:ee:02"),
        ("host-name", "alice"),
        ("active-client-id", "client"),
    ]);
    if let Some(reference) = reference {
        lease.insert("comment".into(), format!("[lqosync-id:{reference}]"));
    }
    BTreeMap::from([
        (
            Table::DhcpServers,
            vec![row(&[
                (".id", "*d"),
                ("name", "lan"),
                ("disabled", "false"),
            ])],
        ),
        (Table::DhcpLeases, vec![lease]),
    ])
}
fn selection(router: &str, kind: SourceKind) -> SourceSelection {
    SourceSelection {
        scope: SourceScope {
            router_id: router.into(),
            source_id: "subscriber-service".into(),
            kind,
        },
        service_names: vec![],
        authority: ReferenceAuthority::OperatorManagedComment,
        policy: SourcePolicy {
            profiles: if kind == SourceKind::Dhcp {
                vec![]
            } else {
                vec![ProfileBinding {
                    current_name: "gold".into(),
                    source_ref: "*p".into(),
                    rate: RateRead::Disabled,
                }]
            },
            ..Default::default()
        },
    }
}
async fn current(router: &str, generation: u64, rows: Rows, roles: &[RouterRole]) -> CurrentRouter {
    let catalog = TopologyCatalog {
        routers: vec![CatalogRouter {
            id: router.into(),
            name: "Router".into(),
            roles: roles.into(),
        }],
        ..Default::default()
    };
    let plan = ReadPlan::for_router(&catalog, router).unwrap();
    let evidence = ReadGuard::new(router.into())
        .unwrap()
        .read(
            &mut Some(Session(rows)),
            &plan,
            generation,
            Duration::from_secs(1),
            Duration::from_secs(10),
        )
        .await
        .unwrap();
    normalize(&evidence).unwrap()
}
fn identity(scope: SourceScope, reference: &str) -> IdentityRef {
    IdentityRef {
        scope,
        account_ref: format!("operator-comment/v1/{reference}"),
    }
}
fn registry() -> RegistryDocument {
    RegistryDocument {
        next_id: 2,
        subscribers: vec![SubscriberRecord {
            id: "tm-sub-0000000000000001".into(),
            identities: vec![identity(
                selection("router-a", SourceKind::Ppp).scope,
                "acct-1",
            )],
            policy: SubscriberPolicy {
                assignment: Some("wan-1".into()),
                pin: Some("wan-1".into()),
                profile_id: Some("local-plan".into()),
            },
        }],
        ..Default::default()
    }
}
fn conflict(auth: &Authorization<'_>, expected: IdentityIssue) {
    assert!(!auth.is_ready());
    assert!(auth.conflicts().iter().any(|c| c.issue == expected));
}

#[tokio::test(start_paused = true)]
async fn enrolled_current_reference_allocates_one_id_and_keeps_all_current_sessions() {
    let registry = RegistryDocument::default();
    let mut rows = ppp(Some("acct-1"));
    let mut extra = rows[&Table::PppActive][0].clone();
    extra.insert(".id".into(), "*a2".into());
    extra.insert("address".into(), "192.0.2.3".into());
    rows.get_mut(&Table::PppActive).unwrap().push(extra);
    let input = current("router-a", 1, rows, &[RouterRole::Ppp]).await;
    let auth = authorize(
        &registry,
        0,
        1,
        vec![input],
        &[selection("router-a", SourceKind::Ppp)],
    )
    .unwrap();
    assert!(auth.is_ready());
    assert!(auth.conflicts().is_empty());
    let result = auth.reconcile(&mut ObservationGate::default()).unwrap();
    assert_eq!(result.reconciliation.active.len(), 1);
    assert_eq!(result.reconciliation.active[0].endpoints.len(), 2);
    assert_eq!(
        result.reconciliation.registry.subscribers[0].identities[0].account_ref,
        "operator-comment/v1/acct-1"
    );
    assert_eq!(registry, RegistryDocument::default());
}

#[tokio::test(start_paused = true)]
async fn return_and_rename_keep_saved_intent_but_use_only_fresh_endpoints() {
    let registry = registry();
    let mut rows = ppp(Some("acct-1"));
    rows.get_mut(&Table::PppSecrets).unwrap()[0].insert("name".into(), "renamed".into());
    let active = &mut rows.get_mut(&Table::PppActive).unwrap()[0];
    active.insert("name".into(), "renamed".into());
    active.insert("address".into(), "192.0.2.99".into());
    active.insert("caller-id".into(), "aa:bb:cc:dd:ee:99".into());
    let input = current("router-a", 2, rows, &[RouterRole::Ppp]).await;
    let result = authorize(
        &registry,
        0,
        2,
        vec![input],
        &[selection("router-a", SourceKind::Ppp)],
    )
    .unwrap()
    .reconcile(&mut ObservationGate::default())
    .unwrap();
    assert_eq!(result.reconciliation.registry, registry);
    let active = &result.reconciliation.active[0];
    assert_eq!(active.subscriber_id, "tm-sub-0000000000000001");
    assert_eq!(
        active.endpoints[0].endpoint.ips[0].to_string(),
        "192.0.2.99"
    );
    assert_eq!(
        active.endpoints[0].endpoint.mac.as_deref(),
        Some("AA:BB:CC:DD:EE:99")
    );
}

#[tokio::test(start_paused = true)]
async fn account_name_or_locator_reuse_without_reference_cannot_receive_old_policy() {
    let registry = registry();
    let input = current("router-a", 1, ppp(None), &[RouterRole::Ppp]).await;
    let auth = authorize(
        &registry,
        0,
        1,
        vec![input],
        &[selection("router-a", SourceKind::Ppp)],
    )
    .unwrap();
    conflict(&auth, IdentityIssue::MissingReference);
    assert!(auth.reconcile(&mut ObservationGate::default()).is_err());
    assert_eq!(registry.subscribers.len(), 1);
}

#[tokio::test(start_paused = true)]
async fn changed_independent_reference_creates_a_distinct_identity_without_the_old_pin() {
    let registry = registry();
    let input = current("router-a", 1, ppp(Some("acct-new")), &[RouterRole::Ppp]).await;
    let result = authorize(
        &registry,
        0,
        1,
        vec![input],
        &[selection("router-a", SourceKind::Ppp)],
    )
    .unwrap()
    .reconcile(&mut ObservationGate::default())
    .unwrap();
    assert_eq!(result.reconciliation.registry.subscribers.len(), 2);
    assert_eq!(
        result.reconciliation.active[0].subscriber_id,
        "tm-sub-0000000000000002"
    );
    assert_eq!(
        result.reconciliation.registry.subscribers[1].policy,
        SubscriberPolicy::default()
    );
    assert_eq!(
        result.reconciliation.registry.subscribers[0],
        registry.subscribers[0]
    );
}

#[tokio::test(start_paused = true)]
async fn complete_offline_reads_keep_intent_without_active_endpoints_or_stored_addresses() {
    let registry = registry();
    let mut rows = ppp(None);
    rows.get_mut(&Table::PppActive).unwrap().clear();
    let input = current("router-a", 1, rows, &[RouterRole::Ppp]).await;
    let result = authorize(
        &registry,
        0,
        1,
        vec![input],
        &[selection("router-a", SourceKind::Ppp)],
    )
    .unwrap()
    .reconcile(&mut ObservationGate::default())
    .unwrap();
    assert!(result.reconciliation.active.is_empty());
    assert_eq!(result.reconciliation.registry, registry);
    let json = serde_json::to_string(&result.reconciliation.registry).unwrap();
    assert!(!json.contains("192.0.2."));
    assert!(!json.contains("AA:BB:"));
}

#[tokio::test(start_paused = true)]
async fn dhcp_requires_an_independent_reference_even_when_hostname_and_mac_match() {
    let registry = RegistryDocument::default();
    for tag in [None, Some("lease-acct")] {
        let input = current("router-a", 1, dhcp(tag), &[RouterRole::Dhcp]).await;
        let auth = authorize(
            &registry,
            0,
            1,
            vec![input],
            &[selection("router-a", SourceKind::Dhcp)],
        )
        .unwrap();
        if tag.is_none() {
            conflict(&auth, IdentityIssue::MissingReference);
        } else {
            assert!(auth.is_ready());
        }
    }
}

#[tokio::test(start_paused = true)]
async fn unconfigured_contract_does_not_interpret_a_comment_marker_automatically() {
    let registry = RegistryDocument::default();
    let mut selected = selection("router-a", SourceKind::Ppp);
    selected.authority = ReferenceAuthority::Unconfigured;
    let input = current("router-a", 1, ppp(Some("acct-1")), &[RouterRole::Ppp]).await;
    conflict(
        &authorize(&registry, 0, 1, vec![input], &[selected]).unwrap(),
        IdentityIssue::UnconfiguredAuthority,
    );
}

#[tokio::test(start_paused = true)]
async fn invalid_duplicate_or_address_shaped_markers_fail_without_echoing_raw_references() {
    let registry = RegistryDocument::default();
    for comment in [
        "[lqosync-id:]",
        "[lqosync-id:private bad]",
        "[lqosync-id:192.0.2.1]",
        "[lqosync-id:AA:BB:CC:DD:EE:FF]",
        "[lqosync-id:private",
        "[lqosync-id:a][lqosync-id:b]",
        "[lqosync-id:.x]",
        "[lqosync-id:réf]",
    ] {
        let mut rows = ppp(None);
        rows.get_mut(&Table::PppSecrets).unwrap()[0].insert("comment".into(), comment.into());
        let input = current("router-a", 1, rows, &[RouterRole::Ppp]).await;
        let auth = authorize(
            &registry,
            0,
            1,
            vec![input],
            &[selection("router-a", SourceKind::Ppp)],
        )
        .unwrap();
        conflict(&auth, IdentityIssue::InvalidReference);
        assert!(!format!("{:?}", auth.conflicts()).contains("private"));
    }
}

#[tokio::test(start_paused = true)]
async fn two_live_local_accounts_cannot_claim_one_reference_in_the_same_scope() {
    let registry = RegistryDocument::default();
    let mut rows = ppp(Some("acct-1"));
    let mut secret = rows[&Table::PppSecrets][0].clone();
    secret.insert(".id".into(), "*s2".into());
    secret.insert("name".into(), "bob".into());
    rows.get_mut(&Table::PppSecrets).unwrap().push(secret);
    let mut active = rows[&Table::PppActive][0].clone();
    active.insert(".id".into(), "*a2".into());
    active.insert("name".into(), "bob".into());
    active.insert("address".into(), "192.0.2.4".into());
    rows.get_mut(&Table::PppActive).unwrap().push(active);
    let input = current("router-a", 1, rows, &[RouterRole::Ppp]).await;
    let auth = authorize(
        &registry,
        0,
        1,
        vec![input],
        &[selection("router-a", SourceKind::Ppp)],
    )
    .unwrap();
    conflict(&auth, IdentityIssue::DuplicateReference);
    assert!(auth.reconcile(&mut ObservationGate::default()).is_err());
}

#[tokio::test(start_paused = true)]
async fn identical_names_and_references_on_different_routers_do_not_merge() {
    let registry = RegistryDocument::default();
    let mut rows = ppp(Some("acct-1"));
    rows.get_mut(&Table::PppActive).unwrap()[0].insert("address".into(), "192.0.2.4".into());
    let a = current("router-a", 1, ppp(Some("acct-1")), &[RouterRole::Ppp]).await;
    let b = current("router-b", 1, rows, &[RouterRole::Ppp]).await;
    let result = authorize(
        &registry,
        0,
        1,
        vec![a, b],
        &[
            selection("router-a", SourceKind::Ppp),
            selection("router-b", SourceKind::Ppp),
        ],
    )
    .unwrap()
    .reconcile(&mut ObservationGate::default())
    .unwrap();
    assert_eq!(result.reconciliation.active.len(), 2);
}

#[tokio::test(start_paused = true)]
async fn existing_explicit_cross_protocol_aliases_reconcile_to_one_circuit() {
    let mut registry = registry();
    registry.subscribers[0].identities.push(identity(
        selection("router-a", SourceKind::Dhcp).scope,
        "lease-acct",
    ));
    let mut rows = ppp(Some("acct-1"));
    rows.extend(dhcp(Some("lease-acct")));
    let input = current("router-a", 1, rows, &[RouterRole::Ppp, RouterRole::Dhcp]).await;
    let result = authorize(
        &registry,
        0,
        1,
        vec![input],
        &[
            selection("router-a", SourceKind::Ppp),
            selection("router-a", SourceKind::Dhcp),
        ],
    )
    .unwrap()
    .reconcile(&mut ObservationGate::default())
    .unwrap();
    assert_eq!(result.reconciliation.active.len(), 1);
    assert_eq!(result.reconciliation.active[0].endpoints.len(), 2);
    assert_eq!(result.reconciliation.registry, registry);
}

#[tokio::test(start_paused = true)]
async fn overlapping_scope_selections_and_partial_multiservice_membership_are_critical() {
    let registry = RegistryDocument::default();
    let a = selection("router-a", SourceKind::Ppp);
    let mut b = selection("router-a", SourceKind::Ppp);
    b.scope.source_id = "other".into();
    let input = current("router-a", 1, ppp(Some("acct-1")), &[RouterRole::Ppp]).await;
    conflict(
        &authorize(&registry, 0, 1, vec![input], &[a, b]).unwrap(),
        IdentityIssue::AmbiguousScope,
    );
    let mut rows = ppp(Some("acct-1"));
    let mut second = rows[&Table::PppActive][0].clone();
    second.insert(".id".into(), "*b".into());
    second.insert("address".into(), "192.0.2.7".into());
    second.insert("service".into(), "pptp".into());
    rows.get_mut(&Table::PppActive).unwrap().push(second);
    let mut selected = selection("router-a", SourceKind::Ppp);
    selected.service_names = vec!["pppoe".into()];
    let input = current("router-a", 1, rows, &[RouterRole::Ppp]).await;
    conflict(
        &authorize(&registry, 0, 1, vec![input], &[selected]).unwrap(),
        IdentityIssue::PartialServiceMembership,
    );
}

#[tokio::test(start_paused = true)]
async fn accounts_outside_explicit_service_selection_do_not_leak_into_the_scope() {
    let registry = RegistryDocument::default();
    let mut selected = selection("router-a", SourceKind::Ppp);
    selected.service_names = vec!["pptp".into()];
    let input = current("router-a", 1, ppp(None), &[RouterRole::Ppp]).await;
    let result = authorize(&registry, 0, 1, vec![input], &[selected])
        .unwrap()
        .reconcile(&mut ObservationGate::default())
        .unwrap();
    assert!(result.reconciliation.active.is_empty());
    assert!(result.reconciliation.registry.subscribers.is_empty());
}

#[tokio::test(start_paused = true)]
async fn an_unread_protocol_cannot_be_reported_as_a_complete_empty_source() {
    let registry = RegistryDocument::default();
    let input = current("router-a", 1, Rows::new(), &[RouterRole::Wan]).await;
    assert!(
        authorize(
            &registry,
            0,
            1,
            vec![input],
            &[selection("router-a", SourceKind::Ppp)]
        )
        .is_err()
    );
}

#[tokio::test(start_paused = true)]
async fn stale_revision_mixed_generation_and_missing_or_duplicate_routers_are_rejected() {
    let registry = RegistryDocument::default();
    let input = current("router-a", 1, ppp(Some("acct-1")), &[RouterRole::Ppp]).await;
    assert!(
        authorize(
            &registry,
            1,
            1,
            vec![input],
            &[selection("router-a", SourceKind::Ppp)]
        )
        .is_err()
    );
    let input = current("router-a", 2, ppp(Some("acct-1")), &[RouterRole::Ppp]).await;
    assert!(
        authorize(
            &registry,
            0,
            1,
            vec![input],
            &[selection("router-a", SourceKind::Ppp)]
        )
        .is_err()
    );
    assert!(
        authorize(
            &registry,
            0,
            1,
            vec![],
            &[selection("router-a", SourceKind::Ppp)]
        )
        .is_err()
    );
    let a = current("router-a", 1, ppp(Some("acct-1")), &[RouterRole::Ppp]).await;
    let b = current("router-a", 1, ppp(Some("acct-1")), &[RouterRole::Ppp]).await;
    assert!(
        authorize(
            &registry,
            0,
            1,
            vec![a, b],
            &[selection("router-a", SourceKind::Ppp)]
        )
        .is_err()
    );
}

#[tokio::test(start_paused = true)]
async fn expired_input_and_expired_authorization_cannot_restore_any_endpoints() {
    let registry = registry();
    let input = current("router-a", 1, ppp(Some("acct-1")), &[RouterRole::Ppp]).await;
    tokio::time::advance(Duration::from_secs(6)).await;
    let auth = authorize(
        &registry,
        0,
        1,
        vec![input],
        &[selection("router-a", SourceKind::Ppp)],
    )
    .unwrap();
    tokio::time::advance(Duration::from_secs(4)).await;
    assert!(!auth.is_ready());
    assert!(auth.reconcile(&mut ObservationGate::default()).is_err());
    let input = current("router-a", 2, ppp(Some("acct-1")), &[RouterRole::Ppp]).await;
    tokio::time::advance(Duration::from_secs(10)).await;
    assert!(
        authorize(
            &registry,
            0,
            2,
            vec![input],
            &[selection("router-a", SourceKind::Ppp)]
        )
        .is_err()
    );
}

#[tokio::test(start_paused = true)]
async fn an_authorized_generation_cannot_be_replayed_through_the_observation_gate() {
    let registry = RegistryDocument::default();
    let mut gate = ObservationGate::default();
    for success in [true, false] {
        let input = current("router-a", 1, ppp(Some("acct-1")), &[RouterRole::Ppp]).await;
        let auth = authorize(
            &registry,
            0,
            1,
            vec![input],
            &[selection("router-a", SourceKind::Ppp)],
        )
        .unwrap();
        assert_eq!(auth.reconcile(&mut gate).is_ok(), success);
    }
}

#[tokio::test(start_paused = true)]
async fn invalid_scope_and_service_inputs_never_accept_an_empty_or_partial_world() {
    let registry = RegistryDocument::default();
    for case in 0..5 {
        let input = current("router-a", 1, ppp(Some("acct-1")), &[RouterRole::Ppp]).await;
        let mut selected = selection("router-a", SourceKind::Ppp);
        match case {
            0 => selected.scope.source_id.clear(),
            1 => selected.service_names = vec!["".into()],
            2 => selected.service_names = vec!["pppoe".into(), "pppoe".into()],
            _ => {}
        }
        let selections = if case == 3 { vec![] } else { vec![selected] };
        assert!(
            authorize(
                &registry,
                0,
                if case == 4 { 0 } else { 1 },
                vec![input],
                &selections
            )
            .is_err()
        );
    }
}

#[tokio::test(start_paused = true)]
async fn operator_tag_has_strict_boundaries_and_does_not_create_numeric_rate_evidence() {
    let registry = RegistryDocument::default();
    for length in [1, 128, 129] {
        let tag = "x".repeat(length);
        let input = current("router-a", 1, ppp(Some(&tag)), &[RouterRole::Ppp]).await;
        let auth = authorize(
            &registry,
            0,
            1,
            vec![input],
            &[selection("router-a", SourceKind::Ppp)],
        )
        .unwrap();
        if length > 128 {
            conflict(&auth, IdentityIssue::InvalidReference);
            continue;
        }
        let result = auth.reconcile(&mut ObservationGate::default()).unwrap();
        let policy = &result.reconciliation.active[0].accounts[0].policy;
        assert!(policy.plan.is_none());
        assert_eq!(
            policy.profile,
            super::super::observation::ProfileObservation::Known(Some("*p".into()))
        );
    }
}

#[tokio::test(start_paused = true)]
async fn hotspot_trial_session_without_independent_reference_remains_a_critical_conflict() {
    let registry = RegistryDocument::default();
    let mut rows = BTreeMap::from([(
        Table::HotspotActive,
        vec![row(&[
            (".id", "*a"),
            ("user", "trial-user"),
            ("login-by", "trial"),
            ("address", "192.0.2.9"),
            ("mac-address", "aa:bb:cc:dd:ee:09"),
            ("server", "wifi"),
        ])],
    )]);
    rows.insert(
        Table::HotspotServers,
        vec![row(&[
            (".id", "*hs"),
            ("name", "wifi"),
            ("disabled", "false"),
        ])],
    );
    let input = current("router-a", 1, rows, &[RouterRole::Hotspot]).await;
    conflict(
        &authorize(
            &registry,
            0,
            1,
            vec![input],
            &[selection("router-a", SourceKind::Hotspot)],
        )
        .unwrap(),
        IdentityIssue::MissingReference,
    );
}

#[tokio::test(start_paused = true)]
async fn hotspot_trial_cannot_borrow_a_matching_local_users_reference_but_verified_local_logins_can()
 {
    let registry = RegistryDocument::default();
    for method in ["http-chap", "mac", "trial"] {
        let mut ppp = ppp(Some("wifi-acct"));
        let rows = BTreeMap::from([
            (
                Table::HotspotServers,
                vec![row(&[
                    (".id", "*hs"),
                    ("name", "wifi"),
                    ("disabled", "false"),
                ])],
            ),
            (
                Table::HotspotProfiles,
                ppp.remove(&Table::PppProfiles).unwrap(),
            ),
            (Table::HotspotUsers, ppp.remove(&Table::PppSecrets).unwrap()),
            (
                Table::HotspotActive,
                vec![row(&[
                    (".id", "*ha"),
                    ("user", "alice"),
                    ("login-by", method),
                    ("address", "192.0.2.8"),
                    ("mac-address", "aa:bb:cc:dd:ee:08"),
                    ("server", "wifi"),
                ])],
            ),
        ]);
        let input = current("router-a", 1, rows, &[RouterRole::Hotspot]).await;
        let auth = authorize(
            &registry,
            0,
            1,
            vec![input],
            &[selection("router-a", SourceKind::Hotspot)],
        )
        .unwrap();
        if method == "trial" {
            conflict(&auth, IdentityIssue::MissingReference);
        } else {
            assert!(auth.is_ready());
        }
    }
}

#[tokio::test(start_paused = true)]
async fn large_service_selection_is_indexed_and_collection_order_does_not_change_identity_ids() {
    let registry = RegistryDocument::default();
    let mut rows = Rows::new();
    let mut selections = vec![];
    for n in 1u32..=10_000 {
        let server = format!("lan-{n}");
        let id = format!("*d{n}");
        let reference = format!("acct-{n}");
        rows.entry(Table::DhcpServers).or_default().push(row(&[
            (".id", &id),
            ("name", &server),
            ("disabled", "false"),
        ]));
        rows.entry(Table::DhcpLeases).or_default().push(row(&[
            (".id", &format!("*l{n}")),
            ("server", &server),
            ("status", "bound"),
            (
                "active-address",
                &std::net::Ipv4Addr::from(0x0a000000 + n).to_string(),
            ),
            ("active-mac-address", "aa:bb:cc:dd:ee:02"),
            ("comment", &format!("[lqosync-id:{reference}]")),
        ]));
        let mut selected = selection("router-a", SourceKind::Dhcp);
        selected.scope.source_id = format!("svc-{n}");
        selected.service_names = vec![server];
        selections.push(selected);
    }
    let a = current("router-a", 1, rows.clone(), &[RouterRole::Dhcp]).await;
    let first = authorize(&registry, 0, 1, vec![a], &selections)
        .unwrap()
        .reconcile(&mut ObservationGate::default())
        .unwrap();
    selections.reverse();
    for table in rows.values_mut() {
        table.reverse();
    }
    let b = current("router-a", 1, rows, &[RouterRole::Dhcp]).await;
    let second = authorize(&registry, 0, 1, vec![b], &selections)
        .unwrap()
        .reconcile(&mut ObservationGate::default())
        .unwrap();
    assert_eq!(first.reconciliation.registry.subscribers.len(), 10_000);
    assert_eq!(
        first.reconciliation.registry,
        second.reconciliation.registry
    );
    assert_eq!(first.reconciliation.active, second.reconciliation.active);
}

#[tokio::test(start_paused = true)]
async fn mapped_profile_and_exact_rate_reach_current_observations_without_locator_fallback() {
    let registry = RegistryDocument::default();
    let mut selected = selection("router-a", SourceKind::Ppp);
    selected.policy.profiles[0].source_ref = "stable-profile".into();
    selected.policy.profiles[0].rate = RateRead::RateLimit(super::super::source_rate::RateFormat {
        order: crate::config::RateLimitOrder::RouterosRxTx,
        unitless: crate::config::UnitlessUnit::Bps,
    });
    let input = current("router-a", 1, ppp(Some("acct-1")), &[RouterRole::Ppp]).await;
    let result = authorize(&registry, 0, 1, vec![input], &[selected])
        .unwrap()
        .reconcile(&mut ObservationGate::default())
        .unwrap();
    let policy = &result.reconciliation.active[0].accounts[0].policy;
    assert_eq!(
        policy.profile,
        super::super::observation::ProfileObservation::Known(Some("stable-profile".into()))
    );
    assert_eq!(
        policy.plan,
        Some(super::super::rates::RateBps {
            down: 10_000_000,
            up: 2_000_000
        })
    );
}

#[tokio::test(start_paused = true)]
async fn missing_current_mapping_and_invalid_enabled_rate_withhold_the_authorized_collection() {
    let registry = RegistryDocument::default();
    for missing in [false, true] {
        let mut selected = selection("router-a", SourceKind::Ppp);
        let mut rows = ppp(Some("acct-1"));
        if missing {
            selected.policy.profiles[0].current_name = "renamed".into();
        } else {
            selected.policy.profiles[0].rate =
                RateRead::RateLimit(super::super::source_rate::RateFormat {
                    order: crate::config::RateLimitOrder::RouterosRxTx,
                    unitless: crate::config::UnitlessUnit::Bps,
                });
            rows.get_mut(&Table::PppProfiles).unwrap()[0]
                .insert("rate-limit".into(), "bad-private-rate".into());
        }
        let input = current("router-a", 1, rows, &[RouterRole::Ppp]).await;
        conflict(
            &authorize(&registry, 0, 1, vec![input], &[selected]).unwrap(),
            if missing {
                IdentityIssue::MissingProfileMapping
            } else {
                IdentityIssue::InvalidSourceRate
            },
        );
    }
}

#[tokio::test(start_paused = true)]
async fn selected_catalog_profile_filter_requires_a_binding_even_for_complete_offline_reads() {
    let registry = RegistryDocument {
        catalog: Some(super::super::catalog::tests::catalog()),
        ..Default::default()
    };
    let mut selected = selection("distro", SourceKind::Ppp);
    selected.scope.source_id = "ppp-service".into();
    for correct in [false, true] {
        if correct {
            selected.policy.profiles[0].source_ref = "provider-profile-id".into();
        }
        let input = current("distro", 1, Rows::new(), &[RouterRole::Ppp]).await;
        assert_eq!(
            authorize(
                &registry,
                0,
                1,
                vec![input],
                std::slice::from_ref(&selected)
            )
            .is_ok(),
            correct
        );
    }
}

#[tokio::test(start_paused = true)]
async fn dhcp_service_name_selection_uses_fresh_server_locators_and_rejects_unavailable_names() {
    let registry = RegistryDocument::default();
    for locator in ["*d", "*recreated"] {
        let mut rows = dhcp(Some("acct-1"));
        rows.get_mut(&Table::DhcpServers).unwrap()[0].insert(".id".into(), locator.into());
        let mut selected = selection("router-a", SourceKind::Dhcp);
        selected.service_names = vec!["lan".into()];
        let input = current("router-a", 1, rows, &[RouterRole::Dhcp]).await;
        let result = authorize(&registry, 0, 1, vec![input], &[selected])
            .unwrap()
            .reconcile(&mut ObservationGate::default())
            .unwrap();
        assert_eq!(result.reconciliation.active.len(), 1);
    }
    let mut selected = selection("router-a", SourceKind::Dhcp);
    selected.service_names = vec!["unavailable".into()];
    let input = current("router-a", 1, dhcp(Some("acct-1")), &[RouterRole::Dhcp]).await;
    assert!(authorize(&registry, 0, 1, vec![input], &[selected]).is_err());
}

#[tokio::test(start_paused = true)]
async fn hotspot_service_names_resolve_against_fresh_inventory_before_endpoint_selection() {
    let registry = RegistryDocument::default();
    for locator in ["*old", "*new"] {
        let mut ppp = ppp(Some("wifi-acct"));
        let rows = BTreeMap::from([
            (
                Table::HotspotServers,
                vec![row(&[
                    (".id", locator),
                    ("name", "wifi"),
                    ("disabled", "false"),
                ])],
            ),
            (
                Table::HotspotProfiles,
                ppp.remove(&Table::PppProfiles).unwrap(),
            ),
            (Table::HotspotUsers, ppp.remove(&Table::PppSecrets).unwrap()),
            (
                Table::HotspotActive,
                vec![row(&[
                    (".id", "*ha"),
                    ("user", "alice"),
                    ("login-by", "http-chap"),
                    ("address", "192.0.2.8"),
                    ("mac-address", "aa:bb:cc:dd:ee:08"),
                    ("server", "wifi"),
                ])],
            ),
        ]);
        let mut selected = selection("router-a", SourceKind::Hotspot);
        selected.service_names = vec!["wifi".into()];
        let input = current("router-a", 1, rows, &[RouterRole::Hotspot]).await;
        let result = authorize(&registry, 0, 1, vec![input], &[selected])
            .unwrap()
            .reconcile(&mut ObservationGate::default())
            .unwrap();
        assert_eq!(result.reconciliation.active.len(), 1);
    }
}

fn rule(router: &str, kind: SourceKind, authority: ReferenceAuthority) -> SourceSelection {
    SourceSelection {
        authority,
        ..selection(router, kind)
    }
}
fn ppp_named(names: &[&str]) -> Rows {
    let mut rows = ppp(None);
    let secret = rows[&Table::PppSecrets][0].clone();
    let active = rows[&Table::PppActive][0].clone();
    rows.insert(Table::PppSecrets, vec![]);
    rows.insert(Table::PppActive, vec![]);
    for (i, name) in names.iter().enumerate() {
        let mut s = secret.clone();
        s.insert(".id".into(), format!("*s{i}"));
        s.insert("name".into(), (*name).into());
        let mut a = active.clone();
        a.insert(".id".into(), format!("*a{i}"));
        a.insert("name".into(), (*name).into());
        a.insert("address".into(), format!("192.0.2.{}", 10 + i));
        rows.get_mut(&Table::PppSecrets).unwrap().push(s);
        rows.get_mut(&Table::PppActive).unwrap().push(a);
    }
    rows
}
fn dhcp_macs(macs: &[Option<&str>]) -> Rows {
    let mut rows = dhcp(None);
    let lease = rows[&Table::DhcpLeases][0].clone();
    rows.insert(Table::DhcpLeases, vec![]);
    for (i, mac) in macs.iter().enumerate() {
        let mut l = lease.clone();
        l.insert(".id".into(), format!("*l{i}"));
        l.insert("active-address".into(), format!("192.0.2.{}", 20 + i));
        match mac {
            Some(m) => l.insert("active-mac-address".into(), (*m).into()),
            None => l.remove("active-mac-address"),
        };
        rows.get_mut(&Table::DhcpLeases).unwrap().push(l);
    }
    rows
}
async fn refs(
    router: &str,
    rows: Rows,
    roles: &[RouterRole],
    selection: SourceSelection,
) -> (Vec<String>, Vec<IdentityIssue>) {
    let input = current(router, 1, rows, roles).await;
    let registry = RegistryDocument::default();
    let auth = authorize(&registry, 0, 1, vec![input], &[selection]).unwrap();
    let issues = auth.conflicts().iter().map(|c| c.issue).collect();
    if !auth.is_ready() {
        return (vec![], issues);
    }
    let result = auth.reconcile(&mut ObservationGate::default()).unwrap();
    let mut refs: Vec<String> = result
        .reconciliation
        .registry
        .subscribers
        .iter()
        .flat_map(|s| s.identities.iter().map(|i| i.account_ref.clone()))
        .collect();
    refs.sort();
    (refs, issues)
}

#[tokio::test(start_paused = true)]
async fn ppp_secret_name_authorizes_by_name() {
    let sel = rule(
        "router-a",
        SourceKind::Ppp,
        ReferenceAuthority::PppSecretName,
    );
    let (refs, issues) = refs(
        "router-a",
        ppp_named(&["juan dela cruz"]),
        &[RouterRole::Ppp],
        sel,
    )
    .await;
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(refs, vec!["ppp-name/v1/juan%20dela%20cruz"]);
}

#[tokio::test(start_paused = true)]
async fn ppp_names_that_differ_only_by_encoding_do_not_collide() {
    let sel = rule(
        "router-a",
        SourceKind::Ppp,
        ReferenceAuthority::PppSecretName,
    );
    let (refs, issues) = refs(
        "router-a",
        ppp_named(&["a b", "a%20b"]),
        &[RouterRole::Ppp],
        sel,
    )
    .await;
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(refs, vec!["ppp-name/v1/a%20b", "ppp-name/v1/a%2520b"]);
}

#[test]
fn encode_name_keeps_reference_characters_and_escapes_the_rest() {
    assert_eq!(encode_name("Ab9._-"), "Ab9._-");
    assert_eq!(encode_name("u@x"), "u%40x");
    assert_eq!(encode_name("ñ"), "%C3%B1");
}

#[tokio::test(start_paused = true)]
async fn same_name_on_two_routers_stays_distinct() {
    let a = current("router-a", 1, ppp_named(&["alice"]), &[RouterRole::Ppp]).await;
    let mut rows_b = ppp_named(&["alice"]);
    rows_b.get_mut(&Table::PppActive).unwrap()[0].insert("address".into(), "192.0.2.200".into());
    let b = current("router-b", 1, rows_b, &[RouterRole::Ppp]).await;
    let sels = [
        rule(
            "router-a",
            SourceKind::Ppp,
            ReferenceAuthority::PppSecretName,
        ),
        rule(
            "router-b",
            SourceKind::Ppp,
            ReferenceAuthority::PppSecretName,
        ),
    ];
    let registry = RegistryDocument::default();
    let auth = authorize(&registry, 0, 1, vec![a, b], &sels).unwrap();
    assert!(auth.conflicts().is_empty());
    let result = auth.reconcile(&mut ObservationGate::default()).unwrap();
    assert_eq!(result.reconciliation.registry.subscribers.len(), 2);
}

#[tokio::test(start_paused = true)]
async fn dhcp_client_mac_authorizes_by_lease_mac() {
    let sel = rule(
        "router-a",
        SourceKind::Dhcp,
        ReferenceAuthority::DhcpClientMac,
    );
    let (refs, issues) = refs(
        "router-a",
        dhcp_macs(&[Some("aa-bb-cc-00-11-22")]),
        &[RouterRole::Dhcp],
        sel,
    )
    .await;
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(refs, vec!["dhcp-mac/v1/AA:BB:CC:00:11:22"]);
}

#[tokio::test(start_paused = true)]
async fn duplicate_mac_on_one_dhcp_server_is_critical() {
    let sel = rule(
        "router-a",
        SourceKind::Dhcp,
        ReferenceAuthority::DhcpClientMac,
    );
    let rows = dhcp_macs(&[Some("aa:bb:cc:00:11:22"), Some("aa:bb:cc:00:11:22")]);
    let (_, issues) = refs("router-a", rows, &[RouterRole::Dhcp], sel).await;
    assert_eq!(
        issues
            .iter()
            .filter(|i| **i == IdentityIssue::DuplicateReference)
            .count(),
        2
    );
}

#[tokio::test(start_paused = true)]
async fn comment_marker_is_ignored_by_name_and_mac_rules() {
    let sel = rule(
        "router-a",
        SourceKind::Dhcp,
        ReferenceAuthority::DhcpClientMac,
    );
    let (refs, _) = refs("router-a", dhcp(Some("other")), &[RouterRole::Dhcp], sel).await;
    assert_eq!(refs, vec!["dhcp-mac/v1/AA:BB:CC:DD:EE:02"]);
}

#[tokio::test(start_paused = true)]
async fn duplicate_ppp_name_in_one_scope_fails_normalization() {
    let mut rows = ppp_named(&["alice"]);
    let mut twin = rows[&Table::PppSecrets][0].clone();
    twin.insert(".id".into(), "*twin".into());
    rows.get_mut(&Table::PppSecrets).unwrap().push(twin);
    let catalog = TopologyCatalog {
        routers: vec![CatalogRouter {
            id: "router-a".into(),
            name: "Router".into(),
            roles: vec![RouterRole::Ppp],
        }],
        ..Default::default()
    };
    let plan = ReadPlan::for_router(&catalog, "router-a").unwrap();
    let evidence = ReadGuard::new("router-a".into())
        .unwrap()
        .read(
            &mut Some(Session(rows)),
            &plan,
            1,
            Duration::from_secs(1),
            Duration::from_secs(10),
        )
        .await
        .unwrap();
    let Err(e) = normalize(&evidence) else {
        panic!("duplicate secret names must fail normalization");
    };
    assert!(format!("{e:?}").contains("DuplicateName"), "{e:?}");
}

#[tokio::test(start_paused = true)]
async fn lease_without_mac_is_missing_reference() {
    let sel = rule(
        "router-a",
        SourceKind::Dhcp,
        ReferenceAuthority::DhcpClientMac,
    );
    let input = current(
        "router-a",
        1,
        dhcp_macs(&[None, Some("aa:bb:cc:00:11:22")]),
        &[RouterRole::Dhcp],
    )
    .await;
    let registry = RegistryDocument::default();
    let auth = authorize(&registry, 0, 1, vec![input], &[sel]).unwrap();
    let issues: Vec<_> = auth
        .conflicts()
        .iter()
        .map(|c| (c.row_ref.as_str(), c.issue))
        .collect();
    assert_eq!(issues, vec![("*l0", IdentityIssue::MissingReference)]);
}
