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
#[derive(Clone, PartialEq)]
pub(crate) struct PolicyDraft {
    pub policy: SourcePolicyDraft,
    pub priority: String,
}
#[derive(Clone, PartialEq)]
pub(crate) struct ProfileDraft {
    pub id: String,
    pub label: String,
    pub username: String,
    pub credential_set: bool,
    pub supported_writes: Vec<WriteScope>,
}
#[derive(Clone, PartialEq)]
pub(crate) struct StudioDraft {
    pub router_id: String,
    pub config_digest: String,
    pub loaded_config_digest: String,
    pub policy_revision: Option<String>,
    pub policies: Vec<PolicyDraft>,
    pub profiles: Vec<ProfileDraft>,
    pub selected: Option<usize>,
    pub epoch: u64,
}
impl From<RouterSourceConfig> for StudioDraft {
    fn from(c: RouterSourceConfig) -> Self {
        Self {
            router_id: c.router_id,
            config_digest: c.config_digest,
            loaded_config_digest: c.loaded_config_digest,
            policy_revision: c.policy_revision,
            selected: (!c.policies.is_empty()).then_some(0),
            epoch: 0,
            policies: c
                .policies
                .into_iter()
                .map(|p| PolicyDraft {
                    priority: p.priority.to_string(),
                    policy: p,
                })
                .collect(),
            profiles: c
                .access_profiles
                .into_iter()
                .map(|p| ProfileDraft {
                    id: p.id,
                    label: p.label,
                    username: String::new(),
                    credential_set: p.credential_set,
                    supported_writes: p.supported_writes,
                })
                .collect(),
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
        self.policies.push(PolicyDraft {
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
    rates
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
}
