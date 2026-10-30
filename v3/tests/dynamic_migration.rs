use lqosync_v3::{model, store};

#[test]
fn dynamic_root_without_a_service_layer_roundtrips() {
    let raw = r#"
[[nodes]]
id="root"
name="Flat root"
kind="group"
mode="manual"
download=100.0
upload=100.0
download_factor=1.0
upload_factor=1.0
[dynamic]
version=1
wans=[]
policies=[]
pins={}
"#;
    let s: model::Settings = toml::from_str(raw).expect("dynamic settings must be readable");
    model::validate(&s).unwrap();
    let dir = tempfile::tempdir().unwrap();
    store::save_settings(dir.path(), &s).unwrap();
    assert_eq!(store::load_settings(dir.path()).unwrap(), s);
    let mut cyclic = s;
    cyclic.nodes[0].parent = "root".into();
    assert!(model::validate(&cyclic).is_err());
}

mod common;
#[test]
fn legacy_migration_preserves_sources_and_creates_a_private_backup_only_on_save() {
    use lqosync_v3::{
        assignment::{Pin, Pool},
        dynamic, normalizer,
    };
    let (mut s, _) = common::fixture();
    s.nodes[0].router = "r1".into();
    s.nodes[0].source = "uplink".into();
    s.assignment.pools.push(Pool {
        source_node: "ppp".into(),
        targets: vec!["ppp".into()],
        balance: true,
        metric: "planned_load".into(),
    });
    let id = normalizer::subscriber_id("r1", "ppp", "offline").unwrap();
    s.assignment.pins.insert(
        id.clone(),
        Pin {
            source_node: "ppp".into(),
            node: "ppp".into(),
        },
    );
    let dir = tempfile::tempdir().unwrap();
    store::save_settings(dir.path(), &s).unwrap();
    let before = std::fs::read(dir.path().join("settings.toml")).unwrap();
    let converted = dynamic::migrate(&s).unwrap();
    assert_eq!(
        converted.nodes.iter().map(|n| &n.id).collect::<Vec<_>>(),
        s.nodes.iter().map(|n| &n.id).collect::<Vec<_>>()
    );
    let config = converted.dynamic.as_ref().unwrap();
    assert_eq!(config.policies.len(), 3);
    assert_eq!(config.pins[&id].target.node, "ppp");
    assert_eq!(
        config
            .policies
            .iter()
            .find(|p| p.id == "ppp")
            .unwrap()
            .metric,
        "planned_load"
    );
    assert_eq!(dynamic::migrate(&s).unwrap(), converted);
    assert_eq!(
        std::fs::read(dir.path().join("settings.toml")).unwrap(),
        before
    );
    assert!(
        !dir.path()
            .join("state/settings-before-dynamic.toml")
            .exists()
    );
    store::save_settings(dir.path(), &converted).unwrap();
    assert_eq!(
        std::fs::read(dir.path().join("state/settings-before-dynamic.toml")).unwrap(),
        before
    );
    store::save_settings(dir.path(), &converted).unwrap();
    assert_eq!(
        std::fs::read(dir.path().join("state/settings-before-dynamic.toml")).unwrap(),
        before
    );
    assert_eq!(dynamic::migrate(&converted).unwrap(), converted);
    let mut bad = converted.clone();
    bad.dynamic.as_mut().unwrap().policies[0].targets.clear();
    assert!(store::save_settings(dir.path(), &bad).is_err());
    assert_eq!(store::load_settings(dir.path()).unwrap(), converted);
}
#[test]
fn migration_rejects_unresolved_wans_without_touching_old_settings() {
    let (s, _) = common::fixture();
    assert!(lqosync_v3::dynamic::migrate(&s).is_err());
    assert!(s.dynamic.is_none());
}
