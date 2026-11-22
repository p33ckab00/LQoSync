//! Probes each WAN node through its router and keeps its health for placement.
//! A probe reads `/interface/print` and runs `/ping`. It changes no router configuration.
use crate::config::Config;
use crate::routeros::{Connector, RouterProbe, RouterSession};
use crate::topology_manager::wan_health::{Sample, WanHealth, WanSnapshot, WanState};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// One WAN node and the router interface that carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WanProbe {
    pub node_id: String,
    /// The configured router name.
    pub router: String,
    pub interface: String,
}

/// The WAN nodes of a registry and its automatic-failback switch.
pub fn probes_of(doc: &crate::topology_manager::model::RegistryDocument) -> (Vec<WanProbe>, bool) {
    use crate::topology_manager::catalog::SourceOrigin;
    let Some(c) = doc.catalog.as_ref() else {
        return (Vec::new(), true);
    };
    let wans = c
        .nodes
        .iter()
        .filter_map(|n| {
            let source = c
                .sources
                .iter()
                .find(|s| Some(&s.id) == n.source_id.as_ref())?;
            let SourceOrigin::Wan {
                router_id,
                endpoint_ref,
            } = &source.origin
            else {
                return None;
            };
            let router = c.routers.iter().find(|r| &r.id == router_id)?;
            Some(WanProbe {
                node_id: n.id.clone(),
                router: router.name.clone(),
                interface: endpoint_ref.clone(),
            })
        })
        .collect();
    (wans, c.auto_failback)
}

/// The display values of one WAN for the topology view.
#[derive(Debug, Clone, PartialEq)]
pub struct WanView {
    pub state: WanState,
    pub quality_percent: Option<u64>,
    /// The last probe error, for the operator (for example a missing `test` policy).
    pub error: Option<String>,
}

struct Shared {
    cfg: Config,
    auto_failback: bool,
    wans: Vec<WanProbe>,
    health: HashMap<String, WanHealth>,
    errors: HashMap<String, String>,
    /// The operator range lists of the routers (not `via-`), as address ranges and WAN nodes.
    legacy: Vec<(u32, u32, String)>,
    legacy_at: Option<u64>,
}

/// The WAN node of each operator address-list: an enabled `prerouting` `mark-routing` rule gives
/// list → routing mark; the enabled default route of that table with the lowest distance gives
/// the interface; the WAN probe with that interface gives the node. `via-` lists are skipped.
pub fn discover_lists(
    mangle: &[crate::model::RosRow],
    routes: &[crate::model::RosRow],
    wans: &[WanProbe],
) -> HashMap<String, String> {
    let get = |r: &crate::model::RosRow, k: &str| r.get(k).cloned().unwrap_or_default();
    let on = |r: &crate::model::RosRow| get(r, "disabled") != "true";
    let mut out = HashMap::new();
    for rule in mangle.iter().filter(|r| on(r)) {
        let list = get(rule, "src-address-list");
        if get(rule, "chain") != "prerouting"
            || get(rule, "action") != "mark-routing"
            || list.is_empty()
            || list.starts_with('!')
            || list.starts_with("via-")
        {
            continue;
        }
        let table = get(rule, "new-routing-mark");
        let route = routes
            .iter()
            .filter(|r| {
                on(r) && get(r, "routing-table") == table && get(r, "dst-address") == "0.0.0.0/0"
            })
            .min_by_key(|r| get(r, "distance").parse::<u32>().unwrap_or(u32::MAX));
        let Some(route) = route else { continue };
        let gateway = get(route, "gateway");
        if let Some(wan) = wans.iter().find(|w| w.interface == gateway) {
            out.entry(list).or_insert_with(|| wan.node_id.clone());
        }
    }
    out
}

/// An address-list address: one address, `a.b.c.d/n`, or `first-last`, as inclusive `u32` bounds.
pub fn parse_range(text: &str) -> Option<(u32, u32)> {
    let ip = |t: &str| t.trim().parse::<std::net::Ipv4Addr>().ok().map(u32::from);
    if let Some((a, b)) = text.split_once('-') {
        let (a, b) = (ip(a)?, ip(b)?);
        return (a <= b).then_some((a, b));
    }
    if let Some((a, n)) = text.split_once('/') {
        let n: u32 = n.parse().ok().filter(|n| *n <= 32)?;
        let mask = if n == 0 { 0 } else { u32::MAX << (32 - n) };
        let a = ip(a)? & mask;
        return Some((a, a | !mask));
    }
    ip(text).map(|a| (a, a))
}

/// The shared WAN state. The runner and the probe task hold clones.
#[derive(Clone)]
pub struct Handle(Arc<Mutex<Shared>>);

impl Handle {
    pub fn new(cfg: &Config) -> Self {
        Self(Arc::new(Mutex::new(Shared {
            cfg: cfg.clone(),
            auto_failback: true,
            wans: Vec::new(),
            health: HashMap::new(),
            errors: HashMap::new(),
            legacy: Vec::new(),
            legacy_at: None,
        })))
    }

    fn with<T>(&self, f: impl FnOnce(&mut Shared) -> T) -> T {
        // A poisoned lock only means a panic in another holder; the data stays usable.
        let mut shared = self.0.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut shared)
    }

    /// The WAN nodes of the current registry. A WAN that stays keeps its history.
    pub fn set_wans(&self, wans: Vec<WanProbe>, auto_failback: bool) {
        self.with(|s| {
            s.auto_failback = auto_failback;
            s.health
                .retain(|id, _| wans.iter().any(|w| &w.node_id == id));
            for wan in &wans {
                s.health
                    .entry(wan.node_id.clone())
                    .or_insert_with(|| WanHealth::new(&s.cfg.wan_monitor, auto_failback))
                    .set_auto_failback(auto_failback);
            }
            s.wans = wans;
        });
    }

    /// A config reload. Changed probe values start a new history.
    pub fn reconfigure(&self, cfg: &Config) {
        self.with(|s| {
            if s.cfg.wan_monitor != cfg.wan_monitor {
                s.health.clear();
            }
            s.cfg = cfg.clone();
        });
        let (wans, auto) = self.with(|s| (s.wans.clone(), s.auto_failback));
        self.set_wans(wans, auto);
    }

    pub(crate) fn observe(&self, node_id: &str, now: u64, sample: Sample, error: Option<String>) {
        self.with(|s| {
            if let Some(h) = s.health.get_mut(node_id) {
                h.observe(now, sample);
            }
            match error {
                Some(e) => s.errors.insert(node_id.to_string(), e),
                None => s.errors.remove(node_id),
            };
        });
    }

    /// The placement view. With the monitor off, the snapshot is empty: every WAN is available.
    pub fn snapshot(&self, now: u64) -> WanSnapshot {
        self.with(|s| {
            if !s.cfg.wan_monitor.enabled {
                return WanSnapshot::default();
            }
            WanSnapshot(
                s.health
                    .iter()
                    .map(|(id, h)| (id.clone(), h.status(now)))
                    .collect(),
            )
        })
    }

    pub fn view(&self, now: u64) -> HashMap<String, WanView> {
        let snapshot = self.snapshot(now);
        self.with(|s| {
            snapshot
                .0
                .into_iter()
                .map(|(id, status)| {
                    let view = WanView {
                        state: status.state,
                        quality_percent: status
                            .quality
                            .map(|q| q.numerator * 100 / q.denominator.max(1)),
                        error: s.errors.get(&id).cloned(),
                    };
                    (id, view)
                })
                .collect()
        })
    }

    /// The WAN node of an address in the operator range lists, from the last read.
    pub fn legacy_wan_of(&self, ip: std::net::Ipv4Addr) -> Option<String> {
        let ip = u32::from(ip);
        self.with(|s| {
            s.legacy
                .iter()
                .find(|(a, b, _)| *a <= ip && ip <= *b)
                .map(|(_, _, node)| node.clone())
        })
    }

    #[cfg(test)]
    pub(crate) fn set_legacy(&self, ranges: Vec<(u32, u32, String)>) {
        self.with(|s| s.legacy = ranges);
    }

    /// A saved Rebalance: each WAN that waits for it is available again.
    pub fn release_waiting(&self) {
        self.with(|s| s.health.values_mut().for_each(WanHealth::release));
    }
}

/// The probe sessions. One tick probes every WAN once.
pub struct Prober<C: Connector> {
    connector: Arc<C>,
    sessions: HashMap<String, C::Session>,
}

impl<C> Prober<C>
where
    C: Connector,
    C::Session: RouterProbe,
{
    pub fn new(connector: Arc<C>) -> Self {
        Self {
            connector,
            sessions: HashMap::new(),
        }
    }

    pub async fn probe_once(&mut self, handle: &Handle, now: u64) {
        let (cfg, wans) = handle.with(|s| (s.cfg.clone(), s.wans.clone()));
        if !cfg.wan_monitor.enabled {
            return;
        }
        let limit = Duration::from_secs(cfg.wan_monitor.interval_secs.max(1));
        let mut routers: Vec<&str> = wans.iter().map(|w| w.router.as_str()).collect();
        routers.sort_unstable();
        routers.dedup();
        for router in routers {
            let mine: Vec<&WanProbe> = wans.iter().filter(|w| w.router == router).collect();
            // The operator range lists change seldom: read them once a minute.
            if handle.with(|s| s.legacy_at.is_none_or(|t| now >= t + 60))
                && let Ok(ranges) = self.read_legacy(&cfg, router, &wans, limit).await
            {
                handle.with(|s| {
                    s.legacy
                        .retain(|(_, _, n)| !mine.iter().any(|w| &w.node_id == n));
                    s.legacy.extend(ranges);
                    s.legacy_at = Some(now);
                });
            }
            match self.probe_router(&cfg, router, &mine, limit).await {
                Ok(samples) => {
                    for (wan, sample) in mine.iter().zip(samples) {
                        handle.observe(&wan.node_id, now, sample, None);
                    }
                }
                Err(error) => {
                    // The session may be dead. The next tick connects again.
                    self.sessions.remove(router);
                    for wan in &mine {
                        handle.observe(&wan.node_id, now, Sample::ProbeError, Some(error.clone()));
                    }
                }
            }
        }
    }

    async fn read_legacy(
        &mut self,
        cfg: &Config,
        router: &str,
        wans: &[WanProbe],
        limit: Duration,
    ) -> Result<Vec<(u32, u32, String)>, String> {
        self.session(cfg, router, limit).await?;
        let session = self.sessions.get_mut(router).expect("connected above");
        let mut rows = Vec::with_capacity(3);
        for (path, fields) in [
            (
                "/ip/firewall/mangle",
                "chain,action,new-routing-mark,src-address-list,disabled",
            ),
            (
                "/ip/route",
                "dst-address,routing-table,gateway,distance,disabled",
            ),
            ("/ip/firewall/address-list", "list,address,disabled,dynamic"),
        ] {
            rows.push(
                tokio::time::timeout(limit, session.print(path, fields))
                    .await
                    .map_err(|_| format!("{path} read timed out"))?
                    .map_err(|e| e.to_string())?,
            );
        }
        let map = discover_lists(&rows[0], &rows[1], wans);
        Ok(rows[2]
            .iter()
            .filter(|r| r.get("disabled").map(String::as_str) != Some("true"))
            .filter_map(|r| {
                let node = map.get(r.get("list")?)?;
                let (a, b) = parse_range(r.get("address")?)?;
                Some((a, b, node.clone()))
            })
            .collect())
    }

    async fn session(&mut self, cfg: &Config, router: &str, limit: Duration) -> Result<(), String> {
        if !self.sessions.contains_key(router) {
            let rc = cfg
                .routers
                .iter()
                .find(|r| r.name == router)
                .ok_or_else(|| format!("router {router} is not in the config"))?;
            let session =
                tokio::time::timeout(limit, self.connector.connect(rc, &cfg.router_access))
                    .await
                    .map_err(|_| "the router did not answer".to_string())?
                    .map_err(|e| e.to_string())?;
            self.sessions.insert(router.to_string(), session);
        }
        Ok(())
    }

    async fn probe_router(
        &mut self,
        cfg: &Config,
        router: &str,
        wans: &[&WanProbe],
        limit: Duration,
    ) -> Result<Vec<Sample>, String> {
        self.session(cfg, router, limit).await?;
        let session = self.sessions.get_mut(router).expect("connected above");
        let rows =
            tokio::time::timeout(limit, session.print("/interface", "name,disabled,running"))
                .await
                .map_err(|_| "the interface read timed out".to_string())?
                .map_err(|e| e.to_string())?;
        let mut samples = Vec::with_capacity(wans.len());
        for wan in wans {
            let up = rows.iter().any(|r| {
                r.get("name") == Some(&wan.interface)
                    && r.get("disabled").map(String::as_str) != Some("true")
                    && r.get("running").map(String::as_str) == Some("true")
            });
            if !up {
                samples.push(Sample::InterfaceDown);
                continue;
            }
            let mut best: Option<u64> = None;
            for target in &cfg.wan_monitor.targets {
                let rtt = tokio::time::timeout(limit, session.ping(target, &wan.interface))
                    .await
                    .map_err(|_| "the ping timed out".to_string())?
                    .map_err(|e| e.to_string())?;
                best = match (best, rtt) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (a, b) => a.or(b),
                };
            }
            samples.push(best.map_or(Sample::Lost, |rtt_us| Sample::Reply { rtt_us }));
        }
        Ok(samples)
    }
}

/// Start the probe task. It probes each interval until the service stops.
pub fn start<C>(connector: Arc<C>, handle: Handle) -> tokio::task::JoinHandle<()>
where
    C: Connector + 'static,
    C::Session: RouterProbe + 'static,
{
    tokio::spawn(async move {
        let mut prober = Prober::new(connector);
        // shortcut: a changed interval_secs applies after a service restart; other values apply
        // at the next tick. Rebuild the timer on reconfigure if operators tune it often.
        let interval = handle.with(|s| s.cfg.wan_monitor.interval_secs.max(1));
        let mut tick = tokio::time::interval(Duration::from_secs(interval));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            prober.probe_once(&handle, now).await;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RouterAccess, RouterCfg};
    use crate::model::RosRow;
    use crate::routeros::ReadError;

    #[derive(Default)]
    struct Fake {
        pings: Mutex<Vec<(String, String)>>,
        down: bool,
        unreachable: bool,
    }
    struct FakeConnector(Arc<Fake>);
    struct FakeSession(Arc<Fake>);
    impl Connector for FakeConnector {
        type Session = FakeSession;
        fn connect(
            &self,
            _: &RouterCfg,
            _: &RouterAccess,
        ) -> impl Future<Output = Result<FakeSession, ReadError>> + Send {
            let (fake, fail) = (self.0.clone(), self.0.unreachable);
            async move {
                if fail {
                    Err(ReadError::Trap("no route to host".into()))
                } else {
                    Ok(FakeSession(fake))
                }
            }
        }
    }
    impl RouterSession for FakeSession {
        fn print(
            &mut self,
            _: &str,
            _: &str,
        ) -> impl Future<Output = Result<Vec<RosRow>, ReadError>> + Send {
            let running = if self.0.down { "false" } else { "true" };
            let rows = ["pppoe-out1", "pppoe-out2"]
                .iter()
                .map(|n| {
                    [("name", *n), ("disabled", "false"), ("running", running)]
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect()
                })
                .collect();
            async move { Ok(rows) }
        }
    }
    impl RouterProbe for FakeSession {
        fn ping(
            &mut self,
            address: &str,
            interface: &str,
        ) -> impl Future<Output = Result<Option<u64>, ReadError>> + Send {
            self.0
                .pings
                .lock()
                .unwrap()
                .push((interface.to_string(), address.to_string()));
            async move { Ok(Some(10_000)) }
        }
    }

    fn cfg() -> Config {
        Config::from_toml_str(
            "[[routers]]\nname='Core'\naddress='192.0.2.1'\nusername='u'\npassword_file='/x'\n",
        )
        .unwrap()
    }

    fn wans() -> Vec<WanProbe> {
        ["pppoe-out1", "pppoe-out2"]
            .iter()
            .enumerate()
            .map(|(i, iface)| WanProbe {
                node_id: format!("wan{}", i + 1),
                router: "Core".into(),
                interface: iface.to_string(),
            })
            .collect()
    }

    async fn run(fake: Fake, ticks: u64) -> (Arc<Fake>, Handle) {
        let fake = Arc::new(fake);
        let handle = Handle::new(&cfg());
        handle.set_wans(wans(), true);
        let mut prober = Prober::new(Arc::new(FakeConnector(fake.clone())));
        for t in 0..ticks {
            prober.probe_once(&handle, t * 10).await;
        }
        (fake, handle)
    }

    fn ros(fields: &[(&str, &str)]) -> RosRow {
        fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// The FSE-CORE mangle rules and routes of 2026-11-22.
    fn core() -> (Vec<RosRow>, Vec<RosRow>) {
        let mark = |list: &str, table: &str, disabled: &str| {
            ros(&[
                ("chain", "prerouting"),
                ("action", "mark-routing"),
                ("new-routing-mark", table),
                ("src-address-list", list),
                ("disabled", disabled),
            ])
        };
        let route = |table: &str, gw: &str, distance: &str| {
            ros(&[
                ("dst-address", "0.0.0.0/0"),
                ("routing-table", table),
                ("gateway", gw),
                ("distance", distance),
                ("disabled", "false"),
            ])
        };
        (
            vec![
                mark("WAN2-CUSTOMERS", "wan2-rt", "false"),
                mark("WAN1-CUSTOMERS", "wan1-rt", "false"),
                mark("OLD-LIST", "wan1-rt", "true"),
                mark("via-WAN1", "wan1-rt", "false"),
            ],
            vec![
                route("wan1-rt", "pppoe-out1", "1"),
                route("wan1-rt", "pppoe-out2", "2"),
                route("wan2-rt", "pppoe-out2", "1"),
                route("wan2-rt", "pppoe-out1", "2"),
            ],
        )
    }

    #[test]
    fn the_core_router_lists_map_to_their_wans() {
        let (mangle, routes) = core();
        let map = discover_lists(&mangle, &routes, &wans());
        assert_eq!(map.len(), 2, "{map:?}");
        assert_eq!(map["WAN1-CUSTOMERS"], "wan1");
        assert_eq!(map["WAN2-CUSTOMERS"], "wan2");
    }

    #[test]
    fn disabled_rules_via_lists_and_fallback_routes_do_not_map() {
        let (mangle, mut routes) = core();
        // Without the distance-1 route, the fallback route gives the other WAN.
        routes.retain(|r| !(r["routing-table"] == "wan1-rt" && r["distance"] == "1"));
        let map = discover_lists(&mangle, &routes, &wans());
        assert_eq!(map["WAN1-CUSTOMERS"], "wan2");
        assert!(!map.contains_key("OLD-LIST") && !map.contains_key("via-WAN1"));
    }

    #[test]
    fn ranges_and_subnets_match_client_addresses() {
        let ip = |t: &str| u32::from(t.parse::<std::net::Ipv4Addr>().unwrap());
        assert_eq!(
            parse_range("10.0.100.211-10.0.100.221"),
            Some((ip("10.0.100.211"), ip("10.0.100.221")))
        );
        assert_eq!(
            parse_range("10.15.0.0/24"),
            Some((ip("10.15.0.0"), ip("10.15.0.255")))
        );
        assert_eq!(
            parse_range("10.0.0.7"),
            Some((ip("10.0.0.7"), ip("10.0.0.7")))
        );
        assert_eq!(parse_range("10.0.0.9-10.0.0.1"), None);
        assert_eq!(parse_range("example.com"), None);
        let h = Handle::new(&cfg());
        h.set_legacy(vec![(ip("10.15.0.0"), ip("10.15.0.255"), "wan1".into())]);
        assert_eq!(
            h.legacy_wan_of("10.15.0.9".parse().unwrap()).as_deref(),
            Some("wan1")
        );
        assert_eq!(h.legacy_wan_of("10.16.0.9".parse().unwrap()), None);
    }

    #[tokio::test]
    async fn each_wan_is_probed_through_its_own_interface() {
        let (fake, handle) = run(Fake::default(), 7).await;
        let pings = fake.pings.lock().unwrap();
        assert_eq!(
            pings[..4],
            [
                ("pppoe-out1".to_string(), "1.1.1.1".to_string()),
                ("pppoe-out1".into(), "8.8.8.8".into()),
                ("pppoe-out2".into(), "1.1.1.1".into()),
                ("pppoe-out2".into(), "8.8.8.8".into()),
            ]
        );
        let snapshot = handle.snapshot(60);
        assert_eq!(snapshot.0["wan1"].state, WanState::Up);
        assert_eq!(snapshot.0["wan2"].state, WanState::Up);
    }

    #[tokio::test]
    async fn a_router_that_does_not_answer_gives_unknown() {
        let fake = Fake {
            unreachable: true,
            ..Fake::default()
        };
        let (_, handle) = run(fake, 30).await;
        let view = handle.view(290);
        assert_eq!(view["wan1"].state, WanState::Unknown);
        assert!(view["wan1"].error.as_deref().unwrap().contains("no route"));
        assert!(!handle.snapshot(290).0["wan1"].excluded());
    }

    #[tokio::test]
    async fn a_not_running_interface_is_degraded_without_a_ping() {
        let fake = Fake {
            down: true,
            ..Fake::default()
        };
        let (fake, handle) = run(fake, 1).await;
        assert!(fake.pings.lock().unwrap().is_empty());
        assert_eq!(handle.snapshot(0).0["wan1"].state, WanState::Degraded);
    }

    #[tokio::test]
    async fn the_monitor_off_gives_an_empty_snapshot() {
        let mut c = cfg();
        c.wan_monitor.enabled = false;
        let (_, handle) = run(Fake::default(), 7).await;
        handle.reconfigure(&c);
        assert!(handle.snapshot(60).0.is_empty());
    }
}
