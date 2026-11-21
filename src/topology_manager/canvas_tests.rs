use super::*;
use crate::topology_manager::store::RegistryStore;

fn dir(root: &std::path::Path) -> std::path::PathBuf {
    drop(RegistryStore::open(root).unwrap());
    root.join("topology-manager")
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
    assert_eq!(save(&d, 0, positions, &known()).unwrap(), 1);
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
    save(&d, 0, one("wan1", [0.0, 0.0]), &known()).unwrap();
    assert!(
        save(&d, 0, one("wan1", [1.0, 1.0]), &known()).is_err(),
        "stale"
    );
    assert!(
        save(&d, 1, one("ghost", [1.0, 1.0]), &known()).is_err(),
        "unknown"
    );
    assert!(
        save(&d, 1, one("wan1", [f64::NAN, 1.0]), &known()).is_err(),
        "nan"
    );
    assert!(
        save(&d, 1, one("wan1", [1e9, 1.0]), &known()).is_err(),
        "bound"
    );
    assert_eq!(load(&d).unwrap().revision, 1);
}
