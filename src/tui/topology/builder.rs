//! The read-only Builder view: Discovery Inventory, Topology Tree, and Details.

use super::{Panel, TopologyView, likely_wan};
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

/// The New sources section: draft sources that the registry does not know.
fn candidate_lines(v: &TopologyView) -> Vec<Line<'static>> {
    let first = v
        .preview()
        .and_then(|p| p["inventory"].as_array())
        .map_or(0, Vec::len);
    let mut out = vec![Line::from(""), Line::from("New sources  [i] load")];
    for (i, c) in v.candidate_list().iter().enumerate() {
        out.push(Line::from(format!(
            "{}{}  {}",
            mark(v, first + i),
            c["name"].as_str().unwrap_or_default(),
            c["kind"].as_str().unwrap_or_default()
        )));
    }
    out
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
    let mut item = v
        .preview()
        .and_then(|p| p["inventory"].as_array())
        .map_or(0, Vec::len)
        + v.candidate_list().len();
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
        // Only likely WANs and used interfaces get a line, so every router fits the panel.
        let (likely, other): (Vec<&Value>, Vec<&Value>) = r["candidates"]
            .as_array()
            .into_iter()
            .flatten()
            .partition(|c| likely_wan(c));
        for c in likely {
            let selectable = r["state"] == "ok";
            let marker = if selectable { mark(v, item) } else { "  " };
            item += usize::from(selectable);
            let mut line = format!(
                "{marker}{}  {}  {}",
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
        if !other.is_empty() {
            out.push(format!("  {} other interfaces", other.len()));
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

/// A short capacity text for a catalog node.
fn capacity(c: &Value) -> String {
    match c["mode"].as_str().unwrap_or_default() {
        "fixed" => format!("fixed {}/{}", c["down_mbps"], c["up_mbps"]),
        "wan_percent" => format!("wan {}%/{}%", c["down_percent"], c["up_percent"]),
        mode => mode.to_string(),
    }
}

/// The selection marker of Inventory item `item`.
fn mark(v: &TopologyView, item: usize) -> &'static str {
    if v.focus == Panel::Inventory && v.inventory_selected == item {
        "> "
    } else {
        "  "
    }
}

fn panel_lines(v: &TopologyView, panel: Panel, height: usize) -> Vec<Line<'static>> {
    let nodes = v.tree_nodes();
    match panel {
        Panel::Inventory => v
            .preview()
            .and_then(|p| p["inventory"].as_array())
            .into_iter()
            .flatten()
            .enumerate()
            .map(|(i, s)| {
                Line::from(format!(
                    "{}{}  {}  {} members",
                    mark(v, i),
                    s["label"].as_str().unwrap_or_default(),
                    s["kind"].as_str().unwrap_or_default(),
                    s["members"]
                ))
            })
            .chain(candidate_lines(v))
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
                    let indent = "  ".repeat(depth);
                    match v.preview_node(n["id"].as_str().unwrap_or_default()) {
                        Some(pv) => Line::from(format!(
                            "{marker}{indent}{}  {} Mbps  calc {}  {} subs",
                            pv["label"].as_str().unwrap_or_default(),
                            pair(&pv["exported_mbps"]),
                            pair(&pv["calculated_mbps"]),
                            pv["subscribers"]
                        )),
                        None => Line::from(format!(
                            "{marker}{indent}{}  {}  {}  no preview",
                            n["name"].as_str().unwrap_or_default(),
                            n["kind"].as_str().unwrap_or_default(),
                            capacity(&n["capacity"])
                        )),
                    }
                })
                .collect()
        }
        Panel::Details => match v.selected_node() {
            None => vec![Line::from("no node selected")],
            Some(n) => {
                let id = n["id"].as_str().unwrap_or_default();
                let pv = v.preview_node(id);
                let label = pv
                    .and_then(|p| p["label"].as_str())
                    .or(n["name"].as_str())
                    .unwrap_or_default();
                let mut out = vec![
                    format!("id: {id}"),
                    format!("label: {label}"),
                    format!("parent: {}", n["parent"].as_str().unwrap_or("none")),
                    format!("kind: {}", n["kind"].as_str().unwrap_or_default()),
                ];
                if !n["capacity"].is_null() {
                    out.push(format!("capacity: {}", capacity(&n["capacity"])));
                }
                match pv {
                    Some(p) => out.extend([
                        format!("source: {}", p["source"].as_str().unwrap_or("none")),
                        format!("exported: {} Mbps", pair(&p["exported_mbps"])),
                        format!("calculated: {} Mbps", pair(&p["calculated_mbps"])),
                        format!("subscribers: {}", p["subscribers"]),
                    ]),
                    None => out.push("no preview for this node".into()),
                }
                out.into_iter().map(Line::from).collect()
            }
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
            "1 other interfaces",
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

    #[test]
    fn the_core_wans_stay_visible_after_a_router_with_many_interfaces() {
        let mut v = sample();
        v.focus = Panel::Inventory;
        let ports: Vec<_> = (0..40)
            .map(|i| json!({"name": format!("vlan{i}"), "type": "vlan", "running": true, "disabled": false,
                "comment": null, "suggested": false, "source_id": null, "used_by": []}))
            .collect();
        v.on_answer(ApiAnswer {
            op: "wan.discover".into(),
            result: Ok(json!({"collected_at": 0, "unconfigured": [], "routers": [
                {"router_id": "d", "name": "Distro", "state": "ok", "error": null, "candidates": ports, "missing": []},
                {"router_id": "c", "name": "Core", "state": "ok", "error": null, "missing": [], "candidates": [
                    {"name": "pppoe-out1", "type": "pppoe-out", "running": true, "disabled": false,
                     "comment": null, "suggested": true, "source_id": null, "used_by": []}]}
            ]})),
        });
        let text = render(&v, 80, 24);
        assert!(text.contains("40 other interfaces"), "{text}");
        assert!(text.contains("pppoe-out1  pppoe-out  up  WAN?"), "{text}");
    }

    fn with_tree(mut v: TopologyView) -> TopologyView {
        v.on_answer(ApiAnswer {
            op: "topology.tree".into(),
            result: Ok(json!({"revision": 2, "sources": [], "nodes": [
                {"id": "wan1", "name": "WAN1", "parent": null, "kind": "wan", "source_id": "w1",
                 "capacity": {"mode": "fixed", "down_mbps": 500, "up_mbps": 500, "factor_percent": null}, "alias": false},
                {"id": "g", "name": "Gold", "parent": "wan1", "kind": "group", "source_id": "src-g",
                 "capacity": {"mode": "automatic"}, "alias": true},
                {"id": "g2", "name": "Gold WAN2", "parent": "wan1", "kind": "group", "source_id": "src-g",
                 "capacity": {"mode": "automatic"}, "alias": false}
            ]})),
        });
        v
    }

    #[test]
    fn the_tree_shows_catalog_nodes_without_a_preview() {
        let mut v = TopologyView::default();
        v.on_answer(ApiAnswer {
            op: "status".into(),
            result: Ok(json!({"registry_revision": 2, "enrolled": true})),
        });
        let v = with_tree(v);
        let text = render(&v, 160, 30);
        assert!(text.contains("WAN1  wan  fixed 500/500"), "{text}");
        assert!(
            text.contains("Gold  group  automatic  no preview"),
            "{text}"
        );
    }

    #[test]
    fn an_empty_instance_stays_visible() {
        let text = render(&with_tree(sample()), 160, 30);
        // `g` joins its preview row; `g2` has none.
        assert!(text.contains("Gold-Hub-1 [g]"), "{text}");
        assert!(
            text.contains("Gold WAN2  group  automatic  no preview"),
            "{text}"
        );
    }

    #[test]
    fn up_and_down_select_inventory_sources_and_wan_candidates() {
        let mut v = discovered();
        assert_eq!(
            v.inventory_item(),
            Some(InventoryItem::Source("src-g".into()))
        );
        v.on_key(ratatui::crossterm::event::KeyCode::Down);
        assert_eq!(
            v.inventory_item(),
            Some(InventoryItem::Source("src-s".into()))
        );
        v.on_key(ratatui::crossterm::event::KeyCode::Down);
        assert_eq!(
            v.inventory_item(),
            Some(InventoryItem::Wan {
                router_id: "r-core".into(),
                endpoint_ref: "pppoe-out1".into(),
            })
        );
        v.on_key(ratatui::crossterm::event::KeyCode::Down);
        assert_eq!(
            v.inventory_item(),
            Some(InventoryItem::Wan {
                router_id: "r-core".into(),
                endpoint_ref: "pppoe-out1".into(),
            }),
            "the last item stays selected"
        );
        assert!(
            render(&v, 160, 30).contains("> "),
            "a marker shows the selection"
        );
    }

    fn edits(v: &mut TopologyView) -> Vec<Value> {
        v.take_outbox()
            .into_iter()
            .filter(|(op, _)| op == "topology.edit.propose")
            .map(|(_, a)| a)
            .collect()
    }

    fn select(v: &mut TopologyView, id: &str) {
        v.node_selected = v
            .tree
            .iter()
            .position(|&(_, i)| v.tree_nodes()[i]["id"] == id)
            .unwrap();
    }

    fn press(v: &mut TopologyView, code: ratatui::crossterm::event::KeyCode) {
        v.on_key(code);
    }

    use ratatui::crossterm::event::KeyCode as K;

    #[test]
    fn promote_proposes_the_grandparent_and_demote_lists_siblings() {
        let mut v = with_tree(sample());
        v.take_outbox();
        select(&mut v, "g");
        press(&mut v, K::Char('<'));
        assert_eq!(
            edits(&mut v),
            vec![
                json!({"expected_revision": 2, "edit": {"kind": "node.move", "node_id": "g", "parent": null}})
            ]
        );
        select(&mut v, "g");
        press(&mut v, K::Char('>'));
        // `g2` is a group, so `g` has no sibling that can hold it.
        assert!(render(&v, 100, 30).contains("no valid parent"));
        press(&mut v, K::Esc);
        assert!(!v.capturing());
    }

    #[test]
    fn enter_on_a_source_proposes_instance_add_below_the_chosen_parent() {
        let mut v = with_tree(sample());
        v.focus = Panel::Inventory;
        v.take_outbox();
        press(&mut v, K::Enter);
        assert!(v.capturing());
        press(&mut v, K::Down);
        press(&mut v, K::Enter);
        assert_eq!(
            edits(&mut v),
            vec![
                json!({"expected_revision": 2, "edit": {"kind": "instance.add", "source_id": "src-g", "parent": "wan1"}})
            ]
        );
    }

    #[test]
    fn enter_on_a_wan_candidate_opens_the_wan_form() {
        let mut v = discovered();
        v.inventory_selected = 2;
        press(&mut v, K::Enter);
        assert!(render(&v, 100, 30).contains("New WAN"));
    }

    #[test]
    fn remove_proposes_node_remove_and_edit_opens_the_form() {
        let mut v = with_tree(sample());
        v.take_outbox();
        select(&mut v, "g2");
        press(&mut v, K::Char('x'));
        assert_eq!(
            edits(&mut v),
            vec![json!({"expected_revision": 2, "edit": {"kind": "node.remove", "node_id": "g2"}})]
        );
        press(&mut v, K::Char('e'));
        assert!(render(&v, 100, 30).contains("Edit node"));
        press(&mut v, K::Esc);
        press(&mut v, K::Char('n'));
        assert!(render(&v, 100, 30).contains("New node"));
    }

    #[test]
    fn g_and_shift_g_propose_one_group_and_all_groups() {
        let mut v = with_tree(sample());
        v.focus = Panel::Inventory;
        v.take_outbox();
        press(&mut v, K::Char('g'));
        press(&mut v, K::Char('G'));
        assert_eq!(
            edits(&mut v),
            vec![
                json!({"expected_revision": 2, "edit": {"kind": "rebalance", "source_id": "src-g"}}),
                json!({"expected_revision": 2, "edit": {"kind": "rebalance", "source_id": null}}),
            ]
        );
    }

    fn with_candidates(mut v: TopologyView) -> TopologyView {
        v.on_answer(ApiAnswer {
            op: "topology.sources.candidates".into(),
            result: Ok(json!({"candidates": [
                {"key": "src-bronze", "kind": "ppp", "router": "r", "name": "Bronze-Hub-1", "subscribers": null}
            ]})),
        });
        v
    }

    #[test]
    fn new_sources_list_and_enter_proposes_source_add() {
        let mut v = with_candidates(with_tree(sample()));
        v.focus = Panel::Inventory;
        let text = render(&v, 160, 30);
        assert!(text.contains("New sources"), "{text}");
        assert!(text.contains("Bronze-Hub-1  ppp"), "{text}");
        v.take_outbox();
        // Inventory items: two preview sources, then the candidate.
        v.inventory_selected = 2;
        press(&mut v, K::Enter);
        assert!(v.capturing());
        press(&mut v, K::Enter);
        assert_eq!(
            edits(&mut v),
            vec![
                json!({"expected_revision": 2, "edit": {"kind": "source.add", "key": "src-bronze", "parent": null}})
            ]
        );
    }

    #[test]
    fn i_loads_the_new_sources() {
        let mut v = with_tree(sample());
        v.take_outbox();
        press(&mut v, K::Char('i'));
        let ops: Vec<String> = v.take_outbox().into_iter().map(|(op, _)| op).collect();
        assert_eq!(ops, ["topology.sources.candidates"]);
    }

    #[test]
    fn the_strip_counts_fallback_subscribers() {
        let mut v = sample();
        v.on_answer(ApiAnswer {
            op: "topology.preview.refresh".into(),
            result: Ok(
                json!({"collected_at": 0, "state": "ok", "nodes": [], "inventory": [],
                "subscribers": [{"id": "a", "fallback": true}, {"id": "b", "fallback": false}]}),
            ),
        });
        assert!(render(&v, 160, 30).contains("1 subscribers in fallback"));
    }

    #[test]
    fn fallback_dialogs_fit_80_columns() {
        let mut v = sample();
        v.on_answer(ApiAnswer {
            op: "topology.tree".into(),
            result: Ok(json!({"revision": 2, "sources": [], "pins": [], "fallback": null, "nodes": [
                {"id": "fb", "name": "Fallback", "parent": null, "kind": "group", "source_id": null,
                 "capacity": {"mode": "automatic"}, "alias": false}
            ]})),
        });
        select(&mut v, "fb");
        press(&mut v, K::Char('F'));
        assert!(
            render(&v, 80, 24).contains("Default down Mbps"),
            "{}",
            render(&v, 80, 24)
        );
    }
}
