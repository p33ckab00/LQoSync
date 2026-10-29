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
