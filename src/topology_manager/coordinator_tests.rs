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
async fn unsupported_selected_source_policies_stop_before_connection() {
    let (mut cfg, registry, fake, mut coordinator) = fixture();
    cfg.exclude.ips.push("192.0.2.0/24".into());
    assert!(
        coordinator
            .prepare(&cfg, &registry, 0, Duration::from_secs(10))
            .await
            .is_err()
    );
    assert_eq!(fake.connects.load(Ordering::SeqCst), 0);
}
