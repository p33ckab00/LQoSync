//! The Builder node form: name and capacity for a new WAN, a new intermediate node, or an edit.

use super::plan_form::popup;
use crate::tui::theme::{Palette, Role};
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::Line;
use serde_json::{Value, json};

/// What the form makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Target {
    Wan {
        router_id: String,
        endpoint_ref: String,
    },
    Intermediate {
        parent: Option<String>,
    },
    Update {
        node_id: String,
        kind: String,
        name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Name,
    Mode,
    Value(usize),
}

#[derive(Debug)]
pub(super) struct NodeForm {
    target: Target,
    modes: &'static [&'static str],
    mode: usize,
    name: String,
    /// Mbps or percent values; the mode decides which ones show.
    values: [String; 4],
    errors: Vec<(Row, &'static str)>,
    focus: usize,
}

/// What a key in the form does.
pub(super) enum Step {
    Stay,
    Cancel,
    /// The edit for `topology.edit.propose`.
    Send(Value),
}

fn modes(kind: &str) -> &'static [&'static str] {
    match kind {
        "wan" => &["fixed"],
        "intermediate" => &["inherit", "fixed", "wan_percent"],
        _ => &["automatic", "fixed"],
    }
}

fn whole(text: &str, max: u64) -> Option<u64> {
    text.parse::<u64>().ok().filter(|n| (1..=max).contains(n))
}

impl NodeForm {
    pub(super) fn wan(router_id: &str, endpoint_ref: &str) -> Self {
        Self {
            target: Target::Wan {
                router_id: router_id.into(),
                endpoint_ref: endpoint_ref.into(),
            },
            modes: modes("wan"),
            mode: 0,
            name: endpoint_ref.into(),
            values: [String::new(), String::new(), "100".into(), "100".into()],
            errors: Vec::new(),
            focus: 0,
        }
    }

    pub(super) fn intermediate(parent: Option<String>) -> Self {
        Self {
            target: Target::Intermediate { parent },
            modes: modes("intermediate"),
            mode: 0,
            name: String::new(),
            values: Default::default(),
            errors: Vec::new(),
            focus: 0,
        }
    }

    /// An edit of a catalog node from the `topology.tree` answer.
    pub(super) fn update(node: &Value) -> Self {
        let kind = node["kind"].as_str().unwrap_or_default().to_string();
        let c = &node["capacity"];
        let all = modes(&kind);
        let mode = all
            .iter()
            .position(|m| c["mode"].as_str() == Some(m))
            .unwrap_or(0);
        let text = |v: &Value| {
            if v.is_null() {
                String::new()
            } else {
                v.to_string()
            }
        };
        let values = match c["mode"].as_str() {
            Some("fixed") => [
                text(&c["down_mbps"]),
                text(&c["up_mbps"]),
                c["factor_percent"][0].as_u64().unwrap_or(100).to_string(),
                c["factor_percent"][1].as_u64().unwrap_or(100).to_string(),
            ],
            Some("wan_percent") => [
                text(&c["down_percent"]),
                text(&c["up_percent"]),
                String::new(),
                String::new(),
            ],
            _ => [String::new(), String::new(), "100".into(), "100".into()],
        };
        let name = node["name"].as_str().unwrap_or_default().to_string();
        Self {
            target: Target::Update {
                node_id: node["id"].as_str().unwrap_or_default().into(),
                kind,
                name: name.clone(),
            },
            modes: all,
            mode,
            name,
            values,
            errors: Vec::new(),
            focus: 0,
        }
    }

    fn is_wan(&self) -> bool {
        match &self.target {
            Target::Wan { .. } => true,
            Target::Update { kind, .. } => kind == "wan",
            Target::Intermediate { .. } => false,
        }
    }

    fn mode(&self) -> &'static str {
        self.modes[self.mode]
    }

    fn rows(&self) -> Vec<(Row, &'static str)> {
        let mut rows = vec![(Row::Name, "Name"), (Row::Mode, "Mode")];
        match self.mode() {
            "fixed" => {
                rows.push((Row::Value(0), "Down Mbps"));
                rows.push((Row::Value(1), "Up Mbps"));
                if self.is_wan() {
                    rows.push((Row::Value(2), "Down factor %"));
                    rows.push((Row::Value(3), "Up factor %"));
                }
            }
            "wan_percent" => {
                rows.push((Row::Value(0), "Down % of WAN"));
                rows.push((Row::Value(1), "Up % of WAN"));
            }
            _ => {}
        }
        rows
    }

    fn validate(&mut self) -> bool {
        self.errors.clear();
        let len = self.name.chars().count();
        if len == 0 || len > 64 || self.name.trim() != self.name {
            self.errors.push((Row::Name, "use 1 to 64 characters"));
        }
        for (row, _) in self.rows() {
            if let Row::Value(i) = row {
                let (max, text) = match (self.mode(), i) {
                    ("fixed", 0 | 1) => (1_000_000, "use a whole number from 1 to 1000000"),
                    _ => (100, "use a whole number from 1 to 100"),
                };
                if whole(&self.values[i], max).is_none() {
                    self.errors.push((row, text));
                }
            }
        }
        self.errors.is_empty()
    }

    fn capacity(&self) -> Value {
        let v = |i: usize| whole(&self.values[i], 1_000_000);
        match self.mode() {
            "fixed" => {
                let factor = match (self.is_wan(), v(2), v(3)) {
                    (true, Some(d), Some(u)) if (d, u) != (100, 100) => json!([d, u]),
                    _ => Value::Null,
                };
                json!({"mode": "fixed", "down_mbps": v(0), "up_mbps": v(1), "factor_percent": factor})
            }
            "wan_percent" => {
                json!({"mode": "wan_percent", "down_percent": v(0), "up_percent": v(1)})
            }
            mode => json!({"mode": mode}),
        }
    }

    fn edit(&self) -> Value {
        match &self.target {
            Target::Wan {
                router_id,
                endpoint_ref,
            } => {
                let mut e = self.capacity();
                e["kind"] = json!("wan.create");
                e["router_id"] = json!(router_id);
                e["endpoint_ref"] = json!(endpoint_ref);
                e["name"] = json!(self.name);
                e.as_object_mut().expect("object").remove("mode");
                e
            }
            Target::Intermediate { parent } => json!({
                "kind": "node.create", "name": self.name, "parent": parent,
                "capacity": self.capacity(),
            }),
            Target::Update { node_id, name, .. } => {
                let mut e =
                    json!({"kind": "node.update", "node_id": node_id, "capacity": self.capacity()});
                // An unchanged name is not sent, so an enrolled node keeps its legacy name.
                if self.name != *name {
                    e["name"] = json!(self.name);
                }
                e
            }
        }
    }

    pub(super) fn key(&mut self, code: KeyCode) -> Step {
        let rows = self.rows();
        let row = rows[self.focus.min(rows.len() - 1)].0;
        match code {
            KeyCode::Esc => return Step::Cancel,
            KeyCode::Tab | KeyCode::Down => self.focus = (self.focus + 1) % rows.len(),
            KeyCode::BackTab | KeyCode::Up => {
                self.focus = (self.focus + rows.len() - 1) % rows.len();
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') if row == Row::Mode => {
                let n = self.modes.len();
                self.mode = if code == KeyCode::Left {
                    (self.mode + n - 1) % n
                } else {
                    (self.mode + 1) % n
                };
            }
            KeyCode::Backspace => match row {
                Row::Name => {
                    self.name.pop();
                }
                Row::Value(i) => {
                    self.values[i].pop();
                }
                Row::Mode => {}
            },
            KeyCode::Char(c) => match row {
                Row::Name if self.name.chars().count() < 64 => self.name.push(c),
                Row::Value(i) if self.values[i].len() < 8 => self.values[i].push(c),
                _ => {}
            },
            KeyCode::Enter if self.validate() => return Step::Send(self.edit()),
            _ => {}
        }
        Step::Stay
    }

    pub(super) fn draw(&self, f: &mut Frame, area: Rect, p: &Palette) {
        let mut lines = Vec::new();
        for (i, (row, label)) in self.rows().into_iter().enumerate() {
            let marker = if i == self.focus { "> " } else { "  " };
            let cursor = if i == self.focus && row != Row::Mode {
                "_"
            } else {
                ""
            };
            let value = match row {
                Row::Name => self.name.clone(),
                Row::Mode => format!("< {} >", self.mode()),
                Row::Value(v) => self.values[v].clone(),
            };
            lines.push(Line::from(format!("{marker}{label:<14} {value}{cursor}")));
            for (_, e) in self.errors.iter().filter(|(r, _)| *r == row) {
                lines.push(Line::from(format!("                 {e}")).style(p.role(Role::Error)));
            }
        }
        lines.push(Line::from(""));
        lines.push(
            Line::from("Tab: next  Left/Right: mode  Enter: propose  Esc: cancel")
                .style(p.role(Role::Secondary)),
        );
        let title = match &self.target {
            Target::Wan { .. } => "New WAN",
            Target::Intermediate { .. } => "New node",
            Target::Update { .. } => "Edit node",
        };
        popup(f, area, lines, title, p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyCode;

    fn send(form: &mut NodeForm) -> Option<Value> {
        match form.key(KeyCode::Enter) {
            Step::Send(v) => Some(v),
            _ => None,
        }
    }

    fn type_into(form: &mut NodeForm, text: &str) {
        for c in text.chars() {
            form.key(KeyCode::Char(c));
        }
    }

    fn focus(form: &mut NodeForm, label: &str) {
        form.focus = form.rows().iter().position(|(_, l)| *l == label).unwrap();
    }

    #[test]
    fn the_wan_form_defaults_to_the_interface_name_and_full_factors() {
        let mut f = NodeForm::wan("r-core", "pppoe-out1");
        assert_eq!(f.name, "pppoe-out1");
        focus(&mut f, "Down Mbps");
        type_into(&mut f, "500");
        focus(&mut f, "Up Mbps");
        type_into(&mut f, "100");
        assert_eq!(
            send(&mut f).unwrap(),
            json!({"kind": "wan.create", "router_id": "r-core", "endpoint_ref": "pppoe-out1",
                   "name": "pppoe-out1", "down_mbps": 500, "up_mbps": 100, "factor_percent": null})
        );
    }

    #[test]
    fn bad_rates_percent_and_names_show_field_errors_and_send_nothing() {
        let mut f = NodeForm::wan("r", "e");
        f.name = " bad".into();
        f.values = ["0".into(), "abc".into(), "101".into(), "1.5".into()];
        assert!(send(&mut f).is_none());
        let rows: Vec<Row> = f.errors.iter().map(|(r, _)| *r).collect();
        assert_eq!(
            rows,
            vec![
                Row::Name,
                Row::Value(0),
                Row::Value(1),
                Row::Value(2),
                Row::Value(3)
            ]
        );
    }

    #[test]
    fn the_mode_cycles_only_through_modes_the_kind_allows() {
        let mut f = NodeForm::intermediate(None);
        focus(&mut f, "Mode");
        let mut seen = vec![f.mode()];
        for _ in 0..3 {
            f.key(KeyCode::Right);
            seen.push(f.mode());
        }
        assert_eq!(seen, ["inherit", "fixed", "wan_percent", "inherit"]);
        let group =
            json!({"id": "g", "name": "Gold", "kind": "group", "capacity": {"mode": "automatic"}});
        let mut f = NodeForm::update(&group);
        focus(&mut f, "Mode");
        f.key(KeyCode::Right);
        assert_eq!(f.mode(), "fixed");
        f.key(KeyCode::Right);
        assert_eq!(f.mode(), "automatic");
        // An unchanged name is not sent.
        assert_eq!(
            send(&mut f).unwrap(),
            json!({"kind": "node.update", "node_id": "g", "capacity": {"mode": "automatic"}})
        );
    }

    #[test]
    fn forms_fit_80_columns() {
        let mut v = super::super::builder::tests::sample();
        v.node_form = Some(NodeForm::wan("r-core", "pppoe-out1"));
        let text = super::super::tests::render(&v, 80, 24);
        assert!(text.contains("Down factor %"), "{text}");
        assert!(text.contains("Enter: propose"), "{text}");
    }
}
