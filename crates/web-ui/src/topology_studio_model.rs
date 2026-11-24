//! Router-scoped Studio drafts. Secrets stay outside this model.
use web_contract::source_studio::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StudioSection {
    Connection,
    Access,
    Sources,
}

pub(crate) fn section_reason(
    section: StudioSection,
    has_router: bool,
    caps: Option<&StudioCapabilities>,
    config: Option<&RouterSourceConfig>,
) -> Option<&'static str> {
    if !has_router {
        return Some("Select a registered router.");
    }
    if section == StudioSection::Connection {
        return None;
    }
    let Some(caps) = caps.filter(|c| c.version == VERSION) else {
        return Some(
            "This backend does not support Studio version 1. Connection editing remains available.",
        );
    };
    if config.is_none_or(|c| c.version != VERSION) {
        return Some(
            "Source configuration is unavailable for this router. Connection editing remains available.",
        );
    }
    if section == StudioSection::Access && !caps.features.contains(&StudioFeature::AccessProfiles) {
        return Some("Access profiles are unavailable on this backend.");
    }
    if section == StudioSection::Sources && caps.fields.is_empty() {
        return Some("The backend advertises no query fields.");
    }
    None
}

pub(crate) fn key<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}
pub(crate) fn fields(
    caps: &StudioCapabilities,
    source: SourceKind,
    groupable: bool,
) -> Vec<FieldKey> {
    caps.fields
        .iter()
        .filter(|f| f.source == source && (!groupable || f.groupable))
        .map(|f| f.field)
        .collect()
}
pub(crate) fn operators(
    caps: &StudioCapabilities,
    source: SourceKind,
    field: FieldKey,
) -> Vec<FilterOp> {
    caps.fields
        .iter()
        .find(|f| f.source == source && f.field == field)
        .map(|f| f.operators.clone())
        .unwrap_or_default()
}
pub(crate) const SOURCES: [(SourceKind, &str); 4] = [
    (SourceKind::Ppp, "PPP / PPPoE"),
    (SourceKind::Dhcp, "DHCP / IPoE"),
    (SourceKind::Hotspot, "Hotspot"),
    (SourceKind::Wan, "WAN"),
];
pub(crate) fn source_label(kind: SourceKind) -> &'static str {
    SOURCES.iter().find(|(k, _)| *k == kind).unwrap().1
}
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct PolicyDraft {
    pub local_token: u64,
    pub policy: SourcePolicyDraft,
    pub priority: String,
}
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct ProfileDraft {
    pub needs_confirmation: bool,
    pub id: String,
    pub label: String,
    pub username: String,
    pub credential_set: bool,
    pub supported_writes: Vec<WriteScope>,
}
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct StudioDraft {
    pub router_id: String,
    pub config_digest: String,
    pub loaded_config_digest: String,
    pub policy_revision: Option<String>,
    pub policies: Vec<PolicyDraft>,
    pub profiles: Vec<ProfileDraft>,
    pub saved_profiles: Vec<ProfileDraft>,
    pub saved_policies: Vec<PolicyDraft>,
    pub next_policy: u64,
    pub selected: Option<usize>,
    pub epoch: u64,
}
impl From<RouterSourceConfig> for StudioDraft {
    fn from(c: RouterSourceConfig) -> Self {
        let profiles: Vec<_> = c
            .access_profiles
            .into_iter()
            .map(|p| ProfileDraft {
                needs_confirmation: false,
                id: p.id,
                label: p.label,
                username: String::new(),
                credential_set: p.credential_set,
                supported_writes: p.supported_writes,
            })
            .collect();
        let policies: Vec<_> = c
            .policies
            .into_iter()
            .enumerate()
            .map(|(i, p)| PolicyDraft {
                local_token: i as u64 + 1,
                priority: p.priority.to_string(),
                policy: p,
            })
            .collect();
        Self {
            router_id: c.router_id,
            config_digest: c.config_digest,
            loaded_config_digest: c.loaded_config_digest,
            policy_revision: c.policy_revision,
            selected: (!policies.is_empty()).then_some(0),
            epoch: 0,
            next_policy: policies.len() as u64 + 1,
            saved_policies: policies.clone(),
            policies,
            profiles: profiles.clone(),
            saved_profiles: profiles,
        }
    }
}
impl StudioDraft {
    pub fn current(&self) -> Option<&PolicyDraft> {
        self.selected.and_then(|i| self.policies.get(i))
    }
    pub fn edit(&mut self, f: impl FnOnce(&mut PolicyDraft)) {
        if let Some(p) = self.selected.and_then(|i| self.policies.get_mut(i)) {
            f(p);
            self.epoch = self.epoch.wrapping_add(1);
        }
    }
    pub fn select_source(&mut self, source: SourceKind, caps: &StudioCapabilities) {
        if let Some(i) = self.policies.iter().position(|p| p.policy.source == source) {
            self.selected = Some(i);
        } else {
            self.add_policy(source, caps);
        }
    }
    pub fn add_policy(&mut self, source: SourceKind, caps: &StudioCapabilities) -> bool {
        if self.policies.len() >= caps.limits.policies_per_router.min(MAX_POLICIES as u64) as usize
        {
            return false;
        }
        let group = fields(caps, source, true)
            .first()
            .map(|field| GroupRule::ByField { field: *field })
            .unwrap_or(GroupRule::Fixed {
                name: source_label(source).into(),
            });
        let policy = SourcePolicyDraft {
            id: None,
            name: format!("New {} policy", source_label(source)),
            enabled: false,
            priority: 100,
            source,
            read_access_ref: self
                .profiles
                .first()
                .map(|p| p.id.clone())
                .unwrap_or_default(),
            members: match source {
                SourceKind::Dhcp => MemberMode::Active,
                SourceKind::Wan => MemberMode::Configured,
                _ => MemberMode::Joined,
            },
            filter: FilterExpr::All { rules: vec![] },
            group,
            identity: IdentityMode::Registry,
            rates: if source == SourceKind::Wan {
                vec![]
            } else {
                vec![RateSource::SubscriberOverride]
            },
            parent: ParentRule::Router,
            capacity: if source == SourceKind::Wan {
                CapacityRule::Fixed {
                    down_bps: String::new(),
                    up_bps: String::new(),
                }
            } else {
                CapacityRule::FromPlans {
                    down_factor: "1".into(),
                    up_factor: "1".into(),
                    minimum_down_bps: "0".into(),
                    minimum_up_bps: "0".into(),
                }
            },
            writes: vec![],
        };
        let local_token = self.next_policy;
        self.next_policy += 1;
        self.policies.push(PolicyDraft {
            local_token,
            policy,
            priority: "100".into(),
        });
        self.selected = Some(self.policies.len() - 1);
        self.epoch = self.epoch.wrapping_add(1);
        true
    }
    pub fn move_rate(&mut self, index: usize, delta: isize) {
        self.edit(|p| {
            let to = index.checked_add_signed(delta);
            if let Some(to) =
                to.filter(|to| index < p.policy.rates.len() && *to < p.policy.rates.len())
            {
                p.policy.rates.swap(index, to);
            }
        });
    }
}
pub(crate) fn filter_at<'a>(root: &'a FilterExpr, path: &[usize]) -> Option<&'a FilterExpr> {
    if path.is_empty() {
        return Some(root);
    }
    match root {
        FilterExpr::All { rules } | FilterExpr::Any { rules } => {
            filter_at(rules.get(path[0])?, &path[1..])
        }
        _ => None,
    }
}
pub(crate) fn filter_at_mut<'a>(
    root: &'a mut FilterExpr,
    path: &[usize],
) -> Option<&'a mut FilterExpr> {
    if path.is_empty() {
        return Some(root);
    }
    match root {
        FilterExpr::All { rules } | FilterExpr::Any { rules } => {
            filter_at_mut(rules.get_mut(path[0])?, &path[1..])
        }
        _ => None,
    }
}
pub(crate) fn filter_leaves(root: &FilterExpr) -> usize {
    match root {
        FilterExpr::Match { .. } => 1,
        FilterExpr::All { rules } | FilterExpr::Any { rules } => {
            rules.iter().map(filter_leaves).sum()
        }
    }
}
pub(crate) fn rate_label(rate: &RateSource) -> &'static str {
    match rate {
        RateSource::SubscriberOverride => "Subscriber override",
        RateSource::AccountComment { .. } => "Account comment",
        RateSource::ActiveComment { .. } => "Active comment",
        RateSource::ProfileMap { .. } => "Profile to plan",
        RateSource::ServerMap { .. } => "Server to plan",
        RateSource::ProfileRateLimit => "Profile rate limit",
        RateSource::ProfileComment { .. } => "Profile comment",
        RateSource::ProfileName => "Profile name",
        RateSource::ServerComment { .. } => "Server comment",
        RateSource::Settings => "Configured defaults",
        RateSource::Fallback { .. } => "Fallback plan",
    }
}
pub(crate) fn rate_choices(source: SourceKind, caps: &StudioCapabilities) -> Vec<RateSource> {
    if source == SourceKind::Wan {
        return vec![];
    }
    let fs = fields(caps, source, false);
    let mut rates = vec![RateSource::SubscriberOverride];
    if fs.contains(&FieldKey::AccountComment) {
        rates.push(RateSource::AccountComment {
            format: CommentRateFormat::LegacyConfigured,
        });
    }
    if fs.contains(&FieldKey::ActiveComment) {
        rates.push(RateSource::ActiveComment {
            format: CommentRateFormat::LegacyConfigured,
        });
    }
    if (source == SourceKind::Ppp && fs.contains(&FieldKey::AccountProfile))
        || (source == SourceKind::Hotspot && fs.contains(&FieldKey::UserProfile))
    {
        rates.push(RateSource::ProfileMap { bindings: vec![] });
        rates.push(RateSource::ProfileRateLimit);
    }
    if source == SourceKind::Dhcp {
        rates.push(RateSource::ServerMap { bindings: vec![] });
    }
    rates.push(RateSource::Fallback {
        plan_id: String::new(),
    });
    if source == SourceKind::Ppp {
        rates.push(RateSource::ProfileName);
        rates.push(RateSource::ProfileComment {
            format: CommentRateFormat::LegacyConfigured,
        });
    }
    if source == SourceKind::Dhcp {
        rates.push(RateSource::ServerComment {
            format: CommentRateFormat::LegacyConfigured,
        });
    }
    rates.push(RateSource::Settings);
    rates
}
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct PendingPolicy {
    pub request: web_contract::topology::EditRequest,
    pub rows: Vec<PolicyDraft>,
    pub saved_revision: Option<String>,
}
impl PendingPolicy {
    pub fn new(d: &StudioDraft, request: web_contract::topology::EditRequest) -> Self {
        let mut rows = d.policies.clone();
        if let web_contract::topology::BuilderEdit::RouterSourcesSet { policies, .. } =
            &request.edit
        {
            for (row, p) in rows.iter_mut().zip(policies) {
                row.policy = p.clone();
                row.priority = p.priority.to_string();
            }
        }
        Self {
            request,
            rows,
            saved_revision: None,
        }
    }
    pub fn needs_ids(&self) -> bool {
        self.rows.iter().any(|r| r.policy.id.is_none())
    }
}
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct AccessState {
    pub phase: String,
    pub digest: String,
    pub before_digest: String,
    pub reload: String,
    pub warnings: Vec<String>,
    pub secret_profile: Option<String>,
}
#[derive(Clone, Debug)]
pub(crate) struct MergeConflict {
    pub key: String,
    pub label: String,
    pub local: String,
    pub saved: String,
}
fn same_policy_values(a: &SourcePolicyDraft, b: &SourcePolicyDraft) -> bool {
    let mut a = a.clone();
    let mut b = b.clone();
    a.id = None;
    b.id = None;
    a == b
}
pub(crate) fn adopt_saved_ids(
    d: &mut StudioDraft,
    p: &PendingPolicy,
    c: &RouterSourceConfig,
) -> bool {
    let Some(saved) = p
        .saved_revision
        .as_ref()
        .and_then(|r| r.parse::<u64>().ok())
    else {
        return false;
    };
    if c.router_id != d.router_id
        || c.policy_revision
            .as_ref()
            .and_then(|r| r.parse::<u64>().ok())
            .is_none_or(|r| r < saved)
        || p.rows.len() != c.policies.len()
    {
        return false;
    }
    for (row, remote) in p.rows.iter().zip(&c.policies) {
        if !same_policy_values(&row.policy, remote)
            || remote.id.as_deref().is_none_or(str::is_empty)
            || row
                .policy
                .id
                .as_ref()
                .is_some_and(|id| Some(id) != remote.id.as_ref())
        {
            return false;
        }
        if d.policies
            .iter()
            .find(|l| l.local_token == row.local_token)
            .is_some_and(|l| l.policy.id.is_some() && l.policy.id != remote.id)
        {
            return false;
        }
    }
    let mut baseline = p.rows.clone();
    for (row, remote) in baseline.iter_mut().zip(&c.policies) {
        row.policy.id = remote.id.clone();
        if let Some(local) = d
            .policies
            .iter_mut()
            .find(|l| l.local_token == row.local_token)
        {
            local.policy.id = remote.id.clone();
        }
    }
    d.saved_policies = baseline;
    true
}
fn merged<T: Clone + PartialEq + serde::Serialize>(
    base: &[(String, T)],
    local: &[(String, T)],
    remote: &[(String, T)],
    choices: &std::collections::HashMap<String, bool>,
    kind: &str,
) -> Result<Vec<T>, Vec<MergeConflict>> {
    let mut keys = Vec::new();
    for (k, _) in remote.iter().chain(local).chain(base) {
        if !keys.contains(k) {
            keys.push(k.clone());
        }
    }
    let mut out = Vec::new();
    let mut conflicts = Vec::new();
    for key in keys {
        let b = base.iter().find(|(k, _)| k == &key).map(|(_, v)| v);
        let l = local.iter().find(|(k, _)| k == &key).map(|(_, v)| v);
        let r = remote.iter().find(|(k, _)| k == &key).map(|(_, v)| v);
        let selected = if l == b {
            r
        } else if r == b || l == r {
            l
        } else {
            match choices.get(&format!("{kind}:{key}")) {
                Some(true) => l,
                Some(false) => r,
                None => {
                    let display = |entry: Option<&T>, remote: bool| {
                        if let Some(v) = entry {
                            let mut v = serde_json::to_value(v).unwrap_or_default();
                            if kind == "Policy" {
                                let priority = v.get("priority").cloned();
                                if let Some(policy) = v.get("policy").cloned() {
                                    v = policy;
                                    if let (Some(priority), Some(p)) = (priority, v.as_object_mut())
                                    {
                                        p.insert("priority".into(), priority);
                                    }
                                }
                            } else if let Some(p) = v.as_object_mut() {
                                p.remove("needs_confirmation");
                            }

                            if remote
                                && kind == "Profile"
                                && key != "default"
                                && let Some(v) = v.as_object_mut()
                            {
                                v.insert(
                                    "username".into(),
                                    serde_json::Value::String(
                                        "Not returned by the API; confirm the local username."
                                            .into(),
                                    ),
                                );
                            }
                            serde_json::to_string_pretty(&v).unwrap_or_default()
                        } else {
                            "Entry removed".into()
                        }
                    };
                    conflicts.push(MergeConflict{key:format!("{kind}:{key}"),label:format!("{kind} {key}: both the draft and saved entry changed (or one removed it)."),local:display(l,false),saved:display(r,true)});
                    continue;
                }
            }
        };
        if let Some(v) = selected {
            out.push(v.clone());
        }
    }
    if conflicts.is_empty() {
        Ok(out)
    } else {
        Err(conflicts)
    }
}
pub(crate) fn rebase_draft(
    d: &mut StudioDraft,
    c: &RouterSourceConfig,
    default_username: Option<&str>,
    choices: &std::collections::HashMap<String, bool>,
) -> Result<(), Vec<MergeConflict>> {
    if d.router_id != c.router_id {
        return Err(vec![MergeConflict {
            local: String::new(),
            saved: String::new(),
            key: "router".into(),
            label: "The saved configuration names another router.".into(),
        }]);
    }
    let mut next = d.clone();
    let mut remote_policies = Vec::new();
    for (i, p) in c.policies.iter().enumerate() {
        let old = d
            .saved_policies
            .iter()
            .chain(&d.policies)
            .find(|b| p.id.is_some() && b.policy.id == p.id)
            .or_else(|| {
                d.saved_policies
                    .iter()
                    .find(|b| b.policy.id.is_none() && same_policy_values(&b.policy, p))
            })
            .or_else(|| {
                d.saved_policies
                    .get(i)
                    .filter(|b| b.policy.id.is_none() && p.id.is_none())
            });
        let token = old.map(|r| r.local_token).unwrap_or_else(|| {
            let t = next.next_policy;
            next.next_policy += 1;
            t
        });
        remote_policies.push(PolicyDraft {
            local_token: token,
            policy: p.clone(),
            priority: p.priority.to_string(),
        });
    }
    let policy_key = |p: &PolicyDraft| {
        p.policy
            .id
            .clone()
            .unwrap_or_else(|| format!("local-{}", p.local_token))
    };
    // A formerly implicit policy can acquire its ID in the saved set.
    let key_for = |p: &PolicyDraft| {
        remote_policies
            .iter()
            .find(|r| r.local_token == p.local_token)
            .map(policy_key)
            .unwrap_or_else(|| policy_key(p))
    };
    let bp: Vec<_> = d
        .saved_policies
        .iter()
        .map(|p| (key_for(p), p.clone()))
        .collect();
    let lp: Vec<_> = d.policies.iter().map(|p| (key_for(p), p.clone())).collect();
    let rp: Vec<_> = remote_policies
        .iter()
        .map(|p| (policy_key(p), p.clone()))
        .collect();
    // ID assignment is metadata, not an overlapping policy-value edit.
    let normalize = |rows: Vec<(String, PolicyDraft)>| {
        rows.into_iter()
            .map(|(k, mut v)| {
                if let Some(r) = remote_policies
                    .iter()
                    .find(|r| r.local_token == v.local_token)
                    && v.policy.id.is_none()
                {
                    v.policy.id = r.policy.id.clone();
                }
                (k, v)
            })
            .collect::<Vec<_>>()
    };
    let policies = merged(&normalize(bp), &normalize(lp), &rp, choices, "Policy");
    let mut remote_profiles = Vec::new();
    for p in &c.access_profiles {
        let baseline = d.saved_profiles.iter().find(|b| b.id == p.id);
        remote_profiles.push(ProfileDraft {
            id: p.id.clone(),
            label: p.label.clone(),
            username: if p.id == "default" {
                default_username
                    .map(str::to_owned)
                    .or_else(|| baseline.map(|b| b.username.clone()))
                    .unwrap_or_default()
            } else {
                baseline.map(|b| b.username.clone()).unwrap_or_default()
            },
            credential_set: p.credential_set,
            supported_writes: p.supported_writes.clone(),
            needs_confirmation: false,
        });
    }
    let profile_rows = |ps: &[ProfileDraft]| {
        ps.iter()
            .map(|p| {
                let mut p = p.clone();
                p.needs_confirmation = false;
                (p.id.clone(), p)
            })
            .collect::<Vec<_>>()
    };
    let profiles = merged(
        &profile_rows(&d.saved_profiles),
        &profile_rows(&d.profiles),
        &profile_rows(&remote_profiles),
        choices,
        "Profile",
    );
    match (policies, profiles) {
        (Ok(policies), Ok(mut profiles)) => {
            if d.config_digest != c.config_digest {
                for p in &mut profiles {
                    p.needs_confirmation = true;
                }
                for p in &mut remote_profiles {
                    p.needs_confirmation = true;
                }
            }
            next.policies = policies;
            next.saved_policies = remote_policies;
            next.profiles = profiles;
            next.saved_profiles = remote_profiles;
            let selected_token = d
                .selected
                .and_then(|i| d.policies.get(i))
                .map(|p| p.local_token);
            next.selected = selected_token
                .and_then(|token| next.policies.iter().position(|p| p.local_token == token))
                .or_else(|| (!next.policies.is_empty()).then_some(0));
            next.config_digest = c.config_digest.clone();
            next.loaded_config_digest = c.loaded_config_digest.clone();
            next.policy_revision = c.policy_revision.clone();
            next.epoch = next.epoch.wrapping_add(1);
            *d = next;
            Ok(())
        }
        (a, b) => {
            let mut errors = a.err().unwrap_or_default();
            errors.extend(b.err().unwrap_or_default());
            Err(errors)
        }
    }
}

/// Only non-secret drafts survive a same-actor reauthentication in this tab.
#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct DraftVault {
    pub actor: Option<String>,
    pub drafts: std::collections::HashMap<String, StudioDraft>,
    pub saves: std::collections::HashMap<String, PendingPolicy>,
    pub last_policies: std::collections::HashMap<String, PendingPolicy>,
    pub access: std::collections::HashMap<String, AccessState>,
    pub messages: std::collections::HashMap<String, String>,
}
impl DraftVault {
    pub fn bind_actor(&mut self, actor: &str) {
        if self.actor.as_deref() != Some(actor) {
            self.drafts.clear();
            self.saves.clear();
            self.last_policies.clear();
            self.access.clear();
            self.messages.clear();
            self.actor = Some(actor.into());
        }
    }
    pub fn remember(&mut self, actor: &str, draft: StudioDraft) -> bool {
        if self.actor.as_deref() != Some(actor) {
            return false;
        }
        self.drafts.insert(draft.router_id.clone(), draft);
        true
    }
    pub fn recover(&self, actor: &str, router: &str) -> Option<StudioDraft> {
        (self.actor.as_deref() == Some(actor))
            .then(|| self.drafts.get(router).cloned())
            .flatten()
    }
    pub fn clear(&mut self) {
        self.drafts.clear();
        self.saves.clear();
        self.last_policies.clear();
        self.access.clear();
        self.messages.clear();
        self.actor = None;
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DraftStamp {
    pub actor: String,
    pub router_id: String,
    pub config_digest: String,
    pub revision: Option<String>,
    pub epoch: u64,
    pub snapshot_id: String,
}
impl DraftStamp {
    pub fn new(actor: &str, d: &StudioDraft, snapshot: &str) -> Option<Self> {
        if actor.is_empty() || d.router_id.is_empty() {
            return None;
        }
        Some(Self {
            actor: actor.into(),
            router_id: d.router_id.clone(),
            config_digest: d.config_digest.clone(),
            revision: d.policy_revision.clone(),
            epoch: d.epoch,
            snapshot_id: snapshot.into(),
        })
    }
    pub fn matches(&self, actor: &str, d: &StudioDraft, snapshot: &str) -> bool {
        self.actor == actor
            && self.router_id == d.router_id
            && self.config_digest == d.config_digest
            && d.policy_revision == self.revision
            && self.epoch == d.epoch
            && self.snapshot_id == snapshot
    }
    pub fn accepts_reference(&self, r: &CandidateRef) -> bool {
        self.config_digest == r.config_digest
            && self.revision.as_deref() == Some(r.base_revision.as_str())
            && self.snapshot_id == r.snapshot_id
    }
}
#[derive(Clone, PartialEq)]
pub(crate) struct BoundCandidate {
    pub stamp: DraftStamp,
    pub summary: CandidateSummary,
    pub unavailable: bool,
}
impl BoundCandidate {
    pub fn valid(&self, actor: &str, d: &StudioDraft, now: u64) -> bool {
        !self.unavailable
            && self.summary.reference.expires_at > now
            && self.stamp.accepts_reference(&self.summary.reference)
            && self
                .stamp
                .matches(actor, d, &self.summary.reference.snapshot_id)
    }
}
#[derive(Clone)]
pub(crate) struct DetailPage<T> {
    pub items: Vec<T>,
    pub total: Option<u64>,
    pub next_cursor: Option<String>,
    seen: Vec<String>,
}
impl<T> Default for DetailPage<T> {
    fn default() -> Self {
        Self {
            items: vec![],
            total: None,
            next_cursor: None,
            seen: vec![],
        }
    }
}
impl<T> DetailPage<T> {
    pub fn append(
        &mut self,
        id: &str,
        cursor: Option<&str>,
        page: CandidatePage<T>,
    ) -> Result<(), &'static str> {
        if page.candidate_id != id
            || page.items.len() > PAGE_MAX as usize
            || page.items.len() as u64 > page.total
        {
            return Err("The detail page does not match this candidate.");
        }
        if cursor.is_some() && (self.items.len() + page.items.len()) as u64 > page.total {
            return Err("The detail page exceeds its total count.");
        }
        if let Some(cursor) = cursor
            && (self.next_cursor.as_deref() != Some(cursor) || self.total != Some(page.total))
        {
            return Err("The detail page changed. Load its first page again.");
        }
        if let Some(next) = &page.next_cursor
            && (cursor == Some(next.as_str()) || self.seen.contains(next))
        {
            return Err("The detail cursor did not advance.");
        }
        if cursor.is_none() {
            self.items.clear();
            self.seen.clear();
        }
        if let Some(cursor) = cursor {
            self.seen.push(cursor.into());
        }
        self.total = Some(page.total);
        self.next_cursor = page.next_cursor;
        self.items.extend(page.items);
        Ok(())
    }
    pub fn incomplete(&self) -> bool {
        self.total.is_some_and(|n| (self.items.len() as u64) < n) && self.next_cursor.is_none()
    }
}
pub(crate) fn evaluation_policies(d: &StudioDraft) -> Result<Vec<SourcePolicyDraft>, String> {
    d.policies
        .iter()
        .map(|p| {
            let mut policy = p.policy.clone();
            policy.priority = p
                .priority
                .parse::<u16>()
                .map_err(|_| "Set each policy priority from 0 to 65535.".to_string())?;
            if policy.name.is_empty() || policy.name.len() > MAX_POLICY_NAME {
                return Err("Set each policy name from 1 to 128 UTF-8 bytes.".into());
            }
            Ok(policy)
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (StudioCapabilities, RouterSourceConfig) {
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "../../web-contract/tests/fixtures/studio/normal.json"
        ))
        .unwrap();
        (
            serde_json::from_value(v["capabilities"].clone()).unwrap(),
            serde_json::from_value(v["source_configs"]["lq3-00000000000000a1"].clone()).unwrap(),
        )
    }
    #[test]
    fn unsupported_field_is_not_selectable() {
        let (caps, _) = fixture();
        assert!(fields(&caps, SourceKind::Ppp, false).contains(&FieldKey::AccountProfile));
        assert!(!fields(&caps, SourceKind::Ppp, false).contains(&FieldKey::ActiveComment));
        assert!(!fields(&caps, SourceKind::Ppp, true).contains(&FieldKey::AccountComment));
        assert!(!fields(&caps, SourceKind::Dhcp, false).contains(&FieldKey::AccountProfile));
        assert!(
            !operators(&caps, SourceKind::Ppp, FieldKey::AccountComment).contains(&FilterOp::Ne)
        );
    }
    #[test]
    fn source_switch_keeps_each_draft() {
        let (caps, config) = fixture();
        let mut d = StudioDraft::from(config);
        d.select_source(SourceKind::Ppp, &caps);
        d.edit(|p| p.policy.name = "Custom PPP".into());
        let epoch = d.epoch;
        d.select_source(SourceKind::Dhcp, &caps);
        d.edit(|p| {
            p.policy.group = GroupRule::Fixed {
                name: "Dynamic leases".into(),
            }
        });
        d.select_source(SourceKind::Ppp, &caps);
        assert_eq!(d.current().unwrap().policy.name, "Custom PPP");
        d.select_source(SourceKind::Dhcp, &caps);
        assert_eq!(
            d.current().unwrap().policy.group,
            GroupRule::Fixed {
                name: "Dynamic leases".into()
            }
        );
        assert!(d.epoch > epoch);
        assert_eq!(d.router_id, "lq3-00000000000000a1");
        assert_eq!(d.config_digest, "0000000000000001");
        assert_eq!(d.policy_revision.as_deref(), Some("42"));
    }
    #[test]
    fn unenrolled_router_keeps_connection_editor() {
        let (_, mut config) = fixture();
        config.policy_revision = None;
        assert!(section_reason(StudioSection::Connection, true, None, Some(&config)).is_none());
        assert!(section_reason(StudioSection::Sources, true, None, Some(&config)).is_some());
        assert!(section_reason(StudioSection::Connection, true, None, None).is_none());
        assert!(section_reason(StudioSection::Connection, false, None, None).is_some());
    }
    #[test]
    fn source_add_uses_only_advertised_fields_and_keeps_saved_policy() {
        let (caps, config) = fixture();
        let saved = config.policies.clone();
        let mut d = StudioDraft::from(config);
        d.select_source(SourceKind::Hotspot, &caps);
        assert_eq!(d.policies.len(), 3);
        assert_eq!(d.current().unwrap().policy.source, SourceKind::Hotspot);
        assert_eq!(d.policies[0].policy, saved[0]);
        assert!(!d.current().unwrap().policy.enabled);
        assert!(d.add_policy(SourceKind::Wan, &caps));
        assert_eq!(d.current().unwrap().policy.writes, vec![]);
    }
    #[test]
    fn rate_moves_keep_exact_precedence_and_raw_capacity() {
        let (_, config) = fixture();
        let mut d = StudioDraft::from(config);
        let original = d.current().unwrap().policy.rates.clone();
        d.move_rate(2, -1);
        let r = &d.current().unwrap().policy.rates;
        assert_eq!(r[1], original[2]);
        assert_eq!(r[2], original[1]);
        d.edit(|p| {
            p.priority = "-".into();
            p.policy.capacity = CapacityRule::Fixed {
                down_bps: "9007199254740993".into(),
                up_bps: "".into(),
            };
        });
        assert_eq!(d.current().unwrap().priority, "-");
        assert!(
            serde_json::to_string(&d.current().unwrap().policy.capacity)
                .unwrap()
                .contains("9007199254740993")
        );
    }
    #[test]
    fn nested_query_edits_keep_siblings_and_typed_values() {
        let (_, config) = fixture();
        let mut q = config.policies[0].filter.clone();
        assert_eq!(filter_leaves(&q), 1);
        if let Some(FilterExpr::Match { value, .. }) = filter_at_mut(&mut q, &[0]) {
            *value = FilterValue::Texts {
                values: vec!["gold".into(), "silver".into()],
            };
        }
        assert!(
            matches!(filter_at(&q,&[0]),Some(FilterExpr::Match{value:FilterValue::Texts{values},..})if values.len()==2)
        );
        assert!(filter_at_mut(&mut q, &[3]).is_none());
        assert_eq!(key(&FieldKey::LeaseServer), "lease.server");
    }
    #[test]
    fn rates_require_their_source_fields_and_access_feature() {
        let (mut caps, config) = fixture();
        let rates = rate_choices(SourceKind::Ppp, &caps);
        assert!(rates.iter().any(|r| rate_label(r) == "Profile to plan"));
        caps.fields.retain(|f| f.field != FieldKey::AccountProfile);
        assert!(
            !rate_choices(SourceKind::Ppp, &caps)
                .iter()
                .any(|r| rate_label(r) == "Profile to plan")
        );
        assert!(rate_choices(SourceKind::Wan, &caps).is_empty());
        caps.features
            .retain(|f| *f != StudioFeature::AccessProfiles);
        assert!(section_reason(StudioSection::Access, true, Some(&caps), Some(&config)).is_some());
    }
    #[test]
    fn old_preview_cannot_replace_new_draft() {
        let (_, c) = fixture();
        let mut d = StudioDraft::from(c);
        let stamp = DraftStamp::new("alice", &d, "snapshot").unwrap();
        assert!(stamp.matches("alice", &d, "snapshot"));
        d.edit(|p| p.policy.name = "New draft".into());
        assert!(!stamp.matches("alice", &d, "snapshot"));
        assert!(!stamp.matches("bob", &d, "snapshot"));
        assert_eq!(d.current().unwrap().policy.name, "New draft");
        d.router_id = "other".into();
        assert!(!stamp.matches("alice", &d, "snapshot"));
    }
    #[test]
    fn expired_candidate_keeps_draft() {
        let (_, c) = fixture();
        let d = StudioDraft::from(c);
        let before = d.clone();
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "../../web-contract/tests/fixtures/studio/normal.json"
        ))
        .unwrap();
        let c: CandidateSummary = serde_json::from_value(v["candidate"].clone()).unwrap();
        let stamp = DraftStamp::new("alice", &d, &c.reference.snapshot_id).unwrap();
        let mut bound = BoundCandidate {
            stamp,
            summary: c,
            unavailable: false,
        };
        assert!(bound.valid("alice", &d, 1));
        assert!(!bound.valid("alice", &d, u64::MAX));
        bound.unavailable = true;
        assert!(!bound.valid("alice", &d, 1));
        assert!(d == before);
    }
    #[test]
    fn different_actor_cannot_recover_draft() {
        let (_, c) = fixture();
        let d = StudioDraft::from(c);
        let mut vault = DraftVault::default();
        vault.bind_actor("alice");
        vault.remember("alice", d.clone());
        assert!(vault.recover("alice", &d.router_id).is_some());
        assert!(vault.recover("bob", &d.router_id).is_none());
        vault.bind_actor("bob");
        assert!(vault.recover("alice", &d.router_id).is_none());
        assert!(!vault.remember("alice", d));
        vault.clear();
        assert!(vault.actor.is_none());
    }
    #[test]
    fn secret_input_is_not_in_draft_state() {
        let (_, c) = fixture();
        let d = StudioDraft::from(c);
        let mut connection = crate::router_form_model::Form::new_router();
        connection.password = "DO_NOT_STORE_THIS_SECRET".into();
        let mut vault = DraftVault::default();
        vault.bind_actor("alice");
        vault.remember("alice", d);
        let json = serde_json::to_string(&vault).unwrap();
        assert!(!json.contains(&connection.password));
        assert!(!json.contains("password"));
        let restored: DraftVault = serde_json::from_str(&json).unwrap();
        assert!(restored.recover("alice", "lq3-00000000000000a1").is_some());
    }
    #[test]
    fn paged_issues_keep_complete_counts() {
        let mut page = DetailPage::<ValidationIssue>::default();
        let issue = ValidationIssue {
            code: "x".into(),
            severity: Severity::Blocker,
            entity_ref: "r".into(),
            field: None,
            message: "Fix the record.".into(),
            remedy: None,
        };
        page.append(
            "candidate",
            None,
            CandidatePage {
                candidate_id: "candidate".into(),
                items: vec![issue.clone()],
                next_cursor: Some("next".into()),
                total: 100,
            },
        )
        .unwrap();
        assert_eq!(page.total, Some(100));
        assert_eq!(page.items.len(), 1);
        assert!(!page.incomplete());
        assert!(
            page.append(
                "foreign",
                Some("next"),
                CandidatePage {
                    candidate_id: "candidate".into(),
                    items: vec![issue.clone()],
                    next_cursor: None,
                    total: 100
                }
            )
            .is_err()
        );
        assert!(
            page.append(
                "candidate",
                Some("next"),
                CandidatePage {
                    candidate_id: "candidate".into(),
                    items: vec![issue],
                    next_cursor: Some("next".into()),
                    total: 100
                }
            )
            .is_err()
        );
    }

    #[test]
    fn invalid_priority_keeps_raw_draft() {
        let (_, c) = fixture();
        let mut d = StudioDraft::from(c);
        d.edit(|p| p.priority = "-".into());
        assert!(evaluation_policies(&d).is_err());
        assert_eq!(d.current().unwrap().priority, "-");
    }
    #[test]
    fn collection_stamp_does_not_require_policy_revision() {
        let (_, mut c) = fixture();
        c.policy_revision = None;
        let d = StudioDraft::from(c);
        let stamp = DraftStamp::new("alice", &d, "").unwrap();
        assert!(stamp.matches("alice", &d, ""));
        assert!(DraftStamp::new("", &d, "").is_none());
    }
}

fn filter_depth(root: &FilterExpr) -> usize {
    match root {
        FilterExpr::Match { .. } => 0,
        FilterExpr::All { rules } | FilterExpr::Any { rules } => {
            1 + rules.iter().map(filter_depth).max().unwrap_or(0)
        }
    }
}
pub(crate) fn binding_count(policy: &SourcePolicyDraft) -> usize {
    policy
        .rates
        .iter()
        .map(|r| match r {
            RateSource::ProfileMap { bindings } | RateSource::ServerMap { bindings } => {
                bindings.len()
            }
            _ => 0,
        })
        .sum()
}
pub(crate) fn policy_edit(
    d: &StudioDraft,
    caps: &StudioCapabilities,
    key: String,
) -> Result<web_contract::topology::EditRequest, String> {
    use web_contract::topology::{BuilderEdit, EditRequest};
    if !caps.features.contains(&StudioFeature::PolicySave) {
        return Err("Policy save is unavailable on this backend.".into());
    }
    if d.config_digest != d.loaded_config_digest {
        return Err("Wait for the service to load the saved access configuration.".into());
    }
    if d.profiles != d.saved_profiles {
        return Err("Save access profiles before the source policy.".into());
    }
    let revision = d
        .policy_revision
        .as_ref()
        .and_then(|r| r.parse::<u64>().ok())
        .ok_or("Initialize the topology and reload its revision first.")?;
    let policies = evaluation_policies(d)?;
    if policies.len() > caps.limits.policies_per_router.min(MAX_POLICIES as u64) as usize {
        return Err("Too many source policies.".into());
    }
    for p in &policies {
        if p.name.len() > caps.limits.policy_name_bytes as usize
            || filter_leaves(&p.filter)
                > caps.limits.filter_leaves.min(MAX_FILTER_LEAVES as u64) as usize
            || filter_depth(&p.filter)
                > caps.limits.filter_depth.min(MAX_FILTER_DEPTH as u64) as usize
            || binding_count(p) > caps.limits.plan_bindings.min(MAX_PLAN_BINDINGS as u64) as usize
        {
            return Err(
                "A policy exceeds the advertised name, query, or combined plan-binding limit."
                    .into(),
            );
        }
    }
    if serde_json::to_vec(&policies)
        .map_err(|_| "Invalid policy set.")?
        .len()
        > caps
            .limits
            .policy_set_bytes
            .min(MAX_POLICY_SET_BYTES as u64) as usize
    {
        return Err("The policy set exceeds the advertised size limit.".into());
    }
    let request = EditRequest {
        key,
        expected_revision: revision,
        edit: BuilderEdit::RouterSourcesSet {
            router_id: d.router_id.clone(),
            expected_config_digest: d.config_digest.clone(),
            policies,
        },
    };
    if serde_json::to_vec(&request)
        .map_err(|_| "Invalid policy request.")?
        .len()
        > MAX_JOB_BODY
    {
        return Err("The policy request exceeds 32 KiB.".into());
    }
    Ok(request)
}
pub(crate) fn access_edit(
    d: &StudioDraft,
    caps: &StudioCapabilities,
    selected: usize,
    password: Option<&str>,
) -> Result<web_contract::routers::RouterEdit, String> {
    if !caps.features.contains(&StudioFeature::AccessProfiles) {
        return Err("Access-profile save is unavailable.".into());
    }
    if d.profiles.len() > caps.limits.access_profiles.min(MAX_ACCESS_PROFILES as u64) as usize {
        return Err("Too many access profiles.".into());
    }
    let mut ids = std::collections::HashSet::new();
    let mut profiles = Vec::new();
    for (i, p) in d.profiles.iter().enumerate() {
        if p.id.is_empty()
            || p.id.len() > 32
            || !p
                .id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || !ids.insert(p.id.clone())
        {
            return Err("Use a unique profile ID: 1 to 32 of a-z, 0-9, or -.".into());
        }
        if p.needs_confirmation {
            return Err(format!(
                "Confirm the displayed username for profile {} after the configuration change. The API does not return explicit-profile usernames.",
                p.label
            ));
        }
        if p.username.is_empty()
            || p.username.chars().count() > 64
            || p.username.chars().any(char::is_control)
        {
            return Err("Enter every profile username before saving the complete set. Saved explicit-profile usernames are absent from this API.".into());
        }
        if p.label.is_empty() || p.label.len() > 128 {
            return Err("Use a profile label from 1 to 128 UTF-8 bytes.".into());
        }
        if p.id == "default" && !p.supported_writes.is_empty() {
            return Err(
                "The default profile is for reads. Use an explicit profile for supported writes."
                    .into(),
            );
        }
        let secret = if i == selected { password } else { None };
        if !p.credential_set && secret.is_none() {
            return Err(
                "Save new profiles one at a time. Select the new profile and enter its credential."
                    .into(),
            );
        }
        if secret.is_some_and(|s| {
            s.is_empty() || s.chars().count() > 256 || s.chars().any(char::is_control)
        }) {
            return Err(
                "Use a credential from 1 to 256 characters without control characters.".into(),
            );
        }
        profiles.push(AccessProfileEdit {
            id: p.id.clone(),
            label: p.label.clone(),
            username: p.username.clone(),
            password: secret.map(|s| SecretInput(s.into())),
            supported_writes: p.supported_writes.clone(),
        });
    }
    Ok(web_contract::routers::RouterEdit {
        expected_digest: d.config_digest.clone(),
        access_profiles: Some(profiles),
        ..Default::default()
    })
}
pub(crate) fn policy_values_match(
    d: &StudioDraft,
    r: &web_contract::topology::EditRequest,
) -> bool {
    match &r.edit {
        web_contract::topology::BuilderEdit::RouterSourcesSet {
            router_id,
            expected_config_digest,
            policies,
        } => {
            d.router_id == *router_id
                && d.config_digest == *expected_config_digest
                && evaluation_policies(d).is_ok_and(|p| {
                    p.len() == policies.len()
                        && p.iter().zip(policies).all(|(a, b)| {
                            if b.id.is_none() {
                                same_policy_values(a, b)
                            } else {
                                a == b
                            }
                        })
                })
        }
        _ => false,
    }
}
pub(crate) fn saved_revision(
    op: &web_contract::topology::Operation,
    r: &web_contract::topology::EditRequest,
) -> Option<String> {
    let receipt = op.receipt.as_ref()?;
    (op.key == r.key
        && op.state == "saved"
        && receipt.key == r.key
        && receipt.state == "saved"
        && receipt.base_revision == r.expected_revision.to_string())
    .then(|| receipt.revision.clone())
    .flatten()
    .filter(|r| r.parse::<u64>().is_ok())
}
pub(crate) fn runtime_applied(
    s: &web_contract::topology::TopologySummary,
    revision: &str,
    now: u64,
) -> bool {
    s.updated.saturating_add(s.fresh_for) >= now
        && s.registry_revision.as_deref() == Some(revision)
        && s.observed
            .as_ref()
            .is_some_and(|o| o.revision == revision && !o.stale && o.state == "ok")
        && s.publication.as_ref().is_some_and(|p| {
            p.policy_revision.as_deref() == Some(revision)
                && p.phase == "applied"
                && p.accepted
                && !p.failed
                && p.superseded_by.is_none()
        })
}

#[cfg(test)]
mod save_tests {
    use super::*;
    use web_contract::topology::*;
    fn input() -> (StudioCapabilities, StudioDraft) {
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "../../web-contract/tests/fixtures/studio/normal.json"
        ))
        .unwrap();
        let c = serde_json::from_value(v["capabilities"].clone()).unwrap();
        let d = StudioDraft::from(
            serde_json::from_value::<RouterSourceConfig>(
                v["source_configs"]["lq3-00000000000000a1"].clone(),
            )
            .unwrap(),
        );
        (c, d)
    }
    #[test]
    fn policy_save_needs_no_preview_and_waits_for_loaded_config() {
        let (c, mut d) = input();
        let r = policy_edit(&d, &c, "same-policy-request".into()).unwrap();
        assert_eq!(r.expected_revision, 42);
        assert!(matches!(r.edit, BuilderEdit::RouterSourcesSet { .. }));
        d.loaded_config_digest = "old".into();
        assert!(policy_edit(&d, &c, "another-policy-request".into()).is_err());
    }
    #[test]
    fn bindings_use_the_advertised_combined_limit() {
        let (mut c, mut d) = input();
        c.limits.plan_bindings = 1;
        let b = PlanBinding {
            selector: "x".into(),
            plan_id: "plan".into(),
        };
        d.policies[0].policy.rates = vec![
            RateSource::ProfileMap {
                bindings: vec![b.clone()],
            },
            RateSource::ServerMap { bindings: vec![b] },
        ];
        assert!(policy_edit(&d, &c, "limited-policy-request".into()).is_err());
    }
    #[test]
    fn profiles_require_all_usernames_and_keep_absent_secrets() {
        let (c, mut d) = input();
        assert!(access_edit(&d, &c, 0, None).is_err());
        for p in &mut d.profiles {
            p.username = "api".into();
        }
        let e = access_edit(&d, &c, 0, None).unwrap();
        assert!(
            e.access_profiles
                .unwrap()
                .iter()
                .all(|p| p.password.is_none())
        );
        d.profiles[0].supported_writes = vec![WriteScope::WanAddressLists];
        assert!(access_edit(&d, &c, 0, None).is_err());
    }
    #[test]
    fn saved_receipt_is_not_runtime_acceptance() {
        let (c, d) = input();
        let r = policy_edit(&d, &c, "receipt-policy-request".into()).unwrap();
        let mut op = Operation {
            key: r.key.clone(),
            state: "saved".into(),
            receipt: None,
            review: None,
            error: None,
            discovery: None,
            source_result: None,
        };
        assert!(saved_revision(&op, &r).is_none());
        op.receipt = Some(WebReceipt {
            key: r.key.clone(),
            state: "saved".into(),
            base_revision: "42".into(),
            revision: Some("43".into()),
            subject_id: None,
            error: None,
        });
        assert_eq!(saved_revision(&op, &r), Some("43".into()));
        assert!(!runtime_applied(&TopologySummary::default(), "43", 1));
        op.receipt.as_mut().unwrap().key = "foreign".into();
        assert!(saved_revision(&op, &r).is_none());
    }
    #[test]
    fn recovered_receipt_does_not_mark_changed_values_saved() {
        let (c, mut d) = input();
        let request = policy_edit(&d, &c, "older-policy-request".into()).unwrap();
        assert!(policy_values_match(&d, &request));
        d.policies[0].policy.name = "Newer draft".into();
        assert!(!policy_values_match(&d, &request));
    }
    #[test]
    fn pending_saves_cannot_cross_actors() {
        let (c, d) = input();
        let r = policy_edit(&d, &c, "actor-policy-request".into()).unwrap();
        let mut v = DraftVault::default();
        v.bind_actor("a");
        v.saves
            .insert(d.router_id.clone(), PendingPolicy::new(&d, r));
        v.bind_actor("a");
        assert_eq!(v.saves.len(), 1);
        v.bind_actor("b");
        assert!(v.saves.is_empty());
    }
    #[test]
    fn applied_requires_current_complete_acceptance_evidence() {
        let at = 100;
        let mut s = TopologySummary {
            updated: at,
            fresh_for: 10,
            registry_revision: Some("43".into()),
            observed: Some(Observed {
                revision: "43".into(),
                collected_at: at,
                generation: None,
                state: "ok".into(),
                stale: false,
            }),
            publication: Some(Publication {
                write_id: "proof".into(),
                phase: "applied".into(),
                started: at,
                updated: at,
                policy_revision: Some("43".into()),
                files: vec![],
                failed: false,
                accepted: true,
                superseded_by: None,
            }),
            ..Default::default()
        };
        assert!(runtime_applied(&s, "43", 105));
        assert!(!runtime_applied(&s, "43", 111));
        s.observed.as_mut().unwrap().stale = true;
        assert!(!runtime_applied(&s, "43", 105));
        s.observed.as_mut().unwrap().stale = false;
        s.publication.as_mut().unwrap().accepted = false;
        assert!(!runtime_applied(&s, "43", 105));
    }
    #[test]
    fn rebase_keeps_other_operators_entries_and_reports_overlap() {
        let (caps, mut d) = input();
        let mut cfg: RouterSourceConfig = serde_json::from_value(
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../../web-contract/tests/fixtures/studio/normal.json"
            ))
            .unwrap()["source_configs"]["lq3-00000000000000a1"]
                .clone(),
        )
        .unwrap();
        d.policies[0].policy.name = "Local change".into();
        let mut remote = cfg.policies[1].clone();
        remote.id = Some("remote-policy".into());
        remote.name = "Other operator".into();
        cfg.policies.push(remote);
        cfg.policy_revision = Some("43".into());
        rebase_draft(&mut d, &cfg, None, &Default::default()).unwrap();
        assert!(
            d.policies
                .iter()
                .any(|p| p.policy.id.as_deref() == Some("remote-policy"))
        );
        assert_eq!(d.policies[0].policy.name, "Local change");
        assert!(policy_edit(&d, &caps, "rebased-request-key".into()).is_ok());
        d.policies[0].policy.name = "Second local".into();
        cfg.policies[0].name = "Other edit same policy".into();
        let before = d.policies.clone();
        let conflicts = rebase_draft(&mut d, &cfg, None, &Default::default()).unwrap_err();
        assert!(!conflicts.is_empty());
        assert!(conflicts[0].label.contains("both"));
        assert!(conflicts[0].local.contains("Second local"));
        assert!(conflicts[0].saved.contains("Other edit same policy"));
        assert!(!conflicts[0].local.contains("local_token"));
        assert!(d.policies == before);
        let choices = std::collections::HashMap::from([(conflicts[0].key.clone(), true)]);
        rebase_draft(&mut d, &cfg, None, &choices).unwrap();
        assert_eq!(d.policies[0].policy.name, "Second local");
    }
    #[test]
    fn assigned_ids_keep_in_flight_edits_and_do_not_attach_to_replacements() {
        let (caps, mut d) = input();
        d.add_policy(SourceKind::Ppp, &caps);
        let i = d.policies.len() - 1;
        let mut saved = PendingPolicy::new(
            &d,
            policy_edit(&d, &caps, "new-policy-request-key".into()).unwrap(),
        );
        assert!(saved.needs_ids());
        saved.saved_revision = Some("43".into());
        let mut cfg: RouterSourceConfig = serde_json::from_value(
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../../web-contract/tests/fixtures/studio/normal.json"
            ))
            .unwrap()["source_configs"]["lq3-00000000000000a1"]
                .clone(),
        )
        .unwrap();
        saved.saved_revision = Some("43".into());
        cfg.policies = evaluation_policies(&d).unwrap();
        cfg.policies[i].id = Some("assigned-policy".into());
        cfg.policy_revision = Some("43".into());
        d.policies[i].policy.name = "Edited while metadata pending".into();
        d.policies[i].priority = "invalid".into();
        assert!(adopt_saved_ids(&mut d, &saved, &cfg));
        assert_eq!(d.policies[i].policy.id.as_deref(), Some("assigned-policy"));
        assert_eq!(d.policies[i].policy.name, "Edited while metadata pending");
        assert_eq!(d.policies[i].priority, "invalid");
        d.policies.remove(i);
        d.add_policy(SourceKind::Ppp, &caps);
        assert!(adopt_saved_ids(&mut d, &saved, &cfg));
        assert!(d.policies.last().unwrap().policy.id.is_none());
    }
    #[test]
    fn amended_rate_kinds_have_labels() {
        for rate in [
            RateSource::ProfileName,
            RateSource::ProfileComment {
                format: CommentRateFormat::LegacyConfigured,
            },
            RateSource::ServerComment {
                format: CommentRateFormat::LegacyConfigured,
            },
            RateSource::Settings,
        ] {
            assert!(!rate_label(&rate).is_empty());
        }
    }
}
