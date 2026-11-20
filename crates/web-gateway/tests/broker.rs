use std::os::unix::fs::PermissionsExt;
use web_gateway::{accounts::Accounts, broker};
#[tokio::test]
async fn wrong_peer_uid_cannot_read_broker_data() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let accounts = Accounts::new(dir.path().join("accounts.db")).unwrap();
    let socket = dir.path().join("broker.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let state = dir.path().to_owned();
    let job = tokio::spawn(async move {
        let _ = broker::serve(
            listener,
            unsafe { libc::geteuid() } + 1,
            accounts,
            state,
            None,
        )
        .await;
    });
    assert!(
        broker::exchange(&socket, &broker::Request::Overview)
            .await
            .is_err()
    );
    job.abort();
}
