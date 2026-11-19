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
                Line::from("Subscriber Speed Policy  [s] edit").style(p.role(Role::Secondary)),
            );
            let rules = v.shown_rules();
            for field in ["down_min", "up_min", "down_max", "up_max"] {
                lines.push(Line::from(format!(
                    "  {field}: {}",
                    super::speed_form::rule_text(&rules[field])
                )));
            }
            lines.push(Line::from(""));
            lines.push(Line::from("Groups  [f] factor").style(p.role(Role::Secondary)));
            let pair_or_none = |v: &Value| {
                if v.is_null() {
                    "none".to_string()
                } else {
                    pair(v)
                }
            };
            for (i, g) in v.groups().iter().enumerate() {
                let marker = if v.plan_selected == plans.len() + i {
                    "> "
                } else {
                    "  "
                };
                lines.push(Line::from(format!(
                    "{marker}{}  factor {}/{}  {}  backstop {}  floor {}",
                    g["name"].as_str().unwrap_or_default(),
                    g["factor"][0].as_str().unwrap_or_default(),
                    g["factor"][1].as_str().unwrap_or_default(),
                    g["formula"].as_str().unwrap_or_default(),
                    pair_or_none(&g["backstop_mbps"]),
                    pair_or_none(&g["floor_mbps"]),
                )));
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
pub(crate) mod tests {
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

    pub(crate) fn speed_view() -> TopologyView {
        let mut v = TopologyView::default();
        v.on_answer(ApiAnswer {
            op: "status".into(),
            result: Ok(json!({"registry_revision": 2, "enrolled": true})),
        });
        v.on_key(KeyCode::Char('p'));
        let legacy = json!({"Override": {"LegacyRoundedBase": {"factor": {"numerator": 1, "denominator": 10}, "floor_bps": 1000000}}});
        v.on_answer(ApiAnswer {
            op: "topology.policies".into(),
            result: Ok(json!({
                "defaults": {"down_min": {"FixedBps": 1000000}, "up_min": {"FixedBps": 1000000},
                             "down_max": {"BaseFraction": {"numerator": 1, "denominator": 1}},
                             "up_max": {"FixedBps": 500000000}},
                "profiles": [{"id": "p-g", "name": "Gold", "base": "source_read",
                              "speed": {"down_min": legacy, "up_min": legacy, "down_max": "Inherit", "up_max": "Inherit"},
                              "min_mbps": null, "max_mbps": null}],
                "plans": [{"id": "plan-1", "name": "100M", "down_mbps": 100, "up_mbps": 100, "subscribers": 0}],
                "groups": [{"source_id": "src-g", "name": "Gold", "factor": ["0.215", "0.5"], "formula": "PlanSum",
                            "backstop_mbps": [620.0, 620.0], "floor_mbps": null}]
            })),
        });
        v
    }

    #[test]
    fn policies_show_speed_rules_and_groups() {
        let text = render(&speed_view(), 160, 30);
        for want in [
            "down_min: legacy 10% rounded",
            "down_max: 100% of plan",
            "up_max: 500 Mbps",
            "Groups  [f] factor",
            "Gold  factor 0.215/0.5  PlanSum  backstop 620/620  floor none",
            "Subscriber Speed Policy  [s] edit",
        ] {
            assert!(text.contains(want), "{want}: {text}");
        }
    }

    #[test]
    fn the_factor_form_sends_only_filled_directions() {
        let mut v = speed_view();
        v.take_outbox();
        v.on_key(KeyCode::Down);
        v.on_key(KeyCode::Char('f'));
        for c in "0.3".chars() {
            v.on_key(KeyCode::Char(c));
        }
        v.on_key(KeyCode::Enter);
        let sent: Vec<_> = v
            .take_outbox()
            .into_iter()
            .filter(|(op, _)| op == "topology.edit.propose")
            .collect();
        assert_eq!(
            sent[0].1,
            json!({"expected_revision": 2, "edit": {"kind": "group.factor", "source_id": "src-g", "down": "0.3"}})
        );
    }
}
