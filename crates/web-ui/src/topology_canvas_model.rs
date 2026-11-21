//! Pure geometry and form rules of the Topology canvas, so native tests cover them.
use crate::topology_model::tree_rows;
use web_contract::topology::{Location, Node};

pub(crate) const CARD_W: f64 = 232.0;
pub(crate) const CARD_H: f64 = 84.0;
const COLUMN: f64 = 300.0;
const ROW: f64 = 120.0;
const MARGIN: f64 = 48.0;
/// The canvas is never smaller than this, so there is room to drag a card outward.
const MIN_SURFACE: (f64, f64) = (1600.0, 1000.0);

/// The card position of each node: the saved position, else a sideways tree position.
/// A first child shares the row of its parent, so a chain reads left to right.
pub(crate) fn positions(nodes: &[Node]) -> Vec<(String, [f64; 2])> {
    let rows = tree_rows(nodes);
    let mut row_of = std::collections::HashMap::new();
    let mut next = 0usize;
    let mut out = Vec::with_capacity(rows.len());
    for (i, (depth, node)) in rows.iter().enumerate() {
        let first_child = i > 0 && node.parent.as_deref() == Some(rows[i - 1].1.id.as_str());
        let row = if first_child {
            row_of[rows[i - 1].1.id.as_str()]
        } else {
            next
        };
        next = next.max(row + 1);
        row_of.insert(node.id.as_str(), row);
        let auto = [MARGIN + *depth as f64 * COLUMN, MARGIN + row as f64 * ROW];
        out.push((node.id.clone(), node.position.unwrap_or(auto)));
    }
    out
}

/// An SVG path from the right port of the parent card to the left port of the child card.
pub(crate) fn wire(parent: [f64; 2], child: [f64; 2]) -> String {
    let (x1, y1) = (parent[0] + CARD_W, parent[1] + CARD_H / 2.0);
    let (x2, y2) = (child[0], child[1] + CARD_H / 2.0);
    let mid = (x1 + x2) / 2.0;
    format!("M{x1} {y1} C{mid} {y1} {mid} {y2} {x2} {y2}")
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
    use web_contract::topology::Capacity;

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
        }
    }

    #[test]
    fn unsaved_cards_form_a_sideways_tree() {
        let nodes = [
            node("wan1", None, None),
            node("g1", Some("wan1"), None),
            node("g2", Some("wan1"), None),
            node("wan2", None, None),
        ];
        let p: std::collections::HashMap<_, _> = positions(&nodes).into_iter().collect();
        assert_eq!(p["wan1"], [48.0, 48.0]);
        assert_eq!(p["g1"], [348.0, 48.0]);
        assert_eq!(p["g2"], [348.0, 168.0]);
        assert_eq!(p["wan2"], [48.0, 288.0]);
    }

    #[test]
    fn a_saved_position_wins_over_the_tree_position() {
        let nodes = [node("wan1", None, Some([900.5, 12.0]))];
        assert_eq!(positions(&nodes), vec![("wan1".to_string(), [900.5, 12.0])]);
    }

    #[test]
    fn a_wire_joins_the_parent_right_port_to_the_child_left_port() {
        assert_eq!(
            wire([0.0, 0.0], [300.0, 100.0]),
            "M232 42 C266 42 266 142 300 142"
        );
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
