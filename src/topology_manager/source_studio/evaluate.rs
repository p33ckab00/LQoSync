//! Studio evaluation (`StudioJob::Evaluate`). Owner decision (task B3): a draft is translated
//! into the current engine structures; the engine runs on a clone. This module holds the
//! validation and the translation. A draft that the engine cannot express is a blocker.
use crate::config::{Config, DEFAULT_PROFILE, RouterCfg};
use crate::topology_manager::legacy_rate::{LegacyPriority, LegacyRateSource};
use web_contract::source_studio::{
    CapacityRule, CommentRateFormat, FieldKey, FilterExpr, FilterOp, FilterValue, GroupRule,
    MAX_FILTER_DEPTH, MAX_FILTER_LEAVES, MAX_PLAN_BINDINGS, MAX_POLICIES, MAX_POLICY_NAME,
    MAX_POLICY_SET_BYTES, MemberMode, ParentRule, RateKind, RateSource, Severity, SourceKind,
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
        }
        if p.writes.iter().any(|w| w.enabled) {
            issues.push(blocker(
                &id,
                None,
                "A source policy cannot enable a router write in version 1.",
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

#[cfg(test)]
#[path = "evaluate_tests.rs"]
mod tests;
