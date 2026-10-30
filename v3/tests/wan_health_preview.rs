use std::process::Command;
#[test]
fn unconfigured_preview_is_read_only_and_does_not_create_state() {
    let dir = tempfile::tempdir().unwrap();
    let settings = dir.path().join("settings.toml");
    std::fs::write(&settings, "").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_wan-health-preview"))
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["configured"], false);
    assert_eq!(report["router_writes"], false);
    assert_eq!(report["wans"], serde_json::json!([]));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}
#[test]
fn wrong_root_and_invalid_round_count_do_not_create_default_settings() {
    let dir = tempfile::tempdir().unwrap();
    for args in [
        vec![dir.path().to_str().unwrap()],
        vec![dir.path().to_str().unwrap(), "0"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_wan-health-preview"))
            .args(args)
            .output()
            .unwrap();
        assert!(!out.status.success());
    }
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}
