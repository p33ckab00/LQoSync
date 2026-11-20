use std::os::unix::fs::PermissionsExt;
use web_gateway::accounts::Accounts;
#[test]
fn credential_changes_revoke_sessions_and_last_admin_survives() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = Accounts::new(dir.path().join("accounts.db")).unwrap();
    store.create("admin-a", "valid-test-password-1").unwrap();
    let initial = store.verify("admin-a", "valid-test-password-1").unwrap();
    assert!(store.verify("admin-a", "incorrect-password").is_err());
    assert!(store.delete("admin-a").is_err());
    assert!(store.disable("admin-a", true).is_err());
    store.create("admin-b", "valid-test-password-2").unwrap();
    store.password("admin-a", "new-test-password-3").unwrap();
    assert!(!store.active(initial.id, initial.epoch).unwrap());
    let current = store.verify("admin-a", "new-test-password-3").unwrap();
    store.rename("admin-a", "renamed").unwrap();
    assert!(!store.active(current.id, current.epoch).unwrap());
    store.disable("renamed", true).unwrap();
    assert!(store.verify("renamed", "new-test-password-3").is_err());
    store.delete("renamed").unwrap();
    assert_eq!(store.list().unwrap().len(), 1);
}
#[test]
fn bad_names_and_duplicate_accounts_fail() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = Accounts::new(dir.path().join("accounts.db")).unwrap();
    assert!(store.create("../../bad", "valid-test-password").is_err());
    assert!(store.create("good", "short").is_err());
    store.create("good", "valid-test-password").unwrap();
    assert!(store.create("good", "other-valid-password").is_err());
}
