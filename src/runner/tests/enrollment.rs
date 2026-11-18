use super::*;
use crate::topology_manager::{legacy_static, staging, store::RegistryStore};

fn proposal() -> legacy_static::StaticAdoption {
    let f = legacy_static::tests::fixture();
    legacy_static::adopt(
        &f.pair,
        &f.rows,
        &f.network,
        &f.manifest,
        &f.roots,
        &f.attachments,
    )
    .unwrap()
}

#[test]
fn startup_recovers_a_staged_migration_before_receipt_initialization() {
    let e = env();
    {
        let mut store = RegistryStore::open(&e.state).unwrap();
        staging::stage(&mut store, &proposal()).unwrap();
    }
    let mut r = runner(&e, cfg(&e, "shadow", ""));
    r.lock().unwrap();
    assert_eq!(
        r.topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .revision,
        1
    );
    assert!(r.pending_tracker().is_some());
    assert!(
        !e.state
            .join("topology-manager/migration-pending.json")
            .exists()
    );
    assert!(!crate::topology_manager::runtime::topology_owns(&e.state).unwrap());
    assert!(!e.lqos.join("ShapedDevices.csv").exists());
    assert!(!e.lqos.join("network.json").exists());
    assert!(RegistryStore::open(&e.state).is_err());
}

fn configured(e: &Env) -> (Config, legacy_static::tests::Fixture) {
    let (mut config, template) = crate::topology_manager::preflight::tests::fixture();
    config.runtime.state_dir = e.state.to_string_lossy().into_owned();
    config.runtime.mode = RuntimeMode::Apply;
    config.libreqos = cfg(e, "apply", "").libreqos;
    config.libreqos.target = Target::V3Pair;
    let mut input = crate::topology_manager::publication::tests::input(1);
    input.reconciliation.registry.catalog = template.catalog;
    input.reconciliation.registry.source_settings = template.source_settings;
    input.reconciliation.registry.subscribers[0].policy.pin = Some("group1".into());
    let f = legacy_static::tests::from_pair(
        crate::topology_manager::publication::prepare(input).unwrap(),
    );
    std::fs::write(
        e.lqos.join("ShapedDevices.csv"),
        render_csv(&f.rows, Columns::V3),
    )
    .unwrap();
    std::fs::write(e.lqos.join("network.json"), render_network_json(&f.network)).unwrap();
    (config, f)
}

fn enroll(r: &mut Runner<FakeConnector>, f: &legacy_static::tests::Fixture) -> anyhow::Result<u64> {
    r.enroll_topology(&f.pair, &f.manifest, &f.roots, &f.attachments)
}

#[test]
fn normal_lock_does_not_create_a_topology_registry() {
    let e = env();
    let mut r = runner(&e, cfg(&e, "shadow", ""));
    r.lock().unwrap();
    r.lock().unwrap();
    assert!(!e.state.join("topology-manager").exists());
    assert!(r.topology_store.is_none());
}

#[test]
fn a_held_owner_recovers_without_replacing_the_receipt_tracker() {
    let e = env();
    let mut r = runner(&e, cfg(&e, "shadow", ""));
    r.lock().unwrap();
    let tracker = r.pending_tracker().unwrap();
    r.open_topology_store().unwrap();
    staging::stage(r.topology_store.as_mut().unwrap(), &proposal()).unwrap();
    r.sessions.insert("old".into(), None);
    r.lock().unwrap();
    assert!(Arc::ptr_eq(&tracker, &r.pending_tracker().unwrap()));
    assert_eq!(
        r.topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .revision,
        1
    );
    assert!(r.sessions.is_empty());
    r.lock().unwrap();
    assert_eq!(
        r.topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn a_failed_startup_can_retry_after_explicit_conflict_repair() {
    let e = env();
    {
        let mut store = RegistryStore::open(&e.state).unwrap();
        staging::stage(&mut store, &proposal()).unwrap();
    }
    let source = e.state.join("topology-manager/static-input.json");
    crate::files::write_secret(&source, b"unknown").unwrap();
    let mut r = runner(&e, cfg(&e, "shadow", ""));
    assert!(r.lock().is_err());
    assert!(r.pending_tracker().is_none());
    assert!(r.topology_store.is_none());
    assert!(r.lock.is_none());
    assert_eq!(std::fs::read(&source).unwrap(), b"unknown\n");
    assert!(!e.state.join("status.json").exists());
    std::fs::remove_file(source).unwrap();
    r.lock().unwrap();
    assert!(r.pending_tracker().is_some());
    assert_eq!(
        r.topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .revision,
        1
    );
}

#[tokio::test]
async fn a_corrupt_pending_journal_blocks_cycles_without_output_or_status_writes() {
    let e = env();
    {
        let mut store = RegistryStore::open(&e.state).unwrap();
        staging::stage(&mut store, &proposal()).unwrap();
    }
    let journal = e.state.join("topology-manager/migration-pending.json");
    crate::files::write_secret(&journal, b"corrupt").unwrap();
    let mut r = runner(&e, cfg(&e, "shadow", ""));
    let result = r.cycle(1000, false).await;
    assert!(result.error.is_some());
    assert!(r.pending_tracker().is_none());
    assert_eq!(std::fs::read(journal).unwrap(), b"corrupt\n");
    assert!(!e.state.join("status.json").exists());
    assert!(!e.state.join("candidate/ShapedDevices.csv").exists());
    assert!(!e.lqos.join("ShapedDevices.csv").exists());
}

#[test]
fn explicit_enrollment_installs_policy_and_preserves_the_live_pair() {
    let e = env();
    let (config, f) = configured(&e);
    let csv = read(&e.lqos.join("ShapedDevices.csv"));
    let network = read(&e.lqos.join("network.json"));
    let mut r = runner(&e, config);
    assert_eq!(enroll(&mut r, &f).unwrap(), 1);
    assert_eq!(read(&e.lqos.join("ShapedDevices.csv")), csv);
    assert_eq!(read(&e.lqos.join("network.json")), network);
    assert!(!crate::topology_manager::runtime::topology_owns(&e.state).unwrap());
    assert!(
        r.pending_tracker()
            .unwrap()
            .lock()
            .unwrap()
            .current()
            .is_none()
    );
    assert!(!e.lqos.join("reloaded").exists());
    assert!(!e.lqos.join("full").exists());
    assert!(enroll(&mut r, &f).is_err());
    assert_eq!(
        r.topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn enrollment_rejects_changed_snapshots_filters_and_legacy_policy() {
    for case in 0..4 {
        let e = env();
        let (mut config, mut f) = configured(&e);
        match case {
            0 => {
                f.rows[0].ipv4 = "192.0.2.99".into();
                std::fs::write(
                    e.lqos.join("ShapedDevices.csv"),
                    render_csv(&f.rows, Columns::V3),
                )
                .unwrap();
            }
            1 => {
                f.network["Legacy WAN"]["downloadBandwidthMbps"] = serde_json::json!(99);
                std::fs::write(e.lqos.join("network.json"), render_network_json(&f.network))
                    .unwrap();
            }
            2 => config.exclude.ips.push("198.51.100.0/24".into()),
            _ => config.bandwidth_defaults.min_rate_percentage = 0.5,
        }
        let csv = read(&e.lqos.join("ShapedDevices.csv"));
        let network = read(&e.lqos.join("network.json"));
        let mut r = runner(&e, config);
        assert!(enroll(&mut r, &f).is_err(), "case {case}");
        assert!(
            !e.state
                .join("topology-manager/migration-pending.json")
                .exists()
        );
        assert!(!e.state.join("topology-manager/static-input.json").exists());
        assert_eq!(read(&e.lqos.join("ShapedDevices.csv")), csv);
        assert_eq!(read(&e.lqos.join("network.json")), network);
    }
}

#[tokio::test(start_paused = true)]
async fn expired_evidence_cannot_start_enrollment() {
    let e = env();
    let (config, f) = configured(&e);
    tokio::time::advance(Duration::from_secs(11)).await;
    let mut r = runner(&e, config);
    assert!(enroll(&mut r, &f).is_err());
    assert!(!e.state.join("topology-manager").exists());
}

#[test]
fn enrollment_requires_the_apply_target_receipt_owner_and_current_instance_lock() {
    for case in 0..4 {
        let e = env();
        let (mut config, f) = configured(&e);
        match case {
            0 => config.runtime.mode = RuntimeMode::Shadow,
            1 => config.libreqos.target = Target::V2Pair,
            _ => {}
        }
        let mut r = runner(&e, config);
        if case == 2 {
            r.lock().unwrap();
            r.write_tracker = None;
        }
        let held = (case == 3).then(|| crate::files::InstanceLock::acquire(&e.state).unwrap());
        assert!(enroll(&mut r, &f).is_err(), "case {case}");
        assert!(!e.state.join("topology-manager").exists());
        drop(held);
    }
}

#[test]
fn enrollment_cannot_replace_an_existing_static_source() {
    let e = env();
    let (config, f) = configured(&e);
    drop(RegistryStore::open(&e.state).unwrap());
    let source = e.state.join("topology-manager/static-input.json");
    crate::files::write_secret(&source, b"existing").unwrap();
    let mut r = runner(&e, config);
    assert!(enroll(&mut r, &f).is_err());
    assert_eq!(std::fs::read(source).unwrap(), b"existing\n");
    assert_eq!(
        r.topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .revision,
        0
    );
}

#[test]
fn enrollment_requires_two_bounded_regular_snapshot_files() {
    for case in 0..6 {
        let e = env();
        let (config, f) = configured(&e);
        let csv = e.lqos.join("ShapedDevices.csv");
        let network = e.lqos.join("network.json");
        match case {
            0 => std::fs::remove_file(&csv).unwrap(),
            1 => {
                std::fs::remove_file(&network).unwrap();
                std::os::unix::fs::symlink("/dev/null", &network).unwrap();
            }
            2 => std::fs::File::options()
                .write(true)
                .open(&csv)
                .unwrap()
                .set_len(crate::topology_manager::admission::MAX_PAIR_BYTES as u64 + 1)
                .unwrap(),
            3 => std::fs::write(&network, b"not json").unwrap(),
            4 => {
                std::fs::remove_file(&csv).unwrap();
                std::fs::create_dir(&csv).unwrap();
            }
            _ => {
                let mut bytes = std::fs::read(&network).unwrap();
                bytes.resize(crate::topology_manager::admission::MAX_PAIR_BYTES - 1, b' ');
                std::fs::write(&network, bytes).unwrap();
            }
        }
        let mut r = runner(&e, config);
        assert!(enroll(&mut r, &f).is_err(), "case {case}");
        assert!(!e.state.join("topology-manager").exists());
    }
}

#[test]
fn failed_recovery_discards_cached_registry_and_keeps_the_held_owner() {
    let e = env();
    let mut r = runner(&e, cfg(&e, "shadow", ""));
    r.lock().unwrap();
    let held = r.lock.as_ref().unwrap().clone();
    let tracker = r.pending_tracker().unwrap();
    r.open_topology_store().unwrap();
    staging::stage(r.topology_store.as_mut().unwrap(), &proposal()).unwrap();
    let source = e.state.join("topology-manager/static-input.json");
    crate::files::write_secret(&source, b"conflict").unwrap();
    assert!(r.lock().is_err());
    assert!(r.topology_store.is_none());
    assert!(Arc::ptr_eq(&held, r.lock.as_ref().unwrap()));
    assert!(Arc::ptr_eq(&tracker, &r.pending_tracker().unwrap()));
    std::fs::remove_file(source).unwrap();
    r.lock().unwrap();
    assert_eq!(
        r.topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .revision,
        1
    );
}

mod cli_flow {
    use super::*;
    use crate::topology_manager::enroll_mapping::{MappingFile, RowDecision};
    use crate::topology_manager::policy_draft::tests as scenario;
    use std::time::Duration;

    const AGE: Duration = Duration::from_secs(60);

    /// The v3_root_promotion scenario as live legacy files and current router rows.
    fn setup(e: &Env, drop_legacy: Option<&str>) -> Config {
        let (mut config, snap) = scenario::scenario();
        let mut old = scenario::legacy(&config, &snap);
        if let Some(name) = drop_legacy {
            old.rows.retain(|r| r.circuit_name != name);
        }
        config.runtime.state_dir = e.state.to_string_lossy().into_owned();
        config.runtime.mode = RuntimeMode::Apply;
        config.libreqos = cfg(e, "apply", "").libreqos;
        config.libreqos.target = Target::V3Pair;
        std::fs::create_dir_all(&e.state).unwrap();
        std::fs::write(
            e.lqos.join("ShapedDevices.csv"),
            render_csv(&old.rows, Columns::V3),
        )
        .unwrap();
        std::fs::write(
            e.lqos.join("network.json"),
            render_network_json(&old.network),
        )
        .unwrap();
        let mut by_path = BTreeMap::new();
        for (table, rows) in scenario::tables(&snap) {
            let (path, fields) = table.command();
            let wanted: Vec<_> = fields.split(',').collect();
            let rows = rows
                .into_iter()
                .map(|r| {
                    r.into_iter()
                        .filter(|(k, _)| wanted.contains(&k.as_str()))
                        .collect()
                })
                .collect();
            by_path.insert(path.to_string(), rows);
        }
        e.fake.rows.lock().unwrap().insert("Hub-1".into(), by_path);
        config
    }

    fn files(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut out = vec![];
        let mut work = vec![dir.to_path_buf()];
        while let Some(d) = work.pop() {
            for entry in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = entry.path();
                if p.is_dir() {
                    work.push(p);
                } else {
                    out.push((p.clone(), std::fs::read(&p).unwrap_or_default()));
                }
            }
        }
        out.sort();
        out
    }

    fn confirm(file: &mut MappingFile) {
        for row in &mut file.rows {
            if let RowDecision::Map { confirmed, .. } = &mut row.decision {
                *confirmed = true;
            }
        }
        for node in &mut file.nodes {
            node.confirmed = true;
        }
    }

    #[tokio::test]
    async fn check_takes_no_lock_and_writes_nothing() {
        let e = env();
        let c = setup(&e, None);
        let _held = crate::files::InstanceLock::acquire(&e.state).unwrap();
        let before = files(&e.state);
        let mut r = runner(&e, c);
        let draft = r.propose_policy(AGE).await.unwrap();
        let check = r.check_topology(&draft, AGE).await.unwrap();
        assert_eq!(files(&e.state), before);
        assert!(check.policy_issues.is_empty(), "{check:?}");
        assert_eq!(check.prepare_error, None);
        assert_eq!((check.suggested, check.unmatched_legacy), (7, 0));
    }

    #[tokio::test]
    async fn check_reports_candidate_only_scope() {
        let e = env();
        let c = setup(&e, Some("DHCP-020000000301"));
        let mut r = runner(&e, c);
        let draft = r.propose_policy(AGE).await.unwrap();
        let check = r.check_topology(&draft, AGE).await.unwrap();
        assert_eq!(
            check.candidate_only_by_scope.get("dhcp-IOT"),
            Some(&1),
            "{check:?}"
        );
    }

    #[tokio::test]
    async fn check_warns_for_locally_administered_mac() {
        let e = env();
        let c = setup(&e, None);
        let mut r = runner(&e, c);
        let draft = r.propose_policy(AGE).await.unwrap();
        let check = r.check_topology(&draft, AGE).await.unwrap();
        // The scenario lease MACs start with 02, the locally administered bit.
        assert_eq!(check.random_macs.len(), 3, "{check:?}");
        assert!(check.policy_issues.is_empty());
    }

    #[tokio::test]
    async fn install_policy_then_propose_then_enroll_installs_one_revision() {
        let e = env();
        let c = setup(&e, None);
        let live = (
            read(&e.lqos.join("ShapedDevices.csv")),
            read(&e.lqos.join("network.json")),
        );
        let mut r = runner(&e, c);
        let draft = r.propose_policy(AGE).await.unwrap();
        assert_eq!(r.install_topology_policy(&draft).unwrap(), 1);
        let mut file = r.propose_enrollment(None, AGE).await.unwrap();
        confirm(&mut file);
        let dry = r.enroll_with_mapping(&file, AGE, false).await.unwrap();
        assert_eq!(dry.revision, None);
        assert_eq!(dry.mapped, 7);
        assert!(!dry.rounded.is_empty());
        let done = r.enroll_with_mapping(&file, AGE, true).await.unwrap();
        assert_eq!(done.revision, Some(2));
        assert_eq!(
            (
                read(&e.lqos.join("ShapedDevices.csv")),
                read(&e.lqos.join("network.json"))
            ),
            live
        );
        assert!(!crate::topology_manager::runtime::topology_owns(&e.state).unwrap());
        assert!(r.install_topology_policy(&draft).is_err());
    }

    #[tokio::test]
    async fn enroll_with_drift_writes_nothing() {
        let e = env();
        let c = setup(&e, None);
        let mut r = runner(&e, c);
        let draft = r.propose_policy(AGE).await.unwrap();
        r.install_topology_policy(&draft).unwrap();
        let file = r.propose_enrollment(None, AGE).await.unwrap();
        let before = files(&e.state);
        let e2 = r.enroll_with_mapping(&file, AGE, true).await.unwrap_err();
        assert!(format!("{e2:#}").contains("unconfirmed"), "{e2:#}");
        assert_eq!(files(&e.state), before);
    }
}
