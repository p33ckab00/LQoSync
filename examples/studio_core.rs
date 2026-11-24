//! The core service for the Studio smoke test: the real `Runner`, cycle, and service API, with
//! the secret files owned by the current user instead of root. It has no WAN monitor, WAN
//! writer, Telegram worker, or apply monitor, and it sends no signal to another process.
//! Usage: studio_core <config>. It refuses a config whose state directory is outside `SMOKE_ROOT`.
use lqosync3::mikrotik::MikrotikConnector;
use lqosync3::runner::Runner;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal::unix::{SignalKind, signal};

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: studio_core <config>"))?;
    let root = std::env::var("SMOKE_ROOT")?;
    let (cfg, digest) = lqosync3::cli::read_config(Path::new(&path))?;
    let inside = |p: &str| Path::new(p).starts_with(&root);
    anyhow::ensure!(
        inside(&path) && inside(&cfg.runtime.state_dir) && inside(&cfg.libreqos.directory),
        "the smoke core uses only paths under SMOKE_ROOT"
    );
    // SAFETY: geteuid has no preconditions.
    let owner_uid = unsafe { libc::geteuid() };
    let mut runner = Runner::new(cfg.clone(), Arc::new(MikrotikConnector { owner_uid }));
    runner.lock()?;
    let (listener, _socket) = runner.api_bind()?;
    let (api_jobs, mut jobs) = tokio::sync::mpsc::channel(16);
    lqosync3::api::server::spawn(listener, api_jobs);
    runner.set_source(Path::new(&path), digest);
    let mut term = signal(SignalKind::terminate())?;
    println!("READY smoke core (test runtime)");
    loop {
        let _summary = runner.cycle(now(), false).await;
        println!("CYCLE done");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        tokio::select! {
            served = lqosync3::api::wait::serve_until(deadline, &mut jobs, &mut runner) => {
                if served.is_err() {
                    anyhow::bail!("instance lock lost");
                }
            }
            _ = term.recv() => break,
        }
    }
    Ok(())
}
