//! The plan form (name, down Mbps, up Mbps) and the delete question in Policies.

use crate::tui::theme::{Palette, Role};
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Clear, Paragraph};
use serde_json::{Value, json};

const LABELS: [&str; 3] = ["Name", "Down Mbps", "Up Mbps"];

#[derive(Debug, Default)]
pub(super) struct PlanForm {
    /// The plan ID when the form edits a plan. `None` creates a new plan.
    plan_id: Option<String>,
    fields: [String; 3],
    errors: [Option<&'static str>; 3],
    focus: usize,
}

/// What a key in the form does.
pub(super) enum Step {
    Stay,
    Cancel,
    /// The operation and its arguments, without `expected_revision`.
    Send(&'static str, Value),
}

fn rate(text: &str) -> Option<u64> {
    text.parse::<u64>().ok().filter(|&n| n > 0)
}

impl PlanForm {
    pub(super) fn edit(plan: &Value) -> Self {
        Self {
            plan_id: plan["id"].as_str().map(str::to_string),
            fields: [
                plan["name"].as_str().unwrap_or_default().to_string(),
                plan["down_mbps"].to_string(),
                plan["up_mbps"].to_string(),
            ],
            ..Self::default()
        }
    }

    /// Check the fields before send. `plans` are the plans from the policy view.
    fn validate(&mut self, plans: &[Value]) -> bool {
        let name = self.fields[0].trim();
        let taken = plans.iter().any(|p| {
            p["name"].as_str() == Some(name) && p["id"].as_str() != self.plan_id.as_deref()
        });
        self.errors[0] = match name.chars().count() {
            0 | 65.. => Some("use 1 to 64 characters"),
            _ if taken => Some("a plan with this name exists"),
            _ => None,
        };
        for i in [1, 2] {
            self.errors[i] = rate(&self.fields[i])
                .is_none()
                .then_some("use a positive whole number");
        }
        self.errors.iter().all(Option::is_none)
    }

    pub(super) fn key(&mut self, code: KeyCode, plans: &[Value]) -> Step {
        match code {
            KeyCode::Esc => return Step::Cancel,
            KeyCode::Tab | KeyCode::Down => self.focus = (self.focus + 1) % 3,
            KeyCode::BackTab | KeyCode::Up => self.focus = (self.focus + 2) % 3,
            KeyCode::Backspace => {
                self.fields[self.focus].pop();
            }
            KeyCode::Char(c) if self.fields[self.focus].chars().count() < 64 => {
                self.fields[self.focus].push(c);
            }
            KeyCode::Enter if self.validate(plans) => {
                let name = self.fields[0].trim();
                let (down, up) = (rate(&self.fields[1]), rate(&self.fields[2]));
                return match &self.plan_id {
                    None => Step::Send(
                        "plan.create",
                        json!({"name": name, "down_mbps": down, "up_mbps": up}),
                    ),
                    Some(id) => Step::Send(
                        "plan.update",
                        json!({"plan_id": id, "name": name, "down_mbps": down, "up_mbps": up}),
                    ),
                };
            }
            _ => {}
        }
        Step::Stay
    }

    pub(super) fn draw(&self, f: &mut Frame, area: Rect, p: &Palette) {
        let mut lines = Vec::new();
        for (i, label) in LABELS.iter().enumerate() {
            let marker = if i == self.focus { "> " } else { "  " };
            let cursor = if i == self.focus { "_" } else { "" };
            lines.push(Line::from(format!(
                "{marker}{label:<10} {}{cursor}",
                self.fields[i]
            )));
            if let Some(e) = self.errors[i] {
                lines.push(Line::from(format!("             {e}")).style(p.role(Role::Error)));
            }
        }
        lines.push(Line::from(""));
        lines.push(
            Line::from("Tab: next field  Enter: save  Esc: cancel").style(p.role(Role::Secondary)),
        );
        let title = if self.plan_id.is_some() {
            "Edit plan"
        } else {
            "New plan"
        };
        popup(f, area, lines, title, p);
    }
}

/// Draw `lines` in a centered box of at most 60 columns.
pub(super) fn popup(f: &mut Frame, area: Rect, lines: Vec<Line>, title: &str, p: &Palette) {
    let width = area.width.min(60);
    let height = (lines.len() as u16 + 2).min(area.height);
    let rect = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    };
    f.render_widget(Clear, rect);
    f.render_widget(
        Paragraph::new(lines)
            .style(p.role(Role::Base))
            .block(crate::tui::design::panel(title.to_string(), p)),
        rect,
    );
}

#[cfg(test)]
mod tests {
    use super::super::tests::render;
    use super::super::*;
    use crate::tui::api::ApiAnswer;
    use ratatui::crossterm::event::KeyCode;
    use serde_json::{Value, json};

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
        v.on_key(KeyCode::Char('p'));
        v.on_answer(ok(
            "topology.policies",
            json!({"defaults": {}, "profiles": [], "plans": [
                {"id": "plan-1", "name": "100M", "down_mbps": 100, "up_mbps": 100, "subscribers": 0},
                {"id": "plan-2", "name": "7M", "down_mbps": 7, "up_mbps": 7, "subscribers": 3}]}),
        ));
        v.take_outbox();
        v
    }
    fn typed(v: &mut TopologyView, text: &str) {
        for c in text.chars() {
            v.on_key(KeyCode::Char(c));
        }
    }
    /// Open a new form and fill the three fields.
    fn fill(v: &mut TopologyView, name: &str, down: &str, up: &str) {
        v.on_key(KeyCode::Char('n'));
        typed(v, name);
        v.on_key(KeyCode::Tab);
        typed(v, down);
        v.on_key(KeyCode::Tab);
        typed(v, up);
    }

    #[test]
    fn the_plans_section_lists_rate_and_use_count() {
        let text = render(&view(), 80, 24);
        for want in ["Plans", "100M", "100/100 Mbps", "7M", "3 subscribers"] {
            assert!(text.contains(want), "{want}: {text}");
        }
    }

    #[test]
    fn bad_rates_and_duplicate_names_show_field_errors_and_send_nothing() {
        for bad in ["0", "abc", "1.5"] {
            let mut v = view();
            fill(&mut v, "50M", bad, "50");
            v.on_key(KeyCode::Enter);
            assert!(v.capturing(), "{bad}: the form stays open");
            assert!(v.take_outbox().is_empty(), "{bad}");
            let text = render(&v, 80, 24);
            assert!(text.contains("positive whole number"), "{bad}: {text}");
        }
        let mut v = view();
        fill(&mut v, "7M", "7", "7");
        v.on_key(KeyCode::Enter);
        assert!(v.take_outbox().is_empty());
        assert!(render(&v, 80, 24).contains("a plan with this name exists"));
        let mut v = view();
        fill(&mut v, "", "7", "7");
        v.on_key(KeyCode::Enter);
        assert!(v.take_outbox().is_empty());
        assert!(render(&v, 80, 24).contains("1 to 64 characters"));
    }

    #[test]
    fn a_valid_form_sends_plan_create() {
        let mut v = view();
        fill(&mut v, "50M", "50", "25");
        v.on_key(KeyCode::Enter);
        assert!(!v.capturing());
        assert_eq!(
            v.take_outbox(),
            vec![(
                "plan.create".to_string(),
                json!({"expected_revision": 2, "name": "50M", "down_mbps": 50, "up_mbps": 25})
            )]
        );
        // Edit starts with the selected plan values and may keep its own name.
        v.on_key(KeyCode::Down);
        v.on_key(KeyCode::Char('e'));
        v.on_key(KeyCode::Tab);
        v.on_key(KeyCode::Backspace);
        typed(&mut v, "8");
        v.on_key(KeyCode::Enter);
        assert_eq!(
            v.take_outbox(),
            vec![(
                "plan.update".to_string(),
                json!({"expected_revision": 2, "plan_id": "plan-2", "name": "7M", "down_mbps": 8, "up_mbps": 7})
            )]
        );
        v.on_answer(ok("plan.update", json!({"revision": 3})));
        let sent: Vec<_> = v.take_outbox().into_iter().map(|(op, _)| op).collect();
        for want in ["topology.preview.refresh", "status", "topology.policies"] {
            assert!(sent.contains(&want.to_string()), "{want}: {sent:?}");
        }
    }

    #[test]
    fn esc_closes_the_form_and_q_types_into_it() {
        let mut v = view();
        v.on_key(KeyCode::Char('n'));
        typed(&mut v, "q");
        assert!(render(&v, 80, 24).contains("q"));
        v.on_key(KeyCode::Esc);
        assert!(!v.capturing());
        assert!(v.take_outbox().is_empty());
    }

    #[test]
    fn delete_asks_first_and_sends_plan_delete_on_y() {
        let mut v = view();
        v.on_key(KeyCode::Char('d'));
        assert!(v.capturing());
        assert!(render(&v, 80, 24).contains("Delete plan 100M? [y/n]"));
        v.on_key(KeyCode::Char('n'));
        assert!(!v.capturing());
        assert!(v.take_outbox().is_empty());
        v.on_key(KeyCode::Char('d'));
        v.on_key(KeyCode::Char('y'));
        assert_eq!(
            v.take_outbox(),
            vec![(
                "plan.delete".to_string(),
                json!({"expected_revision": 2, "plan_id": "plan-1"})
            )]
        );
    }

    #[test]
    fn an_in_use_answer_shows_the_message() {
        let mut v = view();
        v.on_answer(ApiAnswer {
            op: "plan.delete".into(),
            result: Err("Invalid: the plan is in use".into()),
        });
        assert!(render(&v, 80, 24).contains("the plan is in use"));
    }
}
