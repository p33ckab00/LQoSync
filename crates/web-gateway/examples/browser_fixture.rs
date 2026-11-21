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
    if e.exclude_dhcp_servers.is_some()
        || e.exclude_ppp_profiles.is_some()
        || e.exclude_ips.is_some()
        || e.dhcp_clients.is_some()
        || e.dhcp_servers.is_some()
        || e.dhcp_server_mode.is_some()
    {
        let lists = r.lists.get_or_insert_with(Default::default);
        if let Some(v) = &e.exclude_dhcp_servers {
            lists.exclude_dhcp_servers = v.clone();
        }
        if let Some(v) = &e.exclude_ppp_profiles {
            lists.exclude_ppp_profiles = v.clone();
        }
        if let Some(v) = &e.exclude_ips {
            lists.exclude_ips = v.clone();
        }
        if let Some(v) = &e.dhcp_server_mode {
            lists.dhcp_server_mode = v.clone();
        }
        if let Some(v) = &e.dhcp_servers {
            lists.dhcp_servers = v.clone();
            r.options.dhcp_servers = v.len() as u64;
        }
        if let Some(v) = &e.dhcp_clients {
            lists.dhcp_clients = v.clone();
            r.options.dhcp_clients = v.len() as u64;
        }
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
    // The fixture as a fake journalctl: synthetic lines only, never the journal of this machine.
    if args.get(1).map(String::as_str) == Some("-u") {
        let value = |flag: &str| {
            args.iter()
                .position(|a| a == flag)
                .and_then(|i| args.get(i + 1))
        };
        let n: usize = value("-n").and_then(|v| v.parse().ok()).unwrap_or(10);
        let max: u8 = value("-p")
            .and_then(|v| v.rsplit("..").next()?.parse().ok())
            .unwrap_or(7);
        let until: u64 = args
            .iter()
            .find_map(|a| a.strip_prefix("--until=@"))
            .and_then(|t| {
                let (s, us) = t.split_once('.')?;
                Some(s.parse::<u64>().ok()? * 1_000_000 + us.parse::<u64>().ok()?)
            })
            .unwrap_or(u64::MAX);
        let unit = &args[2];
        let base = (broker::now() - 600) * 1_000_000;
        let lines: Vec<String> = (0..600u64)
            .filter(|_| unit == "lqosync3.service")
            .map(|i| {
                let (prio, msg) = if i % 50 == 7 {
                    (3, format!("router Access-02: unreachable (synthetic line {i})"))
                } else if i % 20 == 3 {
                    (4, format!("removal guard: 0 of 40 circuits (synthetic line {i})"))
                } else {
                    (6, format!("cycle {i} done: 3 routers read (synthetic)"))
                };
                (base + i * 1_000_000, prio, msg)
            })
            .filter(|(at, prio, _)| *prio <= max && *at <= until)
            .map(|(at, prio, msg)| {
                serde_json::json!({"__REALTIME_TIMESTAMP": at.to_string(), "PRIORITY": prio.to_string(), "MESSAGE": msg})
                    .to_string()
            })
            .collect();
        for l in &lines[lines.len().saturating_sub(n)..] {
            println!("{l}");
        }
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("sync-now") {
        println!(r#"{{"code":"ok","message":"sent","reply":null}}"#);
        return Ok(());
    }
    // SAFETY: set before the fixture serves; nothing reads the environment at the same time.
    unsafe { std::env::set_var("LQOSYNC_JOURNALCTL", std::env::current_exe()?) };
    let dir = tempfile::tempdir()?;
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))?;
    let accounts = Accounts::new(dir.path().join("accounts.db"))?;
    accounts.create("browser-test", &std::env::var("BROWSER_TEST_PASSWORD")?)?;
    let at = broker::now();
    let status = serde_json::json!({"version":3,"updated":at,"pid":1,"cycle_seq":1,"mode":"shadow","target":"v3_pair","cycle_ms":482,"wan_capacity":{"down_bps":200_000_000,"up_bps":80_000_000,"wan_count":2,"registry_revision":7,"wans":[{"name":"WAN-Fiber","down_bps":150_000_000,"up_bps":60_000_000},{"name":"WAN-LTE","down_bps":50_000_000,"up_bps":20_000_000}]}});
    let collection = serde_json::json!({"version":1,"updated":at,"pid":1,"cycle_seq":1,"fresh_for":300,"routers":[{"state":"online","observed_at":at,"sources":{"ppp":{"enabled":true,"online":2,"offline":1},"dhcp":{"enabled":true}},"subscribers":[{"source":"dhcp","state":"bound"},{"source":"ppp","state":"online"},{"source":"ppp","state":"offline"}]}]});
    let health = serde_json::json!({"version":1,"updated":at,"overall":"HEALTHY","update":{"phase":"Applied"},"components":[{"component":"collector","state":"HEALTHY","observed_at":at,"max_age":300},{"component":"import","state":"HEALTHY","observed_at":at,"max_age":300}]});
    // Synthetic router inventory: a WAN-only core, an online access router, and a failing one.
    let routers = serde_json::json!({"version":1,"updated":at,"fresh_for":300,"config_digest":"0000000000000001",
      "shaping":{"total_circuits":40,"grace_secs":600,"remove_inactive":true,"guard_enabled":true,"guard_max_count":20,"guard_max_percent":30.0,"guard_held":null},"routers":[
      {"id":"lq3-00000000000000c1","name":"Core-01","circuits":0,"address":"192.0.2.1","port":8729,"tls":true,"tls_verify":true,"tls_ca":true,"username":"lqosync","credential":true,
       "roles":["wan"],"wans":["ether1","ether2"],"collection":null,"sources":[{"kind":"ppp","enabled":false,"records":null,"online":null,"offline":null},{"kind":"dhcp","enabled":false,"records":null,"online":null,"offline":null},{"kind":"hotspot","enabled":false,"records":null,"online":null,"offline":null}],
       "location":null,"options":{"ppp_per_plan_node":false,"hotspot_include_mac":false,"dhcp_require_bound":false,"dhcp_discover":false,"dhcp_shape_unassigned":false,"dhcp_servers":0,"dhcp_clients":0,"exclusions":0}},
      {"id":"lq3-00000000000000a1","name":"Access-01",
       "lists":{"exclude_dhcp_servers":[],"exclude_ppp_profiles":["staff"],"exclude_ips":["100.64.9.0/24"],"dhcp_clients":[{"mac":"02:00:00:00:0A:01","download_mbps":20.0,"upload_mbps":5.0,"name":"Office"}],
         "dhcp_server_mode":"per_site","dhcp_servers":[{"name":"LAN","mode":"per_plan","speed_sources":["lease_comment","settings"],"default_plan_down_mbps":50.0,"default_plan_up_mbps":10.0,"download_factor":null,"upload_factor":null}],"truncated":false},"circuits":3,"address":"192.0.2.11","port":8728,"tls":false,"tls_verify":false,"tls_ca":false,"username":"lqosync","credential":true,
       "roles":["ppp","dhcp"],"wans":[],"collection":{"state":"online","observed_at":at,"subscribers":3},"sources":[{"kind":"ppp","enabled":true,"records":3,"online":2,"offline":1},{"kind":"dhcp","enabled":true,"records":1,"online":null,"offline":null},{"kind":"hotspot","enabled":false,"records":null,"online":null,"offline":null}],
       "location":[14.5995,120.9842],"options":{"ppp_per_plan_node":true,"hotspot_include_mac":false,"dhcp_require_bound":true,"dhcp_discover":true,"dhcp_shape_unassigned":false,"dhcp_servers":2,"dhcp_clients":1,"exclusions":3}},
      {"id":"lq3-00000000000000a2","name":"Access-02","circuits":0,"address":"192.0.2.12","port":8729,"tls":true,"tls_verify":false,"tls_ca":false,"username":"lqosync","credential":false,
       "roles":["hotspot"],"wans":[],"collection":{"state":"unreachable","observed_at":0,"subscribers":0},"sources":[{"kind":"ppp","enabled":false,"records":null,"online":null,"offline":null},{"kind":"dhcp","enabled":false,"records":null,"online":null,"offline":null},{"kind":"hotspot","enabled":true,"records":null,"online":null,"offline":null}],
       "location":null,"options":{"ppp_per_plan_node":false,"hotspot_include_mac":true,"dhcp_require_bound":false,"dhcp_discover":false,"dhcp_shape_unassigned":false,"dhcp_servers":0,"dhcp_clients":0,"exclusions":0}},
      {"id":"lq3-00000000000000a3","name":"Spare-01","circuits":0,"enabled":false,"address":"192.0.2.13","port":8729,"tls":true,"tls_verify":true,"tls_ca":false,"username":"lqosync","credential":true,
       "roles":["ppp"],"wans":[],"collection":null,"sources":[{"kind":"ppp","enabled":true,"records":null,"online":null,"offline":null},{"kind":"dhcp","enabled":false,"records":null,"online":null,"offline":null},{"kind":"hotspot","enabled":false,"records":null,"online":null,"offline":null}],
       "location":null,"options":{"ppp_per_plan_node":false,"hotspot_include_mac":false,"dhcp_require_bound":false,"dhcp_discover":false,"dhcp_shape_unassigned":false,"dhcp_servers":0,"dhcp_clients":0,"exclusions":0}}
    ]});
    // Synthetic topology view: two WANs, one shared plan with four unique members, one subscriber
    // with both PPPoE and DHCP identities, and observation-only rows.
    let ident = |kind: &str, account: &str| serde_json::json!({"router_id":"lq3-00000000000000a1","source_id":format!("access-{kind}"),"kind":kind,"account_ref":account});
    let active = |node: &str, ip: &str| serde_json::json!({"node":node,"ips":[ip],"min_mbps":[50.0,25.0],"max_mbps":[100.0,50.0],"fallback":false});
    let obs = |kind: &str,
               name: &str,
               ip: &str,
               state: &str,
               sub: Option<&str>,
               pin: Option<&str>,
               node: Option<&str>| {
        serde_json::json!({"router_id":"lq3-00000000000000a1","router":"Access-01","kind":kind,"name":name,"ipv4":ip,"mac":"",
          "state":state,"link":if sub.is_some(){"verified"}else{"none"},"subscriber_id":sub,
          "plan_id":sub.map(|_|"plan-access-100"),"pin":pin,"node":node})
    };
    let topology = serde_json::json!({"version":1,"updated":at,"fresh_for":300,"snapshot_id":"r42-c1-o42-g7-p1",
      "registry_revision":"42","mode":"apply","writer":"topology","enrolled":true,
      "observed":{"revision":"42","collected_at":at.saturating_sub(8),"generation":7,"state":"ok","stale":false},
      "nodes":[
        {"id":"tm-node-wan-north","name":"WAN North","parent":null,"kind":"wan","source_id":"wan-north","capacity":{"mode":"fixed","down_mbps":1000,"up_mbps":500,"factor_percent":null},"alias":false,"calculated_mbps":[1000.0,500.0],"exported_mbps":[1000.0,500.0],"members":3},
        {"id":"tm-node-ppp-north","name":"PPPoE access","parent":"tm-node-wan-north","kind":"group","source_id":"access-ppp","capacity":{"mode":"automatic"},"alias":false,"calculated_mbps":[400.0,200.0],"exported_mbps":[400.0,200.0],"members":3},
        {"id":"tm-node-wan-south","name":"WAN South","parent":null,"kind":"wan","source_id":"wan-south","capacity":{"mode":"fixed","down_mbps":500,"up_mbps":250,"factor_percent":null},"alias":false,"calculated_mbps":[500.0,250.0],"exported_mbps":[500.0,250.0],"members":1},
        {"id":"tm-node-dhcp-south","name":"DHCP access","parent":"tm-node-wan-south","kind":"group","source_id":"access-dhcp","capacity":{"mode":"automatic"},"alias":false,"calculated_mbps":[200.0,100.0],"exported_mbps":[200.0,100.0],"members":1}
      ],
      "sources":[{"id":"wan-north","name":"pppoe-wan-north","kind":"wan"},{"id":"wan-south","name":"pppoe-wan-south","kind":"wan"},
                 {"id":"access-ppp","name":"PPPoE access","kind":"subscribers"},{"id":"access-dhcp","name":"DHCP service","kind":"subscribers"}],
      "fallback":null,
      "plans":[{"id":"plan-access-100","name":"Access 100","down_mbps":100,"up_mbps":50,"ppp_identities":3,"dhcp_identities":2,"subscribers":4}],
      "subscribers":[
        {"id":"tm-sub-0000000000000002","identities":[ident("ppp","ppp-name/v1/account-02"),ident("dhcp","dhcp-mac/v1/02:00:00:00:00:02")],"plan_id":"plan-access-100","pin":null,"assignment":null,"active":active("tm-node-ppp-north","192.0.2.12")},
        {"id":"tm-sub-0000000000000003","identities":[ident("ppp","ppp-name/v1/account-03")],"plan_id":"plan-access-100","pin":"tm-node-wan-south","assignment":null,"active":active("tm-node-ppp-north","192.0.2.13")},
        {"id":"tm-sub-0000000000000004","identities":[ident("ppp","ppp-name/v1/account-04")],"plan_id":"plan-access-100","pin":"tm-node-wan-north","assignment":null,"active":null},
        {"id":"tm-sub-0000000000000006","identities":[ident("dhcp","dhcp-mac/v1/02:00:00:00:00:06")],"plan_id":"plan-access-100","pin":null,"assignment":null,"active":active("tm-node-dhcp-south","192.0.2.16")}
      ],
      "observations":[
        obs("ppp","account-02","192.0.2.12","online",Some("tm-sub-0000000000000002"),None,Some("tm-node-ppp-north")),
        obs("ppp","account-03","192.0.2.13","online",Some("tm-sub-0000000000000003"),Some("tm-node-wan-south"),Some("tm-node-ppp-north")),
        obs("ppp","account-04","","offline",Some("tm-sub-0000000000000004"),Some("tm-node-wan-north"),None),
        obs("ppp","account-05","","disabled",None,None,None),
        obs("dhcp","client-02","192.0.2.22","bound",Some("tm-sub-0000000000000002"),None,Some("tm-node-ppp-north")),
        obs("dhcp","client-06","192.0.2.16","bound",Some("tm-sub-0000000000000006"),None,Some("tm-node-dhcp-south")),
        obs("dhcp","unknown-host","192.0.2.30","bound",None,None,None)
      ],
      "collection":[{"router_id":"lq3-00000000000000a1","name":"Access-01","state":"online","observed_at":at,
        "sources":[{"kind":"ppp","enabled":true,"records":4,"online":2,"offline":1},{"kind":"dhcp","enabled":true,"records":3,"online":3,"offline":null}]}],
      "conflicts":[],
      "speed":{"down_min":{"unit":"percent","value":"20"},"up_min":{"unit":"percent","value":"20"},
               "down_max":{"unit":"percent","value":"100"},"up_max":{"unit":"legacy","value":"100"},"overrides":1},
      "factors":[{"source_id":"access-ppp","name":"PPPoE access","down":"0.5","up":"0.5"},
                 {"source_id":"access-dhcp","name":"DHCP service","down":"1","up":"1"}],
      "publication":{"write_id":"w-42-1","phase":"import_confirmed","started":at.saturating_sub(30),"updated":at.saturating_sub(20),
        "policy_revision":"42","files":[{"name":"ShapedDevices.csv","outcome":"written"},{"name":"network.json","outcome":"unchanged"}],
        "failed":false,"accepted":false,"superseded_by":null},
      "history":[{"at":at.saturating_sub(60),"actor":"browser-test","change":"Create plan Access 100","state":"saved","base_revision":"41","revision":"42"}]
    });
    for (name, value) in [
        ("topology.json", topology),
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
            // Topology changes: a review that moves one subscriber, then a saved receipt.
            let topology = match v["op"].as_str().unwrap_or("") {
                "topology.edit.propose" => Some(serde_json::json!({
                    "proposal_id": "edit-42-1", "expires_at": broker::now() + 120,
                    "edit": v["args"]["edit"],
                    "nodes": [{"id":"tm-node-wan-north","name":"WAN North","parent":null,"kind":"wan","source_id":"wan-north","capacity":{"mode":"fixed","down_mbps":1000,"up_mbps":500,"factor_percent":null},"alias":false},
                              {"id":"tm-node-ppp-north","name":"PPPoE access","parent":"tm-node-wan-north","kind":"group","source_id":"access-ppp","capacity":{"mode":"automatic"},"alias":false},
                              {"id":"tm-node-wan-south","name":"WAN South","parent":null,"kind":"wan","source_id":"wan-south","capacity":{"mode":"fixed","down_mbps":500,"up_mbps":250,"factor_percent":null},"alias":false},
                              {"id":"tm-node-dhcp-south","name":"DHCP access","parent":"tm-node-wan-south","kind":"group","source_id":"access-dhcp","capacity":{"mode":"automatic"},"alias":false},
                              {"id":"tm-node-new","name":v["args"]["edit"]["name"].as_str().unwrap_or("Changed"),"parent":"tm-node-wan-north","kind":"intermediate","source_id":null,"capacity":{"mode":"automatic"},"alias":false}],
                    "moves": [{"subscriber_id":"tm-sub-0000000000000003","label":"account-03","from":"tm-node-ppp-north","to":"tm-node-wan-south"}],
                    "cleared": [], "assigned": 0, "new_records": 0, "sources": ["access-ppp"],
                    "rates_changed": 1, "rates": [{"subscriber_id":"tm-sub-0000000000000002","label":"account-02",
                        "before":{"min_mbps":[50.0,25.0],"max_mbps":[100.0,50.0]},"after":{"min_mbps":[100.0,50.0],"max_mbps":[200.0,100.0]}}],
                    "capacities": [{"node_id":"tm-node-ppp-north","before_mbps":[400.0,200.0],"after_mbps":[500.0,250.0]}]})),
                "topology.web.mutate" => Some(
                    serde_json::json!({"key": v["args"]["key"], "state": "saved",
                    "base_revision": "42", "revision": "43", "subject_id": null, "error": null}),
                ),
                "topology.edit.cancel" => Some(serde_json::json!({})),
                "topology.preview.refresh" => Some(serde_json::json!({"state": "ok"})),
                "wan.discover" => Some(
                    serde_json::json!({"collected_at": broker::now(), "unconfigured": [], "routers": [
                    {"router_id":"lq3-00000000000000c1","name":"Core-01","state":"ok","error":null,"missing":[],
                     "candidates":[{"name":"pppoe-out3","type":"pppoe-out","running":true,"disabled":false,"comment":null,"suggested":true,"source_id":null,"used_by":[]},
                                   {"name":"pppoe-out1","type":"pppoe-out","running":true,"disabled":false,"comment":null,"suggested":true,"source_id":"wan-north","used_by":["WAN North"]}]},
                    {"router_id":"lq3-00000000000000a2","name":"Access-02","state":"failed","error":"the router did not answer","missing":[],"candidates":[]}]}),
                ),
                "topology.sources.candidates" => Some(serde_json::json!({"candidates": [
                    {"key":"src-hotspot-a1","kind":"hotspot","router":"lq3-00000000000000a1","name":"Hotspot Access-01","subscribers":null,"rebind":false}]})),
                "topology.web.operation" => Some(serde_json::Value::Null),
                _ => None,
            };
            if let Some(result) = topology {
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                let answer = serde_json::json!({"v":1,"id":v["id"],"ok":true,"result":result});
                let _ = write.write_all(format!("{answer}\n").as_bytes()).await;
                continue;
            }
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
