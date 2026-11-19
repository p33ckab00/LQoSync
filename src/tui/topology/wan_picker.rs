//! The Assignments WAN picker: pin a subscriber to a WAN, or unpin it.

use super::plan_form::popup;
use crate::tui::theme::Palette;
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::Line;
use serde_json::{Value, json};

/// Who the pin edit names: an online identity, or an offline record ID (unpin only).
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Who {
    Identity(Value),
    Record(String),
}

#[derive(Debug)]
pub(super) struct WanPicker {
    who: Who,
    /// `(WAN node ID or None for unpin, label)`.
    entries: Vec<(Option<String>, String)>,
    selected: usize,
}

pub(super) enum Step {
    Stay,
    Cancel,
    Send(Value),
}

impl WanPicker {
    /// `wans` are the WAN nodes in tree order. An offline record gets only "unpin".
    pub(super) fn new(who: Who, wans: &[&Value], pinned: bool) -> Self {
        let mut entries = Vec::new();
        if matches!(who, Who::Identity(_)) {
            for w in wans {
                let id = w["id"].as_str().unwrap_or_default();
                entries.push((
                    Some(id.to_string()),
                    w["name"].as_str().unwrap_or(id).into(),
                ));
            }
        }
        if pinned {
            entries.push((None, "unpin".into()));
        }
        Self {
            who,
            entries,
            selected: 0,
        }
    }

    pub(super) fn key(&mut self, code: KeyCode) -> Step {
        match code {
            KeyCode::Esc => return Step::Cancel,
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(self.entries.len().saturating_sub(1));
            }
            KeyCode::Enter => {
                let Some((wan, _)) = self.entries.get(self.selected) else {
                    return Step::Cancel;
                };
                return Step::Send(match &self.who {
                    Who::Identity(i) => {
                        json!({"kind": "subscriber.pin", "identity": i, "wan_node_id": wan})
                    }
                    Who::Record(id) => {
                        json!({"kind": "subscriber.pin", "subscriber_id": id, "wan_node_id": wan})
                    }
                });
            }
            _ => {}
        }
        Step::Stay
    }

    pub(super) fn draw(&self, f: &mut Frame, area: Rect, p: &Palette) {
        let mut lines: Vec<Line> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, (_, label))| {
                let marker = if i == self.selected { "> " } else { "  " };
                Line::from(format!("{marker}{label}"))
            })
            .collect();
        if self.entries.is_empty() {
            lines.push(Line::from("no WAN node"));
        }
        lines.push(Line::from(""));
        lines.push(Line::from("Enter: propose  Esc: cancel"));
        popup(f, area, lines, "Pin to WAN", p);
    }
}
