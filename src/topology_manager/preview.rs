//! Read-only view models of a prepared candidate. A preview is for display. It is never published.

use super::aggregate::AggregateNode;
use super::catalog::SourceOrigin;
use super::model::RegistryDocument;
use super::observation::ActiveSubscriber;
use super::rates::RateBps;
use super::resolver::Placement;
use anyhow::Context;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NodeView {
    pub id: String,
    pub label: String,
    pub parent: Option<String>,
    pub kind: String,
    pub source: Option<String>,
    pub calculated_mbps: (f64, f64),
    pub exported_mbps: (f64, f64),
    pub subscribers: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SubscriberView {
    pub id: String,
    pub account_ref: String,
    pub scope: String,
    pub ips: Vec<String>,
    pub macs: Vec<String>,
    pub node: String,
    pub pin: Option<String>,
    pub profile: Option<String>,
    pub min_mbps: (f64, f64),
    pub max_mbps: (f64, f64),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourceView {
    pub id: String,
    pub label: String,
    pub scope: String,
    pub kind: String,
    pub members: u64,
    pub instances: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PreviewView {
    pub nodes: Vec<NodeView>,
    pub subscribers: Vec<SubscriberView>,
    pub inventory: Vec<SourceView>,
}

fn mbps(rate: RateBps) -> anyhow::Result<(f64, f64)> {
    let r = crate::speed::Rate::try_from(rate)?;
    Ok((r.down, r.up))
}

/// Build the view from the evaluated candidate. `labels` are the export labels in node order.
pub(super) fn build(
    evaluated: &[AggregateNode],
    resolved: &[Placement],
    active_subscribers: &[ActiveSubscriber],
    candidate: &RegistryDocument,
    labels: &[String],
) -> anyhow::Result<PreviewView> {
    let catalog = candidate
        .catalog
        .as_ref()
        .context("preview requires a catalog")?;
    let definitions: HashMap<_, _> = catalog.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let source_names: HashMap<_, _> = catalog
        .sources
        .iter()
        .map(|s| (s.id.as_str(), s.name.as_str()))
        .collect();
    let mut direct: HashMap<&str, u64> = HashMap::new();
    for p in resolved {
        *direct.entry(p.target_id.as_str()).or_default() += 1;
    }
    let mut nodes = Vec::with_capacity(evaluated.len());
    for (i, node) in evaluated.iter().enumerate() {
        let definition = definitions.get(node.node_id.as_str());
        let ceiling = node.rates.ceiling;
        let exported = RateBps {
            down: ceiling.down / 1_000_000 * 1_000_000,
            up: ceiling.up / 1_000_000 * 1_000_000,
        };
        nodes.push(NodeView {
            id: node.node_id.clone(),
            label: labels
                .get(i)
                .cloned()
                .unwrap_or_else(|| node.node_id.clone()),
            parent: node.parent_id.clone(),
            kind: definition.map_or_else(String::new, |d| format!("{:?}", d.kind)),
            source: definition
                .and_then(|d| d.source_id.as_deref())
                .and_then(|id| source_names.get(id))
                .map(|n| (*n).to_string()),
            calculated_mbps: mbps(ceiling)?,
            exported_mbps: mbps(exported)?,
            subscribers: direct.get(node.node_id.as_str()).copied().unwrap_or(0),
        });
    }
    let placements: HashMap<_, _> = resolved
        .iter()
        .map(|p| (p.subscriber_id.as_str(), p))
        .collect();
    let pins: HashMap<_, _> = candidate
        .subscribers
        .iter()
        .map(|s| (s.id.as_str(), s.policy.pin.clone()))
        .collect();
    let mut subscribers = Vec::with_capacity(active_subscribers.len());
    for active in active_subscribers {
        let placement = placements
            .get(active.subscriber_id.as_str())
            .context("preview placement missing")?;
        let first = active
            .endpoints
            .first()
            .context("preview subscriber has no endpoint")?;
        let mut ips: Vec<String> = active
            .endpoints
            .iter()
            .flat_map(|e| e.endpoint.ips.iter().map(ToString::to_string))
            .collect();
        ips.sort_unstable();
        let mut macs: Vec<String> = active
            .endpoints
            .iter()
            .filter_map(|e| e.endpoint.mac.clone())
            .collect();
        macs.sort_unstable();
        macs.dedup();
        subscribers.push(SubscriberView {
            id: active.subscriber_id.clone(),
            account_ref: first.identity.account_ref.clone(),
            scope: first.identity.scope.source_id.clone(),
            ips,
            macs,
            node: placement.target_id.clone(),
            pin: pins.get(active.subscriber_id.as_str()).cloned().flatten(),
            profile: placement.profile_id.clone(),
            min_mbps: mbps(placement.rates.min)?,
            max_mbps: mbps(placement.rates.max)?,
        });
    }
    subscribers.sort_by(|a, b| a.account_ref.cmp(&b.account_ref));
    let mut inventory = Vec::new();
    for source in &catalog.sources {
        let SourceOrigin::Subscribers { scope, .. } = &source.origin else {
            continue;
        };
        let instances: Vec<String> = catalog
            .nodes
            .iter()
            .filter(|n| n.source_id.as_deref() == Some(source.id.as_str()))
            .map(|n| n.id.clone())
            .collect();
        let members = instances
            .iter()
            .map(|id| direct.get(id.as_str()).copied().unwrap_or(0))
            .sum();
        inventory.push(SourceView {
            id: source.id.clone(),
            label: source.name.clone(),
            scope: scope.source_id.clone(),
            kind: format!("{:?}", scope.kind),
            members,
            instances,
        });
    }
    Ok(PreviewView {
        nodes,
        subscribers,
        inventory,
    })
}

#[cfg(test)]
mod tests {
    use crate::topology_manager::policy_draft::tests::{candidate, scenario};

    const GOLD: &str = "lq3-3fac2a02b0faebac";

    #[tokio::test(start_paused = true)]
    async fn scenario_view_lists_every_node_with_calculated_and_exported_ceilings() {
        let (cfg, snap) = scenario();
        let pair = candidate(&cfg, &snap).await;
        let view = pair.view();
        assert_eq!(view.nodes.len(), 4);
        let gold = view.nodes.iter().find(|n| n.id == GOLD).unwrap();
        assert_eq!(gold.calculated_mbps, (52.5, 52.5));
        assert_eq!(gold.exported_mbps, (52.0, 52.0));
        assert_eq!(gold.subscribers, 3);
        assert!(gold.label.starts_with("Gold-Hub-1"));
        assert_eq!(gold.kind, "SubscriberGroup");
    }

    #[tokio::test(start_paused = true)]
    async fn scenario_view_lists_each_subscriber_with_identity_endpoints_and_node() {
        let (cfg, snap) = scenario();
        let pair = candidate(&cfg, &snap).await;
        let view = pair.view();
        assert_eq!(view.subscribers.len(), 7);
        let g1 = view
            .subscribers
            .iter()
            .find(|s| s.account_ref == "ppp-name/v1/g1")
            .unwrap();
        assert_eq!(g1.ips, vec!["100.64.0.11".to_string()]);
        assert_eq!(g1.node, GOLD);
        assert_eq!(g1.scope, "ppp");
        assert_eq!(g1.max_mbps, (50.0, 50.0));
        assert_eq!(g1.min_mbps, (5.0, 5.0));
        assert!(g1.profile.is_some());
        assert_eq!(g1.pin, None);
        let lease = view
            .subscribers
            .iter()
            .find(|s| s.account_ref.starts_with("dhcp-mac/v1/"))
            .unwrap();
        assert_eq!(lease.macs.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn inventory_counts_current_members_per_source() {
        let (cfg, snap) = scenario();
        let pair = candidate(&cfg, &snap).await;
        let gold = pair
            .view()
            .inventory
            .iter()
            .find(|s| s.instances == vec![GOLD.to_string()])
            .unwrap();
        assert_eq!(gold.members, 3);
        assert_eq!(gold.kind, "Ppp");
    }
}
