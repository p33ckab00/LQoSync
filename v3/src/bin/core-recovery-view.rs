//! GET-only local inspection. Does not start the normal console or contact RouterOS.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let root = std::path::PathBuf::from(args.next().ok_or_else(|| {
        anyhow::anyhow!("Usage: core-recovery-view DATA_DIRECTORY [127.0.0.1:9304]")
    })?)
    .canonicalize()?;
    let bind: std::net::SocketAddr = args
        .next()
        .unwrap_or_else(|| "127.0.0.1:9304".into())
        .parse()?;
    anyhow::ensure!(
        args.next().is_none() && bind.ip().is_loopback(),
        "Use a loopback bind and no extra arguments"
    );
    let listener = tokio::net::TcpListener::bind(bind).await?;
    eprintln!(
        "Read-only recovery viewer: http://{} · no router connections or actions",
        listener.local_addr()?
    );
    axum::serve(listener, lqosync_v3::core_recovery_web::viewer(root))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
