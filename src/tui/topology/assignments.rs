//! The read-only Assignments view: subscribers, identity, endpoints, node, plan, and pin.

use super::TopologyView;
use super::builder::mbps;
use crate::tui::theme::{Palette, Role};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use serde_json::Value;

fn join(v: &Value) -> String {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
}

fn cut(text: &str, width: usize) -> String {
    let mut out: String = text.chars().take(width).collect();
    while out.chars().count() < width {
        out.push(' ');
    }
    out
}

/// The pinned node name, or nothing.
fn pin_name(v: &TopologyView, s: &Value) -> String {
    s["pin"]
        .as_str()
        .map(|id| v.node_name(id))
        .unwrap_or_default()
}

/// Offline pinned records after the online rows; `first` is the row index of the first one.
fn offline_lines(v: &TopologyView, first: usize) -> Vec<Line<'static>> {
    v.offline_pins()
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let marker = if first + i == v.sub_selected {
                "> "
            } else {
                "  "
            };
            Line::from(format!(
                "{marker}{}  offline  pinned {}",
                p["subscriber_id"].as_str().unwrap_or_default(),
                v.node_name(p["node_id"].as_str().unwrap_or_default())
            ))
        })
        .collect()
}

/// Below the wide width, keep identity, node, plan, and pin, and show the selected endpoints below.
fn draw_narrow(v: &TopologyView, f: &mut Frame, area: Rect, p: &Palette) {
    let height = area.height.saturating_sub(5) as usize;
    let subs = v.subscribers();
    let start = v.sub_selected.saturating_sub(height.saturating_sub(1));
    let mut lines = vec![
        Line::from(format!(
            "  {} {} {} pin",
            cut("identity", 30),
            cut("node", 20),
            cut("plan", 12)
        ))
        .style(p.role(Role::Secondary)),
    ];
    for (row, s) in subs.iter().enumerate().skip(start).take(height) {
        let marker = if row == v.sub_selected { "> " } else { "  " };
        lines.push(Line::from(format!(
            "{marker}{} {} {} {}",
            cut(s["account_ref"].as_str().unwrap_or_default(), 30),
            cut(&v.node_label(s["node"].as_str().unwrap_or_default()), 20),
            cut(s["profile"].as_str().unwrap_or("none"), 12),
            pin_name(v, s),
        )));
    }
    lines.extend(offline_lines(v, subs.len()));
    if let Some(s) = v.selected_subscriber() {
        lines.push(Line::from(format!("IPs: {}", join(&s["ips"]))).style(p.role(Role::Secondary)));
        lines.push(Line::from(format!("MAC: {}", join(&s["macs"]))).style(p.role(Role::Secondary)));
    }
    f.render_widget(
        Paragraph::new(lines)
            .style(p.role(Role::Base))
            .block(crate::tui::design::panel(
                format!("Assignments ({})", subs.len()),
                p,
            )),
        area,
    );
}

pub(super) fn draw(v: &TopologyView, f: &mut Frame, area: Rect, p: &Palette) {
    if area.width < super::builder::WIDE {
        return draw_narrow(v, f, area, p);
    }
    let height = area.height.saturating_sub(3) as usize;
    let subs = v.subscribers();
    let start = v.sub_selected.saturating_sub(height.saturating_sub(1));
    let mut lines = vec![
        Line::from(format!(
            "  {:<34} {:<16} {:<18} {:<22} {:<14} {:<7} {}",
            "identity", "IPs", "MAC", "node", "plan", "pin", "min / max Mbps"
        ))
        .style(p.role(Role::Secondary)),
    ];
    for (row, s) in subs.iter().enumerate().skip(start).take(height) {
        let marker = if row == v.sub_selected { "> " } else { "  " };
        let node = s["node"].as_str().unwrap_or_default();
        lines.push(Line::from(format!(
            "{marker}{:<34} {:<16} {:<18} {:<22} {:<14} {:<7} {}/{} / {}/{}",
            s["account_ref"].as_str().unwrap_or_default(),
            join(&s["ips"]),
            join(&s["macs"]),
            v.node_label(node),
            s["profile"].as_str().unwrap_or("none"),
            pin_name(v, s),
            mbps(&s["min_mbps"][0]),
            mbps(&s["min_mbps"][1]),
            mbps(&s["max_mbps"][0]),
            mbps(&s["max_mbps"][1]),
        )));
    }
    lines.extend(offline_lines(v, subs.len()));
    f.render_widget(
        Paragraph::new(lines)
            .style(p.role(Role::Base))
            .block(crate::tui::design::panel(
                format!("Assignments ({})", subs.len()),
                p,
            )),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::super::tests::render;
    use super::super::*;
    use crate::tui::api::ApiAnswer;
    use ratatui::crossterm::event::KeyCode;
    use serde_json::{Value, json};

    fn sub(i: usize) -> Value {
        json!({"id": format!("tm-sub-{i}"), "account_ref": format!("ppp-name/v1/acct-{i:04}"), "scope": "ppp",
               "ips": [format!("100.64.{}.{}", i / 250, i % 250 + 1)], "macs": ["02:00:00:00:00:11"],
               "node": "g", "pin": if i == 1 { json!("g") } else { Value::Null },
               "profile": "profile-g", "min_mbps": [5.0, 5.0], "max_mbps": [50.0, 50.0]})
    }
    fn preview(subs: Vec<Value>) -> ApiAnswer {
        ApiAnswer {
            op: "topology.preview.refresh".into(),
            result: Ok(
                json!({"collected_at": 0, "state": "ok", "conflicts": [], "rounded": [], "inventory": [],
                "nodes": [{"id": "g", "label": "Gold-Hub-1 [g]", "parent": null, "kind": "SubscriberGroup",
                           "source": "Gold", "calculated_mbps": [52.5, 52.5], "exported_mbps": [52.0, 52.0], "subscribers": 3}],
                "subscribers": subs}),
            ),
        }
    }
    fn view(subs: Vec<Value>) -> TopologyView {
        let mut v = TopologyView::default();
        v.on_answer(ApiAnswer {
            op: "status".into(),
            result: Ok(json!({"registry_revision": 2, "enrolled": true})),
        });
        v.on_answer(preview(subs));
        v.on_key(KeyCode::Char('a'));
        v
    }

    #[test]
    fn rows_show_identity_endpoints_node_plan_and_pin() {
        let text = render(&view(vec![sub(0), sub(1)]), 160, 20);
        for want in [
            "ppp-name/v1/acct-0001",
            "100.64.0.2",
            "02:00:00:00:00:11",
            "Gold-Hub-1",
            "profile-g",
            "50/50",
        ] {
            assert!(text.contains(want), "{want}: {text}");
        }
    }

    #[test]
    fn selection_survives_a_refresh_when_the_subscriber_remains() {
        let mut v = view(vec![sub(0), sub(1), sub(2)]);
        v.on_key(KeyCode::Down);
        v.on_key(KeyCode::Down);
        assert_eq!(v.selected_subscriber().unwrap()["id"], json!("tm-sub-2"));
        v.on_answer(preview(vec![sub(2), sub(0), sub(1), sub(3)]));
        assert_eq!(v.selected_subscriber().unwrap()["id"], json!("tm-sub-2"));
    }

    #[test]
    fn selection_moves_to_the_nearest_row_when_the_subscriber_is_gone() {
        let mut v = view(vec![sub(0), sub(1), sub(2)]);
        v.on_key(KeyCode::Down);
        v.on_key(KeyCode::Down);
        v.on_answer(preview(vec![sub(0), sub(1)]));
        assert_eq!(v.selected_subscriber().unwrap()["id"], json!("tm-sub-1"));
    }

    #[test]
    fn only_the_viewport_is_rendered_for_1000_subscribers() {
        let mut v = view((0..1000).map(sub).collect());
        for _ in 0..500 {
            v.on_key(KeyCode::Down);
        }
        let text = render(&v, 160, 20);
        assert!(text.contains("acct-0500"), "{text}");
        assert!(!text.contains("acct-0000"), "{text}");
        assert!(text.matches("ppp-name/v1/").count() <= 20);
    }

    #[test]
    fn narrow_rows_keep_node_plan_and_pin_and_show_endpoints_of_the_selected_row() {
        let mut v = view(vec![sub(0), sub(1)]);
        v.on_key(KeyCode::Down);
        let text = render(&v, 80, 20);
        for want in [
            "acct-0001",
            "Gold-Hub-1",
            "profile-g",
            "IPs: 100.64.0.2",
            "MAC: 02:00:00:00:00:11",
        ] {
            assert!(text.contains(want), "{want}: {text}");
        }
    }

    fn with_tree(mut v: TopologyView, pins: Value) -> TopologyView {
        v.on_answer(ApiAnswer {
            op: "topology.tree".into(),
            result: Ok(
                json!({"revision": 2, "sources": [], "pins": pins, "nodes": [
                    {"id": "wan1", "name": "WAN1", "parent": null, "kind": "wan"},
                    {"id": "wan2", "name": "WAN2", "parent": null, "kind": "wan"},
                    {"id": "g", "name": "Gold WAN1", "parent": "wan1", "kind": "group"}
                ]}),
            ),
        });
        v
    }

    fn identified(i: usize) -> Value {
        let mut s = sub(i);
        s["identities"] = json!([{"router_id": "r", "source_id": "ppp", "kind": "ppp",
                                  "account_ref": format!("ppp-name/v1/acct-{i:04}")}]);
        s
    }

    fn edits(v: &mut TopologyView) -> Vec<Value> {
        v.take_outbox()
            .into_iter()
            .filter(|(op, _)| op == "topology.edit.propose")
            .map(|(_, a)| a)
            .collect()
    }

    #[test]
    fn the_wan_picker_lists_wans_and_unpin_only_for_a_pinned_row() {
        let mut v = with_tree(view(vec![identified(0), identified(1)]), json!([]));
        v.on_key(KeyCode::Char('P'));
        let text = render(&v, 100, 24);
        assert!(text.contains("WAN1") && text.contains("WAN2"), "{text}");
        assert!(!text.contains("unpin"), "{text}");
        v.on_key(KeyCode::Esc);
        v.on_key(KeyCode::Down);
        v.on_key(KeyCode::Char('P'));
        assert!(render(&v, 100, 24).contains("unpin"));
    }

    #[test]
    fn enter_in_the_wan_picker_proposes_subscriber_pin_with_the_first_identity() {
        let mut v = with_tree(view(vec![identified(0)]), json!([]));
        v.take_outbox();
        v.on_key(KeyCode::Char('P'));
        v.on_key(KeyCode::Down);
        v.on_key(KeyCode::Enter);
        assert_eq!(
            edits(&mut v),
            vec![
                json!({"expected_revision": 2, "edit": {"kind": "subscriber.pin",
                "identity": identified(0)["identities"][0], "wan_node_id": "wan2"}})
            ]
        );
    }

    #[test]
    fn offline_pinned_records_show_after_online_rows_and_offer_only_unpin() {
        let pins = json!([{"subscriber_id": "tm-sub-off", "node_id": "g", "online": false},
                          {"subscriber_id": "tm-sub-1", "node_id": "g", "online": true}]);
        let mut v = with_tree(view(vec![identified(0)]), pins);
        v.take_outbox();
        let text = render(&v, 160, 20);
        assert!(
            text.contains("tm-sub-off  offline  pinned Gold WAN1"),
            "{text}"
        );
        v.on_key(KeyCode::Down);
        v.on_key(KeyCode::Char('P'));
        let text = render(&v, 100, 24);
        assert!(text.contains("unpin") && !text.contains("WAN2"), "{text}");
        v.on_key(KeyCode::Enter);
        assert_eq!(
            edits(&mut v),
            vec![
                json!({"expected_revision": 2, "edit": {"kind": "subscriber.pin",
                "subscriber_id": "tm-sub-off", "wan_node_id": null}})
            ]
        );
    }

    #[test]
    fn the_pin_column_shows_the_node_name() {
        let v = with_tree(view(vec![sub(0), sub(1)]), json!([]));
        assert!(render(&v, 160, 20).contains("Gold WAN1"));
        assert!(render(&v, 80, 20).contains("Gold WAN1"));
    }

    #[test]
    fn the_wan_picker_fits_80_columns() {
        let mut v = with_tree(view(vec![identified(1)]), json!([]));
        v.on_key(KeyCode::Char('P'));
        let text = render(&v, 80, 24);
        assert!(
            text.contains("unpin") && text.contains("Enter: propose"),
            "{text}"
        );
    }
}
