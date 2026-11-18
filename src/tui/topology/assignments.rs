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

pub(super) fn draw(v: &TopologyView, f: &mut Frame, area: Rect, p: &Palette) {
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
            if s["pin"].is_null() { "" } else { "pinned" },
            mbps(&s["min_mbps"][0]),
            mbps(&s["min_mbps"][1]),
            mbps(&s["max_mbps"][0]),
            mbps(&s["max_mbps"][1]),
        )));
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
            "pinned",
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
}
