//! Current observations and identity reconciliation. No archived endpoint input exists here.

use super::model::{
    IdentityRef, RegistryDocument, SourceScope, SubscriberPolicy, SubscriberRecord,
};
use anyhow::{Context, ensure};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ProfileObservation {
    #[default]
    Unread,
    Known(Option<String>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObservedPolicy {
    pub profile: ProfileObservation,
    pub plan: Option<super::rates::RateBps>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveAccount {
    pub identity: IdentityRef,
    pub policy: ObservedPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub session_id: String,
    pub ips: Vec<IpAddr>,
    pub mac: Option<String>,
}

#[derive(Debug)]
pub struct AccountObservation {
    pub account_ref: String,
    pub endpoints: Vec<Endpoint>,
    pub policy: ObservedPolicy,
}

#[derive(Debug)]
pub enum SourceRead {
    Complete {
        scope: SourceScope,
        accounts: Vec<AccountObservation>,
    },
    Partial {
        scope: SourceScope,
    },
    Failed {
        scope: SourceScope,
    },
}

#[derive(Debug)]
pub struct CompleteSnapshot {
    generation: u64,
    scopes: Vec<SourceScope>,
    accounts: Vec<(IdentityRef, Vec<Endpoint>, ObservedPolicy)>,
}

#[derive(Debug, Default)]
pub struct ObservationGate {
    last_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveEndpoint {
    pub identity: IdentityRef,
    pub endpoint: Endpoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSubscriber {
    pub subscriber_id: String,
    pub endpoints: Vec<ActiveEndpoint>,
    pub accounts: Vec<ActiveAccount>,
}

#[derive(Debug)]
pub struct Reconciliation {
    pub generation: u64,
    pub registry: RegistryDocument,
    pub active: Vec<ActiveSubscriber>,
}

impl ObservationGate {
    pub fn accept(
        &mut self,
        generation: u64,
        required: &[SourceScope],
        reads: Vec<SourceRead>,
    ) -> anyhow::Result<CompleteSnapshot> {
        ensure!(
            generation > self.last_generation,
            "collection generation must advance"
        );
        // Failed attempts cannot be repaired with retained replies under the same generation.
        self.last_generation = generation;
        ensure!(
            !required.is_empty(),
            "collection requires at least one source scope"
        );
        let expected: HashSet<_> = required.iter().collect();
        ensure!(
            expected.len() == required.len(),
            "duplicate required source scope"
        );
        for scope in required {
            scope.validate()?;
        }
        let mut completed = HashSet::with_capacity(required.len());
        let mut identities = HashSet::new();
        let mut sessions = HashSet::new();
        let mut ips = HashSet::new();
        let mut accounts = Vec::new();
        for read in reads {
            let (scope, observed) = match read {
                SourceRead::Complete { scope, accounts } => (scope, accounts),
                SourceRead::Partial { .. } => anyhow::bail!("required source read is partial"),
                SourceRead::Failed { .. } => anyhow::bail!("required source read failed"),
            };
            ensure!(expected.contains(&scope), "unexpected source scope");
            ensure!(completed.insert(scope.clone()), "duplicate source read");
            for account in observed {
                if let ProfileObservation::Known(Some(profile)) = &account.policy.profile {
                    ensure!(
                        !profile.trim().is_empty(),
                        "current source profile reference must not be empty"
                    );
                }
                if let Some(rate) = account.policy.plan {
                    rate.validate()?;
                }
                let identity = IdentityRef {
                    scope: scope.clone(),
                    account_ref: account.account_ref,
                };
                identity.validate()?;
                ensure!(
                    identities.insert(identity.clone()),
                    "duplicate current account observation"
                );
                ensure!(
                    !account.endpoints.is_empty(),
                    "active account requires a current endpoint"
                );
                let mut endpoints = account.endpoints;
                for endpoint in &mut endpoints {
                    ensure!(
                        !endpoint.session_id.trim().is_empty(),
                        "current session ID must not be empty"
                    );
                    ensure!(
                        sessions.insert((scope.clone(), endpoint.session_id.clone())),
                        "duplicate current source session"
                    );
                    ensure!(
                        !endpoint.ips.is_empty(),
                        "current endpoint requires an IP address"
                    );
                    for ip in &endpoint.ips {
                        ensure!(ips.insert(*ip), "conflicting current IP address ownership");
                    }
                    if let Some(mac) = &endpoint.mac {
                        endpoint.mac = Some(
                            crate::config::normalize_mac(mac)
                                .context("invalid current endpoint MAC")?,
                        );
                    }
                    endpoint.ips.sort_unstable();
                }
                accounts.push((identity, endpoints, account.policy));
            }
        }
        ensure!(
            completed.len() == expected.len(),
            "required source read is missing"
        );
        Ok(CompleteSnapshot {
            generation,
            scopes: {
                let mut scopes = required.to_vec();
                scopes.sort_unstable();
                scopes
            },
            accounts,
        })
    }
}

pub fn reconcile(
    registry: &RegistryDocument,
    snapshot: CompleteSnapshot,
) -> anyhow::Result<Reconciliation> {
    let index = registry.index()?;
    if let Some(expected) = &index.required_scopes {
        ensure!(
            *expected == snapshot.scopes,
            "collection scopes do not match the configured catalog"
        );
    }
    let mut candidate = registry.clone();
    let mut unknown: Vec<_> = snapshot
        .accounts
        .iter()
        .map(|(identity, _, _)| identity)
        .filter(|identity| !index.by_identity.contains_key(identity))
        .collect();
    // New identity order determines ID allocation, independently of router reply order.
    unknown.sort_unstable();
    let mut allocated = HashMap::with_capacity(unknown.len());
    for identity in unknown {
        let id = format!("tm-sub-{:016x}", candidate.next_id);
        candidate.next_id = candidate
            .next_id
            .checked_add(1)
            .context("subscriber ID counter is exhausted")?;
        let position = candidate.subscribers.len();
        candidate.subscribers.push(SubscriberRecord {
            id,
            identities: vec![identity.clone()],
            policy: SubscriberPolicy::default(),
        });
        allocated.insert(identity.clone(), position);
    }

    let mut active: Vec<ActiveSubscriber> = Vec::new();
    let mut active_index = HashMap::new();
    for (identity, endpoints, policy) in snapshot.accounts {
        let record = *index
            .by_identity
            .get(&identity)
            .or_else(|| allocated.get(&identity))
            .context("current identity has no registry record")?;
        let position = *active_index.entry(record).or_insert_with(|| {
            let position = active.len();
            active.push(ActiveSubscriber {
                subscriber_id: candidate.subscribers[record].id.clone(),
                endpoints: Vec::new(),
                accounts: Vec::new(),
            });
            position
        });
        active[position].accounts.push(ActiveAccount {
            identity: identity.clone(),
            policy,
        });
        active[position]
            .endpoints
            .extend(endpoints.into_iter().map(|endpoint| ActiveEndpoint {
                identity: identity.clone(),
                endpoint,
            }));
    }
    for subscriber in &mut active {
        subscriber
            .accounts
            .sort_unstable_by(|a, b| a.identity.cmp(&b.identity));
        subscriber.endpoints.sort_unstable_by(|a, b| {
            (&a.identity, &a.endpoint.session_id).cmp(&(&b.identity, &b.endpoint.session_id))
        });
    }
    active.sort_unstable_by(|a, b| a.subscriber_id.cmp(&b.subscriber_id));
    Ok(Reconciliation {
        generation: snapshot.generation,
        registry: candidate,
        active,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology_manager::model::{SourceKind, SubscriberPolicy};

    #[test]
    fn fresh_policy_evidence_survives_reconciliation_without_durable_session_data() {
        let mut observed = account("account", vec![endpoint("s", "192.0.2.1", None)]);
        observed.policy = ObservedPolicy {
            profile: ProfileObservation::Known(Some("current-profile".into())),
            plan: Some(super::super::rates::RateBps {
                down: 50_000_000,
                up: 10_000_000,
            }),
        };
        let result = reconcile(&RegistryDocument::default(), complete(vec![observed])).unwrap();
        assert_eq!(
            result.active[0].accounts[0].policy.profile,
            ProfileObservation::Known(Some("current-profile".into()))
        );
        assert_eq!(
            result.active[0].accounts[0].policy.plan.unwrap().down,
            50_000_000
        );
        let saved = serde_json::to_string(&result.registry).unwrap();
        assert!(!saved.contains("current-profile"));
        assert!(!saved.contains("192.0.2.1"));
    }

    #[test]
    fn invalid_fresh_policy_evidence_fails_the_snapshot() {
        for policy in [
            ObservedPolicy {
                profile: ProfileObservation::Known(Some(" ".into())),
                plan: None,
            },
            ObservedPolicy {
                profile: ProfileObservation::Unread,
                plan: Some(super::super::rates::RateBps {
                    down: 0,
                    up: 10_000_000,
                }),
            },
        ] {
            let mut observed = account("account", vec![endpoint("s", "192.0.2.1", None)]);
            observed.policy = policy;
            assert!(
                ObservationGate::default()
                    .accept(
                        1,
                        &[scope()],
                        vec![SourceRead::Complete {
                            scope: scope(),
                            accounts: vec![observed]
                        }]
                    )
                    .is_err()
            );
        }
    }

    fn scope() -> SourceScope {
        SourceScope {
            router_id: "router-1".into(),
            source_id: "service-1".into(),
            kind: SourceKind::Ppp,
        }
    }

    fn endpoint(session: &str, ip: &str, mac: Option<&str>) -> Endpoint {
        Endpoint {
            session_id: session.into(),
            ips: vec![ip.parse().unwrap()],
            mac: mac.map(str::to_owned),
        }
    }

    fn account(name: &str, endpoints: Vec<Endpoint>) -> AccountObservation {
        AccountObservation {
            account_ref: name.into(),
            endpoints,
            policy: ObservedPolicy::default(),
        }
    }

    fn complete(accounts: Vec<AccountObservation>) -> CompleteSnapshot {
        ObservationGate::default()
            .accept(
                1,
                &[scope()],
                vec![SourceRead::Complete {
                    scope: scope(),
                    accounts,
                }],
            )
            .unwrap()
    }

    fn one(name: &str, ip: &str) -> CompleteSnapshot {
        complete(vec![account(name, vec![endpoint("session-1", ip, None)])])
    }

    #[test]
    fn successful_empty_read_confirms_absence_without_deleting_registry() {
        let first = reconcile(&RegistryDocument::default(), one("account-1", "192.0.2.1")).unwrap();
        let gone = reconcile(&first.registry, complete(Vec::new())).unwrap();
        assert!(gone.active.is_empty());
        assert_eq!(gone.registry, first.registry);
        assert_eq!(gone.generation, 1);
    }

    #[test]
    fn incomplete_and_ambiguous_scope_sets_cannot_make_a_snapshot() {
        let reads = [
            Vec::new(),
            vec![SourceRead::Partial { scope: scope() }],
            vec![SourceRead::Failed { scope: scope() }],
            vec![
                SourceRead::Complete {
                    scope: scope(),
                    accounts: Vec::new(),
                },
                SourceRead::Complete {
                    scope: scope(),
                    accounts: Vec::new(),
                },
            ],
        ];
        for reads in reads {
            assert!(
                ObservationGate::default()
                    .accept(1, &[scope()], reads)
                    .is_err()
            );
        }
        assert!(
            ObservationGate::default()
                .accept(1, &[], Vec::new())
                .is_err()
        );
        assert!(
            ObservationGate::default()
                .accept(1, &[scope(), scope()], Vec::new())
                .is_err()
        );
        let other = SourceScope {
            router_id: "unexpected".into(),
            ..scope()
        };
        assert!(
            ObservationGate::default()
                .accept(
                    1,
                    &[scope()],
                    vec![SourceRead::Complete {
                        scope: other,
                        accounts: Vec::new()
                    }]
                )
                .is_err()
        );
    }

    #[test]
    fn reused_attempts_are_rejected_even_after_failure() {
        let mut gate = ObservationGate::default();
        assert!(
            gate.accept(1, &[scope()], vec![SourceRead::Failed { scope: scope() }])
                .is_err()
        );
        assert!(
            gate.accept(
                1,
                &[scope()],
                vec![SourceRead::Complete {
                    scope: scope(),
                    accounts: Vec::new()
                }]
            )
            .is_err()
        );
        gate.accept(
            2,
            &[scope()],
            vec![SourceRead::Complete {
                scope: scope(),
                accounts: Vec::new(),
            }],
        )
        .unwrap();
        assert!(
            gate.accept(
                2,
                &[scope()],
                vec![SourceRead::Complete {
                    scope: scope(),
                    accounts: Vec::new()
                }]
            )
            .is_err()
        );
        assert!(gate.accept(0, &[scope()], Vec::new()).is_err());
    }

    #[test]
    fn reconnect_changes_endpoints_and_keeps_id_and_policy() {
        let mut first = reconcile(
            &RegistryDocument::default(),
            complete(vec![account(
                "account-1",
                vec![endpoint(
                    "old-session",
                    "192.0.2.1",
                    Some("aa-bb-cc-dd-ee-01"),
                )],
            )]),
        )
        .unwrap();
        first.registry.subscribers[0].policy = SubscriberPolicy {
            profile_id: Some("plan".into()),
            assignment: Some("wan-1-instance".into()),
            pin: Some("wan-1-instance".into()),
        };
        let returned = reconcile(
            &first.registry,
            complete(vec![account(
                "account-1",
                vec![endpoint(
                    "new-session",
                    "192.0.2.99",
                    Some("aa-bb-cc-dd-ee-99"),
                )],
            )]),
        )
        .unwrap();
        assert_eq!(returned.registry, first.registry);
        assert_eq!(
            returned.active[0].subscriber_id,
            first.active[0].subscriber_id
        );
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
    }

    #[test]
    fn current_address_reassignment_does_not_restore_the_previous_owner() {
        let first = reconcile(&RegistryDocument::default(), one("account-1", "192.0.2.1")).unwrap();
        let second = reconcile(&first.registry, one("different-account", "192.0.2.1")).unwrap();
        assert_eq!(second.registry.subscribers.len(), 2);
        assert_ne!(
            second.active[0].subscriber_id,
            first.active[0].subscriber_id
        );
        assert_eq!(
            second.registry.subscribers[1].policy,
            SubscriberPolicy::default()
        );
    }

    #[test]
    fn several_current_sessions_remain_one_subscriber() {
        let result = reconcile(
            &RegistryDocument::default(),
            complete(vec![account(
                "account-1",
                vec![
                    endpoint("session-b", "192.0.2.2", None),
                    endpoint("session-a", "192.0.2.1", None),
                ],
            )]),
        )
        .unwrap();
        assert_eq!(result.registry.subscribers.len(), 1);
        assert_eq!(result.active.len(), 1);
        assert_eq!(result.active[0].endpoints.len(), 2);
        assert_eq!(
            result.active[0].endpoints[0].endpoint.session_id,
            "session-a"
        );
    }

    #[test]
    fn explicit_aliases_merge_current_endpoints_without_a_new_circuit() {
        let mut first =
            reconcile(&RegistryDocument::default(), one("account-1", "192.0.2.1")).unwrap();
        first.registry.subscribers[0].identities.push(IdentityRef {
            scope: scope(),
            account_ref: "verified-alias".into(),
        });
        let result = reconcile(
            &first.registry,
            complete(vec![
                account("account-1", vec![endpoint("a", "192.0.2.1", None)]),
                account("verified-alias", vec![endpoint("b", "192.0.2.2", None)]),
            ]),
        )
        .unwrap();
        assert_eq!(result.registry, first.registry);
        assert_eq!(result.active.len(), 1);
        assert_eq!(result.active[0].endpoints.len(), 2);
    }

    #[test]
    fn absent_identity_or_duplicate_accounts_are_conflicts() {
        for accounts in [
            vec![account("", vec![endpoint("a", "192.0.2.1", None)])],
            vec![account("account", Vec::new())],
            vec![
                account("account", vec![endpoint("a", "192.0.2.1", None)]),
                account("account", vec![endpoint("b", "192.0.2.2", None)]),
            ],
        ] {
            assert!(
                ObservationGate::default()
                    .accept(
                        1,
                        &[scope()],
                        vec![SourceRead::Complete {
                            scope: scope(),
                            accounts
                        }]
                    )
                    .is_err()
            );
        }
    }

    #[test]
    fn duplicate_sessions_and_conflicting_packet_visible_addresses_are_conflicts() {
        for accounts in [
            vec![
                account("one", vec![endpoint("same", "192.0.2.1", None)]),
                account("two", vec![endpoint("same", "192.0.2.2", None)]),
            ],
            vec![
                account("one", vec![endpoint("a", "192.0.2.1", None)]),
                account("two", vec![endpoint("b", "192.0.2.1", None)]),
            ],
        ] {
            assert!(
                ObservationGate::default()
                    .accept(
                        1,
                        &[scope()],
                        vec![SourceRead::Complete {
                            scope: scope(),
                            accounts
                        }]
                    )
                    .is_err()
            );
        }
        let other = SourceScope {
            router_id: "other-router".into(),
            ..scope()
        };
        let reads = vec![
            SourceRead::Complete {
                scope: scope(),
                accounts: vec![account("one", vec![endpoint("a", "192.0.2.1", None)])],
            },
            SourceRead::Complete {
                scope: other.clone(),
                accounts: vec![account("two", vec![endpoint("b", "192.0.2.1", None)])],
            },
        ];
        assert!(
            ObservationGate::default()
                .accept(1, &[scope(), other], reads)
                .is_err()
        );
    }

    #[test]
    fn endpoint_fields_must_come_from_usable_current_data() {
        for bad in [
            Endpoint {
                session_id: "".into(),
                ..endpoint("x", "192.0.2.1", None)
            },
            Endpoint {
                ips: Vec::new(),
                ..endpoint("x", "192.0.2.1", None)
            },
            endpoint("x", "192.0.2.1", Some("invalid")),
        ] {
            assert!(
                ObservationGate::default()
                    .accept(
                        1,
                        &[scope()],
                        vec![SourceRead::Complete {
                            scope: scope(),
                            accounts: vec![account("account", vec![bad])]
                        }]
                    )
                    .is_err()
            );
        }
    }

    #[test]
    fn id_allocation_is_deterministic_and_existing_records_do_not_change() {
        let forward = reconcile(
            &RegistryDocument::default(),
            complete(vec![
                account("a", vec![endpoint("a", "192.0.2.1", None)]),
                account("b", vec![endpoint("b", "192.0.2.2", None)]),
            ]),
        )
        .unwrap();
        let backward = reconcile(
            &RegistryDocument::default(),
            complete(vec![
                account("b", vec![endpoint("b", "192.0.2.2", None)]),
                account("a", vec![endpoint("a", "192.0.2.1", None)]),
            ]),
        )
        .unwrap();
        assert_eq!(forward.registry, backward.registry);
        assert_eq!(forward.active, backward.active);
        assert_eq!(forward.registry.next_id, 3);
        assert_eq!(forward.active[0].subscriber_id, "tm-sub-0000000000000001");
        let unchanged = reconcile(&forward.registry, one("a", "192.0.2.99")).unwrap();
        assert_eq!(unchanged.registry, forward.registry);
        unchanged.registry.validate().unwrap();
    }

    #[test]
    fn exhausted_id_counter_fails_without_mutating_input() {
        let registry = RegistryDocument {
            next_id: u64::MAX,
            ..RegistryDocument::default()
        };
        assert!(reconcile(&registry, one("a", "192.0.2.1")).is_err());
        assert!(registry.subscribers.is_empty());
        assert_eq!(registry.next_id, u64::MAX);
    }

    #[test]
    fn large_current_snapshot_keeps_two_sessions_per_circuit_and_stable_registry() {
        fn large_accounts(offset: u32) -> Vec<AccountObservation> {
            (0..10_000u32)
                .rev()
                .map(|i| {
                    let address =
                        |session| std::net::Ipv4Addr::from(0x6440_0001 + offset + i * 2 + session);
                    account(
                        &format!("account-{i:05}"),
                        vec![
                            Endpoint {
                                session_id: format!("{i}-a"),
                                ips: vec![address(0).into()],
                                mac: None,
                            },
                            Endpoint {
                                session_id: format!("{i}-b"),
                                ips: vec![address(1).into()],
                                mac: None,
                            },
                        ],
                    )
                })
                .collect()
        }
        let first = reconcile(&RegistryDocument::default(), complete(large_accounts(0))).unwrap();
        assert_eq!(first.registry.subscribers.len(), 10_000);
        assert_eq!(first.active.len(), 10_000);
        assert_eq!(first.registry.next_id, 10_001);
        assert!(
            first
                .active
                .iter()
                .all(|subscriber| subscriber.endpoints.len() == 2)
        );
        let returned = reconcile(&first.registry, complete(large_accounts(50_000))).unwrap();
        assert_eq!(returned.registry, first.registry);
        assert_eq!(returned.active.len(), 10_000);
        assert!(
            returned
                .active
                .iter()
                .all(|subscriber| subscriber.endpoints.len() == 2)
        );
        assert_ne!(
            returned.active[0].endpoints[0].endpoint.ips,
            first.active[0].endpoints[0].endpoint.ips
        );
    }

    #[test]
    fn a_configured_catalog_requires_its_selected_collection_scopes() {
        let registry = RegistryDocument {
            catalog: Some(super::super::catalog::tests::catalog()),
            ..RegistryDocument::default()
        };
        assert!(reconcile(&registry, one("wrong-scope-account", "192.0.2.1")).is_err());
        let scope = SourceScope {
            router_id: "distro".into(),
            source_id: "ppp-service".into(),
            kind: SourceKind::Ppp,
        };
        let snapshot = ObservationGate::default()
            .accept(
                1,
                std::slice::from_ref(&scope),
                vec![SourceRead::Complete {
                    scope: scope.clone(),
                    accounts: vec![account(
                        "valid-account",
                        vec![endpoint("session", "192.0.2.1", None)],
                    )],
                }],
            )
            .unwrap();
        let result = reconcile(&registry, snapshot).unwrap();
        result.registry.validate().unwrap();
        assert_eq!(result.active.len(), 1);
    }
}
