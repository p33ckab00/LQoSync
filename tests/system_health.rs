use lqosync3::system_health::{Domain, Hub, Observation, Phase, State, Update};

fn observation(domain: Domain, component: &str, state: State, at: u64) -> Observation {
    Observation {
        domain,
        component: component.into(),
        state,
        reason: "sample_ok".into(),
        observed_at: at,
        max_age: 6,
        source_epoch: "epoch-1".into(),
        last_success: None,
    }
}
fn healthy() -> Hub {
    let hub = Hub::default();
    for domain in Domain::ALL {
        hub.observe(observation(domain, domain.label(), State::Healthy, 10))
            .unwrap();
    }
    hub
}
#[test]
fn stopped_producers_expire_without_another_event() {
    let hub = healthy();
    assert_eq!(hub.snapshot(16, None).overall, State::Healthy);
    let report = hub.snapshot(17, None);
    assert_eq!(report.overall, State::Unknown);
    assert!(report.components.iter().all(|c| c.reason == "source_stale"));
}
#[test]
fn pending_update_keeps_last_good_dataplane_separate() {
    let report = healthy().snapshot(
        11,
        Some(Update {
            write_id: "W1".into(),
            phase: Phase::Pending,
            observed_at: 10,
        }),
    );
    assert_eq!(report.overall, State::Updating);
    assert_eq!(
        report
            .components
            .iter()
            .find(|c| c.domain == Domain::Dataplane)
            .unwrap()
            .state,
        State::Healthy
    );
}
#[test]
fn disabled_source_is_not_a_failure_but_missing_required_domains_are_unknown() {
    let hub = healthy();
    hub.observe(observation(
        Domain::Telemetry,
        "nic",
        State::NotApplicable,
        0,
    ))
    .unwrap();
    assert_eq!(hub.snapshot(11, None).overall, State::Healthy);
    assert_eq!(Hub::default().snapshot(11, None).overall, State::Unknown);
}
#[test]
fn epoch_changes_do_not_reuse_the_last_success() {
    let hub = healthy();
    let mut failed = observation(
        Domain::Telemetry,
        Domain::Telemetry.label(),
        State::Unknown,
        11,
    );
    hub.observe(failed.clone()).unwrap();
    assert_eq!(
        hub.snapshot(11, None)
            .components
            .iter()
            .find(|c| c.domain == Domain::Telemetry)
            .unwrap()
            .last_success,
        Some(10)
    );
    failed.source_epoch = "epoch-2".into();
    hub.observe(failed).unwrap();
    assert_eq!(
        hub.snapshot(11, None)
            .components
            .iter()
            .find(|c| c.domain == Domain::Telemetry)
            .unwrap()
            .last_success,
        None
    );
}
#[test]
fn failures_keep_component_identity_and_override_pending_presentation() {
    let hub = healthy();
    hub.observe(observation(
        Domain::Collection,
        "router-1",
        State::Failed,
        11,
    ))
    .unwrap();
    hub.observe(observation(
        Domain::Collection,
        "router-2",
        State::Failed,
        11,
    ))
    .unwrap();
    let r = hub.snapshot(
        11,
        Some(Update {
            write_id: "W1".into(),
            phase: Phase::Pending,
            observed_at: 11,
        }),
    );
    assert_eq!(r.overall, State::Failed);
    assert!(r.components.iter().any(|c| c.component == "router-1"));
    assert!(r.components.iter().any(|c| c.component == "router-2"));
}
#[test]
fn future_observations_and_unbounded_fields_are_not_healthy() {
    let hub = healthy();
    assert_eq!(hub.snapshot(9, None).overall, State::Unknown);
    let mut invalid = observation(Domain::Service, "x", State::Healthy, 10);
    invalid.reason = "x".repeat(97);
    assert!(hub.observe(invalid).is_err());
}

#[test]
fn same_snapshot_is_published_to_a_new_directory() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let report = healthy().snapshot(11, None);
    let mut publisher = lqosync3::system_health::Publisher::default();
    assert!(publisher.publish(first.path(), &report).unwrap());
    assert!(!publisher.publish(first.path(), &report).unwrap());
    assert!(publisher.publish(second.path(), &report).unwrap());
    assert!(second.path().join(lqosync3::system_health::CACHE).exists());
}

#[test]
fn cached_reports_expire_without_a_new_file_write() {
    let d = tempfile::tempdir().unwrap();
    let report = healthy().snapshot(10, None);
    lqosync3::system_health::Publisher::default()
        .publish(d.path(), &report)
        .unwrap();
    let cached =
        lqosync3::system_health::Report::read(&d.path().join(lqosync3::system_health::CACHE))
            .unwrap();
    assert_eq!(cached.at(16).overall, State::Healthy);
    assert_eq!(cached.at(17).overall, State::Unknown);
    std::fs::write(
        d.path().join(lqosync3::system_health::CACHE),
        vec![b'x'; lqosync3::system_health::MAX_BYTES + 1],
    )
    .unwrap();
    assert!(
        lqosync3::system_health::Report::read(&d.path().join(lqosync3::system_health::CACHE))
            .is_err()
    );
}

#[test]
fn bounded_health_snapshots_reuse_unchanged_cache_writes() {
    let hub = healthy();
    for i in 0..80 {
        hub.observe(observation(
            Domain::Collection,
            &format!("router-{i}"),
            State::Healthy,
            10,
        ))
        .unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let mut publisher = lqosync3::system_health::Publisher::default();
    let start = std::time::Instant::now();
    let mut writes = 0;
    for _ in 0..1000 {
        let report = hub.snapshot(10, None);
        writes += usize::from(publisher.publish(dir.path(), &report).unwrap());
        assert_eq!(report.overall, State::Healthy);
    }
    assert_eq!(writes, 1);
    eprintln!(
        "1000 health snapshots/cache comparisons, {} components, {:?} debug, {writes} cache writes",
        87,
        start.elapsed()
    );
}

#[test]
fn incomplete_or_duplicate_cache_components_cannot_claim_healthy() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join(lqosync3::system_health::CACHE);
    let original = healthy().snapshot(10, None);
    let mut bad = original.clone();
    bad.components.clear();
    std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
    assert!(lqosync3::system_health::Report::read(&path).is_err());
    let mut bad = original;
    bad.components.push(bad.components[0].clone());
    std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
    assert!(lqosync3::system_health::Report::read(&path).is_err());
}
