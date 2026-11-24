//! The gateway and broker for the Studio smoke test: the real broker and HTTP router in one
//! process, as the browser fixture uses them, but connected to the smoke core service.
//! It has no router editor, so no router-edit or sync-now command runs and no process gets a
//! signal. Every path must be under `SMOKE_ROOT`. Never installed.
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use web_gateway::{
    accounts::Accounts,
    broker,
    http::{WebState, router},
    sessions::Sessions,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let root = PathBuf::from(std::env::var("SMOKE_ROOT")?);
    let state = PathBuf::from(std::env::var("SMOKE_STATE")?);
    let port: u16 = std::env::var("SMOKE_PORT")?.parse()?;
    anyhow::ensure!(
        root.is_absolute() && state.starts_with(&root),
        "the smoke gateway uses only paths under SMOKE_ROOT"
    );
    let web = root.join("web");
    std::fs::create_dir_all(web.join("dist"))?;
    std::fs::set_permissions(&web, std::fs::Permissions::from_mode(0o700))?;
    // SMOKE_DIST: a copy of the real Wasm UI under SMOKE_ROOT. Without it, a placeholder page.
    let dist = match std::env::var("SMOKE_DIST").ok().filter(|d| !d.is_empty()) {
        Some(d) => PathBuf::from(d),
        None => {
            std::fs::write(web.join("dist/index.html"), "<html>smoke</html>")?;
            web.join("dist")
        }
    };
    anyhow::ensure!(
        dist.starts_with(&root),
        "SMOKE_DIST must be under SMOKE_ROOT"
    );
    let accounts = Accounts::new(web.join("accounts.db"))?;
    accounts.create("smoke-operator", &std::env::var("SMOKE_PASSWORD")?)?;
    let socket = web.join("broker.sock");
    let listener = tokio::net::UnixListener::bind(&socket)?;
    tokio::spawn(async move {
        // SAFETY: geteuid has no preconditions.
        let uid = unsafe { libc::geteuid() };
        let _ = broker::serve(listener, uid, accounts, state, None, None).await;
    });
    let origin = format!("http://127.0.0.1:{port}");
    let app = router(
        WebState {
            broker: socket,
            sessions: Sessions::new(web.join("sessions.db"))?,
            origin,
            secure: false,
            index: Arc::new(std::fs::read_to_string(dist.join("index.html"))?),
        },
        dist,
    );
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    println!("READY smoke gateway on loopback {port}");
    axum::serve(listener, app).await?;
    Ok(())
}
