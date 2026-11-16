//! Durable selectors and authority. Current source evidence stays in memory.

use super::catalog::{SourceOrigin, TopologyCatalog};
use super::identity::ReferenceAuthority;
use super::identity::SourceSelection;
use super::model::SourceKind;
use anyhow::ensure;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSettings {
    pub selections: Vec<SourceSelection>,
}
impl SourceSettings {
    pub fn validate(&self, catalog: &TopologyCatalog) -> anyhow::Result<()> {
        let expected = catalog.required_scopes()?;
        let mut scopes = BTreeSet::new();
        let mut services: HashMap<_, HashSet<&str>> = HashMap::new();
        let mut whole = HashSet::new();
        let mut by_scope = HashMap::new();
        for selection in &self.selections {
            selection.scope.validate()?;
            ensure!(
                scopes.insert(selection.scope.clone()),
                "duplicate configured source scope"
            );
            selection.policy.validate(selection.scope.kind)?;
            by_scope.insert(
                &selection.scope,
                selection
                    .policy
                    .profiles
                    .iter()
                    .map(|p| p.source_ref.as_str())
                    .collect::<HashSet<_>>(),
            );
            let key = (selection.scope.router_id.as_str(), selection.scope.kind);
            if selection.service_names.is_empty() {
                ensure!(
                    whole.insert(key) && !services.contains_key(&key),
                    "overlapping configured service selections"
                );
            } else {
                ensure!(
                    !whole.contains(&key),
                    "overlapping configured service selections"
                );
                let names = services.entry(key).or_default();
                for name in &selection.service_names {
                    ensure!(!name.trim().is_empty(), "empty current service selector");
                    if selection.scope.kind == SourceKind::Ppp {
                        ensure!(
                            matches!(
                                name.as_str(),
                                "async" | "isdn" | "l2tp" | "pppoe" | "pptp" | "ovpn" | "sstp"
                            ),
                            "unknown PPP service selector"
                        );
                    }
                    ensure!(
                        names.insert(name.as_str()),
                        "duplicate or overlapping current service selector"
                    );
                }
            }
        }
        ensure!(
            scopes.into_iter().collect::<Vec<_>>() == expected,
            "configured source scopes do not match the catalog"
        );
        let selected: HashSet<_> = catalog
            .nodes
            .iter()
            .filter_map(|n| n.source_id.as_deref())
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
                ensure!(
                    by_scope
                        .get(scope)
                        .is_some_and(|references| references.contains(reference.as_str())),
                    "selected catalog profile requires a configured source binding"
                );
            }
        }
        Ok(())
    }

    pub fn ready_selections(
        &self,
        catalog: &TopologyCatalog,
    ) -> anyhow::Result<&[SourceSelection]> {
        self.validate(catalog)?;
        ensure!(
            !self.selections.is_empty(),
            "no configured subscriber sources"
        );
        ensure!(
            self.selections
                .iter()
                .all(|s| s.authority != ReferenceAuthority::Unconfigured),
            "source reference authority is unconfigured"
        );
        Ok(&self.selections)
    }

    pub(super) fn canonicalize(&mut self) {
        self.selections
            .sort_unstable_by(|a, b| a.scope.cmp(&b.scope));
        for selection in &mut self.selections {
            selection.service_names.sort_unstable();
            selection
                .policy
                .profiles
                .sort_unstable_by(|a, b| a.source_ref.cmp(&b.source_ref));
        }
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
