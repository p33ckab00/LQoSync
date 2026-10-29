use lqosync_v3::web::{App, routes};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let root = std::env::var_os("LQOSYNC3_DATA")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?.join("data"));
    let bind: SocketAddr = std::env::var("LQOSYNC3_BIND")
        .unwrap_or("127.0.0.1:9303".into())
        .parse()?;
    anyhow::ensure!(
        bind.ip().is_loopback(),
        "This development console must bind to loopback until authentication is implemented"
    );
    let mut app = App::open(root)?;
    app.reconcile();
    let state = Arc::new(Mutex::new(app));
    let background = state.clone();
    tokio::spawn(async move {
        loop {
            let seconds = background.lock().await.settings.interval_seconds;
            tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;
            lqosync_v3::orchestrator::cycle(background.clone()).await;
        }
    });
    let listener = tokio::net::TcpListener::bind(bind).await?;
    eprintln!(
        "LQoSync 3 development console: http://{} · local staging only",
        listener.local_addr()?
    );
    axum::serve(listener, routes(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
