//! A first topology policy that mirrors the legacy configuration. The operator reviews it before install.
//! Node IDs and labels reuse the legacy `root_promotion` group IDs and `[nodes]` templates.

use super::catalog::{
    BackstopPolicy, BaseRateAuthority, CapacityPolicy, CatalogNode, CatalogRouter, CatalogSource,
    FactorChoice, Factors, GroupDefaults, GroupFormula, RouterRole, SourceOrigin,
    SubscriberProfile, TopologyCatalog,
};
use super::identity::{ReferenceAuthority, SourceSelection};
use super::layout::NodeKind;
use super::model::{RegistryDocument, SourceKind, SourceScope};
use super::normalize::CurrentRouter;
use super::rates::{Fraction, NodeMinimum, RateBps};
use super::settings::SourceSettings;
use super::source_policy::{ProfileBinding, SourcePolicy};
use crate::config::{Config, DhcpMode, NetworkMode, RouterCfg};
use crate::sources::GroupKey;
use crate::topology::{group_node_id, group_node_name, node_id};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDraft {
    pub catalog: TopologyCatalog,
    pub source_settings: SourceSettings,
}

pub(crate) fn router_id(name: &str) -> String {
    node_id("router", name, "")
}

/// A catalog with only the configured routers. It is enough to plan a read before a policy exists.
pub fn read_catalog(cfg: &Config) -> TopologyCatalog {
    TopologyCatalog {
        routers: cfg
            .routers
            .iter()
            .map(|r| CatalogRouter {
                id: router_id(&r.name),
                name: r.name.clone(),
                roles: [
                    (r.pppoe.enabled, RouterRole::Ppp),
                    (r.dhcp.enabled, RouterRole::Dhcp),
                    (r.hotspot.enabled, RouterRole::Hotspot),
                ]
                .into_iter()
                .filter_map(|(on, role)| on.then_some(role))
                .collect(),
            })
            .collect(),
        ..TopologyCatalog::default()
    }
}

fn fraction(value: f64) -> anyhow::Result<Fraction> {
    let scaled = RateBps::try_from(crate::speed::Rate {
        down: value,
        up: value,
    })?;
    Ok(Fraction {
        numerator: scaled.down,
        denominator: 1_000_000,
    })
}

struct Builder<'a> {
    cfg: &'a Config,
    catalog: TopologyCatalog,
    selections: Vec<SourceSelection>,
}

impl Builder<'_> {
    fn group(
        &mut self,
        selection: &SourceSelection,
        router: &RouterCfg,
    ) -> anyhow::Result<GroupDefaults> {
        let rules = super::preflight::expected(self.cfg, router, selection)?;
        let rule = rules.first().context("legacy source policy is absent")?;
        Ok(GroupDefaults {
            allocation: Default::default(),
            // Per-site DHCP uses the plan sum. `propose` refuses DHCP client overrides.
            formula: if rule.count && selection.scope.kind != SourceKind::Dhcp {
                GroupFormula::CountTimesUnit(rule.priority.settings)
            } else {
                GroupFormula::PlanSum
            },
            factors: Factors {
                down: fraction(rule.factors.0)?,
                up: fraction(rule.factors.1)?,
            },
            backstop: rule
                .cap
                .limit()
                .map(RateBps::try_from)
                .transpose()?
                .map(|base| BackstopPolicy {
                    base,
                    factor: Fraction {
                        numerator: 1,
                        denominator: 1,
                    },
                }),
            floor: None,
        })
    }

    fn minimum(
        &self,
        selection: &SourceSelection,
        router: &RouterCfg,
    ) -> anyhow::Result<NodeMinimum> {
        let rules = super::preflight::expected(self.cfg, router, selection)?;
        NodeMinimum::from_legacy(
            rules
                .first()
                .context("legacy source policy is absent")?
                .node,
        )
    }

    /// Add one source, its local profile, and one top-level group node with the legacy ID and label.
    fn add(
        &mut self,
        selection: &SourceSelection,
        router: &RouterCfg,
        key: GroupKey,
        profile_ref: Option<String>,
    ) -> anyhow::Result<()> {
        let id = group_node_id(&router.name, &key);
        let name = group_node_name(self.cfg, &router.name, &key);
        let group = self.group(selection, router)?;
        let minimum = self.minimum(selection, router)?;
        let profile_id = format!("profile-{id}");
        self.catalog.profiles.push(SubscriberProfile {
            id: profile_id.clone(),
            name: name.clone(),
            base: BaseRateAuthority::SourceRead,
            speed: super::legacy_bounds::import(&self.cfg.bandwidth_defaults)?,
            eligible_targets: None,
        });
        self.catalog.sources.push(CatalogSource {
            id: format!("source-{id}"),
            name: name.clone(),
            origin: SourceOrigin::Subscribers {
                scope: selection.scope.clone(),
                profile_ref,
                group,
            },
            profile_id: Some(profile_id),
            targets: vec![],
        });
        self.catalog.nodes.push(CatalogNode {
            address_list: None,
            router_id: None,
            geo: None,
            minimum,
            id,
            name,
            parent: None,
            source_id: Some(format!("source-{}", group_node_id(&router.name, &key))),
            kind: NodeKind::SubscriberGroup,
            capacity: CapacityPolicy::Automatic {
                factors: FactorChoice::Inherit,
            },
        });
        Ok(())
    }
}

/// Propose a policy for the configured routers. `current` supplies PPP profile references.
pub fn propose(cfg: &Config, current: &[CurrentRouter]) -> anyhow::Result<PolicyDraft> {
    ensure!(
        cfg.network.mode == NetworkMode::RootPromotion,
        "the first policy proposal supports only network.mode = \"root_promotion\""
    );
    let mut b = Builder {
        cfg,
        catalog: read_catalog(cfg),
        selections: vec![],
    };
    for router in &cfg.routers {
        let rid = router_id(&router.name);
        let scope = |source: &str, kind| SourceScope {
            router_id: rid.clone(),
            source_id: source.into(),
            kind,
        };
        if router.pppoe.enabled {
            let now = current
                .iter()
                .find(|c| c.router_id() == rid)
                .context("current PPP profiles are missing for a configured router")?;
            let profiles: Vec<_> = now
                .profiles()
                .map_err(|e| anyhow::anyhow!("cannot read current PPP profiles: {e:?}"))?
                .iter()
                .filter(|p| p.kind == SourceKind::Ppp)
                .collect();
            let selection = SourceSelection {
                scope: scope("ppp", SourceKind::Ppp),
                service_names: vec![],
                authority: ReferenceAuthority::PppSecretName,
                policy: SourcePolicy {
                    profiles: profiles
                        .iter()
                        .map(|p| ProfileBinding {
                            source_ref: p.row_ref.clone(),
                            current_name: p.name.clone(),
                            rate: Default::default(),
                        })
                        .collect(),
                    legacy_priority: Some(super::legacy_rate::LegacyPriority::ppp(
                        &cfg.speed,
                        &cfg.bandwidth_defaults.pppoe_default_rate,
                    )?),
                    ..Default::default()
                },
            };
            if router.pppoe.per_plan_node {
                // Owner rule: only a profile that a PPP secret uses gets a group. Every profile
                // stays bound, so a secret that moves to another profile blocks no publication.
                for p in profiles.iter().filter(|p| p.in_use) {
                    b.add(
                        &selection,
                        router,
                        GroupKey::PppPlan(p.name.clone()),
                        Some(p.row_ref.clone()),
                    )?;
                }
            } else {
                b.add(&selection, router, GroupKey::Ppp, None)?;
            }
            b.selections.push(selection);
        }
        if router.dhcp.enabled {
            ensure!(
                router.dhcp.clients.is_empty(),
                "the first policy proposal does not convert DHCP client overrides; bind them explicitly"
            );
            // With discover, the legacy loop also shapes current servers that are not listed.
            let mut names: Vec<String> =
                router.dhcp.servers.iter().map(|s| s.name.clone()).collect();
            if router.dhcp.discover {
                let now = current
                    .iter()
                    .find(|c| c.router_id() == rid)
                    .context("current DHCP servers are missing for a configured router")?;
                let mut found: Vec<String> = now
                    .services()
                    .map_err(|e| anyhow::anyhow!("cannot read current DHCP servers: {e:?}"))?
                    .iter()
                    .filter(|s| s.kind == SourceKind::Dhcp && !names.contains(&s.name))
                    .map(|s| s.name.clone())
                    .collect();
                found.sort_unstable();
                names.extend(found);
            }
            for name in &names {
                let server = router.dhcp.server(name);
                ensure!(
                    server.mode == DhcpMode::PerSite,
                    "the first policy proposal supports only per-site DHCP servers"
                );
                let selection = SourceSelection {
                    scope: scope(
                        &format!("dhcp-{}", super::identity::encode_name(&server.name)),
                        SourceKind::Dhcp,
                    ),
                    service_names: vec![server.name.clone()],
                    authority: ReferenceAuthority::DhcpClientMac,
                    policy: SourcePolicy {
                        legacy_priority: Some(super::legacy_rate::LegacyPriority::dhcp(
                            &cfg.speed,
                            &cfg.bandwidth_defaults,
                            &router.dhcp.server(&server.name),
                        )?),
                        ..Default::default()
                    },
                };
                b.add(
                    &selection,
                    router,
                    GroupKey::DhcpSite(server.name.clone()),
                    None,
                )?;
                b.selections.push(selection);
            }
        }
        ensure!(
            !router.hotspot.enabled,
            "the first policy proposal does not support Hotspot sources"
        );
    }
    let draft = PolicyDraft {
        catalog: b.catalog,
        source_settings: SourceSettings {
            selections: b.selections,
        },
    };
    document(&draft)?;
    Ok(draft)
}

/// The first registry document for an installed draft. It validates the complete policy.
pub fn document(draft: &PolicyDraft) -> anyhow::Result<RegistryDocument> {
    let doc = RegistryDocument {
        catalog: Some(draft.catalog.clone()),
        source_settings: Some(draft.source_settings.clone()),
        ..RegistryDocument::default()
    };
    doc.validate()?;
    Ok(doc)
}

#[cfg(test)]
#[path = "policy_draft_tests.rs"]
pub(crate) mod tests;
