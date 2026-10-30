//! Read-only WAN health preview. No result authorizes RouterOS mutations.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::net::Ipv4Addr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub interval_seconds: u64,
    pub evidence_seconds: u64,
    pub failure_rounds: u32,
    pub success_rounds: u32,
    pub recovery_seconds: u64,
    pub monitors: Vec<Monitor>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            interval_seconds: 10,
            evidence_seconds: 30,
            failure_rounds: 3,
            success_rounds: 3,
            recovery_seconds: 30,
            monitors: vec![],
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Monitor {
    pub wan: String,
    pub probes: [Probe; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Probe {
    pub id: String,
    pub target: Ipv4Addr,
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1..=300).contains(&self.interval_seconds),
            "Health interval must be 1–300 seconds"
        );
        ensure!(
            (self.interval_seconds..=300).contains(&self.evidence_seconds),
            "Health evidence lifetime must cover one interval and be at most 300 seconds"
        );
        ensure!(
            (1..=100).contains(&self.failure_rounds) && (1..=100).contains(&self.success_rounds),
            "Health thresholds must be 1–100 rounds"
        );
        ensure!(
            self.recovery_seconds <= 3600,
            "Recovery delay must be at most 3600 seconds"
        );
        ensure!(
            self.monitors.len() <= 64,
            "At most 64 WAN monitors are supported"
        );
        let mut wans = BTreeSet::new();
        let mut probes = BTreeSet::new();
        let mut targets = BTreeSet::new();
        for m in &self.monitors {
            ensure!(
                crate::model::valid_id(&m.wan) && wans.insert(&m.wan),
                "Invalid or duplicate monitored WAN"
            );
            for p in &m.probes {
                ensure!(
                    p.id.strip_prefix('*').is_some_and(|s| !s.is_empty()
                        && s.len() <= 32
                        && s.bytes().all(|b| b.is_ascii_hexdigit())),
                    "Probe ID must be a RouterOS internal ID"
                );
                ensure!(probes.insert((&m.wan, &p.id)), "Duplicate probe ID");
                let oct = p.target.octets();
                ensure!(
                    !p.target.is_private()
                        && !p.target.is_loopback()
                        && !p.target.is_link_local()
                        && !p.target.is_multicast()
                        && !p.target.is_broadcast()
                        && oct[0] != 0
                        && oct[0] < 224
                        && !(oct[0] == 100 && (64..=127).contains(&oct[1])),
                    "Probe target must be an external unicast IPv4 address"
                );
                ensure!(
                    targets.insert(p.target),
                    "Probe targets must be globally distinct across configured WANs"
                );
            }
        }
        Ok(())
    }
    pub fn validate_settings(&self, settings: &crate::model::Settings) -> Result<()> {
        self.validate()?;
        if self.monitors.is_empty() {
            return Ok(());
        }
        let dynamic = settings
            .dynamic
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WAN health monitors require dynamic topology"))?;
        let mut keys = BTreeSet::new();
        for m in &self.monitors {
            let w = dynamic
                .wans
                .iter()
                .find(|w| w.id == m.wan)
                .ok_or_else(|| anyhow::anyhow!("Health monitor references an unknown WAN"))?;
            let r = settings
                .routers
                .iter()
                .find(|r| r.id == w.router)
                .ok_or_else(|| anyhow::anyhow!("Health WAN has no router"))?;
            ensure!(
                matches!(r.role.as_str(), "core" | "both"),
                "Health monitor requires a core router"
            );
            for p in &m.probes {
                ensure!(
                    keys.insert((&w.router, &p.id)),
                    "A probe cannot monitor two WANs on the same core"
                );
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}
/// Provider context must bind policy, router identity, and verified probe paths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    #[serde(default)]
    pub settings_revision: String,
    #[serde(default)]
    pub wan: String,
    pub context: String,
    pub observed_at: u64,
    pub router_uptime: u64,
    pub counters: [u64; 2],
    pub passed: [bool; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub status: HealthStatus,
    pub reason: String,
    pub checked_at: u64,
    #[serde(default)]
    pub settings_revision: String,
    #[serde(default)]
    pub wan: String,
    #[serde(default)]
    evidence_seconds: u64,
    context: String,
    counters: [u64; 2],
    router_uptime: u64,
    last_sample_at: u64,
    successes: u32,
    failures: u32,
    recovering_since: Option<u64>,
}
fn baseline(e: &Evidence, now: u64, reason: &str, config: &Config) -> State {
    State {
        status: HealthStatus::Unknown,
        reason: reason.into(),
        checked_at: now,
        settings_revision: e.settings_revision.clone(),
        wan: e.wan.clone(),
        evidence_seconds: config.evidence_seconds,
        context: e.context.clone(),
        counters: e.counters,
        router_uptime: e.router_uptime,
        last_sample_at: e.observed_at,
        successes: 0,
        failures: 0,
        recovering_since: None,
    }
}
pub fn evaluate(
    config: &Config,
    previous: Option<&State>,
    evidence: std::result::Result<Evidence, String>,
    now: u64,
) -> Result<State> {
    config.validate()?;
    let mut e = match evidence {
        Ok(e) => e,
        Err(reason) => {
            return Ok(baseline(
                &Evidence {
                    settings_revision: String::new(),
                    wan: String::new(),
                    context: String::new(),
                    observed_at: now,
                    router_uptime: 0,
                    counters: [0; 2],
                    passed: [false; 2],
                },
                now,
                &reason,
                config,
            ));
        }
    };
    if e.context.is_empty() || e.observed_at > now || now - e.observed_at > config.evidence_seconds
    {
        let mut s = baseline(&e, now, "Invalid, future-dated or stale evidence", config);
        s.context.clear();
        return Ok(s);
    }
    e.context = uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_OID,
        &serde_json::to_vec(&(config, &e.context, &e.settings_revision, &e.wan))?,
    )
    .to_string();
    let Some(old) = previous else {
        return Ok(baseline(
            &e,
            now,
            "Waiting for new completed probe rounds",
            config,
        ));
    };
    if old.context != e.context
        || now < old.checked_at
        || e.router_uptime < old.router_uptime
        || e.observed_at < old.last_sample_at
        || e.counters
            .iter()
            .zip(old.counters)
            .any(|(new, old)| *new < old)
    {
        return Ok(baseline(
            &e,
            now,
            "Evidence context changed, clock reversed, or router/probe restarted",
            config,
        ));
    }
    if now.saturating_sub(old.last_sample_at) > config.evidence_seconds {
        return Ok(baseline(
            &e,
            now,
            "Probe progress is stale; new baseline required",
            config,
        ));
    }
    let mut s = old.clone();
    s.checked_at = now;
    s.router_uptime = e.router_uptime;
    if e.counters
        .iter()
        .zip(old.counters)
        .any(|(new, old)| *new == old)
        || e.observed_at.saturating_sub(old.last_sample_at) < config.interval_seconds
    {
        s.reason = "Waiting for both probes to complete a new round".into();
        return Ok(s);
    }
    s.counters = e.counters;
    s.last_sample_at = e.observed_at;
    if e.passed.into_iter().any(|v| v) {
        s.failures = 0;
        s.successes = s.successes.saturating_add(1).min(config.success_rounds);
        let since = *s.recovering_since.get_or_insert(e.observed_at);
        if s.successes >= config.success_rounds
            && e.observed_at.saturating_sub(since) >= config.recovery_seconds
        {
            s.status = HealthStatus::Healthy;
            s.reason = "Fresh completed probe rounds passed".into();
        } else {
            s.reason = "Recovery thresholds or delay not yet satisfied".into();
        }
    } else {
        s.successes = 0;
        s.recovering_since = None;
        s.failures = s.failures.saturating_add(1).min(config.failure_rounds);
        if s.failures >= config.failure_rounds {
            s.status = HealthStatus::Unhealthy;
            s.reason = "Both attributed targets failed for the configured rounds".into();
        } else {
            s.reason = "Failure threshold not yet satisfied".into();
        }
    }
    Ok(s)
}

pub const NETWATCH: &str = "/tool/netwatch/print";
/// Conservative IPv4/main-table provider. Unsupported attribution returns an error/Unknown.
pub fn from_inventory(
    config: &Config,
    monitor: &Monitor,
    inventory: &crate::routeros::Inventory,
    interface: &str,
    policy_revision: &str,
) -> Result<Evidence> {
    use crate::routeros::Row;
    use anyhow::Context;
    config.validate()?;
    ensure!(inventory.errors.is_empty(), "Incomplete health inventory");
    ensure!(
        !policy_revision.is_empty() && !interface.is_empty(),
        "Health context is missing"
    );
    let table = |name: &str| {
        inventory
            .tables
            .get(name)
            .with_context(|| format!("Missing health table: {name}"))
    };
    let flag = |r: &Row, key: &str| -> Result<bool> {
        match r.get(key).map(String::as_str) {
            None | Some("false") => Ok(false),
            Some("true") => Ok(true),
            _ => anyhow::bail!("Unknown health boolean"),
        }
    };
    let resources = table("/system/resource/print")?;
    ensure!(resources.len() == 1, "Ambiguous router resource evidence");
    ensure!(
        resources[0]
            .get("version")
            .is_some_and(|s| s.starts_with("7.")),
        "Health provider requires RouterOS 7"
    );
    let uptime = crate::sessions::uptime(
        resources[0]
            .get("uptime")
            .context("Missing router uptime")?,
    )?;
    let interfaces = table("/interface/print")?
        .iter()
        .filter(|r| r.get("name").is_some_and(|n| n == interface))
        .collect::<Vec<_>>();
    ensure!(
        interfaces.len() == 1
            && !flag(interfaces[0], "disabled")?
            && flag(interfaces[0], "running")?,
        "WAN interface is not uniquely enabled and running"
    );
    for r in table("/routing/rule/print")? {
        ensure!(
            flag(r, "disabled")?,
            "Policy routing prevents verified probe attribution"
        );
    }
    for r in table("/ip/firewall/mangle/print")? {
        ensure!(
            flag(r, "disabled")?
                || r.get("chain").is_some_and(|c| matches!(
                    c.as_str(),
                    "prerouting" | "forward" | "postrouting" | "input"
                )),
            "Output or unknown mangle chain prevents verified probe attribution"
        );
    }
    let rows = table(NETWATCH)?;
    let routes = table("/ip/route/print")?;
    let mut counters = [0; 2];
    let mut passed = [false; 2];
    let mut bound_routes = vec![];
    for (i, p) in monitor.probes.iter().enumerate() {
        let probes = rows
            .iter()
            .filter(|r| r.get(".id") == Some(&p.id))
            .collect::<Vec<_>>();
        ensure!(
            probes.len() == 1,
            "Configured probe is absent or duplicated"
        );
        let r = probes[0];
        ensure!(
            r.get("host") == Some(&p.target.to_string())
                && r.get("type").is_some_and(|s| s == "icmp")
                && !flag(r, "disabled")?,
            "Probe target/type changed or probe disabled"
        );
        for key in ["src-address", "up-script", "down-script", "test-script"] {
            ensure!(
                r.get(key).is_none_or(String::is_empty),
                "Probe source binding or scripts require separate review"
            );
        }
        let interval =
            crate::sessions::uptime(r.get("interval").context("Missing probe interval")?)?;
        let timeout = crate::sessions::uptime(r.get("timeout").context("Missing probe timeout")?)?;
        ensure!(
            interval > 0
                && interval <= config.evidence_seconds
                && timeout > 0
                && timeout <= interval,
            "Probe timing is outside the supported evidence window"
        );
        counters[i] = r
            .get("done-tests")
            .context("Missing completed probe counter")?
            .parse()?;
        ensure!(counters[i] > 0, "No completed probe evidence");
        passed[i] = match r.get("status").map(String::as_str) {
            Some("up") => true,
            Some("down") => false,
            _ => anyhow::bail!("Probe status is unknown"),
        };
        let destination = format!("{}/32", p.target);
        let mut host_routes = vec![];
        for route in routes {
            if route.get("routing-table").is_some_and(|s| s == "main")
                && route.get("dst-address") == Some(&destination)
                && !flag(route, "disabled")?
            {
                host_routes.push(route);
            }
        }
        ensure!(
            host_routes.len() == 1,
            "Probe needs a unique dedicated main-table host route"
        );
        let route = host_routes[0];
        ensure!(
            flag(route, "active")?
                && !flag(route, "dynamic")?
                && !flag(route, "blackhole")?
                && !flag(route, "unreachable")?
                && !flag(route, "prohibit")?,
            "Probe route is not an active static unicast route"
        );
        let gateway = route
            .get("immediate-gw")
            .context("Probe route has no resolved gateway")?;
        ensure!(
            !gateway.contains(',')
                && gateway
                    .rsplit_once('%')
                    .is_some_and(|(ip, name)| ip.parse::<Ipv4Addr>().is_ok() && name == interface),
            "Probe route does not uniquely resolve through the mapped WAN"
        );
        bound_routes.push((destination, gateway.clone()));
    }
    let context = uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_OID,
        &serde_json::to_vec(&(
            config,
            monitor,
            &inventory.router_id,
            interface,
            policy_revision,
            bound_routes,
        ))?,
    )
    .to_string();
    Ok(Evidence {
        settings_revision: policy_revision.into(),
        wan: monitor.wan.clone(),
        context,
        observed_at: inventory.collected_at,
        router_uptime: uptime,
        counters,
        passed,
    })
}

/// Canonical settings identity shared by preview and reconciliation.
pub fn settings_revision(settings: &crate::model::Settings) -> Result<String> {
    Ok(uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &serde_json::to_vec(settings)?).to_string())
}
impl State {
    /// A later read must not extend the lifetime of the last completed sample.
    pub(crate) fn verification_deadline(&self, maximum_age: u64) -> u64 {
        self.checked_at
            .min(self.last_sample_at)
            .saturating_add(maximum_age.min(self.evidence_seconds))
    }
    pub fn status_at(&self, now: u64, maximum_age: u64) -> HealthStatus {
        let ttl = maximum_age.min(self.evidence_seconds);
        if ttl == 0
            || now < self.checked_at
            || now < self.last_sample_at
            || now - self.checked_at > ttl
            || now - self.last_sample_at > ttl
        {
            HealthStatus::Unknown
        } else {
            self.status
        }
    }
}
