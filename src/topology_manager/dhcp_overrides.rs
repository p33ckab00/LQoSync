//! Explicit legacy rate-entry bindings. No endpoint participates in subscriber selection.
use super::{
    catalog::{BaseRateAuthority, SourceOrigin},
    identity::SourceSelection,
    model::{IdentityRef, RegistryDocument, SourceKind},
    preflight::{PolicyIssue, PolicyIssueKind},
    rates::RateBps,
};
use crate::config::RouterCfg;
use std::collections::{HashMap, HashSet};

pub(super) fn issues(
    registry: &RegistryDocument,
    selections: &[(&SourceSelection, &RouterCfg)],
) -> Vec<PolicyIssue> {
    if selections.iter().all(|(s, router)| {
        s.scope.kind != SourceKind::Dhcp
            || (router.dhcp.clients.is_empty() && s.policy.dhcp_overrides.is_empty())
    }) {
        return Vec::new();
    }
    let catalog = registry
        .catalog
        .as_ref()
        .expect("selected catalog was checked");
    let identities: HashMap<_, _> = registry
        .subscribers
        .iter()
        .flat_map(|s| s.identities.iter().map(move |id| (id, s)))
        .collect();
    let profiles: HashMap<_, _> = catalog
        .profiles
        .iter()
        .map(|p| (p.id.as_str(), p.base))
        .collect();
    let selected: HashSet<_> = catalog
        .nodes
        .iter()
        .filter_map(|node| node.source_id.as_deref())
        .collect();
    let mut defaults: HashMap<_, (bool, bool)> = HashMap::new();
    for source in &catalog.sources {
        if (selected.contains(source.id.as_str()) || !source.targets.is_empty())
            && let SourceOrigin::Subscribers { scope, .. } = &source.origin
        {
            let state = defaults.entry(scope).or_insert((false, true));
            if let Some(id) = source.profile_id.as_deref() {
                state.0 = true;
                state.1 &= profiles.get(id) == Some(&BaseRateAuthority::SourceRead);
            }
        }
    }
    let mut by_router: HashMap<&str, Vec<_>> = HashMap::new();
    for &(selection, router) in selections {
        if selection.scope.kind == SourceKind::Dhcp {
            by_router
                .entry(&selection.scope.router_id)
                .or_default()
                .push((selection, router));
        }
    }
    let mut out = Vec::new();
    for scopes in by_router.values() {
        let clients = &scopes[0].1.dhcp.clients;
        let mut covered = vec![false; clients.len()];
        let mut invalid = false;
        for &(selection, _) in scopes {
            for binding in &selection.policy.dhcp_overrides {
                let Some(client) = clients.get(binding.legacy_client_index) else {
                    invalid = true;
                    continue;
                };
                let slot = &mut covered[binding.legacy_client_index];
                invalid |= *slot;
                *slot = true;
                invalid |= RateBps::try_from(crate::speed::Rate {
                    down: client.download_mbps,
                    up: client.upload_mbps,
                })
                .map_or(true, |rate| rate != binding.rate);
                let identity = IdentityRef {
                    scope: selection.scope.clone(),
                    account_ref: binding.account_ref.clone(),
                };
                let Some(subscriber) = identities.get(&identity) else {
                    invalid = true;
                    continue;
                };
                let source_read = if let Some(id) = subscriber.policy.profile_id.as_deref() {
                    profiles.get(id) == Some(&BaseRateAuthority::SourceRead)
                } else {
                    defaults.get(&selection.scope) == Some(&(true, true))
                };
                invalid |= !source_read;
            }
        }
        if invalid || covered.iter().any(|covered| !covered) {
            for &(selection, _) in scopes {
                out.push(PolicyIssue {
                    scope: selection.scope.clone(),
                    subject: "routers.dhcp.clients".into(),
                    kind: PolicyIssueKind::DhcpAddressOverrides,
                });
            }
        }
    }
    out
}
