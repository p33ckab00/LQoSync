//! The Builder parent picker for move, demote, and a new source instance.

use super::plan_form::popup;
use crate::tui::theme::Palette;
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::Line;
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Purpose {
    Move(String),
    Demote(String),
    Instance(String),
    NewSource(String),
}

#[derive(Debug)]
pub(super) struct ParentPicker {
    purpose: Purpose,
    /// `(parent, label)`; `None` is the root.
    entries: Vec<(Option<String>, String)>,
    selected: usize,
}

pub(super) enum Step {
    Stay,
    Cancel,
    Send(Value),
}

fn parent_of<'a>(nodes: &'a [Value], id: &str) -> Option<&'a str> {
    nodes
        .iter()
        .find(|n| n["id"].as_str() == Some(id))
        .and_then(|n| n["parent"].as_str())
}

fn is_below(nodes: &[Value], id: &str, ancestor: &str) -> bool {
    let mut cursor = Some(id);
    while let Some(n) = cursor {
        if n == ancestor {
            return true;
        }
        cursor = parent_of(nodes, n);
    }
    false
}

impl ParentPicker {
    /// `nodes` are the `topology.tree` nodes in tree order. A group cannot hold children.
    pub(super) fn new(nodes: &[&Value], all: &[Value], purpose: Purpose) -> Self {
        let mut entries = Vec::new();
        if !matches!(purpose, Purpose::Demote(_)) {
            entries.push((None, "root".to_string()));
        }
        for n in nodes {
            let id = n["id"].as_str().unwrap_or_default();
            if n["kind"] == "group" {
                continue;
            }
            let keep = match &purpose {
                Purpose::Move(node) => !is_below(all, id, node),
                Purpose::Demote(node) => id != node && parent_of(all, id) == parent_of(all, node),
                Purpose::Instance(_) | Purpose::NewSource(_) => true,
            };
            if keep {
                entries.push((
                    Some(id.to_string()),
                    n["name"].as_str().unwrap_or(id).into(),
                ));
            }
        }
        Self {
            purpose,
            entries,
            selected: 0,
        }
    }

    #[cfg(test)]
    pub(super) fn labels(&self) -> Vec<&str> {
        self.entries.iter().map(|(_, l)| l.as_str()).collect()
    }

    pub(super) fn key(&mut self, code: KeyCode) -> Step {
        match code {
            KeyCode::Esc => return Step::Cancel,
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(self.entries.len().saturating_sub(1));
            }
            KeyCode::Enter => {
                let Some((parent, _)) = self.entries.get(self.selected) else {
                    return Step::Cancel;
                };
                return Step::Send(match &self.purpose {
                    Purpose::Move(id) | Purpose::Demote(id) => {
                        json!({"kind": "node.move", "node_id": id, "parent": parent})
                    }
                    Purpose::Instance(source) => {
                        json!({"kind": "instance.add", "source_id": source, "parent": parent})
                    }
                    Purpose::NewSource(key) => {
                        json!({"kind": "source.add", "key": key, "parent": parent})
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
            lines.push(Line::from("no valid parent"));
        }
        lines.push(Line::from(""));
        lines.push(Line::from("Enter: propose  Esc: cancel"));
        let title = match self.purpose {
            Purpose::Move(_) => "Move to",
            Purpose::Demote(_) => "Demote below",
            Purpose::Instance(_) => "New instance below",
            Purpose::NewSource(_) => "New source below",
        };
        popup(f, area, lines, title, p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes() -> Vec<Value> {
        let n = |id: &str, parent: Option<&str>, kind: &str| json!({"id": id, "name": id.to_uppercase(), "parent": parent, "kind": kind});
        vec![
            n("wan1", None, "wan"),
            n("t1", Some("wan1"), "intermediate"),
            n("t2", Some("t1"), "intermediate"),
            n("g", Some("t1"), "group"),
            n("wan2", None, "wan"),
        ]
    }

    fn picker(purpose: Purpose) -> ParentPicker {
        let all = nodes();
        let refs: Vec<&Value> = all.iter().collect();
        ParentPicker::new(&refs, &all, purpose)
    }

    #[test]
    fn the_parent_picker_leaves_out_the_node_and_its_descendants() {
        assert_eq!(
            picker(Purpose::Move("t1".into())).labels(),
            ["root", "WAN1", "WAN2"]
        );
        assert_eq!(
            picker(Purpose::Instance("src".into())).labels(),
            ["root", "WAN1", "T1", "T2", "WAN2"]
        );
    }

    #[test]
    fn demote_lists_only_siblings() {
        assert_eq!(picker(Purpose::Demote("wan1".into())).labels(), ["WAN2"]);
        assert_eq!(picker(Purpose::Demote("g".into())).labels(), ["T2"]);
    }

    #[test]
    fn enter_sends_the_edit_and_esc_cancels() {
        let mut p = picker(Purpose::Instance("src-g".into()));
        p.key(KeyCode::Down);
        match p.key(KeyCode::Enter) {
            Step::Send(v) => assert_eq!(
                v,
                json!({"kind": "instance.add", "source_id": "src-g", "parent": "wan1"})
            ),
            _ => panic!("no edit"),
        }
        assert!(matches!(p.key(KeyCode::Esc), Step::Cancel));
    }
}
