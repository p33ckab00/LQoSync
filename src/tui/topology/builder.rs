//! The read-only Builder view: Discovery Inventory, Topology Tree, and Details.

use super::{Panel, TopologyView};
use crate::tui::theme::{Palette, Role};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use serde_json::Value;

/// Wide layouts show three panels from this width.
pub(super) const WIDE: u16 = 120;

/// Depth-first tree order: `(depth, node index)`. Built once for each preview.
pub(super) fn index(nodes: &[Value]) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(nodes.len());
    let mut work: Vec<(usize, usize)> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n["parent"].is_null())
        .map(|(i, _)| (0, i))
        .rev()
        .collect();
    while let Some((depth, i)) = work.pop() {
        out.push((depth, i));
        let id = nodes[i]["id"].as_str().unwrap_or_default();
        work.extend(
            nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n["parent"].as_str() == Some(id))
                .map(|(c, _)| (depth + 1, c))
                .rev(),
        );
    }
    out
}

pub(super) fn mbps(v: &Value) -> String {
    let n = v.as_f64().unwrap_or(0.0);
    if n.fract() == 0.0 {
        format!("{n:.0}")
    } else {
        format!("{n}")
    }
}

fn pair(v: &Value) -> String {
    format!("{}/{}", mbps(&v[0]), mbps(&v[1]))
}

fn panel_lines(v: &TopologyView, panel: Panel, height: usize) -> Vec<Line<'static>> {
    let nodes = v.nodes();
    match panel {
        Panel::Inventory => v
            .preview()
            .and_then(|p| p["inventory"].as_array())
            .into_iter()
            .flatten()
            .map(|s| {
                Line::from(format!(
                    "{}  {}  {} members",
                    s["label"].as_str().unwrap_or_default(),
                    s["kind"].as_str().unwrap_or_default(),
                    s["members"]
                ))
            })
            .collect(),
        Panel::Tree => {
            let start = v.node_selected.saturating_sub(height.saturating_sub(1));
            v.tree
                .iter()
                .enumerate()
                .skip(start)
                .take(height)
                .map(|(row, &(depth, i))| {
                    let n = &nodes[i];
                    let marker = if row == v.node_selected { "> " } else { "  " };
                    Line::from(format!(
                        "{marker}{}{}  {} Mbps  calc {}  {} subs",
                        "  ".repeat(depth),
                        n["label"].as_str().unwrap_or_default(),
                        pair(&n["exported_mbps"]),
                        pair(&n["calculated_mbps"]),
                        n["subscribers"]
                    ))
                })
                .collect()
        }
        Panel::Details => match v.selected_node() {
            None => vec![Line::from("no node selected")],
            Some(n) => [
                format!("id: {}", n["id"].as_str().unwrap_or_default()),
                format!("label: {}", n["label"].as_str().unwrap_or_default()),
                format!("parent: {}", n["parent"].as_str().unwrap_or("none")),
                format!("kind: {}", n["kind"].as_str().unwrap_or_default()),
                format!("source: {}", n["source"].as_str().unwrap_or("none")),
                format!("exported: {} Mbps", pair(&n["exported_mbps"])),
                format!("calculated: {} Mbps", pair(&n["calculated_mbps"])),
                format!("subscribers: {}", n["subscribers"]),
            ]
            .into_iter()
            .map(Line::from)
            .collect(),
        },
    }
}

fn title(panel: Panel) -> &'static str {
    match panel {
        Panel::Inventory => "Inventory",
        Panel::Tree => "Tree",
        Panel::Details => "Details",
    }
}

fn draw_panel(v: &TopologyView, f: &mut Frame, area: Rect, p: &Palette, panel: Panel) {
    let height = area.height.saturating_sub(2) as usize;
    f.render_widget(
        Paragraph::new(panel_lines(v, panel, height))
            .style(p.role(Role::Base))
            .block(crate::tui::design::panel(title(panel), p)),
        area,
    );
}

pub(super) fn draw(v: &TopologyView, f: &mut Frame, area: Rect, p: &Palette) {
    if area.width >= WIDE {
        let [a, b, c] = Layout::horizontal([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .areas(area);
        draw_panel(v, f, a, p, Panel::Inventory);
        draw_panel(v, f, b, p, Panel::Tree);
        draw_panel(v, f, c, p, Panel::Details);
        return;
    }
    let [selector, body] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    f.render_widget(
        Paragraph::new(format!(
            "focus: {}   [Left/Right] Inventory | Tree | Details",
            title(v.focus)
        ))
        .style(p.role(Role::Secondary)),
        selector,
    );
    draw_panel(v, f, body, p, v.focus);
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::tests::render;
    use super::super::*;
    use crate::tui::api::ApiAnswer;
    use serde_json::json;

    pub(crate) fn sample() -> TopologyView {
        let mut v = TopologyView::default();
        v.on_answer(ApiAnswer {
            op: "status".into(),
            result: Ok(json!({"registry_revision": 2, "enrolled": true})),
        });
        v.on_answer(ApiAnswer {
            op: "topology.preview.refresh".into(),
            result: Ok(json!({
                "collected_at": 0, "state": "ok", "registry_revision": 2, "conflicts": [], "rounded": [],
                "nodes": [
                    {"id": "g", "label": "Gold-Hub-1 [g]", "parent": null, "kind": "SubscriberGroup", "source": "Gold",
                     "calculated_mbps": [52.5, 52.5], "exported_mbps": [52.0, 52.0], "subscribers": 3},
                    {"id": "s", "label": "Silver-Hub-1 [s]", "parent": null, "kind": "SubscriberGroup", "source": "Silver",
                     "calculated_mbps": [17.5, 1.75], "exported_mbps": [17.0, 1.0], "subscribers": 1}
                ],
                "inventory": [
                    {"id": "src-g", "label": "Gold", "scope": "ppp", "kind": "Ppp", "members": 3, "instances": ["g"]},
                    {"id": "src-s", "label": "Silver", "scope": "ppp", "kind": "Ppp", "members": 1, "instances": ["s"]}
                ],
                "subscribers": []
            })),
        });
        v
    }

    fn corners(text: &str) -> usize {
        text.matches('┌').count()
    }

    #[test]
    fn wide_builder_has_three_panels() {
        let text = render(&sample(), 160, 30);
        for title in ["Inventory", "Tree", "Details"] {
            assert!(text.contains(title), "{title}: {text}");
        }
        assert_eq!(corners(&text), 3, "{text}");
    }

    #[test]
    fn narrow_builder_has_one_focused_panel_and_selectors() {
        let mut v = sample();
        let text = render(&v, 80, 24);
        assert_eq!(corners(&text), 1, "{text}");
        assert!(text.contains("focus: Tree"), "{text}");
        v.on_key(ratatui::crossterm::event::KeyCode::Left);
        assert!(render(&v, 80, 24).contains("focus: Inventory"));
    }

    #[test]
    fn tree_shows_labels_and_exported_and_calculated_mbps() {
        let text = render(&sample(), 160, 30);
        assert!(text.contains("Gold-Hub-1"), "{text}");
        assert!(text.contains("52/52 Mbps"), "{text}");
        assert!(text.contains("calc 52.5/52.5"), "{text}");
        assert!(text.contains("3 subs"), "{text}");
    }

    #[test]
    fn details_show_the_selected_node() {
        let mut v = sample();
        assert!(render(&v, 160, 30).contains("id: g"));
        v.on_key(ratatui::crossterm::event::KeyCode::Down);
        let text = render(&v, 160, 30);
        assert!(text.contains("id: s"), "{text}");
        assert!(text.contains("source: Silver"), "{text}");
    }
}
