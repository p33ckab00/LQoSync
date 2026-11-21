//! Isolated browser-test process. Never installed by the deployment script.
use std::{os::unix::fs::PermissionsExt, sync::Arc};
use web_gateway::{
    accounts::Accounts,
    broker,
    http::{WebState, router},
    sessions::Sessions,
};
/// The fixture as a fake `router-edit` command: it changes the synthetic inventory, never a config file.
fn fake_router_edit(config: &std::path::Path) -> anyhow::Result<()> {
    use web_contract::routers::*;
    let dir = config.parent().unwrap();
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
    let req: RouterEditRequest = serde_json::from_str(&input)?;
    let path = dir.join(CACHE);
    let mut inv: RouterInventory = serde_json::from_slice(&std::fs::read(&path)?)?;
    let e = &req.edit;
    let answer = |code: &str, message: &str, reply: Option<RouterEditReply>| {
        println!(
            "{}",
            serde_json::to_string(&RouterEditOutcome {
                code: code.into(),
                message: message.into(),
                reply
            })
            .unwrap()
        );
        Ok(())
    };
    if e.expected_digest != inv.config_digest {
        return answer(
            "conflict",
            "The configuration changed after this page loaded. Load the page again, then repeat the change.",
            None,
        );
    }
    if e.address.as_deref().is_some_and(|a| a.contains(' ')) {
        return answer("invalid", "Check the address.", None);
    }
    let (i, action) = match &req.router_id {
        None => {
            let name = e.name.clone().unwrap_or_default();
            if inv.routers.iter().any(|r| r.name == name) {
                return answer(
                    "invalid",
                    &format!("A router named {name} exists already."),
                    None,
                );
            }
            let id = format!(
                "lq3-{:016x}",
                name.bytes()
                    .fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(b))
                        .wrapping_mul(0x100000001b3))
            );
            inv.routers.push(RouterSummary {
                id,
                name,
                enabled: false,
                credential: true,
                port: 8729,
                tls: true,
                tls_verify: true,
                ..Default::default()
            });
            (inv.routers.len() - 1, "create")
        }
        Some(id) => match inv.routers.iter().position(|r| r.id == *id) {
            Some(i) => (i, "update"),
            None => {
                return answer(
                    "not_found",
                    "This router is not in the configuration.",
                    None,
                );
            }
        },
    };
    let r = &mut inv.routers[i];
    if let Some(v) = &e.address {
        r.address = v.clone();
    }
    if let Some(v) = &e.username {
        r.username = v.clone();
    }
    if let Some(v) = e.port {
        r.port = v;
    }
    if let Some(v) = e.tls {
        r.tls = v;
    }
    if let Some(v) = e.tls_verify {
        r.tls_verify = v;
    }
    if e.ca_pem.is_some() {
        r.tls_ca = true;
    }
    if e.clear_ca {
        r.tls_ca = false;
    }
    for (flag, kind) in [(e.pppoe, "ppp"), (e.dhcp, "dhcp"), (e.hotspot, "hotspot")] {
        let Some(on) = flag else { continue };
        r.roles.retain(|x| x != kind);
        if on {
            r.roles.push(kind.into());
        }
        match r.sources.iter_mut().find(|s| s.kind == kind) {
            Some(s) => s.enabled = on,
            None => r.sources.push(RouterSource {
                kind: kind.into(),
                enabled: on,
                ..Default::default()
            }),
        }
    }
    if let Some(v) = e.ppp_per_plan_node {
        r.options.ppp_per_plan_node = v;
    }
    if let Some(v) = e.dhcp_require_bound {
        r.options.dhcp_require_bound = v;
    }
    if let Some(v) = e.location {
        r.location = Some(v);
    }
    if e.clear_location {
        r.location = None;
    }
    let action = match (e.enabled, action) {
        (Some(v), "update") => {
            r.enabled = v;
            if !v {
                r.collection = None;
            }
            if v { "enable" } else { "disable" }
        }
        _ => action,
    };
    let event = RouterEvent {
        at: broker::now(),
        actor: req.actor.clone(),
        router_id: r.id.clone(),
        router: r.name.clone(),
        action: action.into(),
        changes: vec!["fixture".into()],
        digest: String::new(),
    };
    let id = r.id.clone();
    let digest = format!(
        "{:016x}",
        u64::from_str_radix(&inv.config_digest, 16).unwrap_or(0) + 1
    );
    inv.config_digest = digest.clone();
    let mut line = serde_json::to_vec(&RouterEvent {
        digest: digest.clone(),
        ..event
    })?;
    line.push(b'\n');
    std::io::Write::write_all(
        &mut std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(dir.join("router-audit.jsonl"))?,
        &line,
    )?;
    std::fs::write(&path, serde_json::to_vec(&inv)?)?;
    answer(
        "ok",
        "",
        Some(RouterEditReply {
            router_id: id,
            digest,
            reload: "sent".into(),
            warnings: vec![],
        }),
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("router-edit") {
        return fake_router_edit(std::path::Path::new(&args[3]));
    }
    if args.get(1).map(String::as_str) == Some("sync-now") {
        println!(r#"{{"code":"ok","message":"sent","reply":null}}"#);
        return Ok(());
    }
    let dir = tempfile::tempdir()?;
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))?;
    let accounts = Accounts::new(dir.path().join("accounts.db"))?;
    accounts.create("browser-test", &std::env::var("BROWSER_TEST_PASSWORD")?)?;
    let at = broker::now();
    let status = serde_json::json!({"version":3,"updated":at,"pid":1,"cycle_seq":1,"mode":"shadow","target":"v3_pair","cycle_ms":482,"wan_capacity":{"down_bps":200_000_000,"up_bps":80_000_000,"wan_count":2,"registry_revision":7}});
    let collection = serde_json::json!({"version":1,"updated":at,"pid":1,"cycle_seq":1,"fresh_for":300,"routers":[{"state":"online","observed_at":at,"sources":{"ppp":{"enabled":true,"online":2,"offline":1},"dhcp":{"enabled":true}},"subscribers":[{"source":"dhcp","state":"bound"},{"source":"ppp","state":"online"},{"source":"ppp","state":"offline"}]}]});
    let health = serde_json::json!({"version":1,"updated":at,"overall":"HEALTHY","update":{"phase":"Applied"},"components":[{"component":"collector","state":"HEALTHY","observed_at":at,"max_age":300},{"component":"import","state":"HEALTHY","observed_at":at,"max_age":300}]});
    // Synthetic router inventory: a WAN-only core, an online access router, and a failing one.
    let routers = serde_json::json!({"version":1,"updated":at,"fresh_for":300,"config_digest":"0000000000000001",
      "shaping":{"total_circuits":40,"grace_secs":600,"remove_inactive":true,"guard_enabled":true,"guard_max_count":20,"guard_max_percent":30.0,"guard_held":null},"routers":[
      {"id":"lq3-00000000000000c1","name":"Core-01","circuits":0,"address":"192.0.2.1","port":8729,"tls":true,"tls_verify":true,"tls_ca":true,"username":"lqosync","credential":true,
       "roles":["wan"],"wans":["ether1","ether2"],"collection":null,"sources":[{"kind":"ppp","enabled":false,"records":null,"online":null,"offline":null},{"kind":"dhcp","enabled":false,"records":null,"online":null,"offline":null},{"kind":"hotspot","enabled":false,"records":null,"online":null,"offline":null}],
       "location":null,"options":{"ppp_per_plan_node":false,"hotspot_include_mac":false,"dhcp_require_bound":false,"dhcp_discover":false,"dhcp_shape_unassigned":false,"dhcp_servers":0,"dhcp_clients":0,"exclusions":0}},
      {"id":"lq3-00000000000000a1","name":"Access-01","circuits":3,"address":"192.0.2.11","port":8728,"tls":false,"tls_verify":false,"tls_ca":false,"username":"lqosync","credential":true,
       "roles":["ppp","dhcp"],"wans":[],"collection":{"state":"online","observed_at":at,"subscribers":3},"sources":[{"kind":"ppp","enabled":true,"records":3,"online":2,"offline":1},{"kind":"dhcp","enabled":true,"records":1,"online":null,"offline":null},{"kind":"hotspot","enabled":false,"records":null,"online":null,"offline":null}],
       "location":[14.5995,120.9842],"options":{"ppp_per_plan_node":true,"hotspot_include_mac":false,"dhcp_require_bound":true,"dhcp_discover":true,"dhcp_shape_unassigned":false,"dhcp_servers":2,"dhcp_clients":1,"exclusions":3}},
      {"id":"lq3-00000000000000a2","name":"Access-02","circuits":0,"address":"192.0.2.12","port":8729,"tls":true,"tls_verify":false,"tls_ca":false,"username":"lqosync","credential":false,
       "roles":["hotspot"],"wans":[],"collection":{"state":"unreachable","observed_at":0,"subscribers":0},"sources":[{"kind":"ppp","enabled":false,"records":null,"online":null,"offline":null},{"kind":"dhcp","enabled":false,"records":null,"online":null,"offline":null},{"kind":"hotspot","enabled":true,"records":null,"online":null,"offline":null}],
       "location":null,"options":{"ppp_per_plan_node":false,"hotspot_include_mac":true,"dhcp_require_bound":false,"dhcp_discover":false,"dhcp_shape_unassigned":false,"dhcp_servers":0,"dhcp_clients":0,"exclusions":0}},
      {"id":"lq3-00000000000000a3","name":"Spare-01","circuits":0,"enabled":false,"address":"192.0.2.13","port":8729,"tls":true,"tls_verify":true,"tls_ca":false,"username":"lqosync","credential":true,
       "roles":["ppp"],"wans":[],"collection":null,"sources":[{"kind":"ppp","enabled":true,"records":null,"online":null,"offline":null},{"kind":"dhcp","enabled":false,"records":null,"online":null,"offline":null},{"kind":"hotspot","enabled":false,"records":null,"online":null,"offline":null}],
       "location":null,"options":{"ppp_per_plan_node":false,"hotspot_include_mac":false,"dhcp_require_bound":false,"dhcp_discover":false,"dhcp_shape_unassigned":false,"dhcp_servers":0,"dhcp_clients":0,"exclusions":0}}
    ]});
    for (name, value) in [
        ("status.json", status),
        ("collection.json", collection),
        ("system-health.json", health),
        ("routers.json", routers),
    ] {
        std::fs::write(dir.path().join(name), serde_json::to_vec(&value)?)?;
    }
    // A fake service API socket: connection tests answer from the synthetic routers.
    let api = tokio::net::UnixListener::bind(dir.path().join("api.sock"))?;
    tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        while let Ok((stream, _)) = api.accept().await {
            let (read, mut write) = stream.into_split();
            let mut line = String::new();
            let _ = tokio::io::BufReader::new(read).read_line(&mut line).await;
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            let id = v["args"]["router_id"].as_str().unwrap_or("").to_string();
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let (class, identity) = match id.as_str() {
                "lq3-00000000000000a2" => ("unreachable", None),
                "lq3-00000000000000a3" => ("disabled", None),
                _ => (
                    "ok",
                    Some(format!("fixture-{}", &id[id.len().saturating_sub(2)..])),
                ),
            };
            let answer = serde_json::json!({"v":1,"id":v["id"],"ok":true,"result":{
                "router_id":id,"class":class,"identity":identity,"elapsed_ms":42,"at":broker::now()}});
            let _ = write.write_all(format!("{answer}\n").as_bytes()).await;
        }
    });
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
                {"queue_id":0,"busy_ppm":250000,"idle_ppm":750000,"backlog_mean":0,"laps_per_s":12000,"busy_laps":3000,"cycles_per_busy_lap":2841956,"phase_cycles":[1512653783663u64,534643071037u64,580037740886u64,0,568981494214u64,123589123462u64,2127463624023u64,10857240998207u64,0,4200000000000u64]},
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
            let nic = serde_json::json!({"version":1,"updated":now,"stale":false,"dpdk_version":"fixture-dpdk","interval_ms":1000,"ports_capped":false,"ports":[
              {"id":0,"identity":{"pci":"0000:01:00.0","netdev":"fixture-wan0","role":"WAN","connector":"sfp+","confidence":"confirmed","evidence":"Synthetic module identifier"},"name":"fixture-port0","status":"up","speed_mbps":10000,"duplex":"full","reset":false,"stats":{"ipackets":1000,"opackets":900,"ibytes":1000000,"obytes":900000,"ierrors":0,"oerrors":0,"imissed":0,"rx_nombuf":0},"deltas":{"ipackets":10,"opackets":9,"ibytes":100000,"obytes":90000,"ierrors":0,"oerrors":0,"imissed":0,"rx_nombuf":0},"xstats":{"fixture_xstat":7,"rx_broadcast_packets":10,"rx_multicast_packets":20}},
              {"id":1,"identity":{"pci":"0000:01:00.1","netdev":"fixture-lan0","role":"LAN","connector":"rj45","confidence":"inferred","evidence":"Synthetic driver media"},"name":"fixture-port1","status":"up","speed_mbps":10000,"duplex":"full","reset":false,"stats":{"ipackets":900,"opackets":1000,"ierrors":0}}
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
        let _ = broker::serve(
            listener,
            unsafe { libc::geteuid() },
            accounts,
            state_dir.clone(),
            None,
            // The fixture answers router edits itself, on its synthetic inventory.
            Some(broker::Editor {
                bin: std::env::current_exe().expect("fixture path"),
                config: state_dir.join("fixture.toml"),
            }),
        )
        .await;
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
