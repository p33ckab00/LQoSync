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
    let status = serde_json::json!({"version":3,"updated":at,"pid":1,"cycle_seq":1,"mode":"shadow","target":"v3_pair","cycle_ms":482,"wan_capacity":{"down_bps":200_000_000,"up_bps":80_000_000,"wan_count":2,"registry_revision":7}});
    let collection = serde_json::json!({"version":1,"updated":at,"pid":1,"cycle_seq":1,"fresh_for":300,"routers":[{"state":"online","observed_at":at,"sources":{"ppp":{"enabled":true,"online":2,"offline":1},"dhcp":{"enabled":true}},"subscribers":[{"source":"dhcp","state":"bound"},{"source":"ppp","state":"online"},{"source":"ppp","state":"offline"}]}]});
    let health = serde_json::json!({"version":1,"updated":at,"overall":"HEALTHY","update":{"phase":"Applied"},"components":[{"component":"collector","state":"HEALTHY","observed_at":at,"max_age":300},{"component":"import","state":"HEALTHY","observed_at":at,"max_age":300}]});
    for (name, value) in [
        ("status.json", status),
        ("collection.json", collection),
        ("system-health.json", health),
    ] {
        std::fs::write(dir.path().join(name), serde_json::to_vec(&value)?)?;
    }
    let cache_dir = dir.path().to_owned();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let now = broker::now();
            let workers = serde_json::json!({"version":1,"updated":now,"stale":false,"traffic":(0..60).map(|i| {
                let at = now - 59 + i;
                serde_json::json!({"at":at,"down_mbps":100.0+(at as f64/3.0).sin()*20.0,"up_mbps":20.0+(at as f64/4.0).cos()*5.0})
            }).collect::<Vec<_>>()});
            let temporary = cache_dir.join("workers.next");
            if std::fs::write(&temporary, serde_json::to_vec(&workers).unwrap()).is_ok() {
                let _ = std::fs::rename(temporary, cache_dir.join("worker-utilization.json"));
            }
        }
    });
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
