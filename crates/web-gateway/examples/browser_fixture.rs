//! Isolated browser-test process. Never installed by the deployment script.
use std::{os::unix::fs::PermissionsExt, sync::Arc};
use web_gateway::{
    accounts::Accounts,
    broker,
    http::{WebState, router},
    sessions::Sessions,
};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))?;
    let accounts = Accounts::new(dir.path().join("accounts.db"))?;
    accounts.create("browser-test", &std::env::var("BROWSER_TEST_PASSWORD")?)?;
    let at = broker::now();
    let status = serde_json::json!({"version":3,"updated":at,"pid":1,"cycle_seq":1,"mode":"shadow","target":"v3_pair","cycle_ms":482});
    let collection = serde_json::json!({"version":1,"updated":at,"pid":1,"cycle_seq":1,"fresh_for":300,"routers":[{"state":"online","observed_at":at,"sources":{"ppp":{"enabled":true,"online":2,"offline":1},"dhcp":{"enabled":true}},"subscribers":[{"source":"dhcp","state":"bound"},{"source":"ppp","state":"online"},{"source":"ppp","state":"offline"}]}]});
    let health = serde_json::json!({"version":1,"updated":at,"overall":"HEALTHY","update":{"phase":"Applied"},"components":[{"component":"collector","state":"HEALTHY","observed_at":at,"max_age":300},{"component":"import","state":"HEALTHY","observed_at":at,"max_age":300}]});
    let workers = serde_json::json!({"version":1,"updated":at,"stale":false,"traffic":(0..60).map(|i|serde_json::json!({"at":at-60+i,"down_mbps":100.0+(i as f64/3.0).sin()*20.0,"up_mbps":20.0+(i as f64/4.0).cos()*5.0})).collect::<Vec<_>>()});
    for (name, value) in [
        ("status.json", status),
        ("collection.json", collection),
        ("system-health.json", health),
        ("worker-utilization.json", workers),
    ] {
        std::fs::write(dir.path().join(name), serde_json::to_vec(&value)?)?;
    }
    let socket = dir.path().join("broker.sock");
    let listener = tokio::net::UnixListener::bind(&socket)?;
    let state_dir = dir.path().to_owned();
    tokio::spawn(async move {
        let _ = broker::serve(listener, unsafe { libc::geteuid() }, accounts, state_dir).await;
    });
    let dist = std::path::PathBuf::from("crates/web-ui/dist");
    let state = WebState {
        broker: socket,
        sessions: Sessions::new(dir.path().join("sessions.db"))?,
        origin: "http://127.0.0.1:19581".into(),
        secure: false,
        index: Arc::new(std::fs::read_to_string(dist.join("index.html"))?),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:19581").await?;
    println!("Browser fixture ready on loopback 19581; synthetic data only");
    axum::serve(listener, router(state, dist)).await?;
    Ok(())
}
