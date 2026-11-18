//! The read-only Policies view: speed defaults and subscriber profiles.

use super::TopologyView;
use super::builder::mbps;
use crate::tui::theme::{Palette, Role};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use serde_json::Value;

fn pair(v: &Value) -> String {
    format!("{}/{}", mbps(&v[0]), mbps(&v[1]))
}

pub(super) fn draw(v: &TopologyView, f: &mut Frame, area: Rect, p: &Palette) {
    let mut lines = Vec::new();
    match v.policies() {
        None => lines.push(Line::from("waiting for the policy view")),
        Some(Value::Null) => lines.push(Line::from("no installed topology policy")),
        Some(policies) => {
            lines.push(
                Line::from("Plans  [n] new  [e] edit  [d] delete").style(p.role(Role::Secondary)),
            );
            let plans = policies["plans"].as_array().map_or(&[][..], Vec::as_slice);
            if plans.is_empty() {
                lines.push(Line::from("  no plans"));
            }
            for (i, plan) in plans.iter().enumerate() {
                let marker = if i == v.plan_selected { "> " } else { "  " };
                lines.push(Line::from(format!(
                    "{marker}{:<24} {}/{} Mbps  {} subscribers",
                    plan["name"].as_str().unwrap_or_default(),
                    plan["down_mbps"],
                    plan["up_mbps"],
                    plan["subscribers"]
                )));
            }
            lines.push(Line::from(""));
            lines.push(
                Line::from("Subscriber Speed Policy defaults").style(p.role(Role::Secondary)),
            );
            if let Some(defaults) = policies["defaults"].as_object() {
                for (k, rule) in defaults {
                    lines.push(Line::from(format!("  {k}: {rule}")));
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from("Profiles").style(p.role(Role::Secondary)));
            // A plan is a profile too. The Plans section above shows it.
            let is_plan = |id: &Value| plans.iter().any(|plan| plan["id"] == *id);
            for prof in policies["profiles"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|prof| !is_plan(&prof["id"]))
            {
                let name = prof["name"].as_str().unwrap_or_default();
                let text = if prof["base"] == "source_read" {
                    format!("  {name}  base from router  min/max from router")
                } else {
                    format!(
                        "  {name}  base {}  min {}  max {}",
                        pair(&prof["base"]["local_mbps"]),
                        pair(&prof["min_mbps"]),
                        pair(&prof["max_mbps"])
                    )
                };
                lines.push(Line::from(text));
            }
        }
    }
    f.render_widget(
        Paragraph::new(lines)
            .style(p.role(Role::Base))
            .block(crate::tui::design::panel("Policies", p)),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::super::tests::render;
    use super::super::*;
    use crate::tui::api::ApiAnswer;
    use ratatui::crossterm::event::KeyCode;
    use serde_json::json;

    #[test]
    fn policies_show_defaults_and_each_profile() {
        let mut v = TopologyView::default();
        v.on_answer(ApiAnswer {
            op: "status".into(),
            result: Ok(json!({"registry_revision": 2, "enrolled": true})),
        });
        v.on_key(KeyCode::Char('p'));
        v.on_answer(ApiAnswer {
            op: "topology.policies".into(),
            result: Ok(json!({
                "defaults": {"down_min": {"BaseFraction": {"numerator": 1, "denominator": 10}}, "up_min": "Unbounded"},
                "profiles": [
                    {"id": "p-g", "name": "Gold", "base": "source_read", "speed": {}, "min_mbps": null, "max_mbps": null},
                    {"id": "p-l", "name": "Local plan", "base": {"local_mbps": [10.0, 10.0]}, "speed": {},
                     "min_mbps": [1.0, 1.0], "max_mbps": [10.0, 10.0]}
                ]
            })),
        });
        let text = render(&v, 160, 24);
        for want in [
            "down_min",
            "Gold",
            "from router",
            "Local plan",
            "base 10/10",
            "max 10/10",
        ] {
            assert!(text.contains(want), "{want}: {text}");
        }
    }
}
