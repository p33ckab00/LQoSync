//! Configured routers and latest collection evidence. No network work occurs in this view.
use super::{
    data::Data,
    design::{self, Scroll},
    theme::{Palette, Role, clip},
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::Paragraph,
};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
struct Router {
    index: usize,
    name: String,
    endpoint: String,
    sources: [bool; 3],
    report: Value,
}
#[derive(Default)]
pub struct RoutersView {
    pub editing: bool,
    query: String,
    state: usize,
    records: Vec<Router>,
    filtered: Vec<usize>,
    cursor: usize,
    table: Scroll,
    details: Scroll,
    controls: [Rect; 3],
    actions: [Rect; 4],
    pages: [Rect; 2],
    config: Option<Arc<String>>,
    status: Option<Arc<Value>>,
    configured: Option<usize>,
    focus: usize,
    expanded: bool,
    horizontal: u16,
}
pub enum RouterAction {
    None,
    Subscribers(String),
    Settings(usize),
    Copy(String),
}
impl RoutersView {
    pub fn on_data(&mut self, d: &Data) {
        if self
            .config
            .as_ref()
            .is_some_and(|p| Arc::ptr_eq(p, &d.config_text))
            && match (&self.status, &d.status) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
        {
            return;
        }
        let selected = self.selected().map(|r| r.name.clone());
        let config_changed = self
            .config
            .as_ref()
            .is_none_or(|p| !Arc::ptr_eq(p, &d.config_text));
        self.status = d.status.clone();
        let reports: std::collections::HashMap<_, _> = d
            .status
            .as_ref()
            .and_then(|s| s["routers"].as_array())
            .into_iter()
            .flatten()
            .filter_map(|r| r["name"].as_str().map(|n| (n, r)))
            .collect();
        if config_changed {
            if self.config.is_none() {
                self.focus = 3;
            }
            self.config = Some(d.config_text.clone());
            let cfg = crate::config::Config::from_toml_str(&d.config_text).ok();
            self.configured = cfg.as_ref().map(|c| c.routers.len());
            self.records = cfg
                .into_iter()
                .flat_map(|c| c.routers)
                .enumerate()
                .map(|(i, r)| {
                    let address = if r.address.contains(':') && !r.address.starts_with('[') {
                        format!("[{}]", r.address)
                    } else {
                        r.address
                    };
                    Router {
                        index: i,
                        report: Value::Null,
                        name: r.name,
                        endpoint: format!("{address}:{}", r.port),
                        sources: [r.pppoe.enabled, r.dhcp.enabled, r.hotspot.enabled],
                    }
                })
                .collect();
        }
        for r in &mut self.records {
            r.report = reports
                .get(r.name.as_str())
                .map_or(Value::Null, |v| (*v).clone());
        }
        self.filter();
        if let Some(name) = selected
            && let Some(i) = self
                .filtered
                .iter()
                .position(|&i| self.records[i].name == name)
        {
            self.cursor = i;
        }
    }
    pub fn clear_area(&mut self) {
        self.controls = [Rect::default(); 3];
        self.actions = [Rect::default(); 4];
        self.pages = [Rect::default(); 2];
        self.table.clear();
        self.details.clear();
    }
    fn selected(&self) -> Option<&Router> {
        self.filtered.get(self.cursor).map(|&i| &self.records[i])
    }
    fn state_of(r: &Router) -> &str {
        r.report["state"].as_str().unwrap_or("unknown")
    }
    fn filter(&mut self) {
        let q = self.query.to_lowercase();
        let state = ["all", "online", "failed", "offline", "unknown"][self.state];
        self.filtered = self
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                (q.is_empty()
                    || r.name.to_lowercase().contains(&q)
                    || r.endpoint.to_lowercase().contains(&q))
                    && (state == "all"
                        || Self::state_of(r) == state
                        || (state == "failed" && Self::state_of(r) == "unreachable"))
            })
            .map(|(i, _)| i)
            .collect();
        self.cursor = self.cursor.min(self.filtered.len().saturating_sub(1));
        self.table.offset = self.table.offset.min(self.filtered.len().saturating_sub(1));
    }
    fn control(&mut self, i: usize) {
        match i {
            0 => self.editing = true,
            1 => {
                self.state = (self.state + 1) % 5;
                self.filter();
            }
            _ => {
                self.query.clear();
                self.editing = false;
                self.state = 0;
                self.filter();
            }
        }
    }
    pub fn on_key(&mut self, k: KeyEvent) -> RouterAction {
        if self.editing {
            match k.code {
                KeyCode::Enter => self.editing = false,
                KeyCode::Esc => {
                    self.editing = false;
                    self.query.clear();
                }
                KeyCode::Backspace => {
                    self.query.pop();
                }
                KeyCode::Char(c) => self.query.push(c),
                _ => return RouterAction::None,
            };
            self.filter();
            return RouterAction::None;
        }
        let last = self.filtered.len().saturating_sub(1);
        match k.code {
            KeyCode::Left => self.horizontal = self.horizontal.saturating_sub(8),
            KeyCode::Right => self.horizontal = self.horizontal.saturating_add(8).min(120),
            KeyCode::Char('/') => self.control(0),
            KeyCode::Char('t') => self.control(1),
            KeyCode::Char('x') => self.control(2),
            KeyCode::Tab => self.focus = (self.focus + 1) % 5,
            KeyCode::BackTab => self.focus = (self.focus + 4) % 5,
            KeyCode::Enter if self.focus < 3 => self.control(self.focus),
            KeyCode::Char('s') => {
                if let Some(r) = self.selected() {
                    return RouterAction::Subscribers(r.name.clone());
                }
            }
            KeyCode::Char('e') => {
                if let Some(r) = self.selected() {
                    return RouterAction::Settings(r.index);
                }
            }
            KeyCode::Char('c') => {
                if let Some(r) = self.selected() {
                    return RouterAction::Copy(r.endpoint.clone());
                }
            }
            KeyCode::Enter | KeyCode::Char('d') => self.expanded = !self.expanded,
            KeyCode::Esc => self.expanded = false,
            KeyCode::Up if self.focus == 4 => {
                self.details.offset = self.details.offset.saturating_sub(1)
            }
            KeyCode::Down if self.focus == 4 => {
                self.details.offset = self.details.offset.saturating_add(1).min(self.details.max)
            }
            KeyCode::Up => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Down => self.cursor = self.cursor.saturating_add(1).min(last),
            KeyCode::PageUp => self.cursor = self.cursor.saturating_sub(10),
            KeyCode::PageDown => self.cursor = self.cursor.saturating_add(10).min(last),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = last,
            _ => {}
        }
        RouterAction::None
    }
    pub fn on_mouse(&mut self, m: MouseEvent) -> RouterAction {
        if m.kind == MouseEventKind::Down(MouseButton::Left) {
            if let Some(i) = self
                .controls
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
            {
                self.control(i);
                return RouterAction::None;
            }
            if let Some(i) = self
                .pages
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
            {
                return self.on_key(KeyEvent::new(
                    if i == 0 {
                        KeyCode::PageUp
                    } else {
                        KeyCode::PageDown
                    },
                    m.modifiers,
                ));
            }
            if let Some(i) = self
                .actions
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
            {
                return self.on_key(KeyEvent::new(
                    KeyCode::Char(['d', 's', 'e', 'c'][i]),
                    m.modifiers,
                ));
            }
            if self.table.area.contains((m.column, m.row).into())
                && m.column < self.table.area.right().saturating_sub(1)
            {
                self.cursor = (self.table.offset + usize::from(m.row - self.table.area.y))
                    .min(self.filtered.len().saturating_sub(1));
                self.focus = 3;
                self.editing = false;
            }
        }
        if self.table.mouse(m) {
            let last = self.filtered.len().saturating_sub(1);
            self.table.offset = self.table.offset.min(last);
            self.cursor = self.table.offset;
        }
        self.details.mouse(m);
        RouterAction::None
    }
    pub fn draw(&mut self, f: &mut Frame, area: Rect, d: &Data, now: u64, p: &Palette) {
        f.render_widget(
            ratatui::widgets::Block::default().style(design::surface(p)),
            area,
        );
        let [cards, tools, body, status] = Layout::vertical([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(4),
        ])
        .areas(area);
        let known = self
            .records
            .iter()
            .filter(|r| Self::state_of(r) != "unknown")
            .count();
        let counts = [
            (
                "Configured",
                self.configured.map_or("unknown".into(), |n| n.to_string()),
            ),
            (
                "Reachable",
                if known > 0 {
                    self.records
                        .iter()
                        .filter(|r| Self::state_of(r) == "online")
                        .count()
                        .to_string()
                } else {
                    "unknown".into()
                },
            ),
            (
                "Failed reads",
                if known > 0 {
                    self.records
                        .iter()
                        .filter(|r| matches!(Self::state_of(r), "failed" | "unreachable"))
                        .count()
                        .to_string()
                } else {
                    "unknown".into()
                },
            ),
        ];
        let slots = Layout::horizontal([Constraint::Ratio(1, 3); 3]).split(cards);
        for (i, (label, count)) in counts.into_iter().enumerate() {
            f.render_widget(
                Paragraph::new(vec![
                    Line::raw(count),
                    Line::styled("latest collection", design::role(p, Role::Secondary)),
                ])
                .block(design::panel(label, p)),
                slots[i],
            );
        }
        let slots = Layout::horizontal([
            Constraint::Fill(3),
            Constraint::Fill(2),
            Constraint::Length(9),
        ])
        .split(tools);
        for (i, label) in [
            format!(
                "Search router / endpoint: {}{}",
                self.query,
                if self.editing { "_" } else { "" }
            ),
            format!(
                "State [{}]",
                ["All", "Reachable", "Failed", "Offline", "Unknown"][self.state]
            ),
            "Clear".into(),
        ]
        .into_iter()
        .enumerate()
        {
            self.controls[i] = slots[i];
            f.render_widget(
                Paragraph::new(label)
                    .style(if self.focus == i {
                        design::accent(p)
                    } else {
                        design::surface(p)
                    })
                    .block(design::panel("", p)),
                slots[i],
            );
        }
        let [table, detail] = if self.expanded {
            [Rect::default(), body]
        } else if body.width < 100 {
            Layout::vertical([Constraint::Percentage(50), Constraint::Min(0)]).areas(body)
        } else {
            Layout::horizontal([Constraint::Percentage(68), Constraint::Percentage(32)]).areas(body)
        };
        if table.height > 0 {
            let block = design::panel("Router list", p);
            let inner = block.inner(table);
            f.render_widget(block, table);
            let [header, rows, footer] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .areas(inner);
            f.render_widget(Paragraph::new("Router             Endpoint                 State       PPP / IPoE    Last read      Result").style(design::accent(p)).scroll((0,self.horizontal)),header);
            let h = rows.height as usize;
            if self.cursor < self.table.offset {
                self.table.offset = self.cursor;
            } else if h > 0 && self.cursor >= self.table.offset + h {
                self.table.offset = self.cursor + 1 - h;
            }
            for (y, &i) in self
                .filtered
                .iter()
                .skip(self.table.offset)
                .take(h)
                .enumerate()
            {
                let r = &self.records[i];
                let n = |key: &str| {
                    r.report["rows"][key]
                        .as_u64()
                        .map_or("?".into(), |n| n.to_string())
                };
                let state = if Self::state_of(r) == "online" {
                    "REACHABLE"
                } else {
                    Self::state_of(r)
                };
                let text = format!(
                    "{:<18} {:<24} {:<11} {:<13} {:<14} {}",
                    clip(&r.name, 18),
                    clip(&r.endpoint, 24),
                    clip(state, 11),
                    format!(
                        "{} / {}",
                        if r.sources[0] {
                            n("/ppp/active")
                        } else {
                            "OFF".into()
                        },
                        if r.sources[1] {
                            n("/ip/dhcp-server/lease")
                        } else {
                            "OFF".into()
                        }
                    ),
                    Self::age(d, now),
                    if r.report["error"].is_string() {
                        "FAILED"
                    } else if Self::state_of(r) == "online" {
                        "OK"
                    } else {
                        "unknown"
                    }
                );
                f.render_widget(
                    Paragraph::new(text).scroll((0, self.horizontal)).style(
                        if self.table.offset + y == self.cursor {
                            design::selected(p)
                        } else {
                            design::surface(p)
                        },
                    ),
                    Rect::new(rows.x, rows.y + y as u16, rows.width, 1),
                );
            }
            self.table.draw(f, rows, self.filtered.len(), p);
            f.render_widget(
                Paragraph::new(format!(
                    "Showing {}-{} / {}",
                    self.table.offset + usize::from(!self.filtered.is_empty()),
                    (self.table.offset + h).min(self.filtered.len()),
                    self.filtered.len()
                )),
                footer,
            );
            if footer.width > 70 {
                let name = self.selected().map_or("none", |r| r.name.as_str());
                f.render_widget(
                    Paragraph::new(format!("Selected: {}", clip(name, 20))),
                    Rect::new(footer.right() - 30, footer.y, 30, 1),
                );
            }
            self.pages = [
                Rect::new(
                    footer.x + (footer.width / 2).saturating_sub(7),
                    footer.y,
                    6,
                    1,
                ),
                Rect::new(footer.x + footer.width / 2, footer.y, 6, 1),
            ];
            for (i, label) in ["[Prev]", "[Next]"].into_iter().enumerate() {
                f.render_widget(
                    Paragraph::new(label).style(design::accent(p)),
                    self.pages[i],
                );
            }
        }
        let block = design::panel("Router details", p);
        let inner = block.inner(detail);
        f.render_widget(block, detail);
        let [fields, actions] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(4)]).areas(inner);
        let lines = if let Some(r) = self.selected() {
            let number = |v: &Value| v.as_u64().map_or("unknown".into(), |n| n.to_string());
            vec![
                Line::raw(format!("Name     {}", r.name)),
                Line::raw(format!("Endpoint {}", r.endpoint)),
                Line::raw(format!("State    {}", Self::state_of(r))),
                Line::raw(format!("Last read {}", Self::age(d, now))),
                Line::raw(format!("Read time {} ms", number(&r.report["read_ms"]))),
                Line::raw(""),
                Line::raw("Enabled sources"),
                Line::raw(format!(
                    "PPPoE {} | DHCP {} | Hotspot {}",
                    if r.sources[0] { "ON" } else { "OFF" },
                    if r.sources[1] { "ON" } else { "OFF" },
                    if r.sources[2] { "ON" } else { "OFF" }
                )),
                Line::raw(""),
                Line::raw("Collected records"),
                Line::raw(format!(
                    "PPP {} | IPoE {}",
                    if r.sources[0] {
                        number(&r.report["rows"]["/ppp/active"])
                    } else {
                        "OFF".into()
                    },
                    if r.sources[1] {
                        number(&r.report["rows"]["/ip/dhcp-server/lease"])
                    } else {
                        "OFF".into()
                    }
                )),
                Line::raw(format!(
                    "Hotspot {} active",
                    if r.sources[2] {
                        number(&r.report["rows"]["/ip/hotspot/active"])
                    } else {
                        "OFF".into()
                    }
                )),
                Line::styled(
                    r.report["error"].as_str().unwrap_or("").to_owned(),
                    design::role(p, Role::Error),
                ),
                Line::raw("Read-only collection evidence"),
            ]
        } else {
            vec![Line::raw(
                "No selected router; configuration or collection unavailable",
            )]
        };
        self.details.offset = self
            .details
            .offset
            .min(lines.len().saturating_sub(fields.height as usize));
        let total = lines.len();
        f.render_widget(
            Paragraph::new(
                lines
                    .into_iter()
                    .skip(self.details.offset)
                    .collect::<Vec<_>>(),
            ),
            fields,
        );
        self.details.draw(f, fields, total, p);
        for (i, label) in [
            "[Enter] Expand details",
            "[s] View subscribers",
            "[e] Open Settings",
            "[c] Copy endpoint",
        ]
        .into_iter()
        .enumerate()
        {
            let r = Rect::new(actions.x, actions.y + i as u16, actions.width, 1);
            self.actions[i] = r;
            f.render_widget(Paragraph::new(label).style(design::accent(p)), r);
        }
        f.render_widget(
            Paragraph::new(vec![
                Line::raw(format!(
                    "Latest collection | Age {} | Source: config + collection status",
                    Self::age(d, now)
                )),
                Line::raw("Reachability is from the last collection, not a new connection test"),
            ])
            .block(design::panel("Data status", p)),
            status,
        );
    }
    fn age(d: &Data, now: u64) -> String {
        d.status
            .as_ref()
            .and_then(|s| s["updated"].as_u64())
            .filter(|&t| t > 0 && t <= now)
            .map_or("unknown".into(), |t| format!("{}s ago", now - t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, crossterm::event::KeyModifiers};
    #[test]
    fn router_filters_navigation_and_snapshot_refresh_preserve_selection() {
        let mut d=Data{config_text:Arc::new("[[routers]]\nname='R1'\naddress='2001:db8::1'\nusername='private-user'\npassword_file='/fixture/secret'\n".into()),status:Some(Arc::new(serde_json::json!({"updated":100,"routers":[{"name":"R1","state":"online","rows":{"/ppp/active":2},"read_ms":10}]}))),..Default::default()};
        let mut v = RoutersView::default();
        v.on_data(&d);
        assert_eq!(v.records.len(), 1);
        let key = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        assert!(
            matches!(v.on_key(key('c')),RouterAction::Copy(s) if s.starts_with("[2001:db8::1]:")&&!s.contains("private-user"))
        );
        assert!(matches!(v.on_key(key('s')),RouterAction::Subscribers(n) if n=="R1"));
        assert!(matches!(v.on_key(key('e')), RouterAction::Settings(0)));
        v.on_key(key('t'));
        assert_eq!(v.filtered.len(), 1);
        v.on_key(key('t'));
        assert!(v.filtered.is_empty());
        v.on_key(key('x'));
        d.status = Some(Arc::new(
            serde_json::json!({"updated":102,"routers":[{"name":"R1","state":"failed"}]}),
        ));
        v.on_data(&d);
        assert_eq!(RoutersView::state_of(v.selected().unwrap()), "failed");
        let mut t = Terminal::new(TestBackend::new(160, 40)).unwrap();
        t.draw(|f| {
            v.draw(
                f,
                f.area(),
                &d,
                102,
                &Palette::new(crate::config::Theme::Dark),
            )
        })
        .unwrap();
        v.editing = true;
        v.query = "no-match".into();
        v.filter();
        let r = v.controls[2];
        v.on_mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: r.x,
            row: r.y,
            modifiers: KeyModifiers::NONE,
        });
        assert!(!v.editing);
        assert_eq!(v.filtered.len(), 1);
    }
}
