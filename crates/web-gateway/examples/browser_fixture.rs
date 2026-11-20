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
            let mut workers = workers;
            workers["workers"] = serde_json::json!([
                {"queue_id":0,"busy_ppm":250000,"idle_ppm":750000,"backlog_mean":0,"laps_per_s":12000,"busy_laps":3000,"cycles_per_busy_lap":42,"phase_cycles":[1,2,3,4,5,6,7,8,9,10]},
                {"queue_id":1,"busy_ppm":750000,"idle_ppm":250000,"backlog_mean":3,"laps_per_s":16000}
            ]);
            workers["pressure"] = serde_json::json!({"diagnostics":{"pool_free_mbufs":800,"pool_total_mbufs":1000,"ring_count":2,"ring_capacity":200,"ring_used":40,"rings_over_75":0},"rx_nombuf_delta":0,"pool_drop_delta":0});
            workers["queue"] = serde_json::json!({"totals":{"drop_pkts":10,"aqm_marks":20,"backlog_bytes":2048},"drop_delta":0,"aqm_delta":1});
            workers["health"] = serde_json::json!({"updated":now,"stale":false,"summary":{"status":"ok","version":"fixture-bridge","uptime_ms":(now-at+1)*1000,"worker_count":2,"missing_rollups":0,"max_rollup_age_ms":100,"aqm_degraded_workers":0,"topology_tier":"pair","classification_empty":false,"degraded_subsystems":[]}});
            workers["classification"] = serde_json::json!({
                "mapped":{"bytes":{"download_bytes":1000000,"upload_bytes":100000},"history":(0..60).map(|i| {
                    let at=now-59+i;
                    serde_json::json!({"at":at,"down_mbps":90.0+(at as f64/3.0).sin()*18.0,"up_mbps":18.0+(at as f64/4.0).cos()*4.0})
                }).collect::<Vec<_>>()},
                "unmapped":{"bytes":{"download_bytes":10000,"upload_bytes":1000},"history":(0..60).map(|i| {
                    let at=now-59+i;
                    if i==25 {serde_json::json!({"at":at,"down_mbps":null,"up_mbps":null})}
                    else {serde_json::json!({"at":at,"down_mbps":5.0+(at as f64/2.0).sin()*3.0,"up_mbps":1.0+(at as f64/3.0).cos()*0.5})}
                }).collect::<Vec<_>>()}
            });
            let nic = serde_json::json!({"version":1,"updated":now,"stale":false,"dpdk_version":"fixture-dpdk","interval_ms":10000,"ports_capped":false,"ports":[
              {"id":0,"name":"fixture-port0","status":"up","speed_mbps":10000,"duplex":"full","reset":false,"stats":{"ipackets":1000,"opackets":900,"ibytes":1000000,"obytes":900000,"ierrors":0,"oerrors":0,"imissed":0,"rx_nombuf":0},"deltas":{"ipackets":10,"opackets":9,"ierrors":0},"xstats":{"fixture_xstat":7}},
              {"id":1,"name":"fixture-port1","status":"up","speed_mbps":10000,"duplex":"full","reset":false,"stats":{"ipackets":900,"opackets":1000,"ierrors":0}}
            ]});
            let nic_temporary = cache_dir.join("nic.next");
            if std::fs::write(&nic_temporary, serde_json::to_vec(&nic).unwrap()).is_ok() {
                let _ = std::fs::rename(nic_temporary, cache_dir.join("nic-telemetry.json"));
            }
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
