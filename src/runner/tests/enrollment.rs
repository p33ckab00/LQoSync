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

pub(crate) mod cli_flow {
    use super::*;
    use crate::topology_manager::enroll_mapping::{MappingFile, RowDecision};
    use crate::topology_manager::policy_draft::tests as scenario;
    use std::time::Duration;

    pub(crate) const AGE: Duration = Duration::from_secs(60);

    /// The v3_root_promotion scenario as live legacy files and current router rows.
    pub(crate) fn setup(e: &Env, drop_legacy: Option<&str>) -> Config {
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

    pub(crate) fn confirm(file: &mut MappingFile) {
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
        let mut file = r.propose_enrollment(None, AGE, false).await.unwrap();
        confirm(&mut file);
        let dry = r
            .enroll_with_mapping(&file, AGE, false, false)
            .await
            .unwrap();
        assert_eq!(dry.revision, None);
        assert_eq!(dry.mapped, 7);
        assert!(!dry.rounded.is_empty());
        let done = r
            .enroll_with_mapping(&file, AGE, true, false)
            .await
            .unwrap();
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
        let file = r.propose_enrollment(None, AGE, false).await.unwrap();
        let before = files(&e.state);
        let e2 = r
            .enroll_with_mapping(&file, AGE, true, false)
            .await
            .unwrap_err();
        assert!(format!("{e2:#}").contains("unconfirmed"), "{e2:#}");
        assert_eq!(files(&e.state), before);
    }
}

mod cli_commands {
    use super::cli_flow::{confirm, setup};
    use super::*;
    use crate::cli::TopologyCommand as T;
    use std::os::unix::fs::PermissionsExt;

    #[tokio::test]
    async fn cli_commands_run_the_enrollment_flow() {
        let e = env();
        let c = setup(&e, None);
        let dir = e.lqos.parent().unwrap().to_path_buf();
        let (policy, mapping) = (dir.join("policy.json"), dir.join("mapping.json"));
        let mut r = runner(&e, c);
        let (_, code) = crate::topology_cli::run(
            &mut r,
            &T::PolicyPropose {
                out: policy.clone(),
            },
        )
        .await
        .unwrap();
        assert_eq!(code, 0);
        assert_eq!(
            std::fs::metadata(&policy).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(
            crate::topology_cli::run(
                &mut r,
                &T::PolicyPropose {
                    out: policy.clone()
                }
            )
            .await
            .is_err()
        );
        let (text, code) = crate::topology_cli::run(
            &mut r,
            &T::Check {
                policy: policy.clone(),
            },
        )
        .await
        .unwrap();
        assert_eq!(code, 0, "{text}");
        let (text, code) = crate::topology_cli::run(
            &mut r,
            &T::PolicyInstall {
                policy: policy.clone(),
            },
        )
        .await
        .unwrap();
        assert!(code == 0 && text.contains("revision 1"), "{text}");
        let (_, code) = crate::topology_cli::run(
            &mut r,
            &T::Propose {
                out: mapping.clone(),
                previous: None,
                replace: false,
            },
        )
        .await
        .unwrap();
        assert_eq!(code, 0);
        let mut file: crate::topology_manager::enroll_mapping::MappingFile =
            serde_json::from_slice(&std::fs::read(&mapping).unwrap()).unwrap();
        confirm(&mut file);
        std::fs::write(&mapping, serde_json::to_vec_pretty(&file).unwrap()).unwrap();
        let (text, code) = crate::topology_cli::run(
            &mut r,
            &T::Enroll {
                mapping: mapping.clone(),
                yes: false,
                replace: false,
            },
        )
        .await
        .unwrap();
        assert!(code == 2 && text.contains("--yes"), "{text}");
        let (text, code) = crate::topology_cli::run(
            &mut r,
            &T::Enroll {
                mapping,
                yes: true,
                replace: false,
            },
        )
        .await
        .unwrap();
        assert!(
            code == 0 && text.contains("installed registry revision 2"),
            "{text}"
        );
    }

    #[tokio::test]
    async fn check_exits_2_and_names_a_scope_that_enters_after_activation() {
        let e = env();
        let c = setup(&e, Some("DHCP-020000000301"));
        let policy = e.lqos.parent().unwrap().join("policy.json");
        let mut r = runner(&e, c);
        crate::topology_cli::run(
            &mut r,
            &T::PolicyPropose {
                out: policy.clone(),
            },
        )
        .await
        .unwrap();
        let (text, code) = crate::topology_cli::run(&mut r, &T::Check { policy })
            .await
            .unwrap();
        assert!(code == 2 && text.contains("dhcp-IOT"), "{text}");
    }

    #[tokio::test]
    async fn a_locked_state_directory_names_the_service() {
        let e = env();
        let c = setup(&e, None);
        let policy = e.lqos.parent().unwrap().join("policy.json");
        let mut r = runner(&e, c.clone());
        crate::topology_cli::run(
            &mut r,
            &T::PolicyPropose {
                out: policy.clone(),
            },
        )
        .await
        .unwrap();
        let _held = crate::files::InstanceLock::acquire(&e.state).unwrap();
        let mut other = runner(&e, c);
        let err = crate::topology_cli::run(&mut other, &T::PolicyInstall { policy })
            .await
            .unwrap_err();
        assert!(
            format!("{err:#}").contains("stop lqosync3.service"),
            "{err:#}"
        );
    }
}

mod reenroll {
    use super::cli_flow::{AGE, confirm, setup};
    use super::*;

    async fn enrolled(e: &Env) -> Runner<FakeConnector> {
        let mut r = runner(e, setup(e, None));
        let draft = r.propose_policy(AGE).await.unwrap();
        r.install_topology_policy(&draft).unwrap();
        let mut file = r.propose_enrollment(None, AGE, false).await.unwrap();
        confirm(&mut file);
        let done = r.enroll_with_mapping(&file, AGE, true, false).await;
        assert_eq!(done.unwrap().revision, Some(2));
        r
    }

    /// Give the first legacy circuit a new ID. Return the old and the new ID.
    fn change_circuit_id(e: &Env) -> (String, String) {
        let path = e.lqos.join("ShapedDevices.csv");
        let text = read(&path);
        let old = text
            .lines()
            .nth(1)
            .and_then(|l| l.split(',').next())
            .unwrap()
            .to_string();
        let new = format!("{old}-new");
        let rewritten: Vec<_> = text
            .lines()
            .map(|l| match l.strip_prefix(&format!("{old},")) {
                Some(rest) => format!("{new},{rest}"),
                None => l.to_string(),
            })
            .collect();
        std::fs::write(&path, rewritten.join("\n") + "\n").unwrap();
        (old, new)
    }

    fn revision(r: &Runner<FakeConnector>) -> u64 {
        r.topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .revision
    }

    async fn replace_flow(r: &mut Runner<FakeConnector>, install: bool) -> anyhow::Result<u64> {
        let mut file = r.propose_enrollment(None, AGE, true).await?;
        confirm(&mut file);
        let out = r.enroll_with_mapping(&file, AGE, install, true).await?;
        Ok(out.revision.unwrap_or(0))
    }

    #[tokio::test]
    async fn a_second_enrollment_without_replace_is_refused() {
        let e = env();
        let mut r = enrolled(&e).await;
        let mut file = r.propose_enrollment(None, AGE, false).await.unwrap();
        confirm(&mut file);
        let err = r
            .enroll_with_mapping(&file, AGE, true, false)
            .await
            .unwrap_err();
        let text = format!("{err:#}");
        assert!(
            text.contains("legacy ownership is already configured"),
            "{text}"
        );
    }

    #[tokio::test]
    async fn replace_enrolls_a_changed_legacy_circuit_id() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (old, new) = change_circuit_id(&e);
        assert_eq!(replace_flow(&mut r, true).await.unwrap(), 3);
        let doc = r.topology_store.as_ref().unwrap().document().unwrap();
        let circuits = &doc.export_ownership.as_ref().unwrap().circuits;
        assert!(circuits.iter().any(|c| c.id == new));
        assert!(circuits.iter().all(|c| c.id != old));
        assert_eq!(doc.revision, 3);
    }

    #[tokio::test]
    async fn replace_keeps_registry_edits() {
        let e = env();
        let mut r = enrolled(&e).await;
        r.lock().unwrap();
        // A plan outside the legacy configuration. Legacy parity does not cover an unused plan.
        r.api_plan_create(2, "50M", 50, 50).unwrap();
        let before = r.api_registry().unwrap().unwrap();
        change_circuit_id(&e);
        assert_eq!(replace_flow(&mut r, true).await.unwrap(), 4);
        let doc = r.api_registry().unwrap().unwrap();
        let catalog = doc.catalog.as_ref().unwrap();
        assert!(catalog.profiles.iter().any(|p| p.id == "plan-1"));
        assert_eq!(doc.subscribers, before.subscribers);
        assert_eq!(doc.source_settings, before.source_settings);
        assert!(doc.next_id >= before.next_id);
    }

    #[tokio::test]
    async fn replace_refuses_a_changed_static_source() {
        let e = env();
        let mut r = enrolled(&e).await;
        crate::files::write_secret(
            &e.state.join("topology-manager/static-input.json"),
            b"changed",
        )
        .unwrap();
        change_circuit_id(&e);
        let err = replace_flow(&mut r, true).await.unwrap_err();
        let text = format!("{err:#}");
        assert!(
            text.contains("re-enrollment requires the same static source as the first enrollment"),
            "{text}"
        );
        assert_eq!(revision(&r), 2);
    }

    #[tokio::test]
    async fn replace_refuses_an_active_manager() {
        let e = env();
        let mut r = enrolled(&e).await;
        crate::files::write_secret(
            &e.state.join("topology-manager/runtime-owner.json"),
            br#"{"schema_version":1,"writer":"topology"}"#,
        )
        .unwrap();
        change_circuit_id(&e);
        let err = replace_flow(&mut r, true).await.unwrap_err();
        let text = format!("{err:#}");
        assert!(
            text.contains("the Topology Manager is active; re-enrollment is not allowed"),
            "{text}"
        );
        assert_eq!(revision(&r), 2);
    }

    #[tokio::test]
    async fn replace_refuses_a_registry_that_is_not_enrolled() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        let draft = r.propose_policy(AGE).await.unwrap();
        r.install_topology_policy(&draft).unwrap();
        let err = r.propose_enrollment(None, AGE, true).await.unwrap_err();
        let text = format!("{err:#}");
        assert!(
            text.contains("the topology registry is not enrolled; use enroll without --replace"),
            "{text}"
        );
    }

    #[tokio::test]
    async fn replace_without_yes_installs_nothing() {
        let e = env();
        let mut r = enrolled(&e).await;
        change_circuit_id(&e);
        replace_flow(&mut r, false).await.unwrap();
        assert_eq!(revision(&r), 2);
    }
}
