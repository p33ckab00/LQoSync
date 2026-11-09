//! Approved Subscribers layout and mouse controls. Existing row indexing remains authoritative.
use super::*;
use crate::tui::{
    design::{self, Scroll},
    theme::clip,
};
use ratatui::crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

#[derive(Debug, Default)]
pub struct SubscribersUi {
    pub source: Option<String>,
    pub router: Option<String>,
    state: usize,
    sources: Vec<String>,
    routers: Vec<String>,
    metadata: Option<Arc<std::collections::HashMap<String, (String, String)>>>,
    controls: [Rect; 5],
    pages: [Rect; 2],
    actions: [Rect; 3],
    table: Scroll,
    detail: Scroll,
    horizontal: u16,
    expanded: bool,
    focus: Option<usize>,
}
impl SubscribersUi {
    pub fn matches(&self, d: &Data, r: &CsvRow) -> bool {
        let (src, router) = source_of(d, &r.circuit_name);
        self.source.as_deref().is_none_or(|s| s == src)
            && self.router.as_deref().is_none_or(|s| s == router)
            && (self.state == 0 || self.state == 3)
    }
}

pub enum Input {
    None,
    Parent(String),
    Copy(String),
}
impl DevicesView {
    pub fn approved_data(&mut self, d: &Data) {
        if self
            .approved
            .metadata
            .as_ref()
            .is_none_or(|v| !Arc::ptr_eq(v, &d.sources))
        {
            self.approved.metadata = Some(d.sources.clone());
            let mut sources = std::collections::BTreeSet::new();
            let mut routers = std::collections::BTreeSet::new();
            for r in d.rows.iter() {
                let (s, r) = source_of(d, &r.circuit_name);
                sources.insert(s.to_owned());
                routers.insert(r.to_owned());
            }
            self.approved.sources = sources.into_iter().collect();
            self.approved.routers = routers.into_iter().collect();
            self.rebuild(d);
        }
    }
    pub fn clear_approved_areas(&mut self) {
        self.approved.controls = [Rect::default(); 5];
        self.approved.actions = [Rect::default(); 3];
        self.approved.pages = [Rect::default(); 2];
        self.approved.table.clear();
        self.approved.detail.clear();
    }
    fn cycle_filter(&mut self, i: usize, d: &Data) {
        match i {
            0 => {
                self.editing = true;
                return;
            }
            1 | 2 => {
                let (value, choices) = if i == 1 {
                    (&mut self.approved.source, &self.approved.sources)
                } else {
                    (&mut self.approved.router, &self.approved.routers)
                };
                *value = match value
                    .as_ref()
                    .and_then(|v| choices.iter().position(|c| c == v))
                {
                    None => choices.first().cloned(),
                    Some(n) => choices.get(n + 1).cloned(),
                };
            }
            3 => self.approved.state = (self.approved.state + 1) % 4,
            _ => {
                self.query.clear();
                self.editing = false;
                self.approved.source = None;
                self.approved.router = None;
                self.approved.state = 0;
            }
        }
        self.cursor = 0;
        self.rebuild(d);
    }
    pub fn approved_key(&mut self, k: KeyEvent, d: &Data) -> Input {
        if self.editing {
            self.on_key(k, d);
            return Input::None;
        }
        if self.approved.focus == Some(6) {
            let scroll = &mut self.approved.detail;
            match k.code {
                KeyCode::Up => scroll.offset = scroll.offset.saturating_sub(1),
                KeyCode::Down => scroll.offset = scroll.offset.saturating_add(1).min(scroll.max),
                KeyCode::Home => scroll.offset = 0,
                KeyCode::End => scroll.offset = scroll.max,
                _ => {}
            }
            if matches!(
                k.code,
                KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End
            ) {
                return Input::None;
            }
        }
        match k.code {
            KeyCode::Tab => self.approved.focus = Some((self.approved.focus.unwrap_or(5) + 1) % 8),
            KeyCode::BackTab => {
                self.approved.focus = Some((self.approved.focus.unwrap_or(5) + 7) % 8)
            }
            KeyCode::Enter if self.approved.focus.is_some_and(|n| n < 5) => {
                self.cycle_filter(self.approved.focus.unwrap_or(0), d)
            }
            KeyCode::Char('s') => self.cycle_filter(1, d),
            KeyCode::Char('r') => self.cycle_filter(2, d),
            KeyCode::Char('t') => self.cycle_filter(3, d),
            KeyCode::Char('n') => {
                if let Some(p) = self.on_key(KeyEvent::new(KeyCode::Enter, k.modifiers), d) {
                    return Input::Parent(p);
                }
            }
            KeyCode::Char('c') => {
                return Input::Copy(
                    self.detail(d)
                        .into_iter()
                        .map(|l| l.to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            }
            KeyCode::Enter | KeyCode::Char('e') => self.approved.expanded = !self.approved.expanded,
            KeyCode::Esc => self.approved.expanded = false,
            KeyCode::Right => {
                self.approved.horizontal = self.approved.horizontal.saturating_add(8).min(120)
            }
            KeyCode::Left => self.approved.horizontal = self.approved.horizontal.saturating_sub(8),
            _ => {
                self.on_key(k, d);
            }
        }
        Input::None
    }
    pub fn approved_mouse(&mut self, m: MouseEvent, d: &Data) -> Input {
        if m.kind == MouseEventKind::Down(MouseButton::Left) {
            if let Some(i) = self
                .approved
                .pages
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
            {
                self.on_key(
                    KeyEvent::new(
                        if i == 0 {
                            KeyCode::PageUp
                        } else {
                            KeyCode::PageDown
                        },
                        m.modifiers,
                    ),
                    d,
                );
                return Input::None;
            }
            if let Some(i) = self
                .approved
                .controls
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
            {
                self.cycle_filter(i, d);
                return Input::None;
            }
            if let Some(i) = self
                .approved
                .actions
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
            {
                return self.approved_key(
                    KeyEvent::new(KeyCode::Char(['e', 'c', 'n'][i]), m.modifiers),
                    d,
                );
            }
            let r = self.approved.table.area;
            if r.contains((m.column, m.row).into()) && m.column < r.right().saturating_sub(1) {
                self.cursor = (self.offset + usize::from(m.row - r.y))
                    .min(self.index.len().saturating_sub(1));
                self.select(d);
                self.approved.focus = Some(5);
                self.editing = false;
            }
        }
        if self.approved.table.mouse(m) {
            self.offset = self.approved.table.offset;
            self.cursor = self.cursor.clamp(
                self.offset,
                (self.offset + self.approved.table.area.height.saturating_sub(1) as usize)
                    .min(self.index.len().saturating_sub(1)),
            );
            self.select(d);
        }
        self.approved.detail.mouse(m);
        match m.kind {
            MouseEventKind::ScrollRight => {
                self.approved.horizontal = self.approved.horizontal.saturating_add(8).min(120)
            }
            MouseEventKind::ScrollLeft => {
                self.approved.horizontal = self.approved.horizontal.saturating_sub(8)
            }
            _ => {}
        }
        Input::None
    }
    pub fn draw_approved(&mut self, f: &mut Frame, area: Rect, d: &Data, now: u64, p: &Palette) {
        use ratatui::style::Modifier;
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
        for (i, (title, value)) in [
            ("Total", self.total_circuits.to_string()),
            ("Online", "unknown".into()),
            ("Offline", "unknown".into()),
        ]
        .into_iter()
        .enumerate()
        {
            let a = Layout::horizontal([Constraint::Ratio(1, 3); 3]).split(cards)[i];
            f.render_widget(Paragraph::new(value).block(design::panel(title, p)), a);
        }
        let slots = Layout::horizontal([
            Constraint::Fill(4),
            Constraint::Fill(2),
            Constraint::Fill(2),
            Constraint::Fill(2),
            Constraint::Length(9),
        ])
        .split(tools);
        let labels = [
            format!(
                "Search: {}{}",
                self.query,
                if self.editing { "_" } else { "" }
            ),
            format!(
                "Source [{}]",
                self.approved.source.as_deref().unwrap_or("All")
            ),
            format!(
                "Router [{}]",
                self.approved.router.as_deref().unwrap_or("All")
            ),
            format!(
                "State [{}]",
                ["All", "Online", "Offline", "Unknown"][self.approved.state]
            ),
            "Clear".into(),
        ];
        for (i, label) in labels.into_iter().enumerate() {
            self.approved.controls[i] = slots[i];
            f.render_widget(
                Paragraph::new(label)
                    .style(if self.approved.focus == Some(i) {
                        design::accent(p)
                    } else {
                        design::surface(p)
                    })
                    .block(design::panel("", p)),
                slots[i],
            );
        }
        let [left, right] = if self.approved.expanded {
            [Rect::default(), body]
        } else if body.width < 100 {
            Layout::vertical([Constraint::Percentage(50), Constraint::Min(0)]).areas(body)
        } else {
            Layout::horizontal([Constraint::Percentage(68), Constraint::Percentage(32)]).areas(body)
        };
        if left.height > 0 {
            let block = design::panel(
                format!(
                    "Subscriber list | {} records / {} circuits",
                    self.index.len(),
                    self.filtered_circuits
                ),
                p,
            );
            let inner = block.inner(left);
            f.render_widget(block, left);
            let [header, rows, footer] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .areas(inner);
            let cols = |r: &[String]| {
                format!(
                    "{:<18} {:<9} {:<16} {:<12} {:<9} {:<14} {}",
                    clip(&r[0], 18),
                    clip(&r[1], 9),
                    clip(&r[2], 16),
                    clip(&r[3], 12),
                    clip(&r[4], 9),
                    clip(&r[5], 14),
                    r[6]
                )
            };
            f.render_widget(
                Paragraph::new(cols(
                    &[
                        "Subscriber",
                        "Source",
                        "IP address",
                        "Router",
                        "State",
                        "Down/Up Mbps",
                        "Parent",
                    ]
                    .map(str::to_owned),
                ))
                .style(design::surface(p).add_modifier(Modifier::BOLD))
                .scroll((0, self.approved.horizontal)),
                header,
            );
            let height = rows.height as usize;
            if self.cursor < self.offset {
                self.offset = self.cursor;
            } else if height > 0 && self.cursor >= self.offset + height {
                self.offset = self.cursor + 1 - height;
            }
            self.rows_built = 0;
            for (y, &i) in self.index.iter().skip(self.offset).take(height).enumerate() {
                let r = &d.rows[i as usize];
                let (source, router) = source_of(d, &r.circuit_name);
                let text = cols(&[
                    r.circuit_name.clone(),
                    source.into(),
                    r.ipv4.clone(),
                    router.into(),
                    "UNKNOWN".into(),
                    format!("{}/{}", r.download_max, r.upload_max),
                    r.parent_node.clone(),
                ]);
                let style = if self.offset + y == self.cursor {
                    design::selected(p)
                } else {
                    design::surface(p)
                };
                f.render_widget(
                    Paragraph::new(text)
                        .style(style)
                        .scroll((0, self.approved.horizontal)),
                    Rect::new(rows.x, rows.y + y as u16, rows.width, 1),
                );
                self.rows_built += 1;
            }
            self.approved.table.offset = self.offset;
            self.approved.table.draw(f, rows, self.index.len(), p);
            self.offset = self.approved.table.offset;
            f.render_widget(
                Paragraph::new(format!(
                    "Records {}-{} / {}",
                    self.offset + usize::from(!self.index.is_empty()),
                    (self.offset + height).min(self.index.len()),
                    self.index.len()
                )),
                footer,
            );
            let x = footer.x + (footer.width / 2).saturating_sub(7);
            self.approved.pages = [
                Rect::new(x, footer.y, 6, 1),
                Rect::new(x + 7, footer.y, 6, 1),
            ];
            if footer.width > 70 {
                let id = self
                    .index
                    .get(self.cursor)
                    .map(|&i| d.rows[i as usize].circuit_id.as_str())
                    .unwrap_or("none");
                f.render_widget(
                    Paragraph::new(format!("Selected: {}", clip(id, 20))),
                    Rect::new(footer.right() - 30, footer.y, 30, 1),
                );
            }
            for (i, label) in ["[Prev]", "[Next]"].into_iter().enumerate() {
                f.render_widget(
                    Paragraph::new(label).style(design::accent(p)),
                    self.approved.pages[i],
                );
            }
        }
        let block = design::panel("Subscriber details", p);
        let inner = block.inner(right);
        f.render_widget(block, right);
        let mut lines = if let Some(&i) = self.index.get(self.cursor) {
            let r = &d.rows[i as usize];
            let (source, router) = source_of(d, &r.circuit_name);
            vec![
                Line::raw(format!("ID       {}", r.circuit_id)),
                Line::raw(format!("Source   {source}")),
                Line::raw(format!("Router   {router}")),
                Line::raw(format!("IP       {}", r.ipv4)),
                Line::raw(format!("MAC      {}", r.mac)),
                Line::styled("State    UNKNOWN", design::role(p, Role::Warning)),
                Line::raw("Last seen unknown"),
                Line::raw(""),
                Line::raw("Shaping"),
                Line::raw(format!("Download {} Mbps", r.download_max)),
                Line::raw(format!("Upload   {} Mbps", r.upload_max)),
                Line::raw(format!("Parent   {}", r.parent_node)),
                Line::raw(format!("Input    {}", d.target)),
                Line::raw("No current session evidence"),
                Line::raw("Profile and WAN controls planned"),
            ]
        } else {
            vec![Line::raw("No selected record")]
        };
        if self.approved.expanded {
            lines.extend(self.detail(d));
        }
        self.approved.detail.max = lines
            .len()
            .saturating_sub(inner.height.saturating_sub(3) as usize);
        self.approved.detail.offset = self.approved.detail.offset.min(self.approved.detail.max);
        let [detail, actions] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(inner);
        let total = lines.len();
        f.render_widget(
            Paragraph::new(
                lines
                    .into_iter()
                    .skip(self.approved.detail.offset)
                    .collect::<Vec<_>>(),
            ),
            detail,
        );
        self.approved.detail.draw(f, detail, total, p);
        for (i, label) in [
            "[Enter] Expand details",
            "[c] Copy details",
            "[n] Open parent node",
        ]
        .into_iter()
        .enumerate()
        {
            let r = Rect::new(actions.x, actions.y + i as u16, actions.width, 1);
            self.approved.actions[i] = r;
            f.render_widget(Paragraph::new(label).style(design::accent(p)), r);
        }
        let age = d
            .status
            .as_ref()
            .and_then(|s| s["updated"].as_u64())
            .filter(|&t| t <= now)
            .map_or("unknown".into(), |t| format!("{}s", now - t));
        f.render_widget(
            Paragraph::new(vec![
                Line::raw(format!(
                    "Shaping input | Cycle age {age} | Source: cached input + collected metadata"
                )),
                Line::raw("Online/offline unknown without current session evidence"),
            ])
            .block(design::panel("Data status", p)),
            status,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};
    #[test]
    fn filters_copy_and_visible_row_work_are_bounded() {
        let rows: Vec<_> = (0..1000)
            .map(|i| CsvRow {
                circuit_id: i.to_string(),
                device_id: i.to_string(),
                circuit_name: format!("subscriber-{i}"),
                parent_node: "Site".into(),
                ..Default::default()
            })
            .collect();
        let sources = rows
            .iter()
            .map(|r| (r.circuit_name.clone(), ("ppp".into(), "R1".into())))
            .collect();
        let d = Data {
            row_search: Arc::new(rows.iter().map(|r| r.circuit_name.clone()).collect()),
            rows: Arc::new(rows),
            sources: Arc::new(sources),
            ..Default::default()
        };
        let mut v = DevicesView::default();
        v.on_data(&d);
        v.approved_data(&d);
        let k = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        v.approved_key(k('s'), &d);
        assert_eq!(v.index.len(), 1000);
        v.approved_key(k('t'), &d);
        assert!(v.index.is_empty(), "unknown records must not become online");
        v.cycle_filter(4, &d);
        assert_eq!(v.index.len(), 1000);
        let mut term = Terminal::new(TestBackend::new(160, 40)).unwrap();
        term.draw(|f| {
            v.draw_approved(
                f,
                f.area(),
                &d,
                1,
                &Palette::new(crate::config::Theme::Dark),
            )
        })
        .unwrap();
        assert!(v.rows_built <= 30);
        assert!(matches!(v.approved_key(k('c'),&d),Input::Copy(s) if s.contains("subscriber-")));
        assert!(matches!(v.approved_key(k('n'),&d),Input::Parent(s) if s=="Site"));
        v.editing = true;
        v.query = "not-a-record".into();
        v.rebuild(&d);
        let r = v.approved.controls[4];
        v.approved_mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: r.x,
                row: r.y,
                modifiers: KeyModifiers::NONE,
            },
            &d,
        );
        assert!(!v.editing && v.query.is_empty());
        assert_eq!(v.index.len(), 1000);
    }
}
