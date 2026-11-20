use anyhow::{Context, Result, ensure};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use web_gateway::{accounts::Accounts, broker};
#[tokio::main]
async fn main() -> Result<()> {
    ensure!(unsafe { libc::geteuid() } == 0, "broker requires root");
    let uid: u32 = std::env::var("WEB_GATEWAY_UID")
        .context("WEB_GATEWAY_UID is required")?
        .parse()?;
    let path = std::path::PathBuf::from(
        std::env::var("WEB_BROKER").unwrap_or("/run/lqosync3-web/broker.sock".into()),
    );
    // The parent is installed root-owned and is not writable by the gateway.
    if let Ok(meta) = std::fs::symlink_metadata(&path) {
        ensure!(
            meta.file_type().is_socket(),
            "refuse to replace a non-socket"
        );
        ensure!(
            tokio::net::UnixStream::connect(&path).await.is_err(),
            "broker already active"
        );
        std::fs::remove_file(&path)?;
    }
    let listener = tokio::net::UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    let c = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())?;
    ensure!(
        unsafe { libc::chown(c.as_ptr(), uid, u32::MAX) } == 0,
        "socket ownership failed"
    );
    let accounts_path: std::path::PathBuf = std::env::var("WEB_ACCOUNTS")
        .unwrap_or("/var/lib/lqosync3-web-accounts/web-accounts.db".into())
        .into();
    // The traffic history lives in the broker's private state directory, next to the accounts.
    let history = accounts_path
        .parent()
        .map(|d| d.join("traffic-history.json"));
    let accounts = Accounts::new(accounts_path)?;
    broker::serve(
        listener,
        uid,
        accounts,
        std::env::var("LQOSYNC_STATE")
            .unwrap_or("/var/lib/lqosync3".into())
            .into(),
        history,
    )
    .await
}
