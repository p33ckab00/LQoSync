//! Pure geometry and form rules of the Topology canvas, so native tests cover them.
use crate::canvas_layout::Tier;
use web_contract::topology::{BuilderEdit, Direction, DiscoveredRouter, Flow, Location, Node};

pub(crate) const CARD_W: f64 = 232.0;
pub(crate) const CARD_H: f64 = 84.0;
pub(crate) const MARGIN: f64 = 48.0;
// Wasm-only modules (canvas, Studio preview) use these items; native tests do not.
/// The free border that "Fit view" keeps for the floating controls: top, right, bottom, left.
#[cfg_attr(test, allow(dead_code))]
pub(crate) const FIT_PAD: [f64; 4] = [130.0, 100.0, 84.0, 16.0];
/// The room above the Sankey for the column headers.
#[cfg_attr(test, allow(dead_code))]
pub(crate) const SANKEY_TOP: f64 = 56.0;
/// The canvas is never smaller than this, so there is room to drag a card outward.
const MIN_SURFACE: (f64, f64) = (1600.0, 1000.0);
// Wasm-only modules (canvas, Studio preview) use this item; native tests do not.
#[cfg_attr(test, allow(dead_code))]
pub(crate) const SIZE: [f64; 2] = [CARD_W, CARD_H];

/// The card ID of a default-route WAN interface that has no WAN node yet.
pub(crate) fn candidate_id(router_id: &str, interface: &str) -> String {
    format!("candidate:{router_id}:{interface}")
}

/// The nodes that the canvas shows. An inactive group is hidden; its settings stay.
pub(crate) fn visible(nodes: &[Node]) -> Vec<Node> {
    nodes.iter().filter(|n| n.active).cloned().collect()
}

/// The card of a node: the service merge key, else the node ID (a view without card IDs).
pub(crate) fn card_of(n: &Node) -> &str {
    if n.card_id.is_empty() {
        &n.id
    } else {
        &n.card_id
    }
}

/// One canvas card: the nodes (instances) that share one card ID, in view order. The canvas
/// never shows the instances; `node` is the first one, with the members of all of them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Card {
    pub id: String,
    pub nodes: Vec<String>,
    pub kind: &'static str,
    pub name: String,
    pub members: Option<u64>,
    pub node: Node,
}

/// One card for each card ID, in the order of the first instance.
pub(crate) fn cards(nodes: &[Node]) -> Vec<Card> {
    let mut out: Vec<Card> = Vec::new();
    for n in nodes {
        match out.iter_mut().find(|c| c.id == card_of(n)) {
            Some(c) => {
                c.nodes.push(n.id.clone());
                c.members = match (c.members, n.members) {
                    (Some(a), Some(b)) => Some(a + b),
                    (a, b) => a.or(b),
                };
                c.node.members = c.members;
                for (total, part) in [
                    (&mut c.node.exported_mbps, n.exported_mbps),
                    (&mut c.node.calculated_mbps, n.calculated_mbps),
                ] {
                    *total = match (*total, part) {
                        (Some(a), Some(b)) => Some([a[0] + b[0], a[1] + b[1]]),
                        (a, b) => a.or(b),
                    };
                }
            }
            None => out.push(Card {
                id: card_of(n).to_string(),
                nodes: vec![n.id.clone()],
                kind: card_kind(n),
                name: n.name.clone(),
                members: n.members,
                node: n.clone(),
            }),
        }
    }
    out
}

// Wasm-only modules (canvas, Studio preview) use this item; native tests do not.
#[cfg_attr(test, allow(dead_code))]
/// The shared tier of a saved or candidate card.
pub(crate) fn node_tier(n: &Node) -> Tier {
    match (n.kind.as_str(), n.router_id.is_some()) {
        ("wan", _) => Tier::Wan,
        (_, true) => Tier::Router,
        ("group", _) => Tier::Group,
        _ => Tier::Other,
    }
}

/// The edits of a card form for every node of the card. With one node the edits stay. With
/// several, a capacity is the card total (`card.capacity`), and a name, a location, a router, or
/// the tree exclusion goes to each node.
pub(crate) fn card_edits(card: &Card, edits: Vec<BuilderEdit>) -> Vec<BuilderEdit> {
    if card.nodes.len() < 2 {
        return edits;
    }
    let mut out = Vec::new();
    for edit in edits {
        match edit {
            BuilderEdit::NodeUpdate { name, capacity, .. } => {
                if let Some(capacity) = capacity {
                    out.push(BuilderEdit::CardCapacity {
                        card_id: card.id.clone(),
                        capacity,
                    });
                }
                if name.is_some() {
                    out.extend(card.nodes.iter().map(|id| BuilderEdit::NodeUpdate {
                        node_id: id.clone(),
                        name: name.clone(),
                        capacity: None,
                    }));
                }
            }
            BuilderEdit::NodeLocation {
                geo_type, location, ..
            } => out.extend(card.nodes.iter().map(|id| BuilderEdit::NodeLocation {
                node_id: id.clone(),
                geo_type,
                location: location.clone(),
            })),
            BuilderEdit::NodeRouter { router_id, .. } => {
                out.extend(card.nodes.iter().map(|id| BuilderEdit::NodeRouter {
                    node_id: id.clone(),
                    router_id: router_id.clone(),
                }))
            }
            BuilderEdit::NodeTreeExclude { excluded, .. } => {
                out.extend(card.nodes.iter().map(|id| BuilderEdit::NodeTreeExclude {
                    node_id: id.clone(),
                    excluded,
                }))
            }
            other => out.push(other),
        }
    }
    out
}

// Wasm-only modules (canvas, Studio preview) use this item; native tests do not.
#[cfg_attr(test, allow(dead_code))]
/// The out port (`out`) or the in port of a card at `p` in a direction.
pub(crate) fn port(p: [f64; 2], direction: Direction, out: bool) -> [f64; 2] {
    let side = match (direction, out) {
        (Direction::TopDown, true) | (Direction::BottomUp, false) => [CARD_W / 2.0, CARD_H],
        (Direction::TopDown, false) | (Direction::BottomUp, true) => [CARD_W / 2.0, 0.0],
        (Direction::LeftRight, true) | (Direction::RightLeft, false) => [CARD_W, CARD_H / 2.0],
        (Direction::LeftRight, false) | (Direction::RightLeft, true) => [0.0, CARD_H / 2.0],
    };
    [p[0] + side[0], p[1] + side[1]]
}

/// One line for each pair of cards with a parent link, in node order. The count comes from the
/// view flows; a flow that the reply limit cut counts 0, but its line stays.
pub(crate) fn lines(nodes: &[Node], flows: &[Flow]) -> Vec<Flow> {
    let mut out: Vec<Flow> = Vec::new();
    for n in nodes {
        let Some(p) = nodes
            .iter()
            .find(|p| Some(p.id.as_str()) == n.parent.as_deref())
        else {
            continue;
        };
        let (from, to) = (card_of(p), card_of(n));
        if out.iter().any(|f| f.from == from && f.to == to) {
            continue;
        }
        let subscribers = flows
            .iter()
            .find(|f| f.from == from && f.to == to)
            .map_or(0, |f| f.subscribers);
        out.push(Flow {
            from: from.into(),
            to: to.into(),
            subscribers,
        });
    }
    out
}

/// The parent node and child node of the line from card `from` to card `to`.
pub(crate) fn link_of(nodes: &[Node], from: &str, to: &str) -> Option<(String, String)> {
    nodes.iter().filter(|n| card_of(n) == to).find_map(|n| {
        let p = nodes
            .iter()
            .find(|p| Some(p.id.as_str()) == n.parent.as_deref())?;
        (card_of(p) == from).then(|| (p.id.clone(), n.id.clone()))
    })
}

/// False for the line between a group and its own router card: the router branch keeps its
/// groups.
pub(crate) fn removable(nodes: &[Node], parent: &str, child: &str) -> bool {
    let find = |id: &str| nodes.iter().find(|n| n.id == id);
    match (find(parent), find(child)) {
        (Some(p), Some(c)) => !(c.kind == "group" && p.kind != "wan" && p.router_id.is_some()),
        _ => false,
    }
}

/// The text of a card with a WAN above it while the routing list writer is off.
pub(crate) const ROUTING_WARNING: &str = "MikroTik routing does not follow yet (writer off)";

/// One card for each default-route WAN interface without a WAN node:
/// card ID, router ID, router name, and interface.
pub(crate) fn candidate_cards(
    routers: &[DiscoveredRouter],
) -> Vec<(String, String, String, String)> {
    routers
        .iter()
        .flat_map(|r| {
            r.candidates.iter().map(|c| {
                (
                    candidate_id(&r.router_id, &c.name),
                    r.router_id.clone(),
                    r.name.clone(),
                    c.name.clone(),
                )
            })
        })
        .collect()
}

/// The kind of a card in one short word.
pub(crate) fn card_kind(n: &Node) -> &'static str {
    match (n.kind.as_str(), n.router_id.is_some()) {
        ("wan", _) => "WAN",
        ("group", _) => "Group",
        (_, true) => "Router",
        _ => "Node",
    }
}

/// A card that can be a parent: a WAN, a router card, or a Site/AP card. A subscriber group
/// has no child cards.
pub(crate) fn can_parent(n: &Node) -> bool {
    n.kind == "wan" || n.kind == "intermediate"
}

/// The edit for a connector that the operator drags from `from` and drops on `target`: `from`
/// becomes the parent of `target`. A WAN gives a second parent through an instance. A group
/// below a router card takes its WANs from the card, so a WAN drop goes on the card.
pub(crate) fn drop_edit(
    nodes: &[Node],
    from: &str,
    target: &str,
) -> Result<BuilderEdit, &'static str> {
    let find = |id: &str| nodes.iter().find(|n| n.id == id);
    // `from` and `target` are card IDs: the edit goes on the first instance of each card.
    let first = |card: &str| {
        nodes
            .iter()
            .find(|n| card_of(n) == card)
            .or_else(|| find(card))
    };
    let t = first(target).ok_or("The card is not on the canvas.")?;
    let f = first(from).ok_or("The card is not on the canvas.")?;
    let instances: Vec<&Node> = nodes.iter().filter(|n| card_of(n) == card_of(t)).collect();
    let from = f.id.as_str();
    if !can_parent(f) {
        return Err("A subscriber group has no child cards.");
    }
    if t.kind == "wan" {
        return Err("A WAN card is always on top. Connect it to a router, Site, or group card.");
    }
    if t.id == f.id {
        return Err("A card cannot be its own parent.");
    }
    if f.kind != "wan" {
        // The target must not be above the parent card: that would make a loop.
        let mut up = f.parent.as_deref();
        while let Some(id) = up {
            if id == t.id {
                return Err("This would make a loop.");
            }
            up = find(id).and_then(|n| n.parent.as_deref());
        }
        if t.parent.as_deref() == Some(from) {
            return Err("This card is already a child of that card.");
        }
        if instances.len() > 1 {
            return Err("This card has several WAN links. Remove all but one first.");
        }
        return Ok(BuilderEdit::NodeMove {
            node_id: t.id.clone(),
            parent: Some(from.to_string()),
        });
    }
    let wan = from;
    if instances.iter().any(|i| i.parent.as_deref() == Some(wan)) {
        return Err("This WAN is already a parent of the card.");
    }
    if t.parent
        .as_deref()
        .and_then(find)
        .is_some_and(|p| p.router_id.is_some())
    {
        return Err("Connect the WAN to the router card.");
    }
    let wan = Some(wan.to_string());
    match (&t.parent, &t.source_id) {
        (None, _) => Ok(BuilderEdit::NodeMove {
            node_id: t.id.clone(),
            parent: wan,
        }),
        (Some(_), _) if t.router_id.is_some() => Ok(BuilderEdit::NodeInstanceAdd {
            node_id: t.id.clone(),
            parent: wan,
        }),
        (Some(_), Some(source)) => Ok(BuilderEdit::InstanceAdd {
            source_id: source.clone(),
            parent: wan,
            name: None,
        }),
        (Some(_), None) => Err("This card has one parent. Change it in its settings."),
    }
}

/// True when a WAN is above `id` and the routing list writer is off: shaping follows the canvas,
/// but MikroTik routing does not.
pub(crate) fn routing_warning(nodes: &[Node], id: &str, writer_off: bool) -> bool {
    writer_off
        && nodes
            .iter()
            .any(|w| w.kind == "wan" && w.id != id && is_below(nodes, id, &w.id))
}

/// The unscaled canvas size: every card plus a margin, never below the minimum.
pub(crate) fn surface(points: &[(String, [f64; 2])]) -> (f64, f64) {
    points.iter().fold(MIN_SURFACE, |(w, h), (_, p)| {
        (
            w.max(p[0] + CARD_W + 4.0 * MARGIN),
            h.max(p[1] + CARD_H + 4.0 * MARGIN),
        )
    })
}

/// The location of the Location form. Both fields empty: no location.
/// A paste fills both fields; otherwise both fields must be present.
pub(crate) fn location_draft(latitude: &str, longitude: &str) -> Result<Option<Location>, String> {
    let (lat, lon) = (latitude.trim(), longitude.trim());
    match (lat.is_empty(), lon.is_empty()) {
        (true, true) => Ok(None),
        (false, false) => {
            let location = Location {
                latitude: lat.into(),
                longitude: lon.into(),
            };
            location.validate()?;
            Ok(Some(location))
        }
        _ => Err("Enter both latitude and longitude, or clear both.".into()),
    }
}

/// True when `candidate` sits below `ancestor`. A parent cycle cannot loop.
pub(crate) fn is_below(nodes: &[Node], candidate: &str, ancestor: &str) -> bool {
    let parent = |id: &str| nodes.iter().find(|n| n.id == id)?.parent.clone();
    let mut at = parent(candidate);
    for _ in 0..nodes.len() {
        match at {
            Some(p) if p == ancestor => return true,
            Some(p) => at = parent(&p),
            None => return false,
        }
    }
    false
}

/// A Google Maps paste into either field: `latitude, longitude`. `None` when it is one value.
pub(crate) fn split_paste(text: &str) -> Option<Result<Location, String>> {
    text.contains(',').then(|| Location::parse_pair(text))
}

/// The color tone of a WAN stream: its index in the WAN list (five tones), or `unknown`.
pub(crate) fn wan_tone(wans: &[String], wan: Option<&str>) -> String {
    match wan.and_then(|w| wans.iter().position(|x| x == w)) {
        Some(i) => (i % 5).to_string(),
        None => "unknown".to_string(),
    }
}

/// The Sankey value of a card or stream as text: its configured capacity, or its subscribers.
pub(crate) fn value_text(value: Option<f64>, metric: crate::sankey::Metric) -> String {
    use crate::sankey::Metric;
    match (value, metric) {
        (None, _) => "No data".into(),
        (Some(v), Metric::Capacity) if v <= 0.0 => "No capacity".into(),
        (Some(v), Metric::Capacity) if v < 10.0 => format!("{v:.1} Mbps"),
        (Some(v), Metric::Capacity) => format!("{v:.0} Mbps"),
        (Some(v), Metric::Subscribers) => format!("{v:.0} sub{}", if v == 1.0 { "" } else { "s" }),
    }
}

/// The lines of a WAN card, most important first: the configured capacity of the WAN, the
/// capacity that its children get (the ribbon width), that share in percent, and the live
/// download. The rest of the configured capacity is configuration headroom, not free bandwidth,
/// so the card does not show it.
pub(crate) fn wan_card_lines(
    configured: Option<f64>,
    allocated: f64,
    live: Option<f64>,
) -> (String, String, Option<String>, Option<String>) {
    let primary = match configured {
        Some(c) if c > 0.0 => format!("{c:.0} Mbps"),
        _ => "No capacity".to_string(),
    };
    let percent = configured
        .filter(|c| *c > 0.0)
        .map(|c| format!("{:.0}% allocated", allocated / c * 100.0));
    (
        primary,
        format!("{allocated:.0} Mbps allocated"),
        percent,
        live_text(live),
    )
}

/// The live download of a card or stream as text, when a fresh host sample has it.
pub(crate) fn live_text(live: Option<f64>) -> Option<String> {
    live.map(|v| format!("{v:.0} Mbps live"))
}

/// The live chip: text, state (`ok`, `warn`, `muted`), and the reason in its tooltip.
pub(crate) fn live_chip(
    t: Option<&web_contract::topology::TopologyThroughput>,
    now: u64,
) -> (String, &'static str, String) {
    match t {
        Some(t) if !t.stale && t.complete => (
            format!("Live · {}s ago", now.saturating_sub(t.updated)),
            "ok",
            "Inner band: live download, the exact sum of the hosts on each WAN.".into(),
        ),
        Some(t) if !t.stale => (
            "Partial".into(),
            "warn",
            if t.unmapped_hosts > 0 {
                format!(
                    "{} hosts have no subscriber. Their rates are in no ribbon.",
                    t.unmapped_hosts
                )
            } else {
                "The host sample or the placement is incomplete.".into()
            },
        ),
        _ => (
            "No live traffic".into(),
            "muted",
            t.and_then(|t| t.reason.clone())
                .unwrap_or_else(|| "No host sample. The inner live band is off.".into()),
        ),
    }
}

/// The name of a stream without a WAN. Subscriber counts carry no WAN split.
pub(crate) fn unknown_stream(metric: crate::sankey::Metric) -> &'static str {
    match metric {
        crate::sankey::Metric::Subscribers => "All WANs",
        crate::sankey::Metric::Capacity => "WAN unknown",
    }
}

/// The name of a service tier card (`tier:<access>`), else `None`.
pub(crate) fn tier_name(id: &str) -> Option<&'static str> {
    match id {
        "tier:ppp" => Some("PPPoE"),
        "tier:dhcp" => Some("IPoE / DHCP"),
        "tier:hotspot" => Some("Hotspot"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use web_contract::topology::{Capacity, Flow, WanCandidate};

    fn node(id: &str, parent: Option<&str>, position: Option<[f64; 2]>) -> Node {
        Node {
            id: id.into(),
            name: id.into(),
            parent: parent.map(str::to_owned),
            kind: "group".into(),
            source_id: None,
            capacity: Capacity::Automatic,
            alias: false,
            calculated_mbps: None,
            exported_mbps: None,
            members: None,
            geo: None,
            position,
            router_id: None,
            auto_members: None,
            wan_health: None,
            address_list: None,
            routed: None,
            active: true,
            card_id: String::new(),
            tree_excluded: false,
        }
    }

    fn kind(mut n: Node, kind: &str) -> Node {
        n.kind = kind.into();
        n
    }

    fn router(mut n: Node) -> Node {
        n.kind = "intermediate".into();
        n.router_id = Some("hub-1".into());
        n
    }

    fn sourced(mut n: Node) -> Node {
        n.source_id = Some(format!("src-{}", n.id));
        n
    }

    fn candidates() -> Vec<DiscoveredRouter> {
        vec![DiscoveredRouter {
            router_id: "core".into(),
            name: "Core".into(),
            state: "ok".into(),
            error: None,
            candidates: vec![WanCandidate {
                name: "pppoe-out2".into(),
                kind: "pppoe-out".into(),
                running: true,
                disabled: false,
                suggested: true,
                used_by: vec![],
                default_route: true,
            }],
        }]
    }

    #[test]
    fn inactive_nodes_are_hidden() {
        let mut gone = node("g2", None, None);
        gone.active = false;
        let shown = visible(&[node("g1", None, None), gone]);
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].id, "g1");
    }

    #[test]
    fn each_candidate_interface_is_one_card_and_wires_join_cards() {
        let cards = candidate_cards(&candidates());
        assert_eq!(
            cards,
            [(
                candidate_id("core", "pppoe-out2"),
                "core".to_string(),
                "Core".to_string(),
                "pppoe-out2".to_string()
            )]
        );
        assert!(ROUTING_WARNING.contains("writer off"));
    }

    #[test]
    fn the_drop_on_a_group_below_a_router_is_disabled() {
        let nodes = [
            kind(node("wan1", None, None), "wan"),
            kind(node("wan2", None, None), "wan"),
            router(node("r1", Some("wan1"), None)),
            sourced(node("g1", Some("r1"), None)),
            sourced(node("g2", Some("wan1"), None)),
            node("g3", None, None),
        ];
        assert_eq!(
            drop_edit(&nodes, "wan2", "g1"),
            Err("Connect the WAN to the router card.")
        );
        assert_eq!(
            drop_edit(&nodes, "wan2", "r1"),
            Ok(BuilderEdit::NodeInstanceAdd {
                node_id: "r1".into(),
                parent: Some("wan2".into())
            })
        );
        assert_eq!(
            drop_edit(&nodes, "wan2", "g2"),
            Ok(BuilderEdit::InstanceAdd {
                source_id: "src-g2".into(),
                parent: Some("wan2".into()),
                name: None
            })
        );
        assert_eq!(
            drop_edit(&nodes, "wan2", "g3"),
            Ok(BuilderEdit::NodeMove {
                node_id: "g3".into(),
                parent: Some("wan2".into())
            })
        );
        assert!(drop_edit(&nodes, "wan1", "g2").is_err());
        assert!(drop_edit(&nodes, "wan1", "wan2").is_err());
    }

    /// WAN1 and WAN2 above copies r1 and r2 of router hub-1, with one group instance below each.
    fn merged() -> Vec<Node> {
        let card = |mut n: Node, c: &str| {
            n.card_id = c.into();
            n
        };
        let group = |id: &str, parent: &str, members: u64| {
            let mut n = card(node(id, Some(parent), None), "source:S");
            n.source_id = Some("S".into());
            n.members = Some(members);
            n.exported_mbps = Some(if members == 3 {
                [10.0, 5.0]
            } else {
                [20.0, 10.0]
            });
            n
        };
        vec![
            card(kind(node("wan1", None, None), "wan"), "wan1"),
            card(kind(node("wan2", None, None), "wan"), "wan2"),
            card(router(node("r1", Some("wan1"), None)), "router:hub-1"),
            card(router(node("r2", Some("wan2"), None)), "router:hub-1"),
            group("g1", "r1", 3),
            group("g2", "r2", 4),
        ]
    }

    #[test]
    fn one_card_for_each_card_id() {
        let nodes = merged();
        let c = cards(&nodes);
        let ids: Vec<&str> = c.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["wan1", "wan2", "router:hub-1", "source:S"]);
        assert_eq!(c[2].nodes, ["r1", "r2"]);
        assert_eq!((c[3].kind, c[3].members), ("Group", Some(7)));
        assert_eq!(
            c[3].node.exported_mbps,
            Some([30.0, 15.0]),
            "the rate of the card is the total"
        );
        // A drop goes on the card: WAN1 is already a parent of the router card.
        assert_eq!(
            drop_edit(&nodes, "wan1", "router:hub-1"),
            Err("This WAN is already a parent of the card.")
        );
        assert_eq!(
            drop_edit(&merged()[..5], "wan2", "source:S").unwrap_err(),
            "Connect the WAN to the router card."
        );
    }

    #[test]
    fn a_merged_card_sends_its_capacity_once_and_its_name_to_each_node() {
        let c = cards(&merged()).remove(3);
        let fixed = Capacity::Fixed {
            down_mbps: 90,
            up_mbps: 30,
            factor_percent: None,
        };
        let edits = card_edits(
            &c,
            vec![BuilderEdit::NodeUpdate {
                node_id: "g1".into(),
                name: Some("Tier".into()),
                capacity: Some(fixed.clone()),
            }],
        );
        let name = |id: &str| BuilderEdit::NodeUpdate {
            node_id: id.into(),
            name: Some("Tier".into()),
            capacity: None,
        };
        assert_eq!(
            edits,
            [
                BuilderEdit::CardCapacity {
                    card_id: "source:S".into(),
                    capacity: fixed
                },
                name("g1"),
                name("g2")
            ]
        );
        assert_eq!(
            card_edits(
                &c,
                vec![BuilderEdit::NodeRouter {
                    node_id: "g1".into(),
                    router_id: None
                }]
            )
            .len(),
            2
        );
        let router = cards(&merged()).remove(2);
        assert_eq!(
            card_edits(
                &router,
                vec![BuilderEdit::NodeTreeExclude {
                    node_id: "r1".into(),
                    excluded: true
                }]
            ),
            ["r1", "r2"].map(|id| BuilderEdit::NodeTreeExclude {
                node_id: id.into(),
                excluded: true
            })
        );
        let single = cards(&merged()).remove(0);
        let move_edit = vec![BuilderEdit::NodeRemove {
            node_id: "wan1".into(),
        }];
        assert_eq!(card_edits(&single, move_edit.clone()), move_edit);
    }

    #[test]
    fn every_parent_link_is_a_line_even_without_its_flow() {
        let nodes = merged();
        // The reply limit cut the flows: only one count arrived.
        let flows = [Flow {
            from: "wan1".into(),
            to: "router:hub-1".into(),
            subscribers: 3,
        }];
        let l = lines(&nodes, &flows);
        let pairs: Vec<(&str, &str, u64)> = l
            .iter()
            .map(|f| (f.from.as_str(), f.to.as_str(), f.subscribers))
            .collect();
        assert_eq!(
            pairs,
            [
                ("wan1", "router:hub-1", 3),
                ("wan2", "router:hub-1", 0),
                ("router:hub-1", "source:S", 0)
            ]
        );
    }

    #[test]
    fn the_router_branch_line_offers_no_removal() {
        let nodes = merged();
        assert_eq!(
            link_of(&nodes, "router:hub-1", "source:S"),
            Some(("r1".to_string(), "g1".to_string()))
        );
        assert!(!removable(&nodes, "r1", "g1"));
        assert_eq!(
            link_of(&nodes, "wan2", "router:hub-1"),
            Some(("wan2".to_string(), "r2".to_string()))
        );
        assert!(removable(&nodes, "wan2", "r2"));
        assert_eq!(link_of(&nodes, "wan1", "source:S"), None);
    }

    #[test]
    fn a_card_names_its_kind_in_one_short_word() {
        assert_eq!(card_kind(&kind(node("w", None, None), "wan")), "WAN");
        assert_eq!(card_kind(&router(node("r", None, None))), "Router");
        assert_eq!(
            card_kind(&kind(node("i", None, None), "intermediate")),
            "Node"
        );
        assert_eq!(card_kind(&node("g", None, None)), "Group");
        // A group with an access router is still a group.
        let mut g = node("g", None, None);
        g.router_id = Some("hub-1".into());
        assert_eq!(card_kind(&g), "Group");
    }

    #[test]
    fn a_connector_from_a_router_or_site_card_sets_the_parent() {
        let nodes = [
            kind(node("wan1", None, None), "wan"),
            router(node("r1", Some("wan1"), None)),
            kind(node("site", Some("r1"), None), "intermediate"),
            sourced(node("g1", Some("r1"), None)),
            sourced(node("g2", None, None)),
        ];
        assert_eq!(
            drop_edit(&nodes, "site", "g1"),
            Ok(BuilderEdit::NodeMove {
                node_id: "g1".into(),
                parent: Some("site".into())
            })
        );
        assert_eq!(
            drop_edit(&nodes, "r1", "g2"),
            Ok(BuilderEdit::NodeMove {
                node_id: "g2".into(),
                parent: Some("r1".into())
            })
        );
        assert!(drop_edit(&nodes, "site", "site").is_err());
        assert!(drop_edit(&nodes, "site", "r1").is_err(), "a loop");
        assert!(drop_edit(&nodes, "r1", "wan1").is_err());
        assert!(
            drop_edit(&nodes, "g2", "g1").is_err(),
            "a group has no children"
        );
        assert!(drop_edit(&nodes, "r1", "g1").is_err(), "already the parent");
        assert!(can_parent(&nodes[1]) && can_parent(&nodes[2]) && can_parent(&nodes[0]));
        assert!(!can_parent(&nodes[3]));
    }

    #[test]
    fn the_routing_warning_needs_a_wan_above_and_the_writer_off() {
        let nodes = [
            kind(node("wan1", None, None), "wan"),
            node("g1", Some("wan1"), None),
            node("g2", None, None),
        ];
        assert!(routing_warning(&nodes, "g1", true));
        assert!(!routing_warning(&nodes, "g1", false));
        assert!(!routing_warning(&nodes, "g2", true));
        assert!(!routing_warning(&nodes, "wan1", true));
    }

    #[test]
    fn the_surface_grows_past_the_last_card_and_keeps_a_minimum() {
        assert_eq!(surface(&[]), (1600.0, 1000.0));
        let far = vec![("n".to_string(), [3000.0, 2000.0])];
        assert_eq!(
            surface(&far),
            (3000.0 + 232.0 + 192.0, 2000.0 + 84.0 + 192.0)
        );
    }

    #[test]
    fn a_location_needs_both_values_and_keeps_the_text() {
        assert_eq!(location_draft(" ", ""), Ok(None));
        let l = location_draft("12.236199459585354", " 122.02419129509887 ")
            .unwrap()
            .unwrap();
        assert_eq!(l.longitude, "122.02419129509887");
        assert!(location_draft("12.5", "").unwrap_err().contains("both"));
        assert!(location_draft("95", "1").unwrap_err().contains("latitude"));
    }

    #[test]
    fn below_follows_parents_and_survives_a_cycle() {
        let nodes = [
            node("a", None, None),
            node("b", Some("a"), None),
            node("c", Some("b"), None),
            node("x", Some("y"), None),
            node("y", Some("x"), None),
        ];
        assert!(is_below(&nodes, "c", "a"));
        assert!(!is_below(&nodes, "a", "c"));
        assert!(!is_below(&nodes, "x", "a"));
    }

    #[test]
    fn a_paste_with_a_comma_fills_both_fields() {
        let l = split_paste("-12.5, 121.75").unwrap().unwrap();
        assert_eq!(
            (l.latitude.as_str(), l.longitude.as_str()),
            ("-12.5", "121.75")
        );
        assert!(split_paste("12.5").is_none());
        assert!(split_paste("12.5, 999").unwrap().is_err());
    }

    #[test]
    fn sankey_labels_name_tones_values_and_tiers() {
        use crate::sankey::Metric;
        let wans = ["wan1".to_string(), "wan2".to_string()];
        assert_eq!(wan_tone(&wans, Some("wan2")), "1");
        assert_eq!(wan_tone(&wans, Some("other")), "unknown");
        assert_eq!(wan_tone(&wans, None), "unknown");
        assert_eq!(value_text(Some(412.4), Metric::Capacity), "412 Mbps");
        assert_eq!(value_text(Some(2.25), Metric::Capacity), "2.2 Mbps");
        assert_eq!(value_text(Some(0.0), Metric::Capacity), "No capacity");
        assert_eq!(value_text(None, Metric::Subscribers), "No data");
        assert_eq!(live_text(Some(80.4)).as_deref(), Some("80 Mbps live"));
        assert_eq!(live_text(None), None);
        assert_eq!(value_text(Some(1.0), Metric::Subscribers), "1 sub");
        assert_eq!(value_text(Some(21.0), Metric::Subscribers), "21 subs");
        assert_eq!(tier_name("tier:dhcp"), Some("IPoE / DHCP"));
        assert_eq!(tier_name("source:x"), None);
    }

    #[test]
    fn the_live_chip_names_live_partial_and_subscriber_states() {
        use web_contract::topology::TopologyThroughput;
        let live = TopologyThroughput {
            updated: 95,
            complete: true,
            ..Default::default()
        };
        assert_eq!(live_chip(Some(&live), 100).0, "Live · 5s ago");
        let partial = TopologyThroughput {
            updated: 95,
            unmapped_hosts: 3,
            ..Default::default()
        };
        let (text, state, why) = live_chip(Some(&partial), 100);
        assert_eq!((text.as_str(), state), ("Partial", "warn"));
        assert!(why.starts_with("3 hosts have no subscriber"));
        let stale = TopologyThroughput {
            stale: true,
            reason: Some("the host sample is old".into()),
            ..Default::default()
        };
        assert_eq!(
            live_chip(Some(&stale), 100),
            (
                "No live traffic".into(),
                "muted",
                "the host sample is old".into()
            )
        );
        assert_eq!(live_chip(None, 100).1, "muted");
    }

    #[test]
    fn a_stream_without_wan_names_its_metric() {
        use crate::sankey::Metric;
        // Subscriber counts have no WAN split: the stream is every WAN, not an unknown one.
        assert_eq!(unknown_stream(Metric::Subscribers), "All WANs");
        assert_eq!(unknown_stream(Metric::Capacity), "WAN unknown");
    }

    #[test]
    fn a_wan_card_leads_with_its_configured_capacity() {
        let lines = wan_card_lines(Some(1000.0), 400.0, Some(275.4));
        assert_eq!(
            lines,
            (
                "1000 Mbps".to_string(),
                "400 Mbps allocated".to_string(),
                Some("40% allocated".to_string()),
                Some("275 Mbps live".to_string())
            )
        );
        // No configured capacity: the allocation stays, with no percent.
        assert_eq!(
            wan_card_lines(None, 400.0, None),
            (
                "No capacity".to_string(),
                "400 Mbps allocated".to_string(),
                None,
                None
            )
        );
        assert_eq!(
            wan_card_lines(Some(0.0), 0.0, None).2,
            None,
            "no percent of zero"
        );
    }
}
