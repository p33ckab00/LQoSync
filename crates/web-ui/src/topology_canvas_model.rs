//! Pure geometry and form rules of the Topology canvas, so native tests cover them.
use crate::canvas_layout::{CardBox, Tier, edge_path, layout};
use web_contract::topology::{BuilderEdit, DiscoveredRouter, Location, Node};

pub(crate) const CARD_W: f64 = 232.0;
pub(crate) const CARD_H: f64 = 84.0;
const MARGIN: f64 = 48.0;
/// The canvas is never smaller than this, so there is room to drag a card outward.
const MIN_SURFACE: (f64, f64) = (1600.0, 1000.0);
pub(crate) const SIZE: [f64; 2] = [CARD_W, CARD_H];

/// The card ID of a default-route WAN interface that has no WAN node yet.
pub(crate) fn candidate_id(router_id: &str, interface: &str) -> String {
    format!("candidate:{router_id}:{interface}")
}

/// The nodes that the canvas shows. An inactive group is hidden; its settings stay.
pub(crate) fn visible(nodes: &[Node]) -> Vec<Node> {
    nodes.iter().filter(|n| n.active).cloned().collect()
}

/// The card position of each node and candidate WAN: the saved position, else the tier layout
/// (WAN row, router row, group row, a child below its parent).
pub(crate) fn positions(
    nodes: &[Node],
    candidates: &[DiscoveredRouter],
) -> Vec<(String, [f64; 2])> {
    let tier = |n: &Node| match (n.kind.as_str(), n.router_id.is_some()) {
        ("wan", _) => Tier::Wan,
        (_, true) => Tier::Router,
        ("group", _) => Tier::Group,
        _ => Tier::Other,
    };
    let mut cards: Vec<CardBox> = nodes
        .iter()
        .map(|n| CardBox {
            id: n.id.clone(),
            tier: tier(n),
            parent: n.parent.clone(),
            size: SIZE,
        })
        .collect();
    cards.extend(candidates.iter().flat_map(|r| {
        r.candidates.iter().map(|c| CardBox {
            id: candidate_id(&r.router_id, &c.name),
            tier: Tier::Wan,
            parent: None,
            size: SIZE,
        })
    }));
    let manual = nodes
        .iter()
        .filter_map(|n| n.position.map(|p| (n.id.clone(), p)))
        .collect();
    let at = layout(&cards, &manual);
    cards
        .into_iter()
        .map(|c| (at[&c.id], c.id))
        .map(|(p, id)| (id, p))
        .collect()
}

/// The positions to save: each saved position of a node (also of a hidden node), with the cards
/// that the operator moved. Automatic positions and candidate WAN cards are not saved, so the
/// service never gets an unknown ID and an unmoved card keeps its automatic place.
pub(crate) fn saved_positions(
    nodes: &[Node],
    moved: &std::collections::HashMap<String, [f64; 2]>,
) -> std::collections::BTreeMap<String, [f64; 2]> {
    let mut out: std::collections::BTreeMap<String, [f64; 2]> = nodes
        .iter()
        .filter_map(|n| n.position.map(|p| (n.id.clone(), p)))
        .collect();
    for n in nodes {
        if let Some(p) = moved.get(&n.id) {
            out.insert(n.id.clone(), *p);
        }
    }
    out
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

/// An SVG path from the bottom port of the parent card to the top port of the child card:
/// straight in one column, else a vertical curve.
pub(crate) fn wire(parent: [f64; 2], child: [f64; 2]) -> String {
    if (parent[0] - child[0]).abs() < 1.0 {
        return edge_path(parent, SIZE, child, SIZE).0;
    }
    let (x1, y1) = (parent[0] + CARD_W / 2.0, parent[1] + CARD_H);
    let (x2, y2) = (child[0] + CARD_W / 2.0, child[1]);
    let mid = (y1 + y2) / 2.0;
    format!("M{x1},{y1} C{x1},{mid} {x2},{mid} {x2},{y2}")
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
    let t = find(target).ok_or("The card is not on the canvas.")?;
    let f = find(from).ok_or("The card is not on the canvas.")?;
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
        return Ok(BuilderEdit::NodeMove {
            node_id: t.id.clone(),
            parent: Some(from.to_string()),
        });
    }
    let wan = from;
    if t.parent.as_deref() == Some(wan) {
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

/// Move a card by a keyboard or pointer step. A card never leaves the top-left edge.
pub(crate) fn nudge(p: [f64; 2], dx: f64, dy: f64) -> [f64; 2] {
    [(p[0] + dx).max(0.0).round(), (p[1] + dy).max(0.0).round()]
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

#[cfg(test)]
mod tests {
    use super::*;
    use web_contract::topology::{Capacity, WanCandidate};

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
    fn cards_follow_the_tiers_and_a_saved_position_wins() {
        let nodes = [
            kind(node("wan1", None, None), "wan"),
            router(node("r1", Some("wan1"), None)),
            node("g1", Some("r1"), None),
            node("g2", None, Some([900.5, 12.0])),
        ];
        let p: std::collections::HashMap<_, _> = positions(&nodes, &[]).into_iter().collect();
        assert!(p["wan1"][1] < p["r1"][1] && p["r1"][1] < p["g1"][1]);
        assert_eq!(p["g1"][0], p["r1"][0]);
        assert_eq!(p["g2"], [900.5, 12.0]);
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
    fn a_candidate_wan_is_a_card() {
        let nodes = [kind(node("wan1", None, None), "wan")];
        let p: std::collections::HashMap<_, _> =
            positions(&nodes, &candidates()).into_iter().collect();
        let id = candidate_id("core", "pppoe-out2");
        assert_eq!(p[&id][1], p["wan1"][1], "the candidate is in the WAN row");
        assert_ne!(p[&id], p["wan1"]);
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
        // One column: a line from the bottom of the parent to the top of the child.
        assert_eq!(wire([0.0, 0.0], [0.0, 200.0]), "M116,84 L116,200");
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
    fn a_wire_leaves_the_bottom_port_and_enters_the_top_port() {
        // Another column: a curve from the bottom center of the parent to the top center.
        assert_eq!(
            wire([0.0, 0.0], [300.0, 200.0]),
            "M116,84 C116,142 416,142 416,200"
        );
    }

    #[test]
    fn the_saved_layout_keeps_hidden_cards_and_skips_candidates() {
        let mut hidden = node("old", None, Some([5.0, 5.0]));
        hidden.active = false;
        let nodes = [
            node("a", None, Some([1.0, 2.0])),
            node("b", None, None),
            hidden,
        ];
        let moved = std::collections::HashMap::from([
            ("b".to_string(), [30.0, 40.0]),
            (candidate_id("core", "pppoe-out1"), [9.0, 9.0]),
        ]);
        let saved = saved_positions(&nodes, &moved);
        assert_eq!(
            saved.into_iter().collect::<Vec<_>>(),
            vec![
                ("a".to_string(), [1.0, 2.0]),
                ("b".to_string(), [30.0, 40.0]),
                ("old".to_string(), [5.0, 5.0]),
            ]
        );
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
    fn a_nudge_stays_inside_the_canvas() {
        assert_eq!(nudge([10.0, 10.0], -16.0, 16.4), [0.0, 26.0]);
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
}
