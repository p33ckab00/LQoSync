//! Small in-memory health receipts shared by the service, notifications, and TUI cache.
use anyhow::ensure;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};
pub const CACHE: &str = "system-health.json";
pub const MAX_BYTES: usize = 128 * 1024;
const MAX_COMPONENTS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    Service,
    Collection,
    Write,
    Apply,
    Dataplane,
    Telemetry,
    Notification,
}
impl Domain {
    pub const ALL: [Self; 7] = [
        Self::Service,
        Self::Collection,
        Self::Write,
        Self::Apply,
        Self::Dataplane,
        Self::Telemetry,
        Self::Notification,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Service => "service_config",
            Self::Collection => "collection",
            Self::Write => "validation_write",
            Self::Apply => "apply",
            Self::Dataplane => "dataplane",
            Self::Telemetry => "telemetry",
            Self::Notification => "notification",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum State {
    Healthy,
    Updating,
    Degraded,
    Failed,
    Unknown,
    Warming,
    NotApplicable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Phase {
    Intent,
    Pending,
    ImportConfirmed,
    Applied,
    WriteFailed,
    ApplyFailed,
    VerifyTimeout,
    Superseded,
    Interrupted,
    Candidate,
}
impl Phase {
    pub fn pending(self) -> bool {
        matches!(
            self,
            Self::Intent | Self::Pending | Self::ImportConfirmed | Self::Interrupted
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Update {
    pub write_id: String,
    pub phase: Phase,
    pub observed_at: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub domain: Domain,
    pub component: String,
    pub state: State,
    pub reason: String,
    pub observed_at: u64,
    pub max_age: u64,
    pub source_epoch: String,
    pub last_success: Option<u64>,
}
fn text(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}
impl Observation {
    fn valid(&self) -> bool {
        text(&self.component, 128) && text(&self.reason, 96) && text(&self.source_epoch, 128)
    }
    fn at(&self, now: u64) -> Self {
        let mut value = self.clone();
        if self.state != State::NotApplicable {
            if now < self.observed_at {
                value.state = State::Unknown;
                value.reason = "observation_in_future".into();
            } else if self.observed_at == 0 || now - self.observed_at > self.max_age {
                value.state = State::Unknown;
                value.reason = "source_stale".into();
            }
        }
        value
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub version: u32,
    pub updated: u64,
    pub overall: State,
    pub update: Option<Update>,
    pub components: Vec<Observation>,
    #[serde(default)]
    pub bridge: Option<crate::worker_util::health::Health>,
    #[serde(default)]
    pub acceptance: Option<crate::apply_evidence::Acceptance>,
}
#[derive(Default)]
struct Store {
    observations: BTreeMap<(Domain, String), Observation>,
    fault: Option<&'static str>,
    update: Option<Update>,
    bridge: Option<crate::worker_util::health::Health>,
    acceptance: Option<crate::apply_evidence::Acceptance>,
}
#[derive(Clone, Default)]
pub struct Hub(Arc<Mutex<Store>>);
impl Hub {
    pub(crate) fn set_acceptance(&self, proof: Option<crate::apply_evidence::Acceptance>) {
        self.0.lock().unwrap_or_else(|p| p.into_inner()).acceptance = proof;
    }
    pub(crate) fn set_bridge(&self, bridge: Option<crate::worker_util::health::Health>) {
        self.0.lock().unwrap_or_else(|p| p.into_inner()).bridge = bridge;
    }
    pub fn observe(&self, mut receipt: Observation) -> anyhow::Result<()> {
        let mut store = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if !receipt.valid() {
            store.fault = Some("invalid_health_observation");
            anyhow::bail!("invalid health observation")
        }
        let key = (receipt.domain, receipt.component.clone());
        if !store.observations.contains_key(&key) && store.observations.len() >= MAX_COMPONENTS {
            store.fault = Some("health_component_capacity");
            anyhow::bail!("health component capacity exceeded")
        }
        receipt.last_success = if receipt.state == State::Healthy {
            Some(receipt.observed_at)
        } else {
            store
                .observations
                .get(&key)
                .filter(|old| old.source_epoch == receipt.source_epoch)
                .and_then(|old| old.last_success)
        };
        store.observations.insert(key, receipt);
        Ok(())
    }
    pub fn set_update(&self, update: Option<Update>) -> anyhow::Result<()> {
        ensure!(
            update.as_ref().is_none_or(|u| text(&u.write_id, 128)),
            "invalid health write identity"
        );
        self.0.lock().unwrap_or_else(|p| p.into_inner()).update = update;
        Ok(())
    }
    pub fn remove(&self, domain: Domain, component: &str) {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .observations
            .remove(&(domain, component.into()));
    }
    pub fn clear(&self, domain: Domain) {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .observations
            .retain(|(d, _), _| *d != domain);
    }
    pub fn snapshot(&self, now: u64, update: Option<Update>) -> Report {
        let store = self.0.lock().unwrap_or_else(|p| p.into_inner());
        let update = update.or_else(|| store.update.clone());
        let mut components: Vec<_> = store.observations.values().map(|o| o.at(now)).collect();
        for domain in Domain::ALL {
            if !components.iter().any(|o| o.domain == domain) {
                components.push(Observation {
                    domain,
                    component: domain.label().into(),
                    state: State::Unknown,
                    reason: "no_observation".into(),
                    observed_at: 0,
                    max_age: 0,
                    source_epoch: "unknown".into(),
                    last_success: None,
                });
            }
        }
        if let Some(reason) = store.fault {
            components.push(Observation {
                domain: Domain::Service,
                component: "health_store".into(),
                state: State::Degraded,
                reason: reason.into(),
                observed_at: now,
                max_age: 0,
                source_epoch: "service".into(),
                last_success: None,
            });
        }
        components.sort_by(|a, b| (a.domain, &a.component).cmp(&(b.domain, &b.component)));
        let overall = overall(&components, update.as_ref());
        let updated = components.iter().map(|c| c.observed_at).max().unwrap_or(0);
        Report {
            version: 1,
            updated,
            overall,
            update,
            components,
            bridge: store.bridge.clone(),
            acceptance: store.acceptance.clone(),
        }
    }
}
fn overall(components: &[Observation], update: Option<&Update>) -> State {
    [
        State::Failed,
        State::Degraded,
        State::Unknown,
        State::Warming,
    ]
    .into_iter()
    .find(|s| components.iter().any(|o| o.state == *s))
    .unwrap_or_else(|| {
        if components.iter().any(|c| c.state == State::Updating)
            || update.as_ref().is_some_and(|u| u.phase.pending())
        {
            State::Updating
        } else if update.as_ref().is_some_and(|u| {
            matches!(
                u.phase,
                Phase::WriteFailed | Phase::ApplyFailed | Phase::VerifyTimeout
            )
        }) {
            State::Degraded
        } else {
            State::Healthy
        }
    })
}
impl Report {
    pub fn at(&self, now: u64) -> Self {
        let mut report = self.clone();
        report.components = self.components.iter().map(|o| o.at(now)).collect();
        report.overall = overall(&report.components, report.update.as_ref());
        report
    }
    pub fn read(path: &Path) -> anyhow::Result<Self> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(bytes.len() <= MAX_BYTES, "health cache exceeds its budget");
        let report: Self = serde_json::from_slice(&bytes)?;
        let mut identities = std::collections::HashSet::new();
        ensure!(
            report.version == 1
                && report.components.len() <= MAX_COMPONENTS + 8
                && report
                    .components
                    .iter()
                    .all(|c| c.valid() && identities.insert((c.domain, c.component.as_str())))
                && Domain::ALL
                    .iter()
                    .all(|d| report.components.iter().any(|c| c.domain == *d))
                && report
                    .update
                    .as_ref()
                    .is_none_or(|u| text(&u.write_id, 128))
                && report.bridge.as_ref().is_none_or(|b| b.valid()),
            "invalid health cache"
        );
        Ok(report)
    }
}

/// One cache writer per service. Do not fsync a replaceable health cache.
#[derive(Default)]
pub struct Publisher {
    last: Vec<u8>,
    directory: Option<std::path::PathBuf>,
}
impl Publisher {
    pub fn publish(&mut self, dir: &Path, report: &Report) -> anyhow::Result<bool> {
        use std::os::unix::fs::PermissionsExt;
        let bytes = serde_json::to_vec(report)?;
        ensure!(bytes.len() <= MAX_BYTES, "health cache exceeds its budget");
        if bytes == self.last && self.directory.as_deref() == Some(dir) {
            return Ok(false);
        }
        let tmp = dir.join(format!(".{CACHE}.tmp-{}", std::process::id()));
        std::fs::write(&tmp, &bytes)?;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644))?;
        std::fs::rename(&tmp, dir.join(CACHE))?;
        self.last = bytes;
        self.directory = Some(dir.to_path_buf());
        Ok(true)
    }
}
