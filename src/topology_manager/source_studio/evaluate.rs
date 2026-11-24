//! Studio evaluation (`StudioJob::Evaluate`). Owner decision (task B3): a draft is translated
//! into the current engine structures; the engine runs on a clone. This module holds the
//! validation and the translation. A draft that the engine cannot express is a blocker.
use super::snapshots::{Studio, table_id};
use crate::config::{Config, DEFAULT_PROFILE, RouterCfg};
use crate::model::RosRow;
use crate::routeros::Connector;
use crate::topology_manager::catalog::{Factors, SourceOrigin};
use crate::topology_manager::coordinator::Coordinator;
use crate::topology_manager::legacy_rate::{LegacyPriority, LegacyRateSource};
use crate::topology_manager::model::{RegistryDocument, SourceKind as EngineKind};
use crate::topology_manager::normalize::{CurrentRouter, Issue, NormalizeError};
use crate::topology_manager::policy_draft::router_id;
use crate::topology_manager::rates::Fraction;
use crate::topology_manager::reader::{RouterRead, Table};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;
use web_contract::source_studio::{
    CandidateDiff, CandidateGraph, CapacityRule, ChangeKind, CommentRateFormat, DiffMarker,
    FieldKey, FilterExpr, FilterOp, FilterValue, GroupRule, MAX_FILTER_DEPTH, MAX_FILTER_LEAVES,
    MAX_PLAN_BINDINGS, MAX_POLICIES, MAX_POLICY_NAME, MAX_POLICY_SET_BYTES, MemberMode, ParentRule,
    Provenance, RateKind, RateSource, ReadState, ResolvedRecord, Severity, SourceKind,
    SourcePolicyDraft, ValidationIssue,
};

/// The PPP part of a router after translation: one selection with scope `ppp`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PppIntent {
    pub policy_id: String,
    /// One group for each profile (the legacy `per_plan_node`), or one fixed group.
    pub per_plan: bool,
    pub parent: ParentRule,
    pub capacity: CapacityRule,
}

/// The DHCP part: per-site selections. `servers: None` selects every current server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DhcpIntent {
    pub policy_id: String,
    pub servers: Option<Vec<String>>,
    pub parent: ParentRule,
    pub capacity: CapacityRule,
}

/// What the enabled drafts of one router ask the engine to do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RouterIntent {
    pub ppp: Option<PppIntent>,
    /// In priority order: a server goes to the first policy that names it.
    pub dhcp: Vec<DhcpIntent>,
}

/// The policy ID of a draft: the saved ID, or `draft:<index>`.
pub fn policy_ref(p: &SourcePolicyDraft, index: usize) -> String {
    p.id.clone().unwrap_or_else(|| format!("draft:{index}"))
}

fn issue(
    code: &str,
    severity: Severity,
    entity: &str,
    field: Option<FieldKey>,
    message: &str,
) -> ValidationIssue {
    ValidationIssue {
        code: code.into(),
        severity,
        entity_ref: entity.into(),
        field,
        message: message.into(),
        remedy: None,
    }
}

fn blocker(entity: &str, field: Option<FieldKey>, message: &str) -> ValidationIssue {
    issue(
        "unsupported_in_v1",
        Severity::Blocker,
        entity,
        field,
        message,
    )
}

fn leaves(f: &FilterExpr, depth: usize) -> Result<usize, String> {
    if depth > MAX_FILTER_DEPTH {
        return Err(format!(
            "A filter can have at most {MAX_FILTER_DEPTH} levels."
        ));
    }
    match f {
        FilterExpr::Match { .. } => Ok(1),
        FilterExpr::All { rules } | FilterExpr::Any { rules } => rules
            .iter()
            .try_fold(0, |n, r| Ok(n + leaves(r, depth + 1)?)),
    }
}

/// Input limits reject the whole request. They are not validation issues.
pub fn check_limits(policies: &[SourcePolicyDraft]) -> Result<(), String> {
    if policies.len() > MAX_POLICIES {
        return Err(format!(
            "Use at most {MAX_POLICIES} policies for one router."
        ));
    }
    let bytes = serde_json::to_vec(policies)
        .map_err(|e| e.to_string())?
        .len();
    if bytes > MAX_POLICY_SET_BYTES {
        return Err("The policy set is larger than 24 KiB.".into());
    }
    for p in policies {
        if leaves(&p.filter, 1)? > MAX_FILTER_LEAVES {
            return Err(format!(
                "A filter can have at most {MAX_FILTER_LEAVES} rules."
            ));
        }
        let bindings: usize = p
            .rates
            .iter()
            .map(|r| match r {
                RateSource::ProfileMap { bindings } | RateSource::ServerMap { bindings } => {
                    bindings.len()
                }
                _ => 0,
            })
            .sum();
        if bindings > MAX_PLAN_BINDINGS {
            return Err(format!(
                "Use at most {MAX_PLAN_BINDINGS} plan bindings in one policy."
            ));
        }
    }
    Ok(())
}

fn rate_kind(r: &RateSource) -> Option<RateKind> {
    let legacy = |f: &CommentRateFormat| *f == CommentRateFormat::LegacyConfigured;
    Some(match r {
        RateSource::SubscriberOverride => RateKind::SubscriberOverride,
        RateSource::AccountComment { format } if legacy(format) => RateKind::AccountComment,
        RateSource::ActiveComment { format } if legacy(format) => RateKind::ActiveComment,
        RateSource::ProfileComment { format } if legacy(format) => RateKind::ProfileComment,
        RateSource::ServerComment { format } if legacy(format) => RateKind::ServerComment,
        RateSource::ProfileRateLimit => RateKind::ProfileRateLimit,
        RateSource::ProfileName => RateKind::ProfileName,
        RateSource::Settings => RateKind::Settings,
        // Plan bindings and plan fallbacks have no engine rate source yet.
        _ => return None,
    })
}

/// The engine order of one source. A subscriber override always comes first; a configured
/// default always comes last for PPP.
fn expected(priority: &LegacyPriority) -> Vec<RateKind> {
    let mut out = vec![RateKind::SubscriberOverride];
    out.extend(priority.sources.iter().map(|s| match s {
        LegacyRateSource::AccountComment => RateKind::AccountComment,
        LegacyRateSource::ActiveComment => RateKind::ActiveComment,
        LegacyRateSource::ProfileComment => RateKind::ProfileComment,
        LegacyRateSource::ProfileRateLimit => RateKind::ProfileRateLimit,
        LegacyRateSource::ProfileName => RateKind::ProfileName,
        LegacyRateSource::ServerComment => RateKind::ServerComment,
        LegacyRateSource::Settings => RateKind::Settings,
    }));
    if priority.fallback.is_some() && out.last() != Some(&RateKind::Settings) {
        out.push(RateKind::Settings);
    }
    out
}

fn rates_match(p: &SourcePolicyDraft, want: &[RateKind]) -> bool {
    let got: Option<Vec<RateKind>> = p.rates.iter().map(rate_kind).collect();
    got.as_deref() == Some(want)
}

fn plain_decimal(text: &str, positive: bool) -> bool {
    let (whole, frac) = text.split_once('.').unwrap_or((text, "0"));
    let digits = |s: &str| !s.is_empty() && s.len() <= 20 && s.bytes().all(|b| b.is_ascii_digit());
    digits(whole) && digits(frac) && (!positive || text.bytes().any(|b| (b'1'..=b'9').contains(&b)))
}

fn capacity_ok(c: &CapacityRule) -> bool {
    match c {
        CapacityRule::FromPlans {
            down_factor,
            up_factor,
            minimum_down_bps,
            minimum_up_bps,
        } => {
            plain_decimal(down_factor, true)
                && plain_decimal(up_factor, true)
                && [minimum_down_bps, minimum_up_bps]
                    .iter()
                    .all(|v| plain_decimal(v, false) && !v.contains('.'))
        }
        CapacityRule::Fixed { down_bps, up_bps } => [down_bps, up_bps]
            .iter()
            .all(|v| plain_decimal(v, true) && !v.contains('.')),
    }
}

/// DHCP servers of a `lease.server` filter. `Ok(None)`: no filter, all servers.
fn dhcp_servers(f: &FilterExpr) -> Result<Option<Vec<String>>, ()> {
    match f {
        FilterExpr::All { rules } if rules.is_empty() => Ok(None),
        FilterExpr::All { rules } if rules.len() == 1 => dhcp_servers(&rules[0]),
        FilterExpr::Match {
            field: FieldKey::LeaseServer,
            op: FilterOp::Eq,
            value: FilterValue::Text { value },
        } => Ok(Some(vec![value.clone()])),
        FilterExpr::Match {
            field: FieldKey::LeaseServer,
            op: FilterOp::In,
            value: FilterValue::Texts { values },
        } if !values.is_empty() => Ok(Some(values.clone())),
        _ => Err(()),
    }
}

/// Validate the drafts of one router and translate the enabled ones. Issues never stop the
/// other checks, so a page lists every blocker at once.
pub fn translate(
    cfg: &Config,
    router: &RouterCfg,
    policies: &[SourcePolicyDraft],
) -> (RouterIntent, Vec<ValidationIssue>) {
    let mut intent = RouterIntent::default();
    let mut issues = Vec::new();
    let mut dhcp: Vec<(u16, DhcpIntent)> = Vec::new();
    for (i, p) in policies.iter().enumerate() {
        let id = policy_ref(p, i);
        let before = issues.len();
        if p.name.is_empty() || p.name.len() > MAX_POLICY_NAME {
            issues.push(issue(
                "invalid_name",
                Severity::Blocker,
                &id,
                None,
                "Use a name of 1 to 128 bytes.",
            ));
        }
        if p.read_access_ref != DEFAULT_PROFILE
            && !router
                .access_profiles
                .iter()
                .any(|a| a.id == p.read_access_ref)
        {
            issues.push(issue(
                "unknown_access_profile",
                Severity::Blocker,
                &id,
                None,
                "Select an access profile of this router.",
            ));
        } else if p.read_access_ref != DEFAULT_PROFILE {
            // Collection and the cycle read with the router credential only.
            issues.push(blocker(
                &id,
                None,
                "Version 1 reads with the router credential (profile default) only.",
            ));
        }
        if p.writes.iter().any(|w| w.enabled) {
            issues.push(blocker(
                &id,
                None,
                "A source policy cannot enable a router write in version 1.",
            ));
        }
        let applied = matches!(&p.capacity, CapacityRule::FromPlans { minimum_down_bps, minimum_up_bps, .. }
            if minimum_down_bps == "0" && minimum_up_bps == "0");
        if capacity_ok(&p.capacity) && !applied {
            issues.push(blocker(
                &id,
                None,
                "Version 1 applies plan factors only: no fixed capacity and no minimum rate.",
            ));
        }
        if !capacity_ok(&p.capacity) {
            issues.push(issue(
                "invalid_capacity",
                Severity::Blocker,
                &id,
                None,
                "Use positive decimal factors and whole bps rates.",
            ));
        }
        match p.source {
            SourceKind::Hotspot => issues.push(blocker(
                &id,
                None,
                "Hotspot sources have no reference authority in the engine. No identity is invented for them.",
            )),
            SourceKind::Wan => issues.push(blocker(
                &id,
                None,
                "Classify WAN interfaces with WAN decisions, not with a source policy.",
            )),
            SourceKind::Ppp => {
                // The cycle reads only a protocol that the router config enables.
                if p.enabled && !router.pppoe.enabled {
                    issues.push(blocker(
                        &id,
                        None,
                        "Enable PPP in the router settings first: the cycle reads only enabled sources.",
                    ));
                }
                if p.members != MemberMode::Joined {
                    issues.push(blocker(&id, None, "A PPP policy uses joined members in version 1."));
                }
                if !matches!(&p.filter, FilterExpr::All { rules } if rules.is_empty()) {
                    issues.push(blocker(&id, Some(FieldKey::AccountProfile), "A PPP policy has no filter in version 1."));
                }
                let per_plan = match &p.group {
                    GroupRule::ByField { field: FieldKey::AccountProfile } => Some(true),
                    GroupRule::Fixed { .. } => Some(false),
                    _ => None,
                };
                if per_plan.is_none() {
                    issues.push(blocker(&id, None, "Group PPP members by profile, or in one fixed group."));
                }
                match LegacyPriority::ppp(&cfg.speed, &cfg.bandwidth_defaults.pppoe_default_rate) {
                    Ok(priority) if rates_match(p, &expected(&priority)) => {}
                    _ => issues.push(blocker(
                        &id,
                        None,
                        "The rate order must be the configured PPP rate order of the service.",
                    )),
                }
                if p.enabled && issues.len() == before {
                    if intent.ppp.is_some() {
                        issues.push(blocker(&id, None, "A router has one enabled PPP policy in version 1."));
                    } else {
                        intent.ppp = Some(PppIntent {
                            policy_id: id.clone(),
                            per_plan: per_plan == Some(true),
                            parent: p.parent.clone(),
                            capacity: p.capacity.clone(),
                        });
                    }
                }
            }
            SourceKind::Dhcp => {
                let d = &router.dhcp;
                if p.enabled && !(d.enabled && d.require_bound && !d.shape_unassigned) {
                    issues.push(blocker(
                        &id,
                        None,
                        "Set the router DHCP settings first: enabled, require_bound = true, shape_unassigned = false.",
                    ));
                }
                if p.members != MemberMode::Active {
                    issues.push(blocker(&id, None, "A DHCP policy uses active members in version 1."));
                }
                let servers = dhcp_servers(&p.filter);
                if servers.is_err() {
                    issues.push(blocker(&id, Some(FieldKey::LeaseServer), "Filter DHCP members by lease.server only."));
                }
                if !matches!(&p.group, GroupRule::ByField { field: FieldKey::LeaseServer }) {
                    issues.push(blocker(&id, None, "Group DHCP members by server in version 1."));
                }
                // Each server keeps its own configured order; every named server must match.
                // Without a filter, discovered servers without an entry use the defaults ("").
                let names: Vec<String> = match servers.clone() {
                    Ok(Some(names)) => names,
                    _ => router
                        .dhcp
                        .servers
                        .iter()
                        .map(|s| s.name.clone())
                        .chain([String::new()])
                        .collect(),
                };
                for server in names.iter().map(|n| router.dhcp.server(n)) {
                    let mode_ok = server.mode == crate::config::DhcpMode::PerSite;
                    let order = LegacyPriority::dhcp(&cfg.speed, &cfg.bandwidth_defaults, &server);
                    if !mode_ok || !order.is_ok_and(|o| rates_match(p, &expected(&o))) {
                        issues.push(blocker(
                            &id,
                            None,
                            "Each selected DHCP server must be per-site, with the same rate order as the policy.",
                        ));
                        break;
                    }
                }
                if p.enabled && issues.len() == before {
                    dhcp.push((
                        p.priority,
                        DhcpIntent {
                            policy_id: id.clone(),
                            servers: servers.unwrap_or(None),
                            parent: p.parent.clone(),
                            capacity: p.capacity.clone(),
                        },
                    ));
                }
            }
        }
    }
    // A router that does not discover DHCP servers shapes exactly its listed servers; the
    // engine preflight refuses any other set. Name them all, with a server filter.
    if !router.dhcp.discover && !dhcp.is_empty() {
        let declared: BTreeSet<&str> = router
            .dhcp
            .servers
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        let named: Option<BTreeSet<&str>> = dhcp
            .iter()
            .map(|(_, d)| d.servers.as_ref())
            .collect::<Option<Vec<_>>>()
            .map(|lists| lists.into_iter().flatten().map(String::as_str).collect());
        if named.as_ref() != Some(&declared) {
            issues.push(blocker(
                &dhcp[0].1.policy_id,
                Some(FieldKey::LeaseServer),
                "This router lists its DHCP servers and does not discover them: name exactly those servers.",
            ));
        }
    }
    // Equal-priority policies that can own the same server are a blocker. A tie is never
    // broken silently; a lower priority value wins.
    for a in 0..dhcp.len() {
        for b in a + 1..dhcp.len() {
            let overlap = match (&dhcp[a].1.servers, &dhcp[b].1.servers) {
                (Some(x), Some(y)) => x.iter().any(|s| y.contains(s)),
                _ => true,
            };
            if overlap && dhcp[a].0 == dhcp[b].0 {
                issues.push(issue(
                    "priority_overlap",
                    Severity::Blocker,
                    &dhcp[b].1.policy_id,
                    Some(FieldKey::LeaseServer),
                    "Two policies with the same priority can own the same DHCP server.",
                ));
            }
        }
    }
    dhcp.sort_by_key(|(priority, _)| *priority);
    intent.dhcp = dhcp.into_iter().map(|(_, d)| d).collect();
    (intent, issues)
}

/// A decimal factor as an exact fraction: `0.5` is 5/10. At most six decimal places.
fn fraction(text: &str) -> Option<Fraction> {
    let (whole, frac) = text.split_once('.').unwrap_or((text, ""));
    if frac.len() > 6 {
        return None;
    }
    let numerator = format!("{whole}{frac}").parse::<u64>().ok()?;
    (numerator > 0).then(|| Fraction {
        numerator,
        denominator: 10u64.pow(frac.len() as u32),
    })
}

/// The policy that owns one translated scope: the PPP intent, or the first DHCP intent (in
/// priority order) that selects the server.
fn owner<'a>(
    intent: &'a RouterIntent,
    kind: EngineKind,
    server: Option<&str>,
) -> Option<(&'a str, &'a ParentRule, &'a CapacityRule)> {
    match kind {
        EngineKind::Ppp => intent
            .ppp
            .as_ref()
            .map(|p| (p.policy_id.as_str(), &p.parent, &p.capacity)),
        EngineKind::Dhcp => intent
            .dhcp
            .iter()
            .find(|d| {
                d.servers
                    .as_ref()
                    .is_none_or(|s| server.is_some_and(|n| s.iter().any(|x| x == n)))
            })
            .map(|d| (d.policy_id.as_str(), &d.parent, &d.capacity)),
        EngineKind::Hotspot => None,
    }
}

/// The registry clone with the target router translated through the current first-policy
/// builder. Other routers keep their sources, nodes, and selections. Existing nodes keep
/// their place and settings; a source with the same ID keeps its operator settings.
pub fn candidate(
    cfg: &Config,
    doc: &RegistryDocument,
    router: &RouterCfg,
    intent: &RouterIntent,
    current: Option<&CurrentRouter>,
) -> Result<RegistryDocument, String> {
    let rid = router_id(&router.name);
    let mut one = cfg.clone();
    let mut r = router.clone();
    r.hotspot.enabled = false;
    r.pppoe.enabled = intent.ppp.is_some();
    r.pppoe.per_plan_node = intent.ppp.as_ref().is_some_and(|p| p.per_plan);
    r.dhcp.enabled = !intent.dhcp.is_empty();
    if r.dhcp.enabled && intent.dhcp.iter().all(|d| d.servers.is_some()) {
        let names: BTreeSet<String> = intent
            .dhcp
            .iter()
            .flat_map(|d| d.servers.clone().unwrap_or_default())
            .collect();
        r.dhcp.servers = names.iter().map(|n| router.dhcp.server(n)).collect();
        r.dhcp.discover = false;
    }
    one.routers = vec![r];
    let draft = crate::topology_manager::policy_draft::propose(
        &one,
        current.map_or(&[], std::slice::from_ref),
    )
    .map_err(|e| format!("{e:#}"))?;
    let scope_owner: HashMap<String, (EngineKind, Option<String>)> = draft
        .source_settings
        .selections
        .iter()
        .map(|s| {
            (
                s.scope.source_id.clone(),
                (s.scope.kind, s.service_names.first().cloned()),
            )
        })
        .collect();
    let mut c = doc.clone();
    let cat = c
        .catalog
        .as_mut()
        .ok_or("the topology is not initialized")?;
    let ours = |o: &SourceOrigin| matches!(o, SourceOrigin::Subscribers { scope, .. } if scope.router_id == rid);
    let mut old: HashMap<String, crate::topology_manager::catalog::CatalogSource> = HashMap::new();
    cat.sources.retain(|s| {
        if ours(&s.origin) {
            old.insert(s.id.clone(), s.clone());
            false
        } else {
            true
        }
    });
    let used: BTreeSet<String> = cat
        .sources
        .iter()
        .filter_map(|s| s.profile_id.clone())
        .collect();
    let old_profiles: BTreeSet<String> =
        old.values().filter_map(|s| s.profile_id.clone()).collect();
    cat.profiles
        .retain(|p| !old_profiles.contains(&p.id) || used.contains(&p.id));
    for p in draft.catalog.profiles {
        cat.profiles.retain(|x| x.id != p.id);
        cat.profiles.push(p);
    }
    let fresh: BTreeSet<String> = draft.catalog.sources.iter().map(|s| s.id.clone()).collect();
    let rule = |source: &crate::topology_manager::catalog::CatalogSource| {
        let SourceOrigin::Subscribers { scope, .. } = &source.origin else {
            return None;
        };
        let (kind, server) = scope_owner.get(&scope.source_id)?;
        owner(intent, *kind, server.as_deref())
    };
    for s in draft.catalog.sources {
        let (_, _, capacity) = rule(&s).ok_or("a translated source has no policy")?;
        let mut kept = old.remove(&s.id).unwrap_or(s);
        if let (
            SourceOrigin::Subscribers { group, .. },
            CapacityRule::FromPlans {
                down_factor,
                up_factor,
                ..
            },
        ) = (&mut kept.origin, capacity)
        {
            group.factors = Factors {
                down: fraction(down_factor).ok_or("check the down factor")?,
                up: fraction(up_factor).ok_or("check the up factor")?,
            };
        }
        cat.sources.push(kept);
    }
    let removed: BTreeSet<&String> = old.keys().filter(|k| !fresh.contains(*k)).collect();
    cat.nodes
        .retain(|n| n.source_id.as_ref().is_none_or(|s| !removed.contains(s)));
    let router_node = cat
        .nodes
        .iter()
        .find(|n| n.router_id.as_deref() == Some(rid.as_str()))
        .map(|n| n.id.clone());
    let sources = cat.sources.clone();
    for mut n in draft.catalog.nodes {
        if cat.nodes.iter().any(|x| x.id == n.id) {
            continue;
        }
        let source = sources.iter().find(|s| Some(&s.id) == n.source_id.as_ref());
        let (_, parent, _) = source
            .and_then(rule)
            .ok_or("a translated node has no policy")?;
        n.parent = match parent {
            ParentRule::Router => router_node.clone(),
            ParentRule::Node { node_id } if cat.nodes.iter().any(|x| &x.id == node_id) => {
                Some(node_id.clone())
            }
            ParentRule::Node { node_id } => {
                return Err(format!("The parent node {node_id} does not exist."));
            }
        };
        cat.nodes.push(n);
    }
    let settings = c
        .source_settings
        .as_mut()
        .ok_or("the topology has no source settings")?;
    settings.selections.retain(|s| s.scope.router_id != rid);
    settings.selections.extend(draft.source_settings.selections);
    // The subscriber roles follow the translation; a WAN role stays. The cycle reads a
    // protocol only when the catalog router has its role.
    use crate::topology_manager::catalog::RouterRole;
    let roles: Vec<RouterRole> = draft
        .catalog
        .routers
        .iter()
        .flat_map(|r| r.roles.clone())
        .collect();
    let cat = c
        .catalog
        .as_mut()
        .ok_or("the topology is not initialized")?;
    if let Some(r) = cat.routers.iter_mut().find(|r| r.id == rid) {
        r.roles.retain(|x| *x == RouterRole::Wan);
        r.roles
            .extend(roles.into_iter().filter(|x| *x != RouterRole::Wan));
    }
    c.validate().map_err(|e| format!("{e:#}"))?;
    Ok(c)
}

/// One evaluation result without its cache reference.
pub struct Evaluation {
    pub graph: CandidateGraph,
    pub diff: CandidateDiff,
    pub records: Vec<ResolvedRecord>,
    pub issues: Vec<ValidationIssue>,
    /// Mandatory evaluation finished. A blocker can still stop publication.
    pub complete: bool,
}

impl Evaluation {
    fn stopped(issues: Vec<ValidationIssue>) -> Self {
        Self {
            graph: CandidateGraph {
                nodes: vec![],
                flows: vec![],
                nodes_truncated: false,
                flows_truncated: false,
            },
            diff: CandidateDiff {
                added: 0,
                removed: 0,
                changed: 0,
                unchanged: 0,
                markers: vec![],
                markers_truncated: false,
            },
            records: vec![],
            issues,
            complete: false,
        }
    }
    pub fn publishable(&self) -> bool {
        self.complete && !self.issues.iter().any(|i| i.severity == Severity::Blocker)
    }
}

const MAX_NODES: usize = 400;
const MAX_MARKERS: usize = 100;
const EVIDENCE_AGE: Duration = Duration::from_secs(60);

pub(crate) fn tables_for(intent: &RouterIntent) -> Vec<Table> {
    let mut t = Vec::new();
    if intent.ppp.is_some() {
        t.extend([Table::PppSecrets, Table::PppProfiles, Table::PppActive]);
    }
    if !intent.dhcp.is_empty() {
        t.extend([Table::DhcpServers, Table::DhcpLeases]);
    }
    t
}

/// The current evidence of one router from its newest complete snapshot, for a policy save.
/// A partial or failed table is refused: absence is never evidence.
pub(crate) fn saved_evidence(
    cfg: &Config,
    router: &RouterCfg,
    studio: &Studio,
    loaded_digest: &str,
    intent: &RouterIntent,
    now: u64,
) -> Result<CurrentRouter, String> {
    let id = router_id(&router.name);
    // A snapshot of another config (for example another router address) is not evidence.
    let snap = studio
        .latest(&id, now)
        .filter(|s| s.reference.config_digest == loaded_digest)
        .ok_or("collect_router_first: the save uses the newest snapshot of the loaded config")?;
    for t in tables_for(intent) {
        let complete = snap
            .reference
            .tables
            .iter()
            .any(|e| e.table == table_id(t) && e.state == ReadState::Complete);
        if !complete {
            return Err(
                "partial_table: collect every table of the policy sources completely".into(),
            );
        }
    }
    let read = RouterRead::from_tables(&id, 1, snap.tables_snapshot(), EVIDENCE_AGE)
        .map_err(|e| format!("invalid_evidence: {e}"))?;
    crate::topology_manager::normalize::normalize_configured(&read, &cfg.exclude, &router.exclude)
        .map_err(|e| match e {
            NormalizeError::Row {
                table: Table::PppActive,
                issue: Issue::MissingAccount,
                ..
            } => "missing_ppp_join: an active PPP session has no PPP secret".to_string(),
            e => format!("invalid_evidence: {e}"),
        })
}

/// Evaluate the drafts of one router on a clone of the current topology. It reads no router,
/// allocates no durable ID, and changes no registry, gate, routing, or output state.
/// `Err` is a refusal of the request, for example an expired snapshot.
#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate<C: Connector + 'static>(
    cfg: &Config,
    doc: &RegistryDocument,
    coordinator: &Coordinator<C>,
    studio: &Studio,
    loaded_digest: &str,
    id: &str,
    snapshot_id: &str,
    policies: &[SourcePolicyDraft],
    now: u64,
) -> Result<Evaluation, String>
where
    C::Session: 'static,
{
    check_limits(policies)?;
    let router = cfg
        .routers
        .iter()
        .find(|r| router_id(&r.name) == id)
        .ok_or("This router is not in the loaded configuration.")?;
    let snapshot = studio
        .snapshot(snapshot_id, now)
        .filter(|s| s.reference.router_id == id && s.reference.config_digest == loaded_digest)
        .ok_or("The snapshot expired, belongs to another router, or has another config. Collect again.")?;
    let (intent, mut issues) = translate(cfg, router, policies);
    if issues.iter().any(|i| i.severity == Severity::Blocker) {
        return Ok(Evaluation::stopped(issues));
    }
    // A partial or failed mandatory table stops the evaluation: absence is not evidence.
    let needed = tables_for(&intent);
    for t in &needed {
        let e = snapshot
            .reference
            .tables
            .iter()
            .find(|e| e.table == table_id(*t));
        if let Some(e) = e.filter(|e| e.state != ReadState::Complete) {
            let code = if e.state == ReadState::Partial {
                "partial_table"
            } else {
                "failed_table"
            };
            issues.push(issue(
                code,
                Severity::Blocker,
                &format!(
                    "table:{}",
                    serde_json::to_value(e.table)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_default()
                ),
                None,
                "Collect this table completely, then evaluate again.",
            ));
        }
    }
    if issues.iter().any(|i| i.severity == Severity::Blocker) {
        return Ok(Evaluation::stopped(issues));
    }
    let rows = |c: &super::snapshots::Collected| -> BTreeMap<Table, Vec<RosRow>> {
        needed
            .iter()
            .filter_map(|t| Some((*t, c.rows(*t)?.to_vec())))
            .collect()
    };
    let target = rows(snapshot);
    let current = RouterRead::from_tables(id, 1, target.clone(), EVIDENCE_AGE)
        .map_err(NormalizeError::Evidence)
        .and_then(|read| {
            crate::topology_manager::normalize::normalize_configured(
                &read,
                &cfg.exclude,
                &router.exclude,
            )
        });
    let current = match current {
        Ok(c) => c,
        Err(NormalizeError::Row {
            table: Table::PppActive,
            index,
            issue: Issue::MissingAccount,
        }) => {
            issues.push(issue(
                "missing_ppp_join",
                Severity::Blocker,
                &format!("table:ppp_active#{index}"),
                None,
                "An active PPP session has no PPP secret. A joined member needs both.",
            ));
            return Ok(Evaluation::stopped(issues));
        }
        Err(e) => {
            issues.push(issue(
                "invalid_evidence",
                Severity::Blocker,
                "snapshot",
                None,
                &format!("The snapshot is not usable: {e}."),
            ));
            return Ok(Evaluation::stopped(issues));
        }
    };
    let candidate = match candidate(cfg, doc, router, &intent, Some(&current)) {
        Ok(c) => c,
        Err(m) => {
            issues.push(issue("engine_refused", Severity::Blocker, id, None, &m));
            return Ok(Evaluation::stopped(issues));
        }
    };
    // Every router with a selected scope needs evidence. Others come from their own snapshot.
    let mut evidence = vec![(id.to_owned(), target)];
    let others: BTreeSet<String> = candidate
        .source_settings
        .iter()
        .flat_map(|s| s.selections.iter().map(|x| x.scope.router_id.clone()))
        .filter(|r| r != id)
        .collect();
    for other in others {
        match studio
            .latest(&other, now)
            .filter(|c| c.reference.config_digest == loaded_digest)
        {
            Some(c) => evidence.push((other, c.tables_snapshot())),
            None => issues.push(issue("collect_router_first", Severity::Blocker, &other, None, "Collect this router, then evaluate again. The evaluation uses its current evidence.")),
        }
    }
    if issues.iter().any(|i| i.severity == Severity::Blocker) {
        return Ok(Evaluation::stopped(issues));
    }
    let pair = match coordinator.prepare_with_evidence(cfg, &candidate, evidence, EVIDENCE_AGE) {
        Ok(p) => p,
        Err(e) => {
            if let Some(c) = e.chain().find_map(|c| {
                c.downcast_ref::<crate::topology_manager::conflicts::PreparationConflicts>()
            }) {
                for x in &c.0 {
                    issues.push(issue(
                        "conflict",
                        Severity::Blocker,
                        &x.subject,
                        None,
                        &x.detail,
                    ));
                }
            } else {
                issues.push(issue(
                    "engine_refused",
                    Severity::Blocker,
                    id,
                    None,
                    &format!("{e:#}"),
                ));
            }
            let mut stopped = Evaluation::stopped(issues);
            stopped.complete = true;
            return Ok(stopped);
        }
    };
    Ok(project(
        doc,
        &candidate,
        &pair,
        id,
        snapshot_id,
        &intent,
        issues,
        now,
    ))
}

fn kind_name(k: EngineKind) -> &'static str {
    match k {
        EngineKind::Ppp => "ppp",
        EngineKind::Dhcp => "dhcp",
        EngineKind::Hotspot => "hotspot",
    }
}

/// The engine view as contract types. A node that the saved catalog does not have gets a
/// `draft:` ID; a record never shows a candidate subscriber ID.
#[allow(clippy::too_many_arguments)]
fn project(
    doc: &RegistryDocument,
    candidate: &RegistryDocument,
    pair: &crate::topology_manager::publication::PreparedPair,
    id: &str,
    snapshot_id: &str,
    intent: &RouterIntent,
    mut issues: Vec<ValidationIssue>,
    now: u64,
) -> Evaluation {
    let view = pair.view();
    let value = serde_json::json!({
        "collected_at": now, "registry_revision": candidate.revision,
        "generation": pair.generation(), "state": "ok", "nodes": view.nodes,
        "subscribers": view.subscribers, "inventory": view.inventory,
        "conflicts": [], "rounded": [],
    });
    let routers = BTreeSet::new();
    let projected =
        crate::topology_manager::web_view::project(&crate::topology_manager::web_view::Input {
            doc: Some(candidate),
            preview: Some(&value),
            collection: None,
            mode: "shadow",
            writer: "legacy",
            cycle_seq: 0,
            now,
            fresh_for: 0,
            publication: None,
            history: &[],
            layout: None,
            wan: None,
            wan_lists: None,
            activity: None,
            grace: 0,
            routing_moves: 0,
            routing: None,
            wan_discovery: None,
            wan_routers: &routers,
        });
    let nodes_of = |d: &RegistryDocument| -> BTreeMap<String, (String, Option<String>)> {
        d.catalog
            .iter()
            .flat_map(|c| {
                c.nodes
                    .iter()
                    .map(|n| (n.id.clone(), (n.name.clone(), n.parent.clone())))
            })
            .collect()
    };
    let (before, after) = (nodes_of(doc), nodes_of(candidate));
    let fresh: BTreeSet<&String> = after.keys().filter(|k| !before.contains_key(*k)).collect();
    let draft = |x: &str| {
        if fresh.contains(&x.to_owned()) {
            format!("draft:{x}")
        } else {
            x.to_owned()
        }
    };
    let mut nodes = projected.nodes;
    for n in &mut nodes {
        n.id = draft(&n.id);
        n.parent = n.parent.as_deref().map(draft);
        n.card_id = draft(&n.card_id);
    }
    let mut flows = projected.flows;
    for f in &mut flows {
        f.from = draft(&f.from);
        f.to = draft(&f.to);
    }
    let nodes_truncated = nodes.len() > MAX_NODES;
    nodes.truncate(MAX_NODES);
    let flows_truncated = flows.len() > MAX_NODES;
    flows.truncate(MAX_NODES);
    let mut diff = CandidateDiff {
        added: 0,
        removed: 0,
        changed: 0,
        unchanged: 0,
        markers: vec![],
        markers_truncated: false,
    };
    let mark = |diff: &mut CandidateDiff, entity: String, change: ChangeKind| {
        if diff.markers.len() < MAX_MARKERS {
            diff.markers.push(DiffMarker {
                entity_ref: entity,
                change,
            });
        } else {
            diff.markers_truncated = true;
        }
    };
    for (node, value) in &after {
        match before.get(node) {
            None => {
                diff.added += 1;
                mark(&mut diff, draft(node), ChangeKind::Added);
            }
            Some(old) if old != value => {
                diff.changed += 1;
                mark(&mut diff, node.clone(), ChangeKind::Changed);
            }
            Some(_) => diff.unchanged += 1,
        }
    }
    for node in before.keys().filter(|k| !after.contains_key(*k)) {
        diff.removed += 1;
        mark(&mut diff, node.clone(), ChangeKind::Removed);
    }
    // The selected server of each translated scope, for the policy of a record.
    let servers: HashMap<&str, Option<&str>> = candidate
        .source_settings
        .iter()
        .flat_map(|s| s.selections.iter())
        .filter(|s| s.scope.router_id == id)
        .map(|s| {
            (
                s.scope.source_id.as_str(),
                s.service_names.first().map(String::as_str),
            )
        })
        .collect();
    let parents: HashMap<&str, Option<&String>> = after
        .iter()
        .map(|(k, v)| (k.as_str(), v.1.as_ref()))
        .collect();
    let mut records = Vec::new();
    let mut new_records = 0;
    for sub in &view.subscribers {
        let Some(ident) = sub.identities.iter().find(|i| i.router_id == id) else {
            continue;
        };
        let saved = doc.subscribers.iter().find(|s| s.id == sub.id);
        let mut codes = Vec::new();
        if saved.is_none() {
            codes.push("new_identity".to_string());
            new_records += 1;
        }
        let server = servers.get(ident.source_id.as_str()).copied().flatten();
        let policy = owner(intent, ident.kind, server)
            .map(|(p, _, _)| p.to_owned())
            .unwrap_or_default();
        let bps = |mbps: f64| Some(format!("{:.0}", mbps * 1_000_000.0));
        records.push(ResolvedRecord {
            record_ref: ident.account_ref.clone(),
            policy_id: policy,
            identity_ref: Some(web_contract::topology::Identity {
                router_id: ident.router_id.clone(),
                source_id: ident.source_id.clone(),
                kind: kind_name(ident.kind).into(),
                account_ref: ident.account_ref.clone(),
            }),
            group_ref: Some(draft(&sub.node)),
            parent_ref: parents
                .get(sub.node.as_str())
                .copied()
                .flatten()
                .map(|p| draft(p)),
            down_bps: bps(sub.max_mbps.0),
            up_bps: bps(sub.max_mbps.1),
            // The engine reports the winning source only for a subscriber plan.
            speed_source: saved
                .and_then(|s| s.policy.profile_id.as_ref())
                .map(|_| RateKind::SubscriberOverride),
            provenance: vec![Provenance {
                router_id: id.into(),
                table: table_id(if ident.kind == EngineKind::Ppp {
                    Table::PppSecrets
                } else {
                    Table::DhcpLeases
                }),
                snapshot_id: snapshot_id.into(),
                record_ref: ident.account_ref.clone(),
                fields: vec![],
            }],
            issue_codes: codes,
        });
    }
    if new_records > 0 {
        issues.push(issue(
            "new_identity",
            Severity::Information,
            id,
            None,
            &format!(
                "A save adds {new_records} new subscriber identities. Their IDs come at save time."
            ),
        ));
    }
    Evaluation {
        graph: CandidateGraph {
            nodes,
            flows,
            nodes_truncated,
            flows_truncated,
        },
        diff,
        records,
        issues,
        complete: true,
    }
}

#[cfg(test)]
#[path = "evaluate_tests.rs"]
mod tests;
