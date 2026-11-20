use std::os::unix::fs::PermissionsExt;
use web_gateway::{accounts::Account, broker::now, sessions::Sessions};
#[test]
fn session_tokens_are_hashed_and_expiry_is_enforced() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = dir.path().join("sessions.db");
    let sessions = Sessions::new(path.clone()).unwrap();
    let account = Account {
        id: 1,
        epoch: 1,
        username: "test-only".into(),
        disabled: false,
    };
    let (token, _) = sessions.create(&account).unwrap();
    assert!(sessions.get(&token).unwrap().is_some());
    let db = rusqlite::Connection::open(path).unwrap();
    let stored: String = db
        .query_row("SELECT token FROM sessions", [], |r| r.get(0))
        .unwrap();
    assert_ne!(stored, token);
    db.execute("UPDATE sessions SET seen=?1", [now() - 1801])
        .unwrap();
    assert!(sessions.get(&token).unwrap().is_none());
    db.execute(
        "UPDATE sessions SET seen=?1,created=?2",
        [now(), now() - 28801],
    )
    .unwrap();
    assert!(sessions.get(&token).unwrap().is_none());
}
