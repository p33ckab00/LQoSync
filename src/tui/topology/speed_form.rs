//! The Policies forms: the global speed rules and the factor of one group.

use super::plan_form::popup;
use crate::tui::theme::{Palette, Role};
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::Line;
use serde_json::{Value, json};

const FIELDS: [&str; 4] = ["down_min", "up_min", "down_max", "up_max"];
const LABELS: [&str; 4] = ["Down minimum", "Up minimum", "Down maximum", "Up maximum"];

pub(super) enum Step {
    Stay,
    Cancel,
    /// The edit for `topology.edit.propose`.
    Send(Value),
}

/// A rule as `(percent mode, value text)`. A legacy rounded rule shows its percent.
fn rule_value(rule: &Value) -> (bool, String) {
    let pct = |f: &Value| {
        let n = f["numerator"].as_f64().unwrap_or(0.0);
        let d = f["denominator"].as_f64().unwrap_or(1.0).max(1.0);
        let p = (n / d * 1000.0).round() / 10.0;
        if p.fract() == 0.0 {
            format!("{p:.0}")
        } else {
            format!("{p}")
        }
    };
    if let Some(bps) = rule["FixedBps"].as_u64() {
        (false, (bps / 1_000_000).to_string())
    } else if !rule["BaseFraction"].is_null() {
        (true, pct(&rule["BaseFraction"]))
    } else if !rule["LegacyRoundedBase"].is_null() {
        (true, pct(&rule["LegacyRoundedBase"]["factor"]))
    } else {
        (true, String::new())
    }
}

/// Plain text for a rule: "10% of plan", "500 Mbps", "legacy 10% rounded".
pub(super) fn rule_text(rule: &Value) -> String {
    let (percent, value) = rule_value(rule);
    match (percent, rule["LegacyRoundedBase"].is_null()) {
        (true, false) => format!("legacy {value}% rounded"),
        (true, true) => format!("{value}% of plan"),
        (false, _) => format!("{value} Mbps"),
    }
}

#[derive(Debug)]
pub(super) struct SpeedForm {
    percent: [bool; 4],
    values: [String; 4],
    errors: [Option<&'static str>; 4],
    focus: usize,
}

impl SpeedForm {
    /// `rules` maps each field to its shown rule (a profile override or the default).
    pub(super) fn new(rules: &Value) -> Self {
        let mut f = Self {
            percent: [true; 4],
            values: Default::default(),
            errors: [None; 4],
            focus: 0,
        };
        for (i, field) in FIELDS.iter().enumerate() {
            let (percent, value) = rule_value(&rules[field]);
            f.percent[i] = percent;
            f.values[i] = value;
        }
        f
    }

    fn rule(&self, i: usize) -> Option<Value> {
        let text = &self.values[i];
        if self.percent[i] {
            let p: f64 = text.parse().ok()?;
            let tenths = p * 10.0;
            let (lo, hi) = if i < 2 {
                (1.0, 1000.0)
            } else {
                (10.0, 10000.0)
            };
            ((tenths - tenths.round()).abs() < 1e-9 && (lo..=hi).contains(&tenths.round()))
                .then(|| json!({"percent": p}))
        } else {
            text.parse::<u64>()
                .ok()
                .filter(|n| (1..=1_000_000).contains(n))
                .map(|n| json!({"mbps": n}))
        }
    }

    pub(super) fn key(&mut self, code: KeyCode) -> Step {
        match code {
            KeyCode::Esc => return Step::Cancel,
            KeyCode::Tab | KeyCode::Down => self.focus = (self.focus + 1) % 4,
            KeyCode::BackTab | KeyCode::Up => self.focus = (self.focus + 3) % 4,
            KeyCode::Left | KeyCode::Right => self.percent[self.focus] = !self.percent[self.focus],
            KeyCode::Backspace => {
                self.values[self.focus].pop();
            }
            KeyCode::Char(c) if self.values[self.focus].len() < 9 => {
                self.values[self.focus].push(c)
            }
            KeyCode::Enter => {
                let rules: Vec<Option<Value>> = (0..4).map(|i| self.rule(i)).collect();
                for (i, r) in rules.iter().enumerate() {
                    self.errors[i] = r.is_none().then_some(if self.percent[i] {
                        if i < 2 {
                            "use 0.1 to 100, one decimal"
                        } else {
                            "use 1 to 1000, one decimal"
                        }
                    } else {
                        "use a whole number from 1 to 1000000"
                    });
                }
                if rules.iter().all(Option::is_some) {
                    let mut edit = json!({"kind": "speed.defaults"});
                    for (field, r) in FIELDS.iter().zip(rules) {
                        edit[field] = r.expect("checked");
                    }
                    return Step::Send(edit);
                }
            }
            _ => {}
        }
        Step::Stay
    }

    pub(super) fn draw(&self, f: &mut Frame, area: Rect, p: &Palette) {
        let mut lines = Vec::new();
        for (i, label) in LABELS.iter().enumerate() {
            let marker = if i == self.focus { "> " } else { "  " };
            let unit = if self.percent[i] { "% of plan" } else { "Mbps" };
            lines.push(Line::from(format!(
                "{marker}{label:<13} {} {unit}",
                self.values[i]
            )));
            if let Some(e) = self.errors[i] {
                lines.push(Line::from(format!("                {e}")).style(p.role(Role::Error)));
            }
        }
        lines.push(Line::from(""));
        lines.push(
            Line::from("Tab: next  Left/Right: % or Mbps  Enter: propose  Esc: cancel")
                .style(p.role(Role::Secondary)),
        );
        popup(f, area, lines, "Subscriber Speed Policy", p);
    }
}

/// A decimal factor from 0.001 to 1 with up to six decimals, or blank.
fn factor_ok(text: &str) -> bool {
    let (int, frac) = text.split_once('.').unwrap_or((text, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    !int.is_empty()
        && digits(int)
        && digits(frac)
        && frac.len() <= 6
        && text
            .parse::<f64>()
            .is_ok_and(|v| (0.001..=1.0).contains(&v))
}

#[derive(Debug)]
pub(super) struct FactorForm {
    source_id: String,
    name: String,
    current: [String; 2],
    values: [String; 2],
    errors: [Option<&'static str>; 2],
    focus: usize,
}

impl FactorForm {
    pub(super) fn new(source_id: &str, name: &str, current: [&str; 2]) -> Self {
        Self {
            source_id: source_id.into(),
            name: name.into(),
            current: current.map(String::from),
            values: Default::default(),
            errors: [None; 2],
            focus: 0,
        }
    }

    pub(super) fn key(&mut self, code: KeyCode) -> Step {
        match code {
            KeyCode::Esc => return Step::Cancel,
            KeyCode::Tab | KeyCode::Down | KeyCode::BackTab | KeyCode::Up => self.focus ^= 1,
            KeyCode::Backspace => {
                self.values[self.focus].pop();
            }
            KeyCode::Char(c) if self.values[self.focus].len() < 9 => {
                self.values[self.focus].push(c)
            }
            KeyCode::Enter => {
                for i in 0..2 {
                    let v = &self.values[i];
                    self.errors[i] = (!v.is_empty() && !factor_ok(v))
                        .then_some("use 0.001 to 1, up to six decimals");
                }
                if self.errors.iter().all(Option::is_none) {
                    let mut edit = json!({"kind": "group.factor", "source_id": self.source_id});
                    for (i, field) in ["down", "up"].iter().enumerate() {
                        if !self.values[i].is_empty() {
                            edit[field] = json!(self.values[i]);
                        }
                    }
                    return Step::Send(edit);
                }
            }
            _ => {}
        }
        Step::Stay
    }

    pub(super) fn draw(&self, f: &mut Frame, area: Rect, p: &Palette) {
        let mut lines = Vec::new();
        for (i, label) in ["Down factor", "Up factor"].iter().enumerate() {
            let marker = if i == self.focus { "> " } else { "  " };
            lines.push(Line::from(format!(
                "{marker}{label:<12} {}_   now {}",
                self.values[i], self.current[i]
            )));
            if let Some(e) = self.errors[i] {
                lines.push(Line::from(format!("               {e}")).style(p.role(Role::Error)));
            }
        }
        lines.push(Line::from(""));
        lines.push(
            Line::from("Blank keeps the value. Enter: propose  Esc: cancel")
                .style(p.role(Role::Secondary)),
        );
        popup(f, area, lines, &format!("Factor of {}", self.name), p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyCode;
    use serde_json::json;

    fn defaults() -> Value {
        json!({"down_min": {"FixedBps": 1000000}, "up_min": {"FixedBps": 1000000},
               "down_max": {"BaseFraction": {"numerator": 1, "denominator": 1}},
               "up_max": {"BaseFraction": {"numerator": 1, "denominator": 1}}})
    }

    fn set(form: &mut SpeedForm, row: usize, text: &str) {
        form.focus = row;
        form.values[row].clear();
        for c in text.chars() {
            form.key(KeyCode::Char(c));
        }
    }

    #[test]
    fn the_speed_form_validates_and_sends_speed_defaults() {
        let mut f = SpeedForm::new(&defaults());
        // Minimum rows start in Mbps (fixed 1 Mbps), maximum rows in percent (100).
        assert_eq!(f.values, ["1", "1", "100", "100"].map(String::from));
        f.focus = 0;
        f.key(KeyCode::Right);
        set(&mut f, 0, "10");
        f.focus = 1;
        f.key(KeyCode::Right);
        set(&mut f, 1, "0.05");
        assert!(matches!(f.key(KeyCode::Enter), Step::Stay));
        assert!(f.errors[1].is_some() && f.errors[0].is_none());
        set(&mut f, 1, "12.5");
        match f.key(KeyCode::Enter) {
            Step::Send(edit) => assert_eq!(
                edit,
                json!({"kind": "speed.defaults", "down_min": {"percent": 10.0}, "up_min": {"percent": 12.5},
                       "down_max": {"percent": 100.0}, "up_max": {"percent": 100.0}})
            ),
            _ => panic!("no edit"),
        }
    }

    #[test]
    fn the_factor_form_validates_decimals() {
        let mut f = FactorForm::new("src-g", "Gold", ["0.215", "0.5"]);
        for c in "1.5".chars() {
            f.key(KeyCode::Char(c));
        }
        assert!(matches!(f.key(KeyCode::Enter), Step::Stay));
        assert!(f.errors[0].is_some());
        assert!(matches!(f.key(KeyCode::Esc), Step::Cancel));
    }

    #[test]
    fn forms_fit_80_columns() {
        let mut v = super::super::policies::tests::speed_view();
        v.on_key(KeyCode::Char('s'));
        let text = super::super::tests::render(&v, 80, 24);
        assert!(
            text.contains("Down minimum") && text.contains("Enter: propose"),
            "{text}"
        );
    }
}
