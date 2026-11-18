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

/// The WAN candidates section of the Inventory panel.
fn wan_lines(v: &TopologyView) -> Vec<Line<'static>> {
    let text = |x: &Value| x.as_str().unwrap_or_default().to_string();
    let mut out = vec![String::new(), "WAN candidates".to_string()];
    if v.discovering {
        out.push("reading routers".into());
    } else if v.wan.is_none() {
        out.push("[w] discover WANs".into());
    }
    let wan = v.wan.as_ref();
    for r in wan
        .and_then(|w| w["routers"].as_array())
        .into_iter()
        .flatten()
    {
        if r["state"] == "ok" {
            out.push(format!("{}  ok", text(&r["name"])));
        } else {
            out.push(format!(
                "{}  failed: {}",
                text(&r["name"]),
                text(&r["error"])
            ));
        }
        for c in r["candidates"].as_array().into_iter().flatten() {
            let mut line = format!(
                "  {}  {}  {}",
                text(&c["name"]),
                text(&c["type"]),
                if c["running"] == true { "up" } else { "down" }
            );
            if c["disabled"] == true {
                line.push_str("  disabled");
            }
            if c["suggested"] == true {
                line.push_str("  WAN?");
            }
            let used = c["used_by"].as_array().map_or(0, Vec::len);
            if used > 0 {
                line.push_str(&format!("  used by {used}"));
            }
            out.push(line);
        }
        for m in r["missing"].as_array().into_iter().flatten() {
            out.push(format!("  {}  missing", text(&m["endpoint_ref"])));
        }
    }
    for u in wan
        .and_then(|w| w["unconfigured"].as_array())
        .into_iter()
        .flatten()
    {
        out.push(format!(
            "{}  router {} not configured",
            text(&u["endpoint_ref"]),
            text(&u["router_id"])
        ));
    }
    out.into_iter().map(Line::from).collect()
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
            .chain(wan_lines(v))
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

    fn key(v: &mut TopologyView, c: char) -> bool {
        v.on_key(ratatui::crossterm::event::KeyCode::Char(c))
    }

    fn ops(v: &mut TopologyView) -> Vec<String> {
        v.take_outbox().into_iter().map(|(op, _)| op).collect()
    }

    fn discovered() -> TopologyView {
        let mut v = sample();
        v.focus = Panel::Inventory;
        v.on_answer(ApiAnswer {
            op: "wan.discover".into(),
            result: Ok(json!({
                "collected_at": 0,
                "routers": [
                    {"router_id": "r-core", "name": "Core", "state": "ok", "error": null,
                     "candidates": [
                        {"name": "pppoe-out1", "type": "pppoe-out", "running": true, "disabled": false,
                         "comment": null, "suggested": true, "source_id": "wan-1", "used_by": ["n1", "n2"]},
                        {"name": "ether5", "type": "ether", "running": false, "disabled": true,
                         "comment": null, "suggested": false, "source_id": null, "used_by": []}
                     ],
                     "missing": [{"source_id": "wan-2", "endpoint_ref": "pppoe-out2"}]},
                    {"router_id": "r-distro", "name": "Distro", "state": "failed", "error": "login refused",
                     "candidates": [], "missing": []}
                ],
                "unconfigured": [{"source_id": "wan-3", "router_id": "r-old", "endpoint_ref": "pppoe-out9"}]
            })),
        });
        v
    }

    #[test]
    fn w_in_the_builder_sends_wan_discover_once_while_it_runs() {
        let mut v = sample();
        ops(&mut v);
        assert!(key(&mut v, 'w'));
        assert_eq!(ops(&mut v), ["wan.discover"]);
        key(&mut v, 'w');
        assert!(ops(&mut v).is_empty(), "one discovery at a time");
        v.on_answer(ApiAnswer {
            op: "wan.discover".into(),
            result: Err("Internal: boom".into()),
        });
        key(&mut v, 'w');
        assert_eq!(
            ops(&mut v),
            ["wan.discover"],
            "an answer ends the discovery"
        );
    }

    #[test]
    fn a_discovery_during_a_preview_refresh_sends_both() {
        let mut v = sample();
        ops(&mut v);
        key(&mut v, 'r');
        key(&mut v, 'w');
        let sent = ops(&mut v);
        assert!(
            sent.contains(&"topology.preview.refresh".to_string()),
            "{sent:?}"
        );
        assert!(sent.contains(&"wan.discover".to_string()), "{sent:?}");
        v.on_answer(ApiAnswer {
            op: "wan.discover".into(),
            result: Ok(json!({"collected_at": 0, "routers": [], "unconfigured": []})),
        });
        assert!(
            v.refreshing,
            "the WAN answer does not end the preview refresh"
        );
        assert!(v.wan.is_some());
    }

    #[test]
    fn before_discovery_the_section_shows_the_key() {
        let mut v = sample();
        v.focus = Panel::Inventory;
        let text = render(&v, 80, 24);
        assert!(text.contains("WAN candidates"), "{text}");
        assert!(text.contains("[w] discover WANs"), "{text}");
        key(&mut v, 'w');
        assert!(render(&v, 80, 24).contains("reading routers"));
    }

    #[test]
    fn the_section_lists_candidates_with_marks_errors_and_missing_entries() {
        let text = render(&discovered(), 160, 30);
        for want in [
            "Core  ok",
            "pppoe-out1  pppoe-out  up  WAN?  used by 2",
            "ether5  ether  down  disabled",
            "pppoe-out2  missing",
            "Distro  failed: login refused",
            "pppoe-out9  router r-old not configured",
        ] {
            assert!(text.contains(want), "{want}: {text}");
        }
    }

    #[test]
    fn the_wan_section_fits_80_columns() {
        let text = render(&discovered(), 80, 30);
        assert!(text.contains("pppoe-out1  pppoe-out  up  WAN?"), "{text}");
        assert!(text.contains("Distro  failed: login refused"), "{text}");
    }
}
