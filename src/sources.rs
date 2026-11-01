//! Circuit collection: RouterOS rows of one router to circuits.

use crate::config::{
    Config, DhcpMode, DhcpSource, PppSource, RouterCfg, normalize_mac, parse_ipv4_net,
};
use crate::model::{RosRow, RouterSnapshot, Source};
use crate::speed::{Rate, SpeedParser, parse_default_rate};
use std::collections::BTreeMap;

/// The group node that a circuit belongs to.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupKey {
    Ppp,
    PppPlan(String),
    Hotspot,
    DhcpSite(String),
    DhcpPlan(String),
}

/// One circuit from the router, before the cycle merges it with the state.
#[derive(Debug, Clone, PartialEq)]
pub struct Collected {
    pub name: String,
    pub device_name: String,
    pub mac: String,
    pub ipv4: String,
    pub comment: String,
    pub source: Source,
    pub base: Rate,
    pub group: GroupKey,
    /// The group rate of one circuit: the server rate for a DHCP site, else `base`.
    pub unit: Rate,
}

/// A field value, trimmed. A missing field is an empty string.
fn f<'a>(row: &'a RosRow, key: &str) -> &'a str {
    row.get(key).map_or("", |v| v.trim())
}

/// A RouterOS flag is on only for "true" or "yes" (case is ignored).
fn flag_on(row: &RosRow, key: &str) -> bool {
    let v = f(row, key);
    v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
}

fn mac_key(mac: &str) -> String {
    mac.replace(':', "").to_ascii_uppercase()
}

/// Plan label for a DHCP plan node: "25M-5M", or "10M" if both rates are the same.
pub fn plan_label(r: Rate) -> String {
    let (d, u) = (r.down.trunc() as i64, r.up.trunc() as i64);
    if r.down == r.up {
        format!("{d}M")
    } else {
        format!("{d}M-{u}M")
    }
}

/// `*` matches any text. Case is ignored.
fn glob(pattern: &str, text: &str) -> bool {
    let (p, t) = (pattern.to_lowercase(), text.to_lowercase());
    let parts: Vec<&str> = p.split('*').collect();
    if parts.len() == 1 {
        return p == t;
    }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !t.starts_with(first) || !t[first.len()..].ends_with(last) {
        return false;
    }
    let mut rest = &t[first.len()..t.len() - last.len()];
    for mid in &parts[1..parts.len() - 1] {
        match rest.find(mid) {
            Some(i) => rest = &rest[i + mid.len()..],
            None => return false,
        }
    }
    true
}

/// The exclusion rules of one router: the global `[exclude]` and the router `exclude`.
struct Excluder<'a> {
    servers: Vec<&'a str>,
    profiles: Vec<&'a str>,
    nets: Vec<(u32, u32)>,
}

impl<'a> Excluder<'a> {
    fn new(cfg: &'a Config, router: &'a RouterCfg) -> Self {
        let both = [&cfg.exclude, &router.exclude];
        Excluder {
            servers: both
                .iter()
                .flat_map(|x| x.dhcp_servers.iter().map(String::as_str))
                .collect(),
            profiles: both
                .iter()
                .flat_map(|x| x.ppp_profiles.iter().map(String::as_str))
                .collect(),
            nets: both
                .iter()
                .flat_map(|x| x.ips.iter().filter_map(|i| parse_ipv4_net(i)))
                .collect(),
        }
    }
    fn server(&self, name: &str) -> bool {
        self.servers.iter().any(|p| glob(p, name))
    }
    fn profile(&self, name: &str) -> bool {
        self.profiles.iter().any(|p| glob(p, name))
    }
    fn ip(&self, ip: &str) -> bool {
        let Ok(a) = ip.trim().parse::<std::net::Ipv4Addr>() else {
            return false;
        };
        let a = u32::from(a);
        self.nets.iter().any(|(net, mask)| a & mask == *net)
    }
}

/// Collect the circuits of one router: PPPoE in name order, then Hotspot and DHCP in reply order.
pub fn collect_router(
    cfg: &Config,
    router: &RouterCfg,
    snap: &RouterSnapshot,
    speed: &SpeedParser,
    report: &mut Vec<String>,
) -> Vec<Collected> {
    let ex = Excluder::new(cfg, router);
    let mut out = Vec::new();
    if router.pppoe.enabled {
        collect_ppp(cfg, router, snap, speed, &ex, report, &mut out);
    }
    if router.hotspot.enabled {
        collect_hotspot(cfg, router, snap, &ex, &mut out);
    }
    if router.dhcp.enabled {
        collect_dhcp(cfg, router, snap, speed, &ex, report, &mut out);
    }
    out
}

/// A comment rate. A comment with a zero speed gives a report line; then the next source applies.
fn comment_rate(
    speed: &SpeedParser,
    text: &str,
    at: &str,
    report: &mut Vec<String>,
) -> Option<Rate> {
    let r = speed.comment(text);
    if r.is_none() && speed.zero_rate(text) {
        report.push(format!(
            "{at}: comment \"{text}\" has a zero speed; the next speed source is used"
        ));
    }
    r
}

fn collect_ppp(
    cfg: &Config,
    router: &RouterCfg,
    snap: &RouterSnapshot,
    speed: &SpeedParser,
    ex: &Excluder,
    report: &mut Vec<String>,
    out: &mut Vec<Collected>,
) {
    let bd = &cfg.bandwidth_defaults;
    // A later row with the same name replaces an earlier row.
    let secrets: BTreeMap<&str, &RosRow> =
        snap.ppp_secret.iter().map(|r| (f(r, "name"), r)).collect();
    let active: BTreeMap<&str, &RosRow> =
        snap.ppp_active.iter().map(|r| (f(r, "name"), r)).collect();
    let mut profiles: BTreeMap<&str, &RosRow> = BTreeMap::new();
    for p in &snap.ppp_profile {
        profiles.entry(f(p, "name")).or_insert(p);
    }
    let default_rate = parse_default_rate(&bd.pppoe_default_rate)
        .expect("config validation parses pppoe_default_rate");
    // One line for each profile, not for each user of the profile.
    if cfg.speed.pppoe_sources.contains(&PppSource::ProfileComment) {
        for (name, p) in &profiles {
            let at = format!("router {}: PPP profile {name}", router.name);
            comment_rate(speed, f(p, "comment"), &at, report);
        }
    }
    let mut no_secret = 0usize;
    for (&user, &act) in &active {
        if user.is_empty() {
            continue;
        }
        let Some(&sec) = secrets.get(user) else {
            no_secret += 1;
            continue;
        };
        if flag_on(sec, "disabled") || flag_on(sec, "inactive") {
            continue;
        }
        let profile = match f(sec, "profile") {
            "" => bd.pppoe_default_profile.as_str(),
            p => p,
        };
        if ex.profile(profile) || ex.ip(f(act, "address")) {
            continue;
        }
        let prof = profiles.get(profile).copied();
        let at = format!("router {}: PPP user {user}", router.name);
        let base = cfg
            .speed
            .pppoe_sources
            .iter()
            .find_map(|src| match src {
                PppSource::SecretComment => {
                    comment_rate(speed, f(sec, "comment"), &format!("{at} secret"), report)
                }
                PppSource::ActiveComment => {
                    comment_rate(speed, f(act, "comment"), &format!("{at} active"), report)
                }
                PppSource::ProfileComment => prof.and_then(|p| speed.comment(f(p, "comment"))),
                PppSource::ProfileRateLimit => {
                    prof.and_then(|p| speed.rate_limit(f(p, "rate-limit")))
                }
                PppSource::ProfileName => prof.and_then(|_| speed.profile_name(profile)),
                PppSource::Default => Some(default_rate),
            })
            .unwrap_or(default_rate);
        let mac = match f(act, "caller-id") {
            "" => f(sec, "caller-id"),
            m => m,
        };
        out.push(Collected {
            name: cfg
                .circuits
                .pppoe
                .replace("{user}", user)
                .replace("{router}", &router.name),
            device_name: user.to_string(),
            mac: mac.to_string(),
            ipv4: f(act, "address").to_string(),
            comment: cfg.circuits.comment_pppoe.clone(),
            source: Source::Ppp,
            unit: base,
            base,
            group: if router.pppoe.per_plan_node {
                GroupKey::PppPlan(profile.to_string())
            } else {
                GroupKey::Ppp
            },
        });
    }
    if no_secret > 0 {
        report.push(format!(
            "router {}: {no_secret} PPP active session(s) have no local secret and are not shaped",
            router.name
        ));
    }
}

fn collect_hotspot(
    cfg: &Config,
    router: &RouterCfg,
    snap: &RouterSnapshot,
    ex: &Excluder,
    out: &mut Vec<Collected>,
) {
    let base = router.hotspot.per_client(&cfg.bandwidth_defaults);
    for row in &snap.hotspot_active {
        let (mac, user) = (f(row, "mac-address"), f(row, "user"));
        let key = if router.hotspot.include_mac && !mac.is_empty() {
            mac_key(mac)
        } else {
            user.to_string()
        };
        if key.is_empty() || ex.ip(f(row, "address")) {
            continue;
        }
        out.push(Collected {
            name: cfg
                .circuits
                .hotspot
                .replace("{key}", &key)
                .replace("{router}", &router.name),
            device_name: if user.is_empty() {
                key.clone()
            } else {
                user.to_string()
            },
            mac: mac.to_string(),
            ipv4: f(row, "address").to_string(),
            comment: cfg.circuits.comment_hotspot.clone(),
            source: Source::Hotspot,
            unit: base,
            base,
            group: GroupKey::Hotspot,
        });
    }
}

/// Where the rate of a DHCP lease comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateSource {
    Override,
    ServerComment,
    LeaseComment,
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LeaseResult {
    Shaped {
        rate: Rate,
        source: RateSource,
        group: GroupKey,
        /// The group rate of one circuit (the server rate for a site).
        unit: Rate,
    },
    /// The reason, for the TUI.
    Skipped(String),
}

/// One DHCP lease and what the tool does with it. The cycle and the TUI use the same list.
#[derive(Debug, Clone, PartialEq)]
pub struct LeaseView {
    pub server: String,
    /// The MAC as the router gives it.
    pub mac: String,
    pub ip: String,
    pub host: String,
    pub comment: String,
    /// The key of the circuit name: the client name of a plan comment, else the host name.
    pub client: String,
    pub has_override: bool,
    pub result: LeaseResult,
}

/// The leases of one router, for the TUI. No report lines.
pub fn router_leases(
    cfg: &Config,
    router: &RouterCfg,
    snap: &RouterSnapshot,
) -> anyhow::Result<Vec<LeaseView>> {
    if !router.dhcp.enabled {
        return Ok(Vec::new());
    }
    let speed = SpeedParser::new(&cfg.speed)?;
    let ex = Excluder::new(cfg, router);
    Ok(dhcp_leases(cfg, router, snap, &speed, &ex, &mut Vec::new()))
}

/// Each lease with a MAC and an IP, and its result. The leases of the shaped servers come first,
/// in the server order and then the reply order (the order of the circuits), then the other leases.
fn dhcp_leases(
    cfg: &Config,
    router: &RouterCfg,
    snap: &RouterSnapshot,
    speed: &SpeedParser,
    ex: &Excluder,
    report: &mut Vec<String>,
) -> Vec<LeaseView> {
    let bd = &cfg.bandwidth_defaults;
    // Servers, as for PPP: all servers on the router (in reply order) and the listed ones.
    let disabled: Vec<&str> = snap
        .dhcp_server
        .iter()
        .filter(|r| flag_on(r, "disabled"))
        .map(|r| f(r, "name"))
        .collect();
    let mut all_names: Vec<String> = Vec::new();
    if router.dhcp.discover {
        all_names.extend(
            snap.dhcp_server
                .iter()
                .map(|r| f(r, "name").to_string())
                .filter(|n| !n.is_empty()),
        );
    }
    for s in &router.dhcp.servers {
        if !all_names.contains(&s.name) {
            all_names.push(s.name.clone());
        }
    }
    let names: Vec<&String> = all_names
        .iter()
        .filter(|n| !disabled.contains(&n.as_str()) && !ex.server(n))
        .collect();
    let mac_of = |lease: &RosRow| f(lease, "mac-address").to_string();
    let ip_of = |lease: &RosRow| match f(lease, "active-address") {
        "" => f(lease, "address").to_string(),
        a => a.to_string(),
    };
    let override_of = |mac: &str| {
        let m = normalize_mac(mac)?;
        router
            .dhcp
            .clients
            .iter()
            .find(|c| normalize_mac(&c.mac).as_deref() == Some(m.as_str()))
            .map(|c| Rate {
                down: c.download_mbps,
                up: c.upload_mbps,
            })
    };
    let view = |lease: &RosRow, client: &str, result: LeaseResult| LeaseView {
        server: f(lease, "server").to_string(),
        mac: mac_of(lease),
        ip: ip_of(lease),
        host: f(lease, "host-name").to_string(),
        comment: f(lease, "comment").to_string(),
        client: client.to_string(),
        has_override: override_of(&mac_of(lease)).is_some(),
        result,
    };
    let skip = |r: &str| LeaseResult::Skipped(r.to_string());
    let mut out = Vec::new();
    let mut seen = vec![false; snap.dhcp_lease.len()];
    for name in &names {
        let server = router.dhcp.server(name);
        let sources = server.sources(&cfg.speed);
        let server_comment = snap
            .dhcp_server
            .iter()
            .find(|r| f(r, "name") == server.name)
            .map_or("", |r| f(r, "comment"));
        // One line for each server, not for each lease.
        if sources.contains(&DhcpSource::ServerComment) {
            let at = format!("router {}: DHCP server {}", router.name, server.name);
            comment_rate(speed, server_comment, &at, report);
        }
        // The server-level rate: the first server-level source that gives a rate.
        let server_rate = |from: &DhcpSource| match from {
            DhcpSource::ServerComment => speed.comment(server_comment),
            DhcpSource::Settings => Some(server.settings_rate(bd)),
            DhcpSource::LeaseComment => None,
        };
        // The site rate of one circuit: it does not change with a client override.
        let unit = sources
            .iter()
            .find_map(server_rate)
            .unwrap_or_else(|| server.settings_rate(bd));
        for (i, lease) in snap.dhcp_lease.iter().enumerate() {
            let (mac, ip) = (mac_of(lease), ip_of(lease));
            if f(lease, "server") != server.name || mac.is_empty() || ip.is_empty() {
                continue;
            }
            seen[i] = true;
            let host = f(lease, "host-name");
            if ex.ip(&ip) {
                out.push(view(lease, host, skip("excluded IP")));
                continue;
            }
            if router.dhcp.require_bound && f(lease, "status") != "bound" {
                out.push(view(lease, host, skip("not bound")));
                continue;
            }
            let lease_comment = f(lease, "comment");
            let found = match override_of(&mac) {
                Some(r) => Some((r, RateSource::Override)),
                None => sources.iter().find_map(|src| match src {
                    DhcpSource::LeaseComment => comment_rate(
                        speed,
                        lease_comment,
                        &format!("router {}: DHCP lease {ip}", router.name),
                        report,
                    )
                    .map(|r| (r, RateSource::LeaseComment)),
                    DhcpSource::ServerComment => {
                        server_rate(src).map(|r| (r, RateSource::ServerComment))
                    }
                    DhcpSource::Settings => server_rate(src).map(|r| (r, RateSource::Settings)),
                }),
            };
            let Some((base, source)) = found else {
                report.push(format!(
                    "router {}: DHCP lease {ip} on server {} has no rate and is skipped",
                    router.name, server.name
                ));
                out.push(view(lease, host, skip("no rate")));
                continue;
            };
            let (client, group, unit) = match server.mode {
                DhcpMode::PerSite => (host, GroupKey::DhcpSite(server.name.clone()), unit),
                DhcpMode::PerPlan => {
                    let named = lease_comment
                        .split_once('|')
                        .map(|(n, _)| n.trim())
                        .filter(|n| !n.is_empty());
                    (
                        named.unwrap_or(host),
                        GroupKey::DhcpPlan(plan_label(base)),
                        base,
                    )
                }
            };
            out.push(view(
                lease,
                client,
                LeaseResult::Shaped {
                    rate: base,
                    source,
                    group,
                    unit,
                },
            ));
        }
    }
    // The other leases: no server, or a server that the tool does not shape.
    let mut no_server = 0;
    let default_rate = Rate {
        down: bd.dhcp_default_per_client_mbps,
        up: bd.dhcp_default_per_client_mbps,
    };
    for (i, lease) in snap.dhcp_lease.iter().enumerate() {
        let (mac, ip) = (mac_of(lease), ip_of(lease));
        if seen[i] || mac.is_empty() || ip.is_empty() {
            continue;
        }
        let srv = f(lease, "server");
        let host = f(lease, "host-name");
        if (srv.is_empty() || srv == "all") && !all_names.iter().any(|n| n == srv) {
            if !router.dhcp.shape_unassigned {
                no_server += 1;
                out.push(view(
                    lease,
                    host,
                    skip("no server (dhcp.shape_unassigned = false)"),
                ));
                continue;
            }
            if ex.ip(&ip) {
                out.push(view(lease, host, skip("excluded IP")));
                continue;
            }
            if router.dhcp.require_bound && f(lease, "status") != "bound" {
                out.push(view(lease, host, skip("not bound")));
                continue;
            }
            let (rate, source) = match override_of(&mac) {
                Some(r) => (r, RateSource::Override),
                None => match comment_rate(
                    speed,
                    f(lease, "comment"),
                    &format!("router {}: DHCP lease {ip}", router.name),
                    report,
                ) {
                    Some(r) => (r, RateSource::LeaseComment),
                    None => (default_rate, RateSource::Settings),
                },
            };
            out.push(view(
                lease,
                host,
                LeaseResult::Shaped {
                    rate,
                    source,
                    group: GroupKey::DhcpSite("unassigned".into()),
                    unit: default_rate,
                },
            ));
            continue;
        }
        let reason = if disabled.contains(&srv) {
            "disabled server"
        } else if ex.server(srv) {
            "excluded server"
        } else if router.dhcp.discover {
            "unknown server"
        } else {
            "server not listed (dhcp.discover = false)"
        };
        out.push(view(lease, host, skip(reason)));
    }
    if no_server > 0 {
        report.push(format!(
            "router {}: {no_server} DHCP leases have no server and are not shaped (routers.dhcp.shape_unassigned = false)",
            router.name
        ));
    }
    out
}

fn collect_dhcp(
    cfg: &Config,
    router: &RouterCfg,
    snap: &RouterSnapshot,
    speed: &SpeedParser,
    ex: &Excluder,
    report: &mut Vec<String>,
    out: &mut Vec<Collected>,
) {
    for l in dhcp_leases(cfg, router, snap, speed, ex, report) {
        let LeaseResult::Shaped {
            rate, group, unit, ..
        } = l.result
        else {
            continue;
        };
        let key = if l.client.is_empty() {
            mac_key(&l.mac)
        } else {
            l.client.clone()
        };
        out.push(Collected {
            name: cfg
                .circuits
                .dhcp
                .replace("{key}", &key)
                .replace("{router}", &router.name),
            device_name: key.clone(),
            mac: l.mac,
            ipv4: l.ip,
            comment: if l.client.is_empty() {
                cfg.circuits.comment_dhcp.clone()
            } else {
                l.client
            },
            source: Source::Dhcp,
            base: rate,
            group,
            unit,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::model::{RouterSnapshot, Source};
    use crate::speed::{Rate, SpeedParser};

    const ROUTER: &str = r#"
[[routers]]
name = "R1"
address = "192.0.2.1"
username = "u"
password_file = "/x"
[routers.pppoe]
enabled = true
per_plan_node = true
[routers.hotspot]
enabled = true
download_limit_mbps = 10
upload_limit_mbps = 5
[routers.dhcp]
enabled = true
require_bound = REQUIRE_BOUND
[[routers.dhcp.servers]]
name = "LAN"
default_plan_down_mbps = 50
default_plan_up_mbps = 50
[[routers.dhcp.servers]]
name = "IOT"
default_plan_down_mbps = 20
default_plan_up_mbps = 10
[[routers.dhcp.servers]]
name = "PLANS"
mode = "per_plan"
"#;

    fn cfg(require_bound: bool) -> Config {
        Config::from_toml_str(&ROUTER.replace("REQUIRE_BOUND", &require_bound.to_string())).unwrap()
    }
    fn snap(json: &str) -> RouterSnapshot {
        serde_json::from_str(json).unwrap()
    }
    fn run(c: &Config, s: &RouterSnapshot) -> (Vec<Collected>, Vec<String>) {
        let sp = SpeedParser::new(&c.speed).unwrap();
        let mut rep = Vec::new();
        let out = collect_router(c, &c.routers[0], s, &sp, &mut rep);
        (out, rep)
    }
    fn by_name<'a>(out: &'a [Collected], n: &str) -> &'a Collected {
        out.iter()
            .find(|x| x.name == n)
            .unwrap_or_else(|| panic!("no circuit {n}"))
    }
    fn rate(down: f64, up: f64) -> Rate {
        Rate { down, up }
    }

    #[test]
    fn ppp_needs_active_and_enabled_secret() {
        let s = snap(
            r#"{"ppp_secret":[{"name":"alice","profile":"P"},{"name":"bob","disabled":"true"},{"name":"carol","inactive":"true"}],
                "ppp_active":[{"name":"alice","address":"100.64.0.2"},{"name":"bob"},{"name":"carol"},{"name":"dave"}]}"#,
        );
        let (out, rep) = run(&cfg(false), &s);
        assert_eq!(
            out.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(),
            ["alice"]
        );
        assert!(
            rep.iter().any(|l| l.contains('1') && l.contains("secret")),
            "{rep:?}"
        );
        let a = by_name(&out, "alice");
        assert_eq!(
            (a.source, a.ipv4.as_str(), a.comment.as_str()),
            (Source::Ppp, "100.64.0.2", "PPP")
        );
        assert_eq!(a.group, GroupKey::PppPlan("P".into()));
    }

    #[test]
    fn ppp_speed_chain_order() {
        let s = snap(
            r#"{"ppp_secret":[
                  {"name":"u1","profile":"P","comment":"|30M/6M"},
                  {"name":"u2","profile":"plain"},
                  {"name":"u3","profile":"P"},
                  {"name":"u4","profile":"Q"},
                  {"name":"u5","profile":"Gold-12M"},
                  {"name":"u6","profile":"missing"}],
                "ppp_active":[{"name":"u1"},{"name":"u2","comment":"25M"},{"name":"u3"},{"name":"u4"},{"name":"u5"},{"name":"u6"}],
                "ppp_profile":[{"name":"P","comment":"20M/4M"},{"name":"Q","rate-limit":"2M/8M"},{"name":"Gold-12M"},{"name":"plain"}]}"#,
        );
        let (out, _) = run(&cfg(false), &s);
        assert_eq!(by_name(&out, "u1").base, rate(30.0, 6.0));
        assert_eq!(by_name(&out, "u2").base, rate(25.0, 25.0));
        assert_eq!(by_name(&out, "u3").base, rate(20.0, 4.0));
        assert_eq!(by_name(&out, "u4").base, rate(8.0, 2.0));
        assert_eq!(by_name(&out, "u5").base, rate(12.0, 12.0));
        assert_eq!(by_name(&out, "u6").base, rate(10.0, 10.0));
        // PPPoE output is in name order.
        let names: Vec<_> = out.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, ["u1", "u2", "u3", "u4", "u5", "u6"]);
    }

    #[test]
    fn zero_speed_comments_give_a_report_line() {
        let s = snap(
            r#"{"ppp_secret":[{"name":"z","profile":"P","comment":"|10M/0M"}],"ppp_active":[{"name":"z"}],
                "ppp_profile":[{"name":"P","comment":"20M/4M"}],
                "dhcp_server":[{"name":"LAN","comment":"0M/5M"}],
                "dhcp_lease":[{"server":"LAN","address":"10.0.0.5","mac-address":"AA:BB:CC:00:00:05"},
                              {"server":"LAN","address":"10.0.0.6","mac-address":"AA:BB:CC:00:00:06"},
                              {"server":"PLANS","address":"10.1.0.5","mac-address":"AA:BB:CC:00:01:05","comment":"Bob|0M/0M"}]}"#,
        );
        let (out, rep) = run(&cfg(false), &s);
        assert_eq!(by_name(&out, "z").base, rate(20.0, 4.0));
        let zero: Vec<_> = rep.iter().filter(|l| l.contains("zero speed")).collect();
        assert_eq!(
            zero.len(),
            3,
            "one line for each comment, not for each lease: {rep:?}"
        );
        assert!(zero.iter().any(|l| l.contains("PPP user z")), "{zero:?}");
        assert!(zero.iter().any(|l| l.contains("server LAN")), "{zero:?}");
        assert!(zero.iter().any(|l| l.contains("10.1.0.5")), "{zero:?}");
    }

    #[test]
    fn a_zero_speed_profile_comment_gives_one_line() {
        let s = snap(
            r#"{"ppp_secret":[{"name":"a","profile":"Z"},{"name":"b","profile":"Z"}],"ppp_active":[{"name":"a"},{"name":"b"}],
                "ppp_profile":[{"name":"Z","comment":"0M/0M"}]}"#,
        );
        let (_, rep) = run(&cfg(false), &s);
        let zero: Vec<_> = rep.iter().filter(|l| l.contains("zero speed")).collect();
        assert_eq!(zero.len(), 1, "{rep:?}");
        assert!(zero[0].contains("profile Z"), "{zero:?}");
    }

    #[test]
    fn ppp_mac_falls_back_to_secret_caller_id() {
        let s = snap(
            r#"{"ppp_secret":[{"name":"m","caller-id":"AA:BB:CC:00:00:01"}],"ppp_active":[{"name":"m","caller-id":""}]}"#,
        );
        let (out, _) = run(&cfg(false), &s);
        let m = by_name(&out, "m");
        assert_eq!(m.mac, "AA:BB:CC:00:00:01");
        assert_eq!(m.group, GroupKey::PppPlan("default".into()));
    }

    #[test]
    fn hotspot_key_rules() {
        let s = snap(
            r#"{"hotspot_active":[
                  {"user":"ann","mac-address":"aa:bb:cc:dd:ee:ff","address":"100.65.0.2"},
                  {"user":"guest","address":"100.65.0.3"},
                  {"address":"100.65.0.4"}]}"#,
        );
        let (out, _) = run(&cfg(false), &s);
        assert_eq!(out.len(), 2);
        let a = by_name(&out, "HS-AABBCCDDEEFF");
        assert_eq!(
            (a.device_name.as_str(), a.mac.as_str(), a.ipv4.as_str()),
            ("ann", "aa:bb:cc:dd:ee:ff", "100.65.0.2")
        );
        assert_eq!(
            (a.comment.as_str(), a.source, a.group.clone()),
            ("HS", Source::Hotspot, GroupKey::Hotspot)
        );
        assert_eq!(a.base, rate(10.0, 5.0));
        assert_eq!(by_name(&out, "HS-guest").device_name, "guest");
    }

    #[test]
    fn dhcp_per_site_filters_and_names() {
        let leases = r#"{"dhcp_lease":[
              {"server":"LAN","host-name":"pc1","mac-address":"02:00:00:00:02:01","address":"100.66.0.9","active-address":"100.66.0.2","status":"bound"},
              {"server":"LAN","mac-address":"02:00:00:00:02:0a","address":"100.66.0.3","status":"waiting"},
              {"server":"OTHER","host-name":"x","mac-address":"02:00:00:00:02:03","address":"100.66.0.4"},
              {"server":"LAN","host-name":"nomac","address":"100.66.0.5"},
              {"server":"LAN","host-name":"noaddr","mac-address":"02:00:00:00:02:06"}]}"#;
        let (out, _) = run(&cfg(false), &snap(leases));
        assert_eq!(out.len(), 2);
        let p = by_name(&out, "DHCP-pc1");
        assert_eq!(
            (p.device_name.as_str(), p.ipv4.as_str(), p.comment.as_str()),
            ("pc1", "100.66.0.2", "pc1")
        );
        assert_eq!(
            (p.source, p.group.clone(), p.base),
            (
                Source::Dhcp,
                GroupKey::DhcpSite("LAN".into()),
                rate(50.0, 50.0)
            )
        );
        let m = by_name(&out, "DHCP-02000000020A");
        assert_eq!(
            (m.device_name.as_str(), m.comment.as_str()),
            ("02000000020A", "DHCP")
        );
        let (out, _) = run(&cfg(true), &snap(leases));
        assert_eq!(
            out.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(),
            ["DHCP-pc1"]
        );
    }

    #[test]
    fn dhcp_server_comment_beats_settings() {
        let s = snap(
            r#"{"dhcp_server":[{"name":"LAN","comment":"Site|80M/40M"},{"name":"IOT"}],
                "dhcp_lease":[
                  {"server":"LAN","host-name":"a","mac-address":"02:00:00:00:03:01","address":"100.66.1.2"},
                  {"server":"LAN","host-name":"b","mac-address":"02:00:00:00:03:02","address":"100.66.1.3"},
                  {"server":"IOT","host-name":"c","mac-address":"02:00:00:00:03:03","address":"100.66.1.4"}]}"#,
        );
        let (out, _) = run(&cfg(false), &s);
        assert_eq!(by_name(&out, "DHCP-a").base, rate(80.0, 40.0));
        assert_eq!(by_name(&out, "DHCP-b").base, rate(80.0, 40.0));
        assert_eq!(by_name(&out, "DHCP-c").base, rate(20.0, 10.0));
    }

    #[test]
    fn dhcp_per_plan_uses_lease_comment() {
        let s = snap(
            r#"{"dhcp_lease":[
                  {"server":"PLANS","host-name":"h1","mac-address":"02:00:00:00:04:01","address":"100.66.2.2","comment":"Juan|25M/5M"},
                  {"server":"PLANS","host-name":"h2","mac-address":"02:00:00:00:04:02","address":"100.66.2.3","comment":"Plan 2.5M"},
                  {"server":"PLANS","host-name":"h3","mac-address":"02:00:00:00:04:03","address":"100.66.2.4","comment":"no plan"}]}"#,
        );
        let (out, rep) = run(&cfg(false), &s);
        assert_eq!(out.len(), 2);
        let j = by_name(&out, "DHCP-Juan");
        assert_eq!(
            (j.device_name.as_str(), j.comment.as_str()),
            ("Juan", "Juan")
        );
        assert_eq!(
            (j.base, j.group.clone()),
            (rate(25.0, 5.0), GroupKey::DhcpPlan("25M-5M".into()))
        );
        let h = by_name(&out, "DHCP-h2");
        assert_eq!(
            (h.base, h.group.clone()),
            (rate(2.5, 2.5), GroupKey::DhcpPlan("2M".into()))
        );
        assert_eq!(
            rep.iter().filter(|l| l.contains("100.66.2.4")).count(),
            1,
            "{rep:?}"
        );
    }

    #[test]
    fn disabled_sources_give_nothing() {
        let mut c = cfg(false);
        let r = &mut c.routers[0];
        r.pppoe.enabled = false;
        r.hotspot.enabled = false;
        r.dhcp.enabled = false;
        let s = snap(
            r#"{"ppp_secret":[{"name":"a"}],"ppp_active":[{"name":"a"}],"hotspot_active":[{"user":"g"}],
                "dhcp_lease":[{"server":"LAN","host-name":"x","mac-address":"02:00:00:00:05:01","address":"100.66.3.2"}]}"#,
        );
        assert!(run(&c, &s).0.is_empty());
    }

    #[test]
    fn plan_label_truncates() {
        assert_eq!(plan_label(rate(25.0, 5.0)), "25M-5M");
        assert_eq!(plan_label(rate(10.0, 10.0)), "10M");
        assert_eq!(plan_label(rate(2.5, 2.5)), "2M");
    }

    #[test]
    fn circuit_templates_can_use_router() {
        let mut c = cfg(false);
        c.circuits.pppoe = "{user}@{router}".into();
        let s = snap(r#"{"ppp_secret":[{"name":"alice"}],"ppp_active":[{"name":"alice"}]}"#);
        assert_eq!(run(&c, &s).0[0].name, "alice@R1");
    }

    fn cfg_text(extra_global: &str, extra_router: &str) -> Config {
        Config::from_toml_str(&format!(
            "{extra_global}\n[[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n\
             [routers.pppoe]\nenabled=true\n[routers.hotspot]\nenabled=true\n[routers.dhcp]\nenabled=true\n{extra_router}\n"
        ))
        .unwrap()
    }
    fn names_of(out: &[Collected]) -> Vec<&str> {
        out.iter().map(|x| x.name.as_str()).collect()
    }
    const DISCOVER: &str = r#"{"dhcp_server":[{"name":"LAN"},{"name":"GUEST"},{"name":"OFF","disabled":"true"},{"name":"MgtNet"}],
        "dhcp_lease":[
          {"server":"LAN","host-name":"a","mac-address":"02:00:00:00:07:01","address":"100.66.7.1"},
          {"server":"GUEST","host-name":"b","mac-address":"02:00:00:00:07:02","address":"100.66.7.2"},
          {"server":"OFF","host-name":"c","mac-address":"02:00:00:00:07:03","address":"100.66.7.3"},
          {"server":"MgtNet","host-name":"d","mac-address":"02:00:00:00:07:04","address":"100.66.7.4"}]}"#;

    const SITE: &str = r#"{"dhcp_server":[{"name":"LAN","comment":"Site|80M/40M"}],
        "dhcp_lease":[
          {"server":"LAN","host-name":"a","mac-address":"0A:00:00:00:03:01","address":"100.66.1.2"},
          {"server":"LAN","host-name":"b","mac-address":"0A:00:00:00:03:02","address":"100.66.1.3"}]}"#;
    const OVERRIDE_B: &str =
        "[[routers.dhcp.clients]]\nmac=\"0a-00-00-00-03-02\"\ndownload_mbps=50\nupload_mbps=20\n";

    #[test]
    fn an_override_changes_only_that_client() {
        let (out, _) = run(&cfg_text("", OVERRIDE_B), &snap(SITE));
        let (a, b) = (by_name(&out, "DHCP-a"), by_name(&out, "DHCP-b"));
        assert_eq!((a.base, b.base), (rate(80.0, 40.0), rate(50.0, 20.0)));
        assert_eq!(a.group, GroupKey::DhcpSite("LAN".into()));
        assert_eq!(b.group, GroupKey::DhcpSite("LAN".into()));
        assert_eq!((a.unit, b.unit), (rate(80.0, 40.0), rate(80.0, 40.0)));
        assert_eq!(b.mac, "0A:00:00:00:03:02");
    }

    #[test]
    fn an_override_in_per_plan_moves_to_its_plan_node() {
        let c = cfg_text(
            "",
            "[[routers.dhcp.servers]]\nname=\"PLANS\"\nmode=\"per_plan\"\n[[routers.dhcp.clients]]\nmac=\"02:00:00:00:04:01\"\ndownload_mbps=50\nupload_mbps=20\n",
        );
        let s = snap(
            r#"{"dhcp_lease":[{"server":"PLANS","host-name":"h1","mac-address":"02:00:00:00:04:01","address":"100.66.2.2","comment":"Juan|25M/5M"}]}"#,
        );
        let (out, _) = run(&c, &s);
        let j = by_name(&out, "DHCP-Juan");
        assert_eq!(
            (j.base, j.group.clone()),
            (rate(50.0, 20.0), GroupKey::DhcpPlan("50M-20M".into()))
        );
    }

    #[test]
    fn an_override_for_a_missing_mac_has_no_effect() {
        let c = cfg_text("", &OVERRIDE_B.replace("03-02", "09-09"));
        let (out, rep) = run(&c, &snap(SITE));
        assert_eq!(by_name(&out, "DHCP-b").base, rate(80.0, 40.0));
        assert!(rep.is_empty(), "{rep:?}");
    }

    #[test]
    fn an_override_on_a_skipped_lease_gives_no_circuit() {
        let c = cfg_text("[exclude]\nips=[\"100.66.1.3\"]", OVERRIDE_B);
        let (out, _) = run(&c, &snap(SITE));
        assert_eq!(names_of(&out), ["DHCP-a"]);
        let c = cfg_text("", &format!("require_bound=true\n{OVERRIDE_B}"));
        let s = snap(
            &SITE
                .replace(
                    "\"address\":\"100.66.1.3\"",
                    "\"address\":\"100.66.1.3\",\"status\":\"offered\"",
                )
                .replace(
                    "\"address\":\"100.66.1.2\"",
                    "\"address\":\"100.66.1.2\",\"status\":\"bound\"",
                ),
        );
        let (out, _) = run(&c, &s);
        assert_eq!(names_of(&out), ["DHCP-a"]);
    }

    const NO_SERVER: &str = r#"{"dhcp_lease":[
          {"server":"all","host-name":"s1","mac-address":"02:00:00:00:05:01","address":"100.66.5.1","comment":"30M/10M"},
          {"host-name":"s2","mac-address":"02:00:00:00:05:02","address":"100.66.5.2"}]}"#;

    #[test]
    fn no_server_leases_are_skipped_with_one_warning() {
        let (out, rep) = run(&cfg_text("", ""), &snap(NO_SERVER));
        assert!(out.is_empty());
        let w: Vec<&String> = rep
            .iter()
            .filter(|l| l.contains("have no server"))
            .collect();
        assert_eq!(w.len(), 1, "{rep:?}");
        assert!(
            w[0].contains("2 DHCP leases") && w[0].contains("shape_unassigned"),
            "{w:?}"
        );
    }

    #[test]
    fn shape_unassigned_puts_them_in_the_unassigned_node() {
        let (out, rep) = run(&cfg_text("", "shape_unassigned=true\n"), &snap(NO_SERVER));
        let (a, b) = (by_name(&out, "DHCP-s1"), by_name(&out, "DHCP-s2"));
        assert_eq!((a.base, b.base), (rate(30.0, 10.0), rate(15.0, 15.0)));
        assert_eq!(a.group, GroupKey::DhcpSite("unassigned".into()));
        assert_eq!((a.unit, b.unit), (rate(15.0, 15.0), rate(15.0, 15.0)));
        assert!(!rep.iter().any(|l| l.contains("have no server")), "{rep:?}");
    }

    #[test]
    fn dhcp_leases_gives_each_skip_reason() {
        let c = cfg_text(
            "[exclude]\ndhcp_servers=[\"Mgt*\"]\nips=[\"100.66.8.5\"]\n[speed]\ndhcp_per_site_sources=[\"server_comment\"]",
            "require_bound=true\n",
        );
        let s = snap(
            r#"{"dhcp_server":[{"name":"LAN"},{"name":"OFF","disabled":"true"},{"name":"MgtNet"}],
            "dhcp_lease":[
              {"server":"all","mac-address":"02:00:00:00:08:01","address":"100.66.8.1","status":"bound"},
              {"server":"OFF","mac-address":"02:00:00:00:08:02","address":"100.66.8.2","status":"bound"},
              {"server":"MgtNet","mac-address":"02:00:00:00:08:03","address":"100.66.8.3","status":"bound"},
              {"server":"OTHER","mac-address":"02:00:00:00:08:04","address":"100.66.8.4","status":"bound"},
              {"server":"LAN","mac-address":"02:00:00:00:08:05","address":"100.66.8.5","status":"bound"},
              {"server":"LAN","mac-address":"02:00:00:00:08:06","address":"100.66.8.6","status":"offered"},
              {"server":"LAN","mac-address":"02:00:00:00:08:07","address":"100.66.8.7","status":"bound"}]}"#,
        );
        let sp = SpeedParser::new(&c.speed).unwrap();
        let ex = Excluder::new(&c, &c.routers[0]);
        let mut rep = Vec::new();
        let v = dhcp_leases(&c, &c.routers[0], &s, &sp, &ex, &mut rep);
        let reason = |mac: &str| match &v.iter().find(|l| l.mac == mac).unwrap().result {
            LeaseResult::Skipped(r) => r.clone(),
            LeaseResult::Shaped { .. } => "shaped".into(),
        };
        assert!(reason("02:00:00:00:08:01").starts_with("no server"));
        assert_eq!(reason("02:00:00:00:08:02"), "disabled server");
        assert_eq!(reason("02:00:00:00:08:03"), "excluded server");
        assert_eq!(reason("02:00:00:00:08:04"), "unknown server");
        assert_eq!(reason("02:00:00:00:08:05"), "excluded IP");
        assert_eq!(reason("02:00:00:00:08:06"), "not bound");
        assert_eq!(reason("02:00:00:00:08:07"), "no rate");
        assert_eq!(v.len(), 7);
        let c = cfg_text("", "discover=false\n");
        let ex = Excluder::new(&c, &c.routers[0]);
        let v = dhcp_leases(&c, &c.routers[0], &snap(SITE), &sp, &ex, &mut rep);
        assert!(
            matches!(&v[0].result, LeaseResult::Skipped(r) if r.starts_with("server not listed"))
        );
    }

    #[test]
    fn dhcp_servers_are_discovered_like_ppp() {
        let c = cfg_text("", "");
        let (out, _) = run(&c, &snap(DISCOVER));
        assert_eq!(names_of(&out), ["DHCP-a", "DHCP-b", "DHCP-d"]);
        let b = by_name(&out, "DHCP-b");
        // No config entry for GUEST: the default rate applies (dhcp_default_per_client_mbps = 15).
        assert_eq!(
            (b.base, b.group.clone()),
            (rate(15.0, 15.0), GroupKey::DhcpSite("GUEST".into()))
        );
    }

    #[test]
    fn discover_off_uses_only_listed_servers() {
        let c = cfg_text(
            "",
            "discover=false\n[[routers.dhcp.servers]]\nname=\"GUEST\"\n",
        );
        assert_eq!(names_of(&run(&c, &snap(DISCOVER)).0), ["DHCP-b"]);
    }

    #[test]
    fn excluded_dhcp_server_names_use_wildcards() {
        let c = cfg_text(
            "",
            "[routers.exclude]\ndhcp_servers=[\"mgt*\", \"GUEST\"]\n",
        );
        assert_eq!(names_of(&run(&c, &snap(DISCOVER)).0), ["DHCP-a"]);
    }

    #[test]
    fn excluded_ppp_profiles_are_skipped() {
        let c = cfg_text("[exclude]\nppp_profiles=[\"default-*\"]", "");
        let s = snap(
            r#"{"ppp_secret":[{"name":"a","profile":"default-encryption"},{"name":"b","profile":"Gold"}],
                "ppp_active":[{"name":"a"},{"name":"b"}]}"#,
        );
        assert_eq!(names_of(&run(&c, &s).0), ["b"]);
    }

    #[test]
    fn excluded_ips_apply_to_all_sources_and_merge_router_lists() {
        let c = cfg_text(
            "[exclude]\nips=[\"100.64.0.0/30\"]",
            "[routers.exclude]\nips=[\"100.66.7.2\"]\n",
        );
        let s = snap(
            r#"{"ppp_secret":[{"name":"p1"},{"name":"p2"}],
                "ppp_active":[{"name":"p1","address":"100.64.0.2"},{"name":"p2","address":"100.64.0.5"}],
                "hotspot_active":[{"user":"h1","address":"100.64.0.1"}],
                "dhcp_server":[{"name":"LAN"},{"name":"GUEST"}],
                "dhcp_lease":[
                  {"server":"LAN","host-name":"a","mac-address":"02:00:00:00:07:01","address":"100.66.7.1"},
                  {"server":"GUEST","host-name":"b","mac-address":"02:00:00:00:07:02","address":"100.66.7.2"}]}"#,
        );
        assert_eq!(names_of(&run(&c, &s).0), ["p2", "DHCP-a"]);
    }

    #[test]
    fn only_true_or_yes_turns_a_flag_on() {
        let s = snap(
            r#"{"ppp_secret":[{"name":"a","disabled":"no"},{"name":"b","disabled":"yes"},{"name":"c","inactive":"TRUE"},{"name":"d","disabled":"false"}],
                "ppp_active":[{"name":"a"},{"name":"b"},{"name":"c"},{"name":"d"}]}"#,
        );
        let (out, _) = run(&cfg(false), &s);
        assert_eq!(
            out.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(),
            ["a", "d"]
        );
    }
}
