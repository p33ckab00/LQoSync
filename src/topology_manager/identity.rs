//! Independent operator references authorize continuity. Current locators never do so by themselves.

use super::catalog::SourceOrigin;
use super::model::{IdentityRef, RegistryDocument, SourceKind, SourceScope};
use super::normalize::{AuthorityEvidence, CurrentRouter};
use super::observation::{
    AccountObservation, ObservationGate, Reconciliation, SourceRead, reconcile,
};
use super::source_policy::{SourceIndex, SourcePolicy, SourcePolicyIssue};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use tokio::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceAuthority {
    Unconfigured,
    /// The operator assigns independent references and never reuses them for another subscriber.
    OperatorManagedComment,
    /// The operator asserts that a PPP secret name is never reused for another subscriber.
    PppSecretName,
    /// The operator asserts that each client on the selected DHCP scope keeps one stable MAC.
    DhcpClientMac,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSelection {
    pub scope: SourceScope,
    /// Explicit service names. An empty list selects the whole router/protocol.
    pub service_names: Vec<String>,
    pub authority: ReferenceAuthority,
    pub policy: SourcePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IdentityIssue {
    UnconfiguredAuthority,
    MissingReference,
    InvalidReference,
    DuplicateReference,
    AmbiguousScope,
    PartialServiceMembership,
    MissingProfileMapping,
    InvalidSourceRate,
}

#[derive(Debug, PartialEq, Eq)]
pub struct IdentityConflict {
    pub scope: SourceScope,
    pub row_ref: String,
    pub issue: IdentityIssue,
    /// The identity reference when it is known, for example for a duplicate.
    pub account: Option<String>,
}

pub struct Authorization<'a> {
    registry: &'a RegistryDocument,
    generation: u64,
    required: Vec<SourceScope>,
    expires: Instant,
    conflicts: Vec<IdentityConflict>,
    reads: Option<Vec<SourceRead>>,
}

pub struct AuthorizedReconciliation {
    pub reconciliation: Reconciliation,
    /// Keep this deadline through later placement and publication checks.
    pub expires: Instant,
    /// Router name to (latitude, longitude) from `config.toml`. Empty when no router has a location.
    pub router_locations: HashMap<String, (f64, f64)>,
}

impl Authorization<'_> {
    pub fn conflicts(&self) -> &[IdentityConflict] {
        &self.conflicts
    }
    pub fn is_ready(&self) -> bool {
        self.reads.is_some() && Instant::now() < self.expires
    }
    pub fn reconcile(self, gate: &mut ObservationGate) -> anyhow::Result<AuthorizedReconciliation> {
        ensure!(
            Instant::now() < self.expires,
            "identity evidence expired before acceptance"
        );
        let Some(reads) = self.reads else {
            anyhow::bail!(super::conflicts::PreparationConflicts(
                self.conflicts
                    .iter()
                    .map(|c| super::conflicts::ConflictView {
                        kind: "identity",
                        subject: match &c.account {
                            Some(a) => format!("{} ({}/{})", a, c.scope.source_id, c.row_ref),
                            None => format!("{}/{}", c.scope.source_id, c.row_ref),
                        },
                        detail: format!("{:?}", c.issue),
                    })
                    .collect()
            ));
        };
        let snapshot = gate.accept(self.generation, &self.required, reads)?;
        ensure!(
            Instant::now() < self.expires,
            "identity evidence expired during acceptance"
        );
        let reconciliation = reconcile(self.registry, snapshot)?;
        ensure!(
            Instant::now() < self.expires,
            "identity evidence expired during reconciliation"
        );
        Ok(AuthorizedReconciliation {
            reconciliation,
            expires: self.expires,
            router_locations: HashMap::new(),
        })
    }
}

/// Authorize only the source settings stored with this exact registry revision.
pub fn authorize_configured(
    registry: &RegistryDocument,
    expected_revision: u64,
    generation: u64,
    current: Vec<CurrentRouter>,
) -> anyhow::Result<Authorization<'_>> {
    ensure!(
        registry.revision == expected_revision,
        "identity authorization revision conflict"
    );
    let settings = registry
        .source_settings
        .as_ref()
        .context("source settings are unconfigured")?;
    let catalog = registry
        .catalog
        .as_ref()
        .context("source settings require a catalog")?;
    let selections = settings.ready_selections(catalog)?;
    authorize(registry, expected_revision, generation, current, selections)
}

pub fn authorize<'a>(
    registry: &'a RegistryDocument,
    expected_revision: u64,
    generation: u64,
    current: Vec<CurrentRouter>,
    selections: &[SourceSelection],
) -> anyhow::Result<Authorization<'a>> {
    ensure!(
        registry.revision == expected_revision,
        "identity authorization revision conflict"
    );
    let index = registry.index()?;
    ensure!(generation > 0, "identity generation must be positive");
    ensure!(
        !selections.is_empty(),
        "identity authorization requires selected sources"
    );
    let mut source_indices = HashMap::with_capacity(current.len());
    for router in &current {
        ensure!(
            router.generation() == generation,
            "mixed current collection generations"
        );
        ensure!(
            source_indices
                .insert(router.router_id(), SourceIndex::new(router)?)
                .is_none(),
            "duplicate current router evidence"
        );
    }
    let mut required = BTreeSet::new();
    let mut routers = BTreeSet::new();
    let mut by_kind = HashSet::new();
    let mut all_services = HashMap::new();
    let mut by_service = HashMap::new();
    let mut by_router = HashMap::new();
    let mut service_sets = Vec::with_capacity(selections.len());
    for (position, selection) in selections.iter().enumerate() {
        selection.scope.validate()?;
        ensure!(
            required.insert(selection.scope.clone()),
            "duplicate selected source scope"
        );
        routers.insert(selection.scope.router_id.as_str());
        by_router
            .entry(selection.scope.router_id.as_str())
            .or_insert_with(Vec::new)
            .push(position);
        by_kind.insert((selection.scope.router_id.as_str(), selection.scope.kind));
        let mut services = HashSet::new();
        let source_index = source_indices
            .get(selection.scope.router_id.as_str())
            .context("selected current router evidence is missing")?;
        for service in &selection.service_names {
            ensure!(!service.trim().is_empty(), "empty current service selector");
            ensure!(
                services.insert(source_index.service_ref(selection.scope.kind, service)?),
                "duplicate current service selector"
            );
        }
        service_sets.push(services);
    }
    for (position, services) in service_sets.iter().enumerate() {
        let selection = &selections[position];
        for service in services {
            by_service
                .entry((
                    selection.scope.router_id.as_str(),
                    selection.scope.kind,
                    service.as_str(),
                ))
                .or_insert_with(Vec::new)
                .push(position);
        }
        if services.is_empty() {
            all_services
                .entry((selection.scope.router_id.as_str(), selection.scope.kind))
                .or_insert_with(Vec::new)
                .push(position);
        }
    }
    let required: Vec<_> = required.into_iter().collect();
    if let Some(expected) = index.required_scopes {
        ensure!(
            expected == required,
            "identity source scopes do not match the configured catalog"
        );
    }
    ensure!(
        current.len() == routers.len(),
        "current router evidence is missing or duplicated"
    );
    let mut seen_routers = HashSet::new();
    let mut expires = None;
    for router in &current {
        ensure!(
            routers.contains(router.router_id()),
            "unexpected current router evidence"
        );
        ensure!(
            seen_routers.insert(router.router_id()),
            "duplicate current router evidence"
        );
        ensure!(
            router.generation() == generation,
            "mixed current collection generations"
        );
        router.accounts()?;
        expires = Some(expires.map_or(router.expires(), |previous: Instant| {
            previous.min(router.expires())
        }));
        for kind in [SourceKind::Ppp, SourceKind::Dhcp, SourceKind::Hotspot] {
            if by_kind.contains(&(router.router_id(), kind)) {
                ensure!(router.was_read(kind), "selected protocol was not read");
            }
        }
    }
    let expires = expires.context("current router evidence is missing")?;
    let mut accounts: Vec<Vec<AccountObservation>> =
        (0..selections.len()).map(|_| Vec::new()).collect();
    let mut conflicts = Vec::new();
    let mut claimed = HashMap::new();
    let required_profiles = required_profile_refs(registry);
    for router in &current {
        let profile_index = &source_indices[router.router_id()];
        let mut policies = HashMap::new();
        for &position in &by_router[router.router_id()] {
            let selection = &selections[position];
            let mut policy = selection
                .policy
                .compile(selection.scope.kind, profile_index)?;
            if registry
                .catalog
                .as_ref()
                .is_some_and(|c| c.fallback.is_some())
            {
                policy = policy.accept_unknown();
            }
            if let Some(references) = required_profiles.get(&selection.scope) {
                ensure!(
                    references
                        .iter()
                        .all(|reference| policy.has_reference(reference)),
                    "selected catalog profile requires an explicit source binding"
                );
            }
            policies.insert(position, policy);
        }
        for account in router.accounts()? {
            let key = (router.router_id(), account.kind);
            if !by_kind.contains(&key) {
                continue;
            }
            let mut choices = all_services.get(&key).cloned().unwrap_or_default();
            for service in &account.service_refs {
                if let Some(positions) =
                    by_service.get(&(router.router_id(), account.kind, service.as_str()))
                {
                    choices.extend(positions);
                }
            }
            choices.sort_unstable();
            choices.dedup();
            let mut matched = None;
            let mut ambiguous = false;
            for position in choices {
                let services = &service_sets[position];
                let count = account
                    .service_refs
                    .iter()
                    .filter(|s| services.contains(s.as_str()))
                    .count();
                if !services.is_empty() && count == 0 {
                    continue;
                }
                if !services.is_empty() && count != account.service_refs.len() {
                    conflicts.push(conflict(
                        &selections[position],
                        &account.row_ref,
                        IdentityIssue::PartialServiceMembership,
                    ));
                    ambiguous = true;
                } else if let Some(previous) = matched {
                    conflicts.push(conflict(
                        &selections[previous],
                        &account.row_ref,
                        IdentityIssue::AmbiguousScope,
                    ));
                    conflicts.push(conflict(
                        &selections[position],
                        &account.row_ref,
                        IdentityIssue::AmbiguousScope,
                    ));
                    ambiguous = true;
                } else {
                    matched = Some(position);
                }
            }
            if ambiguous {
                continue;
            }
            let Some(position) = matched else {
                continue;
            };
            let selection = &selections[position];
            if selection.authority == ReferenceAuthority::Unconfigured {
                conflicts.push(conflict(
                    selection,
                    &account.row_ref,
                    IdentityIssue::UnconfiguredAuthority,
                ));
                continue;
            }
            let account_ref = match account_ref(selection.authority, account) {
                Ok(account_ref) => account_ref,
                Err(issue) => {
                    conflicts.push(conflict(selection, &account.row_ref, issue));
                    continue;
                }
            };
            let identity = IdentityRef {
                scope: selection.scope.clone(),
                account_ref,
            };
            if let Some(previous) = claimed.insert(identity.clone(), account.row_ref.as_str()) {
                for row in [previous, account.row_ref.as_str()] {
                    let mut c = conflict(selection, row, IdentityIssue::DuplicateReference);
                    c.account = Some(identity.account_ref.clone());
                    conflicts.push(c);
                }
                continue;
            }
            let policy =
                match policies[&position].observe_identified(account, &identity.account_ref) {
                    Ok(policy) => policy,
                    Err(issue) => {
                        conflicts.push(conflict(
                            selection,
                            &account.row_ref,
                            match issue {
                                SourcePolicyIssue::MissingProfileMapping => {
                                    IdentityIssue::MissingProfileMapping
                                }
                                SourcePolicyIssue::InvalidSourceRate => {
                                    IdentityIssue::InvalidSourceRate
                                }
                            },
                        ));
                        continue;
                    }
                };
            accounts[position].push(AccountObservation {
                account_ref: identity.account_ref,
                endpoints: account.endpoints.clone(),
                policy,
            });
        }
    }
    conflicts.sort_unstable_by(|a, b| {
        (&a.scope, &a.row_ref, a.issue).cmp(&(&b.scope, &b.row_ref, b.issue))
    });
    conflicts.dedup();
    let reads = if conflicts.is_empty() {
        Some(
            selections
                .iter()
                .zip(accounts)
                .map(|(selection, mut accounts)| {
                    accounts.sort_unstable_by(|a, b| a.account_ref.cmp(&b.account_ref));
                    SourceRead::Complete {
                        scope: selection.scope.clone(),
                        accounts,
                    }
                })
                .collect(),
        )
    } else {
        None
    };
    ensure!(
        Instant::now() < expires,
        "identity evidence expired during authorization"
    );
    Ok(Authorization {
        registry,
        generation,
        required,
        expires,
        conflicts,
        reads,
    })
}

fn required_profile_refs(registry: &RegistryDocument) -> HashMap<&SourceScope, HashSet<&str>> {
    let mut required = HashMap::new();
    let Some(catalog) = &registry.catalog else {
        return required;
    };
    let selected: HashSet<_> = catalog
        .nodes
        .iter()
        .filter_map(|node| node.source_id.as_deref())
        .collect();
    for source in &catalog.sources {
        if source.targets.is_empty() && !selected.contains(source.id.as_str()) {
            continue;
        }
        if let SourceOrigin::Subscribers {
            scope,
            profile_ref: Some(reference),
            ..
        } = &source.origin
        {
            required
                .entry(scope)
                .or_insert_with(HashSet::new)
                .insert(reference.as_str());
        }
    }
    required
}

fn conflict(selection: &SourceSelection, row_ref: &str, issue: IdentityIssue) -> IdentityConflict {
    IdentityConflict {
        scope: selection.scope.clone(),
        row_ref: row_ref.into(),
        issue,
        account: None,
    }
}

fn account_ref(
    authority: ReferenceAuthority,
    account: &super::normalize::CurrentAccount,
) -> Result<String, IdentityIssue> {
    match authority {
        ReferenceAuthority::Unconfigured => Err(IdentityIssue::UnconfiguredAuthority),
        ReferenceAuthority::OperatorManagedComment => {
            let comment = match &account.authority {
                AuthorityEvidence::LocalAccount { comment, .. }
                | AuthorityEvidence::EndpointOnly { comment, .. } => comment,
            };
            reference(comment.as_deref()).map(|r| format!("operator-comment/v1/{r}"))
        }
        ReferenceAuthority::PppSecretName => match &account.authority {
            AuthorityEvidence::LocalAccount { name, .. } if !name.is_empty() => {
                Ok(format!("ppp-name/v1/{}", encode_name(name)))
            }
            _ => Err(IdentityIssue::MissingReference),
        },
        ReferenceAuthority::DhcpClientMac => {
            let macs: BTreeSet<_> = account.endpoints.iter().map(|e| e.mac.as_deref()).collect();
            match macs.into_iter().collect::<Vec<_>>()[..] {
                [Some(mac)] => Ok(format!("dhcp-mac/v1/{mac}")),
                [] | [None] => Err(IdentityIssue::MissingReference),
                _ => Err(IdentityIssue::InvalidReference),
            }
        }
    }
}

/// Encode a source name for an identity reference. `%` is encoded, so distinct names stay distinct.
pub(crate) fn encode_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() || b"._-".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn reference(comment: Option<&str>) -> Result<&str, IdentityIssue> {
    const MARKER: &str = "[lqosync-id:";
    let (_, tail) = comment
        .unwrap_or_default()
        .split_once(MARKER)
        .ok_or(IdentityIssue::MissingReference)?;
    let (value, suffix) = tail
        .split_once(']')
        .ok_or(IdentityIssue::InvalidReference)?;
    if suffix.contains(MARKER)
        || value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        || !value.as_bytes()[0].is_ascii_alphanumeric()
        || !value.as_bytes()[value.len() - 1].is_ascii_alphanumeric()
        || value.parse::<std::net::IpAddr>().is_ok()
        || crate::config::normalize_mac(value).is_some()
    {
        return Err(IdentityIssue::InvalidReference);
    }
    Ok(value)
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
