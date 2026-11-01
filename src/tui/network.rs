//! The Network tab: the node tree of `network.json`, and the selected node.

use super::app::split_main;
use super::data::Data;
use crate::tui::theme::{BoxKind, Palette, Role, clip};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;

/// One shown line of the tree, with the values of the detail panel.
#[derive(Debug, Clone)]
pub struct Node {
    pub prefix: String,
    pub depth: u16,
    pub name: String,
    pub kids: usize,
    pub kind: String,
    pub id: String,
    pub rate: Option<(f64, f64)>,
}

#[derive(Debug, Default)]
pub struct NetworkView {
    pub download_scale: Option<f64>,
    /// Node names are unique in LibreQoS, so a name is the key.
    pub expanded: HashSet<String>,
    pub visible: Vec<Node>,
    pub selected: usize,
    pub query: String,
    pub editing: bool,
    tree: Option<Arc<Value>>,
    offset: usize,
}

fn children(n: &Value) -> Option<&serde_json::Map<String, Value>> {
    n.get("children").and_then(Value::as_object)
}

/// The names from a root to `name`, or `None`.
fn path_to<'a>(tree: &'a Value, name: &str, out: &mut Vec<&'a str>) -> bool {
    let Some(m) = tree.as_object() else {
        return false;
    };
    for (k, n) in m {
        out.push(k);
        if k == name || n.get("children").is_some_and(|c| path_to(c, name, out)) {
            return true;
        }
        out.pop();
    }
    false
}

impl NetworkView {
    pub fn on_data(&mut self, d: &Data) {
        if self.tree.as_ref().is_none_or(|t| !Arc::ptr_eq(t, &d.tree)) {
            self.download_scale = tree_scale(&d.tree);
            self.tree = Some(d.tree.clone());
            self.rebuild(&d.tree);
        }
    }

    pub fn selected_name(&self) -> Option<&str> {
        self.visible.get(self.selected).map(|n| n.name.as_str())
    }

    /// Flatten the expanded nodes. The selection stays on the same name.
    pub fn rebuild(&mut self, tree: &Value) {
        let keep = self.selected_name().map(str::to_string);
        self.visible.clear();
        fn walk(
            m: &serde_json::Map<String, Value>,
            depth: u16,
            guides: &str,
            open: &HashSet<String>,
            out: &mut Vec<Node>,
        ) {
            for (index, (name, n)) in m.iter().enumerate() {
                let last = index + 1 == m.len();
                let kids = children(n);
                let num = |k: &str| n.get(k).and_then(Value::as_f64);
                out.push(Node {
                    prefix: format!("{guides}{}", if last { "└─" } else { "├─" }),
                    depth,
                    name: name.clone(),
                    kids: kids.map_or(0, |k| k.len()),
                    kind: n
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("site")
                        .to_string(),
                    id: n
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    rate: num("downloadBandwidthMbps").zip(num("uploadBandwidthMbps")),
                });
                if let Some(k) = kids
                    && open.contains(name)
                {
                    let next = format!("{guides}{}", if last { "  " } else { "│ " });
                    walk(k, depth + 1, &next, open, out);
                }
            }
        }
        if let Some(m) = tree.as_object() {
            walk(m, 0, "", &self.expanded, &mut self.visible);
        }
        self.selected = keep
            .and_then(|k| self.visible.iter().position(|n| n.name == k))
            .unwrap_or(self.selected.min(self.visible.len().saturating_sub(1)));
    }

    /// Expand the path to `name` and select it. An unknown name changes nothing.
    pub fn reveal(&mut self, d: &Data, name: &str) {
        let mut path = Vec::new();
        if !path_to(&d.tree, name, &mut path) {
            return;
        }
        for p in &path[..path.len() - 1] {
            self.expanded.insert(p.to_string());
        }
        self.tree = Some(d.tree.clone());
        self.rebuild(&d.tree);
        if let Some(i) = self.visible.iter().position(|n| n.name == name) {
            self.selected = i;
        }
    }

    fn find(&mut self, d: &Data) {
        let q = self.query.to_lowercase();
        if q.is_empty() {
            return;
        }
        fn first(tree: &Value, q: &str) -> Option<String> {
            for (k, n) in tree.as_object()? {
                if k.to_lowercase().contains(q) {
                    return Some(k.clone());
                }
                if let Some(hit) = n.get("children").and_then(|c| first(c, q)) {
                    return Some(hit);
                }
            }
            None
        }
        if let Some(name) = first(&d.tree, &q) {
            self.reveal(d, &name);
        }
    }

    pub fn on_key(&mut self, k: KeyEvent, d: &Data) {
        if self.editing {
            match k.code {
                KeyCode::Esc => {
                    self.editing = false;
                    self.query.clear();
                }
                KeyCode::Enter => self.editing = false,
                KeyCode::Backspace => {
                    self.query.pop();
                }
                KeyCode::Char(c) => self.query.push(c),
                _ => {}
            }
            self.find(d);
            return;
        }
        let last = self.visible.len().saturating_sub(1);
        match k.code {
            KeyCode::Char('/') => self.editing = true,
            KeyCode::Down | KeyCode::Char('j') => self.selected = (self.selected + 1).min(last),
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::PageDown => self.selected = (self.selected + 20).min(last),
            KeyCode::PageUp => self.selected = self.selected.saturating_sub(20),
            KeyCode::Home => self.selected = 0,
            KeyCode::End => self.selected = last,
            KeyCode::Char(' ') | KeyCode::Right | KeyCode::Left => {
                if let Some(n) = self.visible.get(self.selected).filter(|n| n.kids > 0) {
                    let name = n.name.clone();
                    let open = self.expanded.contains(&name);
                    match (k.code, open) {
                        (KeyCode::Right, true) | (KeyCode::Left, false) => return,
                        (_, true) => self.expanded.remove(&name),
                        (_, false) => self.expanded.insert(name),
                    };
                    self.rebuild(&d.tree);
                }
            }
            KeyCode::Char('e') => {
                fn all(tree: &Value, out: &mut HashSet<String>) {
                    for (k, n) in tree.as_object().into_iter().flatten() {
                        if let Some(c) = n.get("children") {
                            out.insert(k.clone());
                            all(c, out);
                        }
                    }
                }
                all(&d.tree, &mut self.expanded);
                self.rebuild(&d.tree);
            }
            KeyCode::Char('c') => {
                self.expanded.clear();
                self.rebuild(&d.tree);
            }
            _ => {}
        }
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect, d: &Data, palette: &Palette) {
        let (main, side) = split_main(area);
        let title = format!(
            " network.json {} shown | /{}{} ",
            self.visible.len(),
            self.query,
            if self.editing { "_" } else { "" }
        );
        let scale = self
            .download_scale
            .map_or("unknown".into(), |v| v.to_string());
        let block = palette
            .panel(BoxKind::Net, &title, true)
            .title_bottom(format!(" bar: download / max {scale} Mbps "));
        if self.visible.is_empty() {
            f.render_widget(
                Paragraph::new("no network.json, or no nodes").block(block),
                main,
            );
            return;
        }
        let height = main.height.saturating_sub(2) as usize;
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if height > 0 && self.selected >= self.offset + height {
            self.offset = self.selected + 1 - height;
        }
        let end = (self.offset + height).min(self.visible.len());
        let items: Vec<ListItem> = self.visible[self.offset..end]
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let selected = self.offset + i == self.selected;
                let style = |role| {
                    if selected {
                        palette.selection()
                    } else {
                        palette.role(role)
                    }
                };
                let mark = match (n.kids, self.expanded.contains(&n.name)) {
                    (0, _) => "",
                    (_, true) => "▾ ",
                    (_, false) => "▸ ",
                };
                let role = match n.kind.as_str() {
                    "site" => Role::Accent,
                    "ap" => Role::Ok,
                    "plan" => Role::Warning,
                    _ => Role::Secondary,
                };
                let name_width = main.width.saturating_sub(
                    Line::from(format!("{}{} [{}]", n.prefix, mark, n.kind)).width() as u16 + 3,
                ) as usize;
                let mut spans = vec![
                    Span::styled(n.prefix.clone(), style(Role::Secondary)),
                    Span::raw(mark),
                    Span::styled(clip(&n.name, name_width), style(Role::Base)),
                    Span::styled(format!(" [{}]", n.kind), style(role)),
                ];
                let numeric = n
                    .rate
                    .filter(|(d, u)| d.is_finite() && u.is_finite() && *d >= 0.0 && *u >= 0.0);
                let rate = numeric.map_or(" unknown".into(), |(d, u)| {
                    let fill = (rate_fill(Some(d), self.download_scale).unwrap_or(0.0) * 8.0)
                        .round() as usize;
                    format!(
                        " {}{} D/U Mbps {d}/{u}",
                        "█".repeat(fill),
                        "░".repeat(8 - fill)
                    )
                });
                if Line::from(spans.clone()).width() + Line::from(rate.as_str()).width() + 2
                    <= main.width as usize
                {
                    spans.push(Span::styled(rate, style(Role::Accent)));
                }
                ListItem::new(Line::from(spans))
            })
            .collect();
        let mut state = ListState::default().with_selected(Some(self.selected - self.offset));
        f.render_stateful_widget(
            List::new(items)
                .block(block)
                .highlight_style(palette.selection()),
            main,
            &mut state,
        );
        f.render_widget(
            Paragraph::new(self.detail(d, side.height.saturating_sub(10) as usize))
                .block(palette.panel(BoxKind::Mem, "node", false)),
            side,
        );
    }

    fn detail(&self, d: &Data, max_lines: usize) -> Vec<Line<'static>> {
        let Some(n) = self.visible.get(self.selected) else {
            return Vec::new();
        };
        let circuits = d
            .node_circuits
            .get(&n.name)
            .map_or(&[][..], |v| v.as_slice());
        let mut out = vec![
            Line::from(format!("name: {}", n.name)),
            Line::from(format!("type: {}", n.kind)),
            Line::from(format!(
                "id: {}",
                if n.id.is_empty() { "unknown" } else { &n.id }
            )),
            Line::from(match n.rate {
                Some((dn, up)) => format!("rate: {dn} / {up} Mbps"),
                None => "rate: unknown".into(),
            }),
            Line::from(format!("child nodes: {}", n.kids)),
            Line::from(format!("circuits: {}", circuits.len())),
        ];
        for &i in circuits.iter().take(max_lines) {
            if let Some(r) = d.rows.get(i as usize) {
                out.push(Line::from(format!("  {} {}", r.circuit_name, r.ipv4)));
            }
        }
        if circuits.len() > max_lines {
            out.push(Line::from(format!(
                "  ...and {} more",
                circuits.len() - max_lines
            )));
        }
        out
    }
}

fn rate_fill(value: Option<f64>, scale: Option<f64>) -> Option<f64> {
    let value = value.filter(|v| v.is_finite() && *v >= 0.0)?;
    if value == 0.0 {
        return Some(0.0);
    }
    let scale = scale.filter(|s| s.is_finite() && *s > 0.0)?;
    Some((value / scale).clamp(0.0, 1.0))
}

fn tree_scale(tree: &Value) -> Option<f64> {
    tree.as_object()?
        .values()
        .flat_map(|n| {
            [
                n.get("downloadBandwidthMbps").and_then(Value::as_f64),
                n.get("children").and_then(tree_scale),
            ]
        })
        .flatten()
        .filter(|v| v.is_finite() && *v > 0.0)
        .max_by(f64::total_cmp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CsvRow;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use serde_json::json;
    use std::collections::HashMap;

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    fn tree() -> Value {
        json!({
            "Site": {"id": "s1", "type": "site", "downloadBandwidthMbps": 100, "uploadBandwidthMbps": 90, "children": {
                "AP": {"id": "a1", "type": "ap", "downloadBandwidthMbps": 50, "uploadBandwidthMbps": 50,
                       "children": {"AP2": {"id": "a2"}}},
                "AP3": {"id": "a3"}
            }},
            "Other": {"id": "o1"}
        })
    }

    fn names(v: &NetworkView) -> Vec<&str> {
        v.visible.iter().map(|n| n.name.as_str()).collect()
    }

    fn view() -> (NetworkView, Data) {
        let d = Data {
            tree: Arc::new(tree()),
            ..Data::default()
        };
        let mut v = NetworkView::default();
        v.on_data(&d);
        (v, d)
    }

    fn screen(v: &mut NetworkView, d: &Data) -> String {
        let mut t = Terminal::new(TestBackend::new(120, 20)).unwrap();
        t.draw(|f| v.draw(f, f.area(), d, &Palette::new(crate::config::Theme::Dark)))
            .unwrap();
        let buf = t.backend().buffer();
        (0..20)
            .map(|y| (0..120).map(|x| buf[(x, y)].symbol()).collect::<String>() + "\n")
            .collect()
    }

    #[test]
    fn collapsed_tree_shows_only_roots() {
        let (v, _) = view();
        assert_eq!(names(&v), ["Other", "Site"]);
    }

    #[test]
    fn space_expands_and_collapses() {
        let (mut v, d) = view();
        v.on_key(key(KeyCode::Down), &d);
        v.on_key(key(KeyCode::Char(' ')), &d);
        assert_eq!(names(&v), ["Other", "Site", "AP", "AP3"]);
        assert_eq!(v.visible[2].depth, 1);
        v.on_key(key(KeyCode::Char(' ')), &d);
        assert_eq!(names(&v), ["Other", "Site"]);
    }

    #[test]
    fn expand_all_and_collapse_all() {
        let (mut v, d) = view();
        v.on_key(key(KeyCode::Char('e')), &d);
        assert_eq!(names(&v), ["Other", "Site", "AP", "AP2", "AP3"]);
        v.on_key(key(KeyCode::Char('c')), &d);
        assert_eq!(names(&v), ["Other", "Site"]);
    }

    #[test]
    fn reveal_expands_the_path() {
        let (mut v, d) = view();
        v.reveal(&d, "AP2");
        assert_eq!(names(&v), ["Other", "Site", "AP", "AP2", "AP3"]);
        assert_eq!(v.selected_name(), Some("AP2"));
        v.reveal(&d, "nothing");
        assert_eq!(v.selected_name(), Some("AP2"));
    }

    #[test]
    fn node_details_count_circuits() {
        let (mut v, mut d) = view();
        let row = |n: &str| CsvRow {
            circuit_name: n.into(),
            parent_node: "AP".into(),
            ..CsvRow::default()
        };
        d.rows = Arc::new(vec![row("alice"), row("bob")]);
        d.node_circuits = Arc::new(HashMap::from([("AP".to_string(), vec![0, 1])]));
        v.reveal(&d, "AP");
        let s = screen(&mut v, &d);
        for want in [
            "type: ap",
            "id: a1",
            "rate: 50 / 50 Mbps",
            "child nodes: 1",
            "circuits: 2",
            "alice",
            "bob",
        ] {
            assert!(s.contains(want), "{want}\n{s}");
        }
    }

    #[test]
    fn search_selects_the_first_match() {
        let (mut v, d) = view();
        for c in ['/', 'a', 'p', '2'] {
            v.on_key(key(KeyCode::Char(c)), &d);
        }
        assert_eq!(v.selected_name(), Some("AP2"));
        v.on_key(key(KeyCode::Enter), &d);
        assert!(!v.editing);
    }

    #[test]
    fn network_tab_renders() {
        let (mut v, d) = view();
        let s = screen(&mut v, &d);
        assert!(s.contains("▸ Site") && s.contains("Other"), "{s}");
        let mut empty = NetworkView::default();
        assert!(screen(&mut empty, &Data::default()).contains("no network.json"));
    }
    #[test]
    fn tree_guides_and_scale_survive_search() {
        let (mut v, d) = view();
        assert_eq!(v.download_scale, Some(100.0));
        v.reveal(&d, "AP2");
        assert!(v.visible.iter().all(|n| n.prefix.contains("─")));
        assert!(v.visible.iter().any(|n| n.prefix.contains("│")));
        assert_eq!(v.selected_name(), Some("AP2"));
        assert_eq!(v.download_scale, Some(100.0));
    }
    #[test]
    fn configured_rate_scale_handles_missing_and_zero() {
        assert_eq!(rate_fill(Some(25.0), Some(100.0)), Some(0.25));
        assert_eq!(rate_fill(Some(0.0), None), Some(0.0));
        for v in [None, Some(-1.0), Some(f64::NAN), Some(f64::INFINITY)] {
            assert_eq!(rate_fill(v, Some(100.0)), None);
        }
    }
}
