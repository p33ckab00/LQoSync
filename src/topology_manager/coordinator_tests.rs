use super::*;
use crate::model::RosRow;
use crate::routeros::{ReadError, RouterSession};
use crate::topology_manager::{
    catalog::RouterRole,
    identity::{ReferenceAuthority, SourceSelection},
    reader::Table,
    settings::SourceSettings,
    source_policy::{ProfileBinding, RateRead, SourcePolicy},
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Default)]
struct Data {
    rows: std::collections::BTreeMap<Table, Vec<RosRow>>,
    connect_delay: Duration,
    read_delay: Duration,
    fail: bool,
}
#[derive(Default)]
struct Fake {
    data: Mutex<HashMap<String, Data>>,
    connects: AtomicUsize,
    alive: AtomicUsize,
}
struct Session {
    fake: Arc<Fake>,
    name: String,
}
impl Drop for Session {
    fn drop(&mut self) {
        self.fake.alive.fetch_sub(1, Ordering::SeqCst);
    }
}
struct Connection(Arc<Fake>);
impl Connector for Connection {
    type Session = Session;
    async fn connect(
        &self,
        router: &crate::config::RouterCfg,
        _: &crate::config::RouterAccess,
    ) -> Result<Session, ReadError> {
        self.0.connects.fetch_add(1, Ordering::SeqCst);
        let delay = self.0.data.lock().unwrap()[&router.name].connect_delay;
        tokio::time::sleep(delay).await;
        self.0.alive.fetch_add(1, Ordering::SeqCst);
        Ok(Session {
            fake: self.0.clone(),
            name: router.name.clone(),
        })
    }
}
impl RouterSession for Session {
    async fn print(&mut self, path: &str, fields: &str) -> Result<Vec<RosRow>, ReadError> {
        let table = [
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
        .ok_or_else(|| ReadError::Trap("unselected protocol was read".into()))?;
        let (delay, fail, rows) = {
            let data = self.fake.data.lock().unwrap();
            let data = &data[&self.name];
            (
                data.read_delay,
                data.fail,
                data.rows
                    .get(&table)
                    .cloned()
                    .ok_or_else(|| ReadError::Trap("unselected protocol was read".into()))?,
            )
        };
        tokio::time::sleep(delay).await;
        if fail {
            Err(ReadError::Trap("private account information".into()))
        } else {
            Ok(rows)
        }
    }
}
fn row(fields: &[(&str, &str)]) -> RosRow {
    fields
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
fn data() -> Data {
    Data {
        rows: std::collections::BTreeMap::from([
            (
                Table::PppSecrets,
                vec![row(&[
                    (".id", "*s"),
                    ("name", "alice"),
                    ("profile", "gold"),
                    ("disabled", "false"),
                    ("comment", "[lqosync-id:customer-1]"),
                ])],
            ),
            (
                Table::PppProfiles,
                vec![row(&[
                    (".id", "*p"),
                    ("name", "gold"),
                    ("rate-limit", "10M/50M"),
                ])],
            ),
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
        ]),
        ..Default::default()
    }
}
fn fixture() -> (Config, RegistryDocument, Arc<Fake>, Coordinator<Connection>) {
    let cfg = Config::from_toml_str("[[routers]]\nname='Distro'\naddress='192.0.2.1'\nusername='u'\npassword_file='/not-used'\n[routers.pppoe]\nenabled=true\n").unwrap();
    let mut registry = super::super::publication::tests::input(0)
        .reconciliation
        .registry;
    let scope = registry
        .catalog
        .as_ref()
        .unwrap()
        .required_scopes()
        .unwrap()[0]
        .clone();
    registry.source_settings = Some(SourceSettings {
        selections: vec![SourceSelection {
            scope,
            service_names: vec![],
            authority: ReferenceAuthority::OperatorManagedComment,
            policy: SourcePolicy {
                profiles: vec![ProfileBinding {
                    source_ref: "provider-profile-id".into(),
                    current_name: "gold".into(),
                    rate: RateRead::Disabled,
                }],
                ..Default::default()
            },
        }],
    });
    let fake = Arc::new(Fake::default());
    fake.data.lock().unwrap().insert("Distro".into(), data());
    let coordinator = Coordinator::new(Arc::new(Connection(fake.clone())));
    (cfg, registry, fake, coordinator)
}

#[tokio::test(start_paused = true)]
async fn selected_sources_prepare_a_fresh_pair_without_reading_other_protocols_or_wans() {
    let (cfg, registry, _, mut coordinator) = fixture();
    let pair = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(pair.rows().len(), 1);
    assert_eq!(pair.rows()[0].ipv4, "192.0.2.1");
    assert_eq!(pair.rows()[0].download_max, "50");
    assert_eq!(pair.network().as_object().unwrap().len(), 2);
}

#[tokio::test(start_paused = true)]
async fn invalid_source_bindings_stop_before_any_connection() {
    for case in 0..5 {
        let (mut cfg, mut registry, fake, mut coordinator) = fixture();
        match case {
            0 => cfg.routers.clear(),
            1 => cfg.routers.push(cfg.routers[0].clone()),
            2 => cfg.routers[0].pppoe.enabled = false,
            3 => {
                registry.source_settings.as_mut().unwrap().selections[0].authority =
                    ReferenceAuthority::Unconfigured
            }
            _ => registry.catalog.as_mut().unwrap().routers[1].roles = vec![RouterRole::Dhcp],
        }
        assert!(
            coordinator
                .prepare(&cfg, &registry, 0, Duration::from_secs(10))
                .await
                .is_err()
        );
        assert_eq!(fake.connects.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test(start_paused = true)]
async fn disconnect_and_reconnect_use_current_endpoints_with_the_same_authorized_identity() {
    let (cfg, registry, fake, mut coordinator) = fixture();
    let first = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    let registry = first.candidate.clone();
    fake.data
        .lock()
        .unwrap()
        .get_mut("Distro")
        .unwrap()
        .rows
        .insert(Table::PppActive, vec![]);
    let offline = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert!(offline.rows().is_empty());
    let mut online = data();
    let active = &mut online.rows.get_mut(&Table::PppActive).unwrap()[0];
    active.insert("address".into(), "192.0.2.99".into());
    active.insert("caller-id".into(), "aa:bb:cc:dd:ee:99".into());
    fake.data.lock().unwrap().insert("Distro".into(), online);
    let reconnect = coordinator
        .prepare(&cfg, &offline.candidate, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(reconnect.rows()[0].circuit_id, first.rows()[0].circuit_id);
    assert_eq!(reconnect.rows()[0].ipv4, "192.0.2.99");
    assert_eq!(reconnect.rows()[0].mac, "AA:BB:CC:DD:EE:99");
    assert!(reconnect.generation() > offline.generation());
}

#[tokio::test(start_paused = true)]
async fn failed_reads_cannot_reuse_old_subscribers_or_expose_raw_errors() {
    let (cfg, registry, fake, mut coordinator) = fixture();
    let first = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    fake.data.lock().unwrap().get_mut("Distro").unwrap().fail = true;
    let error = coordinator
        .prepare(&cfg, &first.candidate, 0, Duration::from_secs(10))
        .await
        .err()
        .unwrap();
    assert!(!format!("{error:#}").contains("private account information"));
    assert_eq!(fake.alive.load(Ordering::SeqCst), 0);
    let mut empty = data();
    empty.rows.insert(Table::PppActive, vec![]);
    fake.data.lock().unwrap().insert("Distro".into(), empty);
    let next = coordinator
        .prepare(&cfg, &first.candidate, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert!(next.rows().is_empty());
    assert!(next.generation() > first.generation() + 1);
}

#[tokio::test(start_paused = true)]
async fn connection_time_counts_against_the_batch_deadline() {
    let (mut cfg, registry, fake, mut coordinator) = fixture();
    cfg.router_access.connect_timeout = 30;
    fake.data
        .lock()
        .unwrap()
        .get_mut("Distro")
        .unwrap()
        .connect_delay = Duration::from_secs(5);
    let started = Instant::now();
    assert!(
        coordinator
            .prepare(&cfg, &registry, 0, Duration::from_secs(2))
            .await
            .is_err()
    );
    assert!(started.elapsed() <= Duration::from_secs(2));
    assert_eq!(fake.alive.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn cancelled_batches_close_sessions_and_next_attempt_reads_again() {
    let (cfg, registry, fake, mut coordinator) = fixture();
    fake.data
        .lock()
        .unwrap()
        .get_mut("Distro")
        .unwrap()
        .read_delay = Duration::from_secs(5);
    assert!(
        tokio::time::timeout(
            Duration::from_secs(1),
            coordinator.prepare(&cfg, &registry, 0, Duration::from_secs(20))
        )
        .await
        .is_err()
    );
    tokio::task::yield_now().await;
    assert_eq!(fake.alive.load(Ordering::SeqCst), 0);
    fake.data
        .lock()
        .unwrap()
        .get_mut("Distro")
        .unwrap()
        .read_delay = Duration::ZERO;
    let next = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(next.rows().len(), 1);
    assert_eq!(fake.connects.load(Ordering::SeqCst), 2);
}

#[tokio::test(start_paused = true)]
async fn missing_independent_reference_blocks_preparation() {
    let (cfg, registry, fake, mut coordinator) = fixture();
    fake.data
        .lock()
        .unwrap()
        .get_mut("Distro")
        .unwrap()
        .rows
        .get_mut(&Table::PppSecrets)
        .unwrap()[0]
        .remove("comment");
    assert!(
        coordinator
            .prepare(&cfg, &registry, 0, Duration::from_secs(10))
            .await
            .is_err()
    );
    assert!(registry.subscribers.is_empty());
}

#[tokio::test(start_paused = true)]
async fn selected_routers_collect_in_parallel_and_one_failure_blocks_the_whole_batch() {
    let (mut cfg, mut registry, fake, mut coordinator) = fixture();
    let mut router = cfg.routers[0].clone();
    router.name = "Distro2".into();
    cfg.routers.push(router);
    let catalog = registry.catalog.as_mut().unwrap();
    let mut router = catalog.routers[1].clone();
    router.id = "distro2".into();
    router.name = "Distro2".into();
    catalog.routers.push(router);
    let mut source = catalog.sources[2].clone();
    source.id = "source2".into();
    if let super::super::catalog::SourceOrigin::Subscribers { scope, .. } = &mut source.origin {
        scope.router_id = "distro2".into();
    }
    source.targets = vec!["group3".into(), "group4".into()];
    for (index, id) in [(2, "group3"), (3, "group4")] {
        let mut node = catalog.nodes[index].clone();
        node.id = id.into();
        node.source_id = Some("source2".into());
        catalog.nodes.push(node);
    }
    catalog.sources.push(source);
    let mut selection = registry.source_settings.as_ref().unwrap().selections[0].clone();
    selection.scope.router_id = "distro2".into();
    registry
        .source_settings
        .as_mut()
        .unwrap()
        .selections
        .push(selection);
    let mut second = data();
    second.rows.get_mut(&Table::PppActive).unwrap()[0].insert("address".into(), "192.0.2.2".into());
    {
        let mut all = fake.data.lock().unwrap();
        all.insert("Distro2".into(), second);
        for data in all.values_mut() {
            data.read_delay = Duration::from_millis(200);
        }
    }
    let started = Instant::now();
    let pair = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(pair.rows().len(), 2);
    assert!(started.elapsed() < Duration::from_secs(2));
    fake.data.lock().unwrap().get_mut("Distro2").unwrap().fail = true;
    assert!(
        coordinator
            .prepare(&cfg, &pair.candidate, 0, Duration::from_secs(10))
            .await
            .is_err()
    );
}

#[tokio::test(start_paused = true)]
async fn clearing_sessions_reconnects_without_reusing_generation_numbers() {
    let (cfg, registry, fake, mut coordinator) = fixture();
    let first = coordinator
        .prepare(&cfg, &registry, 50, Duration::from_secs(10))
        .await
        .unwrap();
    let second = coordinator
        .prepare(&cfg, &first.candidate, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(fake.connects.load(Ordering::SeqCst), 1);
    coordinator.clear_sessions();
    assert_eq!(fake.alive.load(Ordering::SeqCst), 0);
    let third = coordinator
        .prepare(&cfg, &second.candidate, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert!(first.generation() > 50);
    assert!(third.generation() > second.generation());
    assert_eq!(fake.connects.load(Ordering::SeqCst), 2);
}

#[tokio::test(start_paused = true)]
async fn total_read_time_cannot_refresh_the_deadline_after_each_table() {
    let (cfg, registry, fake, mut coordinator) = fixture();
    fake.data
        .lock()
        .unwrap()
        .get_mut("Distro")
        .unwrap()
        .read_delay = Duration::from_millis(600);
    let started = Instant::now();
    assert!(
        coordinator
            .prepare(&cfg, &registry, 0, Duration::from_secs(2))
            .await
            .is_err()
    );
    assert!(started.elapsed() <= Duration::from_secs(2));
    tokio::task::yield_now().await;
    assert_eq!(fake.alive.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn zero_age_and_exhausted_generation_do_not_connect() {
    let (cfg, registry, fake, mut coordinator) = fixture();
    assert!(
        coordinator
            .prepare(&cfg, &registry, 0, Duration::ZERO)
            .await
            .is_err()
    );
    assert!(
        coordinator
            .prepare(&cfg, &registry, u64::MAX, Duration::from_secs(10))
            .await
            .is_err()
    );
    assert_eq!(fake.connects.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn dhcp_and_hotspot_use_the_selected_protocol_and_current_active_state() {
    for dhcp in [true, false] {
        let (mut cfg, mut registry, fake, mut coordinator) = fixture();
        let kind = if dhcp {
            super::super::model::SourceKind::Dhcp
        } else {
            super::super::model::SourceKind::Hotspot
        };
        cfg.routers[0].pppoe.enabled = false;
        cfg.routers[0].dhcp.enabled = dhcp;
        cfg.routers[0].dhcp.require_bound = true;
        cfg.routers[0].dhcp.shape_unassigned = false;
        cfg.routers[0].hotspot.enabled = !dhcp;
        let catalog = registry.catalog.as_mut().unwrap();
        if let super::super::catalog::SourceOrigin::Subscribers {
            scope,
            profile_ref,
            group,
        } = &mut catalog.sources[2].origin
        {
            scope.kind = kind;
            if dhcp {
                *profile_ref = None;
            } else {
                group.formula = super::super::catalog::GroupFormula::CountTimesUnit(
                    super::super::rates::RateBps {
                        down: 50_000_000,
                        up: 10_000_000,
                    },
                );
            }
        }
        let selection = &mut registry.source_settings.as_mut().unwrap().selections[0];
        selection.scope.kind = kind;
        if dhcp {
            selection.policy.profiles.clear();
        }
        let active_table;
        let rows = if dhcp {
            active_table = Table::DhcpLeases;
            std::collections::BTreeMap::from([
                (
                    Table::DhcpServers,
                    vec![row(&[
                        (".id", "*d"),
                        ("name", "lan"),
                        ("disabled", "false"),
                    ])],
                ),
                (
                    Table::DhcpLeases,
                    vec![row(&[
                        (".id", "*l"),
                        ("server", "lan"),
                        ("status", "bound"),
                        ("active-address", "192.0.2.20"),
                        ("active-mac-address", "aa:bb:cc:dd:ee:20"),
                        ("host-name", "alice"),
                        ("comment", "[lqosync-id:customer-1]"),
                    ])],
                ),
            ])
        } else {
            active_table = Table::HotspotActive;
            std::collections::BTreeMap::from([
                (
                    Table::HotspotServers,
                    vec![row(&[(".id", "*h"), ("name", "hs"), ("disabled", "false")])],
                ),
                (
                    Table::HotspotUsers,
                    vec![row(&[
                        (".id", "*u"),
                        ("name", "alice"),
                        ("profile", "gold"),
                        ("disabled", "false"),
                        ("comment", "[lqosync-id:customer-1]"),
                    ])],
                ),
                (
                    Table::HotspotProfiles,
                    vec![row(&[
                        (".id", "*p"),
                        ("name", "gold"),
                        ("rate-limit", "10M/50M"),
                    ])],
                ),
                (
                    Table::HotspotActive,
                    vec![row(&[
                        (".id", "*a"),
                        ("user", "alice"),
                        ("address", "192.0.2.20"),
                        ("mac-address", "aa:bb:cc:dd:ee:20"),
                        ("server", "hs"),
                        ("login-by", "http-chap"),
                    ])],
                ),
            ])
        };
        fake.data.lock().unwrap().get_mut("Distro").unwrap().rows = rows;
        let active = coordinator
            .prepare(&cfg, &registry, 0, Duration::from_secs(10))
            .await
            .unwrap();
        assert_eq!(active.rows().len(), 1);
        assert_eq!(active.rows()[0].ipv4, "192.0.2.20");
        if dhcp {
            fake.data
                .lock()
                .unwrap()
                .get_mut("Distro")
                .unwrap()
                .rows
                .get_mut(&active_table)
                .unwrap()[0]
                .insert("status".into(), "waiting".into());
        } else {
            fake.data
                .lock()
                .unwrap()
                .get_mut("Distro")
                .unwrap()
                .rows
                .insert(active_table, vec![]);
        }
        let inactive = coordinator
            .prepare(&cfg, &active.candidate, 0, Duration::from_secs(10))
            .await
            .unwrap();
        assert!(inactive.rows().is_empty());
    }
}

#[tokio::test(start_paused = true)]
async fn changed_router_binding_cannot_reuse_the_previous_routers_session() {
    let (mut cfg, mut registry, fake, mut coordinator) = fixture();
    let first = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    registry = first.candidate.clone();
    cfg.routers[0].name = "Distro2".into();
    cfg.routers[0].address = "192.0.2.200".into();
    registry.catalog.as_mut().unwrap().routers[1].name = "Distro2".into();
    let mut second = data();
    second.rows.get_mut(&Table::PppActive).unwrap()[0]
        .insert("address".into(), "192.0.2.200".into());
    fake.data.lock().unwrap().insert("Distro2".into(), second);
    let pair = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(pair.rows()[0].ipv4, "192.0.2.200");
    assert_eq!(fake.connects.load(Ordering::SeqCst), 2);
}

#[tokio::test(start_paused = true)]
async fn malformed_source_exclusions_stop_before_connection() {
    let (mut cfg, registry, fake, mut coordinator) = fixture();
    cfg.exclude.ips.push("not-a-network".into());
    assert!(
        coordinator
            .prepare(&cfg, &registry, 0, Duration::from_secs(10))
            .await
            .is_err()
    );
    assert_eq!(fake.connects.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn exclusions_remove_exported_members_and_reconnect_uses_fresh_endpoints() {
    let (mut cfg, registry, fake, mut coordinator) = fixture();
    let first = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    cfg.exclude.ips.push("192.0.2.0/24".into());
    let excluded = coordinator
        .prepare(&cfg, &first.candidate, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert!(excluded.rows().is_empty());
    excluded.check_source_filters(&cfg).unwrap();
    assert_eq!(excluded.network().as_object().unwrap().len(), 2);
    for node in excluded.network().as_object().unwrap().values() {
        assert_eq!(node["children"], serde_json::json!({}));
    }
    fake.data
        .lock()
        .unwrap()
        .get_mut("Distro")
        .unwrap()
        .rows
        .get_mut(&Table::PppActive)
        .unwrap()[0]
        .insert("address".into(), "192.0.2.99".into());
    cfg.exclude.ips.clear();
    let restored = coordinator
        .prepare(&cfg, &excluded.candidate, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(restored.rows()[0].circuit_id, first.rows()[0].circuit_id);
    assert_eq!(restored.rows()[0].ipv4, "192.0.2.99");
}

#[tokio::test(start_paused = true)]
async fn excluded_accounts_do_not_need_identity_enrollment_and_keep_profile_inventory() {
    let (mut cfg, registry, fake, mut coordinator) = fixture();
    cfg.routers[0].exclude.ppp_profiles.push("G*D".into());
    fake.data
        .lock()
        .unwrap()
        .get_mut("Distro")
        .unwrap()
        .rows
        .get_mut(&Table::PppSecrets)
        .unwrap()[0]
        .remove("comment");
    let pair = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert!(pair.rows().is_empty());
    assert!(pair.candidate.subscribers.is_empty());
}

#[tokio::test(start_paused = true)]
async fn filtered_pair_cannot_outlive_its_exclusion_policy() {
    let (mut cfg, registry, _, mut coordinator) = fixture();
    cfg.exclude.ips.push("198.51.100.0/24".into());
    let pair = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    pair.check_source_filters(&cfg).unwrap();
    cfg.exclude.ips.push("192.0.2.1".into());
    assert!(pair.check_source_filters(&cfg).is_err());
    let unbound = crate::topology_manager::publication::prepare(
        crate::topology_manager::publication::tests::input(1),
    )
    .unwrap();
    assert!(unbound.check_source_filters(&cfg).is_err());
}

#[tokio::test(start_paused = true)]
async fn configured_filtered_adoption_requires_current_filter_proof_and_exact_mappings() {
    use crate::topology_manager::{
        legacy_rate::LegacyPriority,
        migration::{MigrationManifest, RowIdentity, RowMapping, fingerprint},
        rates::{Fraction, RateRule},
    };
    let (mut cfg, mut registry, _, mut coordinator) = fixture();
    cfg.speed.pppoe_sources = vec![crate::config::PppSource::Default];
    cfg.bandwidth_defaults.pppoe_default_rate = "50M/10M".into();
    cfg.bandwidth_defaults.min_rate_percentage = 0.1;
    cfg.bandwidth_defaults.pppoe_aggregate_factor = 0.8;
    cfg.exclude.ips.push("198.51.100.0/24".into());
    let catalog = registry.catalog.as_mut().unwrap();
    catalog.defaults.down_min = RateRule::BaseFraction(Fraction {
        numerator: 1,
        denominator: 10,
    });
    catalog.defaults.up_min = catalog.defaults.down_min;
    if let crate::topology_manager::catalog::SourceOrigin::Subscribers { group, .. } =
        &mut catalog.sources[2].origin
    {
        group.factors.up = group.factors.down;
    }
    registry.source_settings.as_mut().unwrap().selections[0]
        .policy
        .legacy_priority =
        Some(LegacyPriority::ppp(&cfg.speed, &cfg.bandwidth_defaults.pppoe_default_rate).unwrap());
    let pair = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    let mut legacy = pair.rows().to_vec();
    legacy[0].circuit_id = "legacy-circuit".into();
    legacy[0].device_id = "legacy-device".into();
    let manifest = MigrationManifest {
        expected_revision: pair.expected_revision(),
        generation: pair.generation(),
        baseline_digest: fingerprint(&legacy, pair.network()),
        candidate_digest: fingerprint(pair.rows(), pair.network()),
        rows: vec![RowMapping {
            legacy: RowIdentity {
                circuit_id: "legacy-circuit".into(),
                device_id: "legacy-device".into(),
            },
            current: RowIdentity {
                circuit_id: pair.rows()[0].circuit_id.clone(),
                device_id: pair.rows()[0].device_id.clone(),
            },
            legacy_circuit_name: legacy[0].circuit_name.clone(),
            legacy_device_name: legacy[0].device_name.clone(),
        }],
        nodes: vec![],
        protected_rows: vec![],
    };
    let adopted = crate::topology_manager::ownership::adopt_configured_legacy(
        &cfg,
        &pair,
        &legacy,
        pair.network(),
        &manifest,
        &[],
    )
    .unwrap();
    assert_eq!(
        adopted.export_ownership.unwrap().devices[0].id,
        "legacy-device"
    );
    cfg.exclude.ips.clear();
    assert!(
        crate::topology_manager::ownership::adopt_configured_legacy(
            &cfg,
            &pair,
            &legacy,
            pair.network(),
            &manifest,
            &[]
        )
        .is_err()
    );
}

#[tokio::test(start_paused = true)]
async fn dhcp_override_follows_identity_after_disconnect_and_mac_reassignment() {
    use crate::topology_manager::{
        catalog::{BaseRateAuthority, SourceOrigin},
        model::{IdentityRef, SubscriberRecord},
        rates::RateBps,
    };
    let (mut cfg, mut registry, fake, mut coordinator) = fixture();
    cfg.routers[0].pppoe.enabled = false;
    cfg.routers[0].dhcp.enabled = true;
    cfg.routers[0].dhcp.require_bound = true;
    cfg.routers[0].dhcp.shape_unassigned = false;
    cfg.bandwidth_defaults.dhcp_default_per_client_mbps = 50.0;
    cfg.bandwidth_defaults.min_rate_percentage = 0.1;
    cfg.routers[0].dhcp.clients.push(crate::config::DhcpClient {
        mac: "AA:BB:CC:DD:EE:20".into(),
        download_mbps: 20.0,
        upload_mbps: 7.0,
        name: None,
    });
    let catalog = registry.catalog.as_mut().unwrap();
    catalog.profiles[0].base = BaseRateAuthority::SourceRead;
    catalog.profiles[0].speed =
        crate::topology_manager::legacy_bounds::import(&cfg.bandwidth_defaults).unwrap();
    if let SourceOrigin::Subscribers {
        scope, profile_ref, ..
    } = &mut catalog.sources[2].origin
    {
        scope.kind = SourceKind::Dhcp;
        *profile_ref = None;
    }
    let selection = &mut registry.source_settings.as_mut().unwrap().selections[0];
    selection.scope.kind = SourceKind::Dhcp;
    selection.policy = SourcePolicy {
        legacy_priority: Some(
            crate::topology_manager::legacy_rate::LegacyPriority::dhcp(
                &cfg.speed,
                &cfg.bandwidth_defaults,
                &cfg.routers[0].dhcp.server("lan"),
            )
            .unwrap(),
        ),
        dhcp_overrides: vec![
            crate::topology_manager::source_policy::DhcpOverrideBinding {
                legacy_client_index: 0,
                account_ref: "operator-comment/v1/customer-1".into(),
                rate: RateBps {
                    down: 20_000_000,
                    up: 7_000_000,
                },
            },
        ],
        ..Default::default()
    };
    registry.subscribers.push(SubscriberRecord {
        id: "enrolled-customer".into(),
        identities: vec![IdentityRef {
            scope: selection.scope.clone(),
            account_ref: "operator-comment/v1/customer-1".into(),
        }],
        policy: Default::default(),
    });
    let lease = row(&[
        (".id", "*l"),
        ("server", "lan"),
        ("status", "bound"),
        ("active-address", "192.0.2.20"),
        ("active-mac-address", "aa:bb:cc:dd:ee:20"),
        ("host-name", "alice"),
        ("comment", "[lqosync-id:customer-1]"),
    ]);
    fake.data.lock().unwrap().get_mut("Distro").unwrap().rows = std::collections::BTreeMap::from([
        (
            Table::DhcpServers,
            vec![row(&[
                (".id", "*d"),
                ("name", "lan"),
                ("disabled", "false"),
            ])],
        ),
        (Table::DhcpLeases, vec![lease.clone()]),
    ]);
    let first = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(first.rows()[0].download_max, "20");
    assert_eq!(first.rows()[0].upload_max, "7");
    assert_eq!(first.rows()[0].download_min, "2");
    assert_eq!(first.rows()[0].upload_min, "0.7");
    let mut changed = cfg.clone();
    changed.routers[0].dhcp.clients[0].download_mbps = 30.0;
    assert!(first.check_source_filters(&changed).is_err());
    changed.routers[0].dhcp.clients.clear();
    assert!(first.check_source_filters(&changed).is_err());
    fake.data
        .lock()
        .unwrap()
        .get_mut("Distro")
        .unwrap()
        .rows
        .get_mut(&Table::DhcpLeases)
        .unwrap()
        .clear();
    let offline = coordinator
        .prepare(&cfg, &first.candidate, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert!(offline.rows().is_empty());
    assert_eq!(offline.candidate.subscribers[0].id, "enrolled-customer");
    for wan in offline.network().as_object().unwrap().values() {
        assert_eq!(wan["children"], serde_json::json!({}));
    }
    let mut returned = lease.clone();
    returned.insert(".id".into(), "*new".into());
    returned.insert("active-address".into(), "192.0.2.99".into());
    returned.insert("active-mac-address".into(), "aa:bb:cc:dd:ee:99".into());
    let mut other = lease;
    other.insert("comment".into(), "[lqosync-id:customer-2]".into());
    fake.data
        .lock()
        .unwrap()
        .get_mut("Distro")
        .unwrap()
        .rows
        .insert(Table::DhcpLeases, vec![other, returned]);
    let back = coordinator
        .prepare(&cfg, &offline.candidate, 0, Duration::from_secs(10))
        .await
        .unwrap();
    let customer = back.rows().iter().find(|r| r.ipv4 == "192.0.2.99").unwrap();
    assert_eq!(customer.circuit_id, first.rows()[0].circuit_id);
    assert_eq!(customer.download_max, "20");
    assert_eq!(customer.mac, "AA:BB:CC:DD:EE:99");
    let other = back.rows().iter().find(|r| r.ipv4 == "192.0.2.20").unwrap();
    assert_ne!(other.circuit_id, customer.circuit_id);
    assert_eq!(other.download_max, "50");
    assert_eq!(other.mac, "AA:BB:CC:DD:EE:20");
}

#[tokio::test(start_paused = true)]
async fn imported_bounds_follow_fresh_profile_rates_without_stored_speed_or_endpoints() {
    let (mut cfg, mut registry, fake, mut coordinator) = fixture();
    cfg.bandwidth_defaults.min_rate_percentage = 0.1;
    cfg.speed.pppoe_sources = vec![crate::config::PppSource::ProfileRateLimit];
    let profile = &mut registry.catalog.as_mut().unwrap().profiles[0];
    profile.base = crate::topology_manager::catalog::BaseRateAuthority::SourceRead;
    profile.speed =
        crate::topology_manager::legacy_bounds::import(&cfg.bandwidth_defaults).unwrap();
    registry.source_settings.as_mut().unwrap().selections[0]
        .policy
        .legacy_priority = Some(
        crate::topology_manager::legacy_rate::LegacyPriority::ppp(
            &cfg.speed,
            &cfg.bandwidth_defaults.pppoe_default_rate,
        )
        .unwrap(),
    );
    let first = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(
        (&*first.rows()[0].download_min, &*first.rows()[0].upload_min),
        ("5", "1")
    );
    assert_eq!(
        (&*first.rows()[0].download_max, &*first.rows()[0].upload_max),
        ("50", "10")
    );
    let active = fake
        .data
        .lock()
        .unwrap()
        .get_mut("Distro")
        .unwrap()
        .rows
        .remove(&Table::PppActive)
        .unwrap();
    fake.data
        .lock()
        .unwrap()
        .get_mut("Distro")
        .unwrap()
        .rows
        .insert(Table::PppActive, vec![]);
    let offline = coordinator
        .prepare(&cfg, &first.candidate, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert!(offline.rows().is_empty());
    let mut active = active;
    active[0].insert("address".into(), "192.0.2.99".into());
    {
        let mut data = fake.data.lock().unwrap();
        let data = data.get_mut("Distro").unwrap();
        data.rows.insert(Table::PppActive, active);
        data.rows.get_mut(&Table::PppProfiles).unwrap()[0]
            .insert("rate-limit".into(), "7M/23M".into());
    }
    let returned = coordinator
        .prepare(&cfg, &offline.candidate, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(returned.rows()[0].circuit_id, first.rows()[0].circuit_id);
    assert_eq!(returned.rows()[0].ipv4, "192.0.2.99");
    assert_eq!(
        (
            &*returned.rows()[0].download_min,
            &*returned.rows()[0].upload_min
        ),
        ("2.3", "0.7")
    );
    assert_eq!(
        (
            &*returned.rows()[0].download_max,
            &*returned.rows()[0].upload_max
        ),
        ("23", "7")
    );
}

#[tokio::test(start_paused = true)]
async fn protected_static_source_is_current_and_bound_to_the_configured_owner() {
    use crate::topology_manager::ownership::ExportOwnership;
    use std::os::unix::fs::PermissionsExt;
    let (mut cfg, mut registry, fake, mut coordinator) = fixture();
    let state = tempfile::tempdir().unwrap();
    cfg.runtime.state_dir = state.path().to_str().unwrap().into();
    let row = crate::model::CsvRow {
        circuit_id: "static-circuit".into(),
        device_id: "static-device".into(),
        circuit_name: "Management".into(),
        device_name: "Management device".into(),
        parent_node: "Static root".into(),
        parent_node_id: "static-root".into(),
        ipv4: "192.0.2.240".into(),
        download_min: "1".into(),
        upload_min: "1".into(),
        download_max: "5".into(),
        upload_max: "5".into(),
        ..Default::default()
    };
    registry.export_ownership = Some(ExportOwnership {
        protected_rows: vec![crate::topology_manager::migration::RowIdentity {
            circuit_id: row.circuit_id.clone(),
            device_id: row.device_id.clone(),
        }],
        protected_roots: vec!["static-root".into()],
        shared_attachments: vec![crate::topology_manager::ownership::StaticAttachment {
            root_id: "static-root".into(),
            parent_id: "wan1".into(),
        }],
        ..Default::default()
    });
    assert!(
        coordinator
            .prepare(&cfg, &registry, 0, Duration::from_secs(10))
            .await
            .is_err()
    );
    assert_eq!(fake.connects.load(Ordering::SeqCst), 0);
    let dir = state.path().join("topology-manager");
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = dir.join("static-input.json");
    let mut document = serde_json::json!({"schema_version":1,"revision":1,"rows":[row],
        "network":{"Static root":{"id":"static-root","type":"site",
            "downloadBandwidthMbps":20,"uploadBandwidthMbps":20,"children":{}}}});
    crate::files::write_secret(&path, &serde_json::to_vec(&document).unwrap()).unwrap();
    let first = coordinator
        .prepare(&cfg, &registry, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(
        crate::topology_manager::migration::nodes(first.network()).unwrap()["wan1"].1["children"]["Static root"]
            ["downloadBandwidthMbps"],
        20
    );
    assert_eq!(first.rows().len(), 2);
    first.check_source_filters(&cfg).unwrap();
    let other = tempfile::tempdir().unwrap();
    let mut changed_cfg = cfg.clone();
    changed_cfg.runtime.state_dir = other.path().to_str().unwrap().into();
    assert!(first.check_source_filters(&changed_cfg).is_err());
    document["revision"] = serde_json::json!(2);
    document["network"]["Static root"]["downloadBandwidthMbps"] = serde_json::json!(30);
    crate::files::write_secret(&path, &serde_json::to_vec(&document).unwrap()).unwrap();
    assert!(first.check_fresh().is_err());
    let next = coordinator
        .prepare(&cfg, &first.candidate, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(
        crate::topology_manager::migration::nodes(next.network()).unwrap()["wan1"].1["children"]["Static root"]
            ["downloadBandwidthMbps"],
        30
    );
    assert_eq!(next.static_source.as_ref().unwrap().revision, 2);
    fake.data
        .lock()
        .unwrap()
        .get_mut("Distro")
        .unwrap()
        .rows
        .insert(Table::PppActive, vec![]);
    let offline = coordinator
        .prepare(&cfg, &next.candidate, 0, Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(offline.rows().len(), 1);
    assert_eq!(offline.rows()[0].device_id, "static-device");
    assert_eq!(offline.rows()[0].ipv4, "192.0.2.240");
    document["network"] = serde_json::json!({});
    document["revision"] = serde_json::json!(3);
    crate::files::write_secret(&path, &serde_json::to_vec(&document).unwrap()).unwrap();
    assert!(
        coordinator
            .prepare(&cfg, &next.candidate, 0, Duration::from_secs(10))
            .await
            .is_err()
    );
}
