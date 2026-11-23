use super::*;
use crate::topology_manager::store::RegistryStore;

fn dir(root: &std::path::Path) -> std::path::PathBuf {
    drop(RegistryStore::open(root).unwrap());
    root.join("topology-manager")
}

fn one(id: &str, p: [f64; 2]) -> BTreeMap<String, [f64; 2]> {
    [(id.to_string(), p)].into()
}

fn known() -> BTreeSet<String> {
    ["wan1", "group1"].map(String::from).into()
}

#[test]
fn a_missing_layout_is_empty_at_revision_zero() {
    let root = tempfile::tempdir().unwrap();
    let l = load(&dir(root.path())).unwrap();
    assert_eq!((l.revision, l.positions.len()), (0, 0));
}

#[test]
fn a_layout_save_round_trips_and_leaves_the_registry_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let d = dir(root.path());
    let registry = std::fs::read(d.join("registry.json")).ok();
    let positions = [("wan1".to_string(), [10.5, -20.25])].into();
    assert_eq!(
        save(
            &d,
            0,
            web_contract::topology::Direction::TopDown,
            positions,
            &known()
        )
        .unwrap(),
        1
    );
    let l = load(&d).unwrap();
    assert_eq!((l.revision, l.positions["wan1"]), (1, [10.5, -20.25]));
    assert_eq!(std::fs::read(d.join("registry.json")).ok(), registry);
}

#[test]
fn a_layout_save_refuses_stale_revisions_unknown_ids_and_bad_numbers() {
    let root = tempfile::tempdir().unwrap();
    let d = dir(root.path());
    let one =
        |id: &str, p: [f64; 2]| -> BTreeMap<String, [f64; 2]> { [(id.to_string(), p)].into() };
    save(
        &d,
        0,
        web_contract::topology::Direction::TopDown,
        one("wan1", [0.0, 0.0]),
        &known(),
    )
    .unwrap();
    assert!(
        save(
            &d,
            0,
            web_contract::topology::Direction::TopDown,
            one("wan1", [1.0, 1.0]),
            &known()
        )
        .is_err(),
        "stale"
    );
    assert!(
        save(
            &d,
            1,
            web_contract::topology::Direction::TopDown,
            one("ghost", [1.0, 1.0]),
            &known()
        )
        .is_err(),
        "unknown"
    );
    assert!(
        save(
            &d,
            1,
            web_contract::topology::Direction::TopDown,
            one("wan1", [f64::NAN, 1.0]),
            &known()
        )
        .is_err(),
        "nan"
    );
    assert!(
        save(
            &d,
            1,
            web_contract::topology::Direction::TopDown,
            one("wan1", [1e9, 1.0]),
            &known()
        )
        .is_err(),
        "bound"
    );
    assert_eq!(load(&d).unwrap().revision, 1);
}

#[test]
fn each_direction_keeps_its_own_manual_positions() {
    use web_contract::topology::Direction;
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    assert_eq!(
        save(d, 0, Direction::TopDown, one("wan1", [1.0, 1.0]), &known()).unwrap(),
        1
    );
    save(
        d,
        1,
        Direction::LeftRight,
        one("group1", [2.0, 2.0]),
        &known(),
    )
    .unwrap();
    let l = load(d).unwrap();
    assert_eq!(l.direction, Direction::LeftRight);
    assert_eq!(l.positions, one("group1", [2.0, 2.0]));
    // Back to top-down with no positions: the saved top-down positions return.
    save(d, 2, Direction::TopDown, BTreeMap::new(), &known()).unwrap();
    assert_eq!(load(d).unwrap().positions, one("wan1", [1.0, 1.0]));
    // An empty save in the same direction is the reset layout.
    save(d, 3, Direction::TopDown, BTreeMap::new(), &known()).unwrap();
    assert!(load(d).unwrap().positions.is_empty());
}

#[test]
fn a_schema_1_layout_loads_as_top_down() {
    use web_contract::topology::Direction;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(FILE),
        br#"{"schema_version":1,"revision":3,"positions":{"wan1":[1.0,2.0]}}"#,
    )
    .unwrap();
    let l = load(dir.path()).unwrap();
    assert_eq!((l.revision, l.direction), (3, Direction::TopDown));
    assert_eq!(l.positions, one("wan1", [1.0, 2.0]));
}

#[test]
fn a_top_down_layout_stays_schema_1_for_an_older_build() {
    use web_contract::topology::Direction;
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    save(d, 0, Direction::TopDown, one("wan1", [1.0, 1.0]), &known()).unwrap();
    let text = std::fs::read_to_string(d.join(FILE)).unwrap();
    assert!(
        text.contains(r#""schema_version":1"#) && !text.contains("direction"),
        "{text}"
    );
    save(d, 1, Direction::LeftRight, BTreeMap::new(), &known()).unwrap();
    let text = std::fs::read_to_string(d.join(FILE)).unwrap();
    assert!(text.contains(r#""schema_version":2"#), "{text}");
}
