//! The router inventory of the Topology canvas: for each router, its WAN interfaces and its
//! subscriber sources, each with one state and one action. Pure rules, so native tests cover them.
use crate::topology_canvas_model::card_of;
use web_contract::topology::{DiscoveredRouter, SourceCandidate, TopologySummary};

/// One router and what LQoSync knows of it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InvRouter {
    pub id: String,
    pub name: String,
    /// `online`, `failed`, `unreachable`, `offline`, or `unknown` (no collection yet).
    pub state: String,
    /// The last collection, Unix seconds.
    pub observed_at: Option<u64>,
    /// The access kind, the records, and the online records of each collected source kind.
    pub counts: Vec<(String, u64, Option<u64>)>,
    pub wans: Vec<InvWan>,
    pub sources: Vec<InvSource>,
    /// Sources that the last source discovery found on this router.
    pub found: Vec<SourceCandidate>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InvWan {
    pub interface: String,
    pub state: WanState,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum WanState {
    /// A WAN node uses the interface: its card ID and name.
    OnCanvas { card: String, name: String },
    /// A saved WAN source without a node on the canvas.
    Saved,
    /// An active default route uses the interface, and no WAN node does.
    NotConfigured,
    /// The last WAN discovery found the interface; no WAN node uses it.
    Found { running: bool },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InvSource {
    pub id: String,
    pub name: String,
    /// `ppp`, `dhcp`, or `hotspot`.
    pub access: String,
    /// The subscribers of its groups in the latest preview.
    pub subscribers: Option<u64>,
    pub state: SourceState,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SourceState {
    /// A group on the canvas shows the source: its card ID.
    OnCanvas { card: String },
    /// Its groups exist, but no subscriber of it is active, so the canvas hides them.
    Idle,
    /// Not on the canvas: automatic placement puts its subscribers below these cards.
    Auto { parents: Vec<String> },
    /// Not on the canvas, and automatic placement has no parent for it.
    Unplaced,
    /// The operator excluded the source: LQoSync does not read or shape it.
    Excluded,
}

/// The inventory in collection order. A router that only a source or a WAN interface names
/// comes after them. `wan_reads` and `source_reads` are the last discovery answers.
pub(crate) fn inventory(
    s: &TopologySummary,
    wan_reads: &[DiscoveredRouter],
    source_reads: &[SourceCandidate],
) -> Vec<InvRouter> {
    let mut out: Vec<InvRouter> = s
        .collection
        .iter()
        .map(|r| InvRouter {
            id: r.router_id.clone(),
            name: r.name.clone(),
            state: r.state.clone(),
            observed_at: Some(r.observed_at),
            counts: r
                .sources
                .iter()
                .filter(|c| c.enabled)
                .map(|c| (c.kind.clone(), c.records, c.online))
                .collect(),
            wans: Vec::new(),
            sources: Vec::new(),
            found: Vec::new(),
        })
        .collect();
    let router = |out: &mut Vec<InvRouter>, id: &str, name: &str| -> usize {
        if let Some(i) = out.iter().position(|r| r.id == id) {
            return i;
        }
        out.push(InvRouter {
            id: id.into(),
            name: name.into(),
            state: "unknown".into(),
            observed_at: None,
            counts: Vec::new(),
            wans: Vec::new(),
            sources: Vec::new(),
            found: Vec::new(),
        });
        out.len() - 1
    };
    let nodes_of = |source: &str| -> Vec<&web_contract::topology::Node> {
        s.nodes
            .iter()
            .filter(|n| n.source_id.as_deref() == Some(source))
            .collect()
    };
    let name_of = |id: &str| {
        s.nodes
            .iter()
            .find(|n| n.id == id)
            .map_or_else(|| id.to_string(), |n| n.name.clone())
    };
    for src in &s.sources {
        let Some(rid) = src.router_id.as_deref() else {
            continue;
        };
        let i = router(&mut out, rid, rid);
        if src.kind == "wan" {
            let state = match nodes_of(&src.id).first() {
                Some(n) => WanState::OnCanvas {
                    card: card_of(n).to_string(),
                    name: n.name.clone(),
                },
                None => WanState::Saved,
            };
            out[i].wans.push(InvWan {
                interface: src.endpoint.clone().unwrap_or_else(|| src.name.clone()),
                state,
            });
            continue;
        }
        let groups = nodes_of(&src.id);
        let subscribers = groups.iter().filter_map(|n| n.members).reduce(|a, b| a + b);
        let state = if src.excluded {
            SourceState::Excluded
        } else if let Some(shown) = groups.iter().find(|n| n.active) {
            SourceState::OnCanvas {
                card: card_of(shown).to_string(),
            }
        } else if !groups.is_empty() {
            SourceState::Idle
        } else if src.auto_parents.is_empty() {
            SourceState::Unplaced
        } else {
            SourceState::Auto {
                parents: src.auto_parents.iter().map(|p| name_of(p)).collect(),
            }
        };
        out[i].sources.push(InvSource {
            id: src.id.clone(),
            name: src.name.clone(),
            access: src.access.clone().unwrap_or_default(),
            subscribers,
            state,
        });
    }
    // Default-route interfaces without a WAN node, then the other interfaces of a discovery.
    for r in s.wan_candidates.iter().chain(wan_reads) {
        let i = router(&mut out, &r.router_id, &r.name);
        for c in &r.candidates {
            if !c.used_by.is_empty() || out[i].wans.iter().any(|w| w.interface == c.name) {
                continue;
            }
            let state = if c.default_route {
                WanState::NotConfigured
            } else {
                WanState::Found { running: c.running }
            };
            out[i].wans.push(InvWan {
                interface: c.name.clone(),
                state,
            });
        }
    }
    // A source candidate names its router by ID or by name.
    for c in source_reads {
        let i = match out
            .iter()
            .position(|r| r.id == c.router || r.name == c.router)
        {
            Some(i) => i,
            None => router(&mut out, &c.router, &c.router),
        };
        out[i].found.push(c.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use web_contract::topology::{
        Capacity, CollectionRouter, CollectionSource, Node, Source, WanCandidate,
    };

    fn node(id: &str, source: &str, members: u64, active: bool) -> Node {
        Node {
            id: id.into(),
            name: id.into(),
            parent: None,
            kind: "group".into(),
            source_id: Some(source.into()),
            capacity: Capacity::Automatic,
            alias: false,
            calculated_mbps: None,
            exported_mbps: None,
            members: Some(members),
            geo: None,
            position: None,
            router_id: None,
            auto_members: None,
            wan_health: None,
            address_list: None,
            routed: None,
            active,
            card_id: format!("source:{source}"),
            tree_excluded: false,
        }
    }

    fn source(id: &str, kind: &str, router: &str) -> Source {
        Source {
            id: id.into(),
            name: id.into(),
            kind: kind.into(),
            auto_parents: Vec::new(),
            excluded: false,
            router_id: Some(router.into()),
            access: (kind != "wan").then(|| "ppp".into()),
            endpoint: (kind == "wan").then(|| format!("if-{id}")),
        }
    }

    fn candidate(name: &str, default_route: bool, used: bool) -> WanCandidate {
        WanCandidate {
            name: name.into(),
            kind: "pppoe-out".into(),
            running: true,
            disabled: false,
            suggested: false,
            used_by: if used { vec!["n".into()] } else { vec![] },
            default_route,
        }
    }

    fn summary() -> TopologySummary {
        let mut wan_node = node("wan1", "w1", 0, true);
        wan_node.kind = "wan".into();
        wan_node.card_id = "wan1".into();
        wan_node.name = "WAN1".into();
        let mut tier = node("g-tier", "tier", 0, true);
        tier.name = "Tier".into();
        let mut auto = source("silver", "subscribers", "distro");
        auto.auto_parents = vec!["wan1".into()];
        let mut excluded = source("trial", "subscribers", "distro");
        excluded.excluded = true;
        TopologySummary {
            collection: vec![CollectionRouter {
                router_id: "distro".into(),
                name: "Distro".into(),
                state: "online".into(),
                observed_at: 100,
                sources: vec![CollectionSource {
                    kind: "ppp".into(),
                    enabled: true,
                    records: 12,
                    online: Some(9),
                    offline: Some(3),
                }],
            }],
            nodes: vec![
                wan_node,
                node("g1", "gold", 5, true),
                node("g2", "gold", 4, false),
                node("g3", "bronze", 2, false),
                tier,
            ],
            sources: vec![
                source("w1", "wan", "distro"),
                source("gold", "subscribers", "distro"),
                source("bronze", "subscribers", "distro"),
                auto,
                source("lonely", "subscribers", "distro"),
                excluded,
                source("other", "subscribers", "core"),
            ],
            wan_candidates: vec![DiscoveredRouter {
                router_id: "distro".into(),
                name: "Distro".into(),
                state: "ok".into(),
                error: None,
                candidates: vec![candidate("pppoe-out3", true, false)],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn each_router_lists_its_wans_and_sources_with_one_state() {
        let inv = inventory(&summary(), &[], &[]);
        let ids: Vec<&str> = inv.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            ["distro", "core"],
            "collection order, then other routers"
        );
        let d = &inv[0];
        assert_eq!((d.state.as_str(), d.observed_at), ("online", Some(100)));
        assert_eq!(d.counts, [("ppp".to_string(), 12, Some(9))]);
        assert_eq!(
            d.wans,
            [
                InvWan {
                    interface: "if-w1".into(),
                    state: WanState::OnCanvas {
                        card: "wan1".into(),
                        name: "WAN1".into()
                    }
                },
                InvWan {
                    interface: "pppoe-out3".into(),
                    state: WanState::NotConfigured
                },
            ]
        );
        let state = |id: &str| {
            d.sources
                .iter()
                .find(|s| s.id == id)
                .map(|s| (s.state.clone(), s.subscribers))
                .unwrap()
        };
        assert_eq!(
            state("gold"),
            (
                SourceState::OnCanvas {
                    card: "source:gold".into()
                },
                Some(9)
            )
        );
        assert_eq!(state("bronze"), (SourceState::Idle, Some(2)));
        assert_eq!(
            state("silver"),
            (
                SourceState::Auto {
                    parents: vec!["WAN1".into()]
                },
                None
            )
        );
        assert_eq!(state("lonely"), (SourceState::Unplaced, None));
        assert_eq!(state("trial"), (SourceState::Excluded, None));
        assert_eq!(inv[1].state, "unknown");
        assert_eq!(inv[1].sources[0].id, "other");
    }

    #[test]
    fn a_discovery_adds_free_interfaces_and_found_sources_once() {
        let read = DiscoveredRouter {
            router_id: "distro".into(),
            name: "Distro".into(),
            state: "ok".into(),
            error: None,
            candidates: vec![
                candidate("if-w1", false, true),
                candidate("pppoe-out3", true, false),
                candidate("ether5", false, false),
            ],
        };
        let found = SourceCandidate {
            key: "k".into(),
            kind: "ppp".into(),
            router: "Distro".into(),
            name: "Tier-100M".into(),
            rebind: false,
        };
        let inv = inventory(&summary(), &[read], std::slice::from_ref(&found));
        let wans: Vec<(&str, &WanState)> = inv[0]
            .wans
            .iter()
            .map(|w| (w.interface.as_str(), &w.state))
            .collect();
        assert_eq!(wans.len(), 3, "{wans:?}");
        assert_eq!(wans[2], ("ether5", &WanState::Found { running: true }));
        assert_eq!(inv[0].found, [found], "a router name finds the router");
    }
}
