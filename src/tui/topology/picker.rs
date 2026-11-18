//! The plan picker for the selected subscriber in Assignments.

use super::TopologyView;
use crate::tui::theme::{Palette, Role};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Clear, Paragraph};
use serde_json::Value;

#[derive(Debug, Default)]
pub(super) struct Picker {
    pub(super) selected: usize,
}

/// "source default" first, then each plan with its rate, in the service order.
pub(super) fn entries(policies: Option<&Value>) -> Vec<(Option<String>, String)> {
    let mut out = vec![(None, "source default".to_string())];
    let plans = policies.and_then(|p| p["plans"].as_array());
    for plan in plans.into_iter().flatten() {
        out.push((
            plan["id"].as_str().map(str::to_string),
            format!(
                "{}  {}/{} Mbps",
                plan["name"].as_str().unwrap_or_default(),
                plan["down_mbps"],
                plan["up_mbps"]
            ),
        ));
    }
    out
}

pub(super) fn draw(v: &TopologyView, picker: &Picker, f: &mut Frame, area: Rect, p: &Palette) {
    let list = entries(v.policies());
    let width = area.width.min(60);
    let height = (list.len() as u16 + 4).min(area.height);
    let popup = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    };
    let visible = height.saturating_sub(4) as usize;
    let start = picker.selected.saturating_sub(visible.saturating_sub(1));
    let mut lines: Vec<Line> = list
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .map(|(i, (_, label))| {
            let marker = if i == picker.selected { "> " } else { "  " };
            Line::from(format!("{marker}{label}"))
        })
        .collect();
    if v.policies().is_none() {
        lines.push(Line::from("loading plans").style(p.role(Role::Secondary)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from("Enter: attach  Esc: cancel").style(p.role(Role::Secondary)));
    let account = v
        .selected_subscriber()
        .and_then(|s| s["account_ref"].as_str())
        .unwrap_or_default();
    f.render_widget(Clear, popup);
    f.render_widget(
        Paragraph::new(lines)
            .style(p.role(Role::Base))
            .block(crate::tui::design::panel(format!("Plan for {account}"), p)),
        popup,
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
        json!({"id": format!("tm-sub-{i}"), "account_ref": format!("dhcp-mac/v1/02:00:00:00:00:{i:02}"), "scope": "dhcp-IOT",
               "identities": [{"router_id": "lq3-hub", "source_id": "dhcp-IOT", "kind": "dhcp",
                               "account_ref": format!("dhcp-mac/v1/02:00:00:00:00:{i:02}")}],
               "ips": ["100.64.0.9"], "macs": [], "node": "g", "pin": null,
               "profile": null, "min_mbps": [5.0, 5.0], "max_mbps": [50.0, 50.0]})
    }
    fn ok(op: &str, v: Value) -> ApiAnswer {
        ApiAnswer {
            op: op.into(),
            result: Ok(v),
        }
    }
    fn view() -> TopologyView {
        let mut v = TopologyView::default();
        v.on_answer(ok(
            "status",
            json!({"registry_revision": 2, "enrolled": true}),
        ));
        v.on_answer(ok(
            "topology.preview.refresh",
            json!({"collected_at": 0, "state": "ok", "conflicts": [], "rounded": [], "inventory": [],
                   "nodes": [{"id": "g", "label": "IOT-Hub-1 [g]", "parent": null, "kind": "SubscriberGroup",
                              "source": "IOT", "calculated_mbps": [50.0, 50.0], "exported_mbps": [50.0, 50.0], "subscribers": 2}],
                   "subscribers": [sub(0), sub(1)]}),
        ));
        v.on_answer(ok(
            "topology.policies",
            json!({"defaults": {}, "profiles": [], "plans": [
                {"id": "plan-1", "name": "100M", "down_mbps": 100, "up_mbps": 100, "subscribers": 0},
                {"id": "plan-2", "name": "7M", "down_mbps": 7, "up_mbps": 7, "subscribers": 3}]}),
        ));
        v.on_key(KeyCode::Char('a'));
        v.take_outbox();
        v
    }
    fn ops(v: &mut TopologyView) -> Vec<String> {
        v.take_outbox().into_iter().map(|(op, _)| op).collect()
    }

    #[test]
    fn the_picker_lists_source_default_then_plans() {
        let mut v = view();
        assert!(v.on_key(KeyCode::Enter));
        assert!(v.capturing());
        let text = render(&v, 120, 30);
        let default = text.find("source default").expect(&text);
        let fast = text.find("100M").expect(&text);
        let slow = text.find("7M").expect(&text);
        assert!(default < fast && fast < slow, "{text}");
        assert!(text.contains("100/100 Mbps"), "{text}");
    }

    #[test]
    fn enter_sends_subscriber_plan_with_the_first_identity() {
        let mut v = view();
        v.on_key(KeyCode::Down);
        v.on_key(KeyCode::Enter);
        v.on_key(KeyCode::Down);
        v.on_key(KeyCode::Enter);
        assert!(!v.capturing());
        let out = v.take_outbox();
        assert_eq!(
            out,
            vec![(
                "subscriber.plan".to_string(),
                json!({"expected_revision": 2, "identity": sub(1)["identities"][0], "plan_id": "plan-1"})
            )]
        );
        // "source default" detaches the plan.
        v.on_key(KeyCode::Enter);
        v.on_key(KeyCode::Enter);
        assert_eq!(v.take_outbox()[0].1["plan_id"], Value::Null);
    }

    #[test]
    fn esc_sends_nothing() {
        let mut v = view();
        v.on_key(KeyCode::Enter);
        // The picker owns all keys: a view key does not leave Assignments.
        assert!(v.on_key(KeyCode::Char('b')));
        assert_eq!(v.screen, Screen::Assignments);
        assert!(v.on_key(KeyCode::Esc));
        assert!(!v.capturing());
        assert!(v.take_outbox().is_empty());
    }

    #[test]
    fn a_conflict_refreshes_and_shows_a_message() {
        let mut v = view();
        v.on_answer(ApiAnswer {
            op: "subscriber.plan".into(),
            result: Err("Conflict: subscriber registry revision conflict".into()),
        });
        let sent = ops(&mut v);
        for want in ["topology.preview.refresh", "status", "topology.policies"] {
            assert!(sent.contains(&want.to_string()), "{want}: {sent:?}");
        }
        assert!(render(&v, 120, 30).contains("changed by another operator, try again"));
        v.on_answer(ok(
            "subscriber.plan",
            json!({"revision": 3, "subscriber_id": "tm-sub-1"}),
        ));
        assert!(ops(&mut v).contains(&"topology.preview.refresh".to_string()));
    }

    #[test]
    fn the_strip_says_edits_apply_after_activation() {
        assert!(render(&view(), 80, 20).contains("edits apply after activation"));
    }

    #[test]
    fn the_picker_fits_80_columns() {
        let mut v = view();
        v.on_key(KeyCode::Enter);
        let text = render(&v, 80, 20);
        for want in ["source default", "100M", "7M", "Enter", "Esc"] {
            assert!(text.contains(want), "{want}: {text}");
        }
    }
}
