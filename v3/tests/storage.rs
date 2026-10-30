use lqosync_v3::{model::*, store::*};
use std::collections::BTreeMap;

#[test]
fn invalid_edit_does_not_replace_last_valid_toml() {
    let dir = tempfile::tempdir().unwrap();
    let s = Settings {
        interval_seconds: 30,
        auto_stage: true,
        ..Default::default()
    };
    save_settings(dir.path(), &s).unwrap();
    let before = std::fs::read(dir.path().join("settings.toml")).unwrap();
    let bad = Settings {
        interval_seconds: 0,
        ..s
    };
    assert!(save_settings(dir.path(), &bad).is_err());
    assert_eq!(
        before,
        std::fs::read(dir.path().join("settings.toml")).unwrap()
    );
}
#[test]
fn generation_contains_both_artifacts_and_unchanged_data_reuses_it() {
    let dir = tempfile::tempdir().unwrap();
    let p = Plan {
        capacities: BTreeMap::new(),
        csv: "header\n".into(),
        network: "{}".into(),
    };
    let a = publish(dir.path(), &p).unwrap();
    let b = publish(dir.path(), &p).unwrap();
    assert_eq!(a, b);
    let base = dir.path().join("staging").join(&a);
    assert_eq!(
        std::fs::read_to_string(base.join("ShapedDevices.csv")).unwrap(),
        p.csv
    );
    assert_eq!(
        std::fs::read_to_string(base.join("network.json")).unwrap(),
        p.network
    );
    let pointer = std::fs::read_to_string(dir.path().join("state/current.toml")).unwrap();
    assert!(pointer.contains(&a));
    assert!(!pointer.contains("applied"));
}

#[test]
fn normalized_snapshot_and_pair_switch_together_and_mirror_is_not_authority() {
    let dir = tempfile::tempdir().unwrap();
    let settings: Settings = toml::from_str(include_str!("../examples/settings.toml")).unwrap();
    let subscribers = toml::from_str::<Snapshot>(include_str!("../examples/subscribers.toml"))
        .unwrap()
        .subscribers;
    let plan = compile(&settings, &subscribers).unwrap();
    let published = publish_normalized(dir.path(), &plan, &subscribers).unwrap();
    assert!(published.mirror_warning.is_none());
    assert_eq!(load_snapshot(dir.path()).unwrap().len(), 2);
    let initial_assignments = load_assignments(dir.path()).unwrap();
    for subscriber in &subscribers {
        assert_eq!(
            initial_assignments.assignments[&subscriber.id].source_node,
            subscriber.node
        );
        assert_eq!(
            initial_assignments.assignments[&subscriber.id].node,
            subscriber.node
        );
    }
    let base = dir.path().join("staging").join(&published.generation);
    assert!(base.join("subscribers.toml").exists());
    assert!(base.join("ShapedDevices.csv").exists());
    assert!(base.join("network.json").exists());
    std::fs::write(dir.path().join("state/subscribers.toml"), "corrupt mirror").unwrap();
    assert_eq!(load_snapshot(dir.path()).unwrap().len(), 2);
    std::fs::write(base.join("subscribers.toml"), "corrupt committed snapshot").unwrap();
    assert!(load_snapshot(dir.path()).is_err());
}

#[test]
fn mirror_write_failure_leaves_the_authoritative_snapshot_available() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("state/subscribers.toml")).unwrap();
    let settings: Settings = toml::from_str(include_str!("../examples/settings.toml")).unwrap();
    let subscribers = toml::from_str::<Snapshot>(include_str!("../examples/subscribers.toml"))
        .unwrap()
        .subscribers;
    let plan = compile(&settings, &subscribers).unwrap();
    let p = publish_normalized(dir.path(), &plan, &subscribers).unwrap();
    assert!(p.mirror_warning.is_some());
    assert_eq!(load_snapshot(dir.path()).unwrap().len(), 2);
    assert!(is_managed(dir.path()).unwrap());
    assert_eq!(
        publish_normalized(dir.path(), &plan, &subscribers)
            .unwrap()
            .generation,
        p.generation
    );
}

#[test]
fn assignments_commit_with_the_snapshot_and_survive_restart_and_corruption_is_blocked() {
    use lqosync_v3::assignment::{AssignmentState, Pin};
    let dir = tempfile::tempdir().unwrap();
    let settings: Settings = toml::from_str(include_str!("../examples/settings.toml")).unwrap();
    let subscribers = toml::from_str::<Snapshot>(include_str!("../examples/subscribers.toml"))
        .unwrap()
        .subscribers;
    let state = AssignmentState {
        routes: Default::default(),
        assignments: subscribers
            .iter()
            .map(|s| {
                (
                    s.id.clone(),
                    Pin {
                        source_node: s.node.clone(),
                        node: s.node.clone(),
                    },
                )
            })
            .collect(),
    };
    let plan = compile(&settings, &subscribers).unwrap();
    let published = publish_assigned(dir.path(), &plan, &subscribers, &state).unwrap();
    assert_eq!(load_assignments(dir.path()).unwrap(), state);
    let (snapshot, loaded) = load_assignment_snapshot(dir.path()).unwrap();
    assert_eq!(snapshot, subscribers);
    assert_eq!(loaded, state);
    assert_eq!(
        publish_assigned(dir.path(), &plan, &subscribers, &state)
            .unwrap()
            .generation,
        published.generation
    );
    let path = dir
        .path()
        .join("staging")
        .join(published.generation)
        .join("assignments.toml");
    let mut bad = state.clone();
    bad.assignments.values_mut().next().unwrap().node = "other-node".into();
    std::fs::write(&path, toml::to_string(&bad).unwrap()).unwrap();
    assert!(load_assignments(dir.path()).is_err());
    std::fs::remove_file(&path).unwrap();
    assert!(load_assignments(dir.path()).is_err());
    std::fs::write(path, "corrupt assignment state").unwrap();
    assert!(load_assignments(dir.path()).is_err());
}
