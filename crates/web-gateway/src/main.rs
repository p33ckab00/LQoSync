use anyhow::{Context, Result, ensure};
use std::{path::PathBuf, sync::Arc};
use web_gateway::{
    http::{WebState, router},
    sessions::Sessions,
};
#[tokio::main]
async fn main() -> Result<()> {
    let origin = std::env::var("WEB_ORIGIN")
        .context("WEB_ORIGIN is required, for example https://lqosync.fsehub.qzz.io")?;
    let bind = std::env::var("WEB_BIND").unwrap_or("127.0.0.1:9581".into());
    let address: std::net::SocketAddr = bind.parse()?;
    let uri: axum::http::Uri = origin.parse().context("invalid WEB_ORIGIN")?;
    ensure!(
        matches!(uri.scheme_str(), Some("http" | "https"))
            && uri.authority().is_some()
            && uri.path() == "/"
            && uri.query().is_none(),
        "WEB_ORIGIN must contain only a scheme and host"
    );
    let secure = uri.scheme_str() == Some("https");
    ensure!(
        secure
            || (address.ip().is_loopback()
                && std::env::var("WEB_LOCAL_HTTP").as_deref() == Ok("1")),
        "HTTPS origin required except explicit loopback development"
    );
    ensure!(
        !origin.ends_with('/') && !origin.contains(['\n', '\r']),
        "use a bare origin without a trailing slash"
    );
    let dist = PathBuf::from(std::env::var("WEB_DIST").unwrap_or("crates/web-ui/dist".into()));
    let state = WebState {
        broker: std::env::var("WEB_BROKER")
            .unwrap_or("/run/lqosync3-web/broker.sock".into())
            .into(),
        sessions: Sessions::new(
            std::env::var("WEB_SESSIONS")
                .unwrap_or("/var/lib/lqosync3-web/web-auth.db".into())
                .into(),
        )?,
        origin,
        secure,
        index: Arc::new(std::fs::read_to_string(dist.join("index.html"))?),
    };
    let listener = tokio::net::TcpListener::bind(address).await?;
    eprintln!("LQoSync web gateway listening on {address}");
    axum::serve(listener, router(state, dist))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
