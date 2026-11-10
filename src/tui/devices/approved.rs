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
    pub(super) now: u64,
    live: bool,
    counts: [usize; 7],
    evidence: Option<Arc<std::collections::HashMap<String, crate::tui::presence::Evidence>>>,
    sources: Vec<String>,
    routers: Vec<String>,
    metadata: Option<Arc<std::collections::HashMap<String, (String, String)>>>,
    controls: [Rect; 5],
    pages: [Rect; 2],
    actions: [Rect; 4],
    queue: crate::circuit_queue::View,
    table: Scroll,
    detail: Scroll,
    horizontal: u16,
    expanded: bool,
    focus: Option<usize>,
}
impl SubscribersUi {
    pub fn matches(&self, d: &Data, r: &CsvRow) -> bool {
        let (src, router) = crate::tui::presence::source(d, r);
        self.source.as_deref().is_none_or(|s| s == src)
            && self.router.as_deref().is_none_or(|s| s == router)
            && match self.state {
                0 => true,
                1 => {
                    crate::tui::presence::state(d, r, self.now) == crate::collection::State::Online
                }
                2 => {
                    crate::tui::presence::state(d, r, self.now) == crate::collection::State::Offline
                }
                3 => matches!(
                    crate::tui::presence::state(d, r, self.now),
                    crate::collection::State::Unknown | crate::collection::State::Stale
                ),
                4 => crate::tui::presence::state(d, r, self.now) == crate::collection::State::Bound,
                5 => {
                    crate::tui::presence::state(d, r, self.now) == crate::collection::State::Unbound
                }
                _ => {
                    crate::tui::presence::state(d, r, self.now)
                        == crate::collection::State::Disabled
                }
            }
    }
}

pub enum Input {
    None,
    Parent(String),
    Copy(String),
    Queue(crate::circuit_queue::Request),
    Notice(String),
}
impl DevicesView {
    fn queue_circuit(&self, d: &Data) -> Option<String> {
        if !d.target.starts_with("v3_") {
            return None;
        }
        let r = &d.rows[*self.index.get(self.cursor)? as usize];
        (!r.circuit_id.is_empty() && !r.circuit_id.starts_with("observed:"))
            .then(|| r.circuit_id.clone())
    }
    fn queue_details(&self, d: &Data) -> Vec<String> {
        self.queue_circuit(d).map_or_else(
            || vec!["Queue diagnostics unavailable: select a v3 shaping circuit".into()],
            |c| self.approved.queue.measurements(&c, self.approved.now),
        )
    }
    pub fn queue_done(&mut self, done: crate::circuit_queue::Done) {
        self.approved.queue.complete(done);
    }
    fn host_details(&self, d: &Data) -> Vec<String> {
        let Some(&i) = self.index.get(self.cursor) else {
            return vec!["DPDK host observations unavailable: no selected record".into()];
        };
        d.subscriber_stats.as_ref().map_or_else(
            || {
                crate::subscriber_stats::Report::default().measurements(
                    &d.rows[i as usize],
                    &d.ip_owners,
                    self.approved.now,
                )
            },
            |s| s.measurements(&d.rows[i as usize], &d.ip_owners, self.approved.now),
        )
    }
    pub fn filter_router(&mut self, name: String, d: &Data) {
        self.approved.router = Some(name);
        self.cursor = 0;
        self.rebuild(d);
    }
    pub fn approved_data(&mut self, d: &Data) {
        if self
            .approved
            .metadata
            .as_ref()
            .is_none_or(|v| !Arc::ptr_eq(v, &d.sources))
            || self
                .approved
                .evidence
                .as_ref()
                .is_none_or(|v| !Arc::ptr_eq(v, &d.presence))
        {
            self.approved.metadata = Some(d.sources.clone());
            self.approved.evidence = Some(d.presence.clone());
            self.count_states(d);
            let mut sources = std::collections::BTreeSet::new();
            let mut routers = std::collections::BTreeSet::new();
            for r in d.rows.iter() {
                let (s, r) = crate::tui::presence::source(d, r);
                sources.insert(s.to_owned());
                routers.insert(r.to_owned());
            }
            self.approved.sources = sources.into_iter().collect();
            self.approved.routers = routers.into_iter().collect();
            self.rebuild(d);
        }
    }
    fn count_states(&mut self, d: &Data) {
        self.approved.counts = [0; 7];
        let mut seen = std::collections::HashSet::new();
        for r in d.rows.iter() {
            if !seen.insert(&r.circuit_id) {
                continue;
            }
            let index = match crate::tui::presence::state(d, r, self.approved.now) {
                crate::collection::State::Online => 1,
                crate::collection::State::Offline => 2,
                crate::collection::State::Bound => 4,
                crate::collection::State::Unbound => 5,
                crate::collection::State::Disabled => 6,
                _ => 3,
            };
            self.approved.counts[index] += 1;
        }
    }
    pub fn approved_tick(&mut self, d: &Data, now: u64) {
        self.approved.now = now;
        let live = crate::tui::presence::fresh(d, now);
        if live != self.approved.live {
            self.approved.live = live;
            self.count_states(d);
            self.rebuild(d);
        }
    }
    pub fn clear_approved_areas(&mut self) {
        self.approved.controls = [Rect::default(); 5];
        self.approved.actions = [Rect::default(); 4];
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
            3 => self.approved.state = (self.approved.state + 1) % 7,
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
                KeyCode::PageUp => {
                    scroll.offset = scroll
                        .offset
                        .saturating_sub(scroll.area.height.max(1) as usize)
                }
                KeyCode::PageDown => {
                    scroll.offset = scroll
                        .offset
                        .saturating_add(scroll.area.height.max(1) as usize)
                        .min(scroll.max)
                }
                KeyCode::Home => scroll.offset = 0,
                KeyCode::End => scroll.offset = scroll.max,
                _ => {}
            }
            if matches!(
                k.code,
                KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::PageUp
                    | KeyCode::PageDown
                    | KeyCode::Home
                    | KeyCode::End
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
                if let Some(p) = self.on_key(KeyEvent::new(KeyCode::Enter, k.modifiers), d)
                    && !p.is_empty()
                {
                    return Input::Parent(p);
                }
            }
            KeyCode::Char('d') => {
                let Some(circuit) = self.queue_circuit(d) else {
                    return Input::Notice(
                        "Select a v3 shaping circuit for queue diagnostics".into(),
                    );
                };
                return match self
                    .approved
                    .queue
                    .begin(&circuit, std::time::Instant::now())
                {
                    Ok(request) => Input::Queue(request),
                    Err(reason) => Input::Notice(reason.into()),
                };
            }
            KeyCode::Char('c') => {
                let mut lines: Vec<_> = self.detail(d).into_iter().map(|l| l.to_string()).collect();
                lines.extend(self.host_details(d));
                lines.extend(self.queue_details(d));
                return Input::Copy(lines.join("\n"));
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
                    KeyEvent::new(KeyCode::Char(['e', 'c', 'n', 'd'][i]), m.modifiers),
                    d,
                );
            }
            let detail = self.approved.detail.area;
            if detail.contains((m.column, m.row).into())
                && m.column < detail.right().saturating_sub(1)
            {
                self.approved.focus = Some(6);
                self.editing = false;
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
        self.approved_tick(d, now);
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
            ("Total", format!("{} records", d.rows.len())),
            (
                "Online",
                if crate::tui::presence::fresh(d, now)
                    && d.collection
                        .as_ref()
                        .is_some_and(|c| c.routers.iter().any(|r| r.state == "online"))
                {
                    format!("{} known", self.approved.counts[1])
                } else {
                    "unknown".into()
                },
            ),
            (
                "Offline",
                if crate::tui::presence::fresh(d, now)
                    && d.collection
                        .as_ref()
                        .is_some_and(|c| c.routers.iter().any(|r| r.state == "online"))
                {
                    format!("{} known", self.approved.counts[2])
                } else {
                    "unknown".into()
                },
            ),
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
                [
                    "All", "Online", "Offline", "Unknown", "Bound", "Unbound", "Disabled"
                ][self.approved.state]
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
            let block = design::panel(format!("Subscriber list | {} records", self.index.len()), p);
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
                let (source, router) = crate::tui::presence::source(d, r);
                let text = cols(&[
                    r.circuit_name.clone(),
                    source.into(),
                    r.ipv4.clone(),
                    router.into(),
                    crate::tui::presence::state(d, r, now).label().into(),
                    if r.circuit_id.starts_with("observed:") {
                        "unmatched".into()
                    } else {
                        format!("{}/{}", r.download_max, r.upload_max)
                    },
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
                    .map(|&i| {
                        if d.rows[i as usize].circuit_id.starts_with("observed:") {
                            d.rows[i as usize].circuit_name.as_str()
                        } else {
                            d.rows[i as usize].circuit_id.as_str()
                        }
                    })
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
        let title = self
            .index
            .get(self.cursor)
            .map_or("Subscriber details".into(), |&i| {
                let r = &d.rows[i as usize];
                format!(
                    "Subscriber details | {}",
                    clip(
                        if r.circuit_name.is_empty() {
                            &r.circuit_id
                        } else {
                            &r.circuit_name
                        },
                        24
                    )
                )
            });
        let title = self
            .queue_circuit(d)
            .and_then(|c| self.approved.queue.label(&c, now))
            .map_or_else(
                || title.clone(),
                |label| {
                    let badge = match label {
                        "STALE (last observed)" => "STALE",
                        "OBSERVED (source age unknown)" => "UNKNOWN AGE",
                        "OBSERVED (source clock is ahead)" => "CLOCK AHEAD",
                        "GAP/RESET observation" => "GAP/RESET",
                        other => other,
                    };
                    format!(
                        "Queues {badge} | {}",
                        title
                            .strip_prefix("Subscriber details | ")
                            .unwrap_or(&title)
                    )
                },
            );
        let block = design::panel(title, p);
        let inner = block.inner(right);
        f.render_widget(block, right);
        let mut lines = if let Some(&i) = self.index.get(self.cursor) {
            let r = &d.rows[i as usize];
            let (source, router) = crate::tui::presence::source(d, r);
            vec![
                Line::raw(format!(
                    "ID       {}",
                    if r.circuit_id.starts_with("observed:") {
                        "not assigned; collection only"
                    } else {
                        &r.circuit_id
                    }
                )),
                Line::raw(format!("Source   {source}")),
                Line::raw(format!("Router   {router}")),
                Line::raw(format!("IP       {}", r.ipv4)),
                Line::raw(format!("MAC      {}", r.mac)),
                Line::styled(
                    format!(
                        "State    {}",
                        crate::tui::presence::state(d, r, now).label()
                    ),
                    design::role(p, Role::Warning),
                ),
                Line::raw(
                    d.presence
                        .get(&r.circuit_id)
                        .map_or("Last observed unknown".into(), |p| {
                            format!("Observed {}", crate::tui::shell::local_clock(p.observed_at))
                        }),
                ),
                Line::raw(""),
                Line::raw("Shaping"),
                Line::raw(format!(
                    "Download {}",
                    if r.download_max.is_empty() {
                        "unknown".into()
                    } else {
                        format!("{} Mbps", r.download_max)
                    }
                )),
                Line::raw(format!(
                    "Upload   {}",
                    if r.upload_max.is_empty() {
                        "unknown".into()
                    } else {
                        format!("{} Mbps", r.upload_max)
                    }
                )),
                Line::raw(format!("Parent   {}", r.parent_node)),
                Line::raw(format!("Input    {}", d.target)),
                Line::raw(if r.circuit_id.starts_with("observed:") {
                    "Collection only; no shaping match"
                } else {
                    "Shaping input record; cycle state shown above"
                }),
                Line::raw("Profile and WAN controls planned"),
            ]
        } else {
            vec![Line::raw("No selected record")]
        };
        lines.extend(self.host_details(d).into_iter().map(Line::raw));
        lines.extend(self.queue_details(d).into_iter().map(Line::raw));
        if self.approved.expanded {
            lines.extend(self.detail(d));
        }
        let [detail, actions] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(4)]).areas(inner);
        let content = Rect {
            width: detail.width.saturating_sub(1),
            ..detail
        };
        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        let total = paragraph.line_count(content.width);
        self.approved.detail.offset = self
            .approved
            .detail
            .offset
            .min(total.saturating_sub(content.height as usize));
        f.render_widget(
            paragraph.scroll((
                u16::try_from(self.approved.detail.offset).unwrap_or(u16::MAX),
                0,
            )),
            content,
        );
        self.approved.detail.draw(f, detail, total, p);
        let has_parent = self
            .index
            .get(self.cursor)
            .is_some_and(|&i| !d.rows[i as usize].parent_node.is_empty());
        for (i, label) in [
            "[Enter] Expand details",
            "[c] Copy details",
            "[n] Open parent node",
            "[d] Read / refresh queue diagnostics",
        ]
        .into_iter()
        .enumerate()
        {
            let r = Rect::new(actions.x, actions.y + i as u16, actions.width, 1);
            let enabled = (i != 2 || has_parent) && (i != 3 || self.queue_circuit(d).is_some());
            self.approved.actions[i] = if enabled { r } else { Rect::default() };
            f.render_widget(
                Paragraph::new(if enabled {
                    label
                } else if i == 3 {
                    "Queues require a v3 shaping circuit"
                } else {
                    "No shaping parent"
                })
                .style(if enabled {
                    design::accent(p)
                } else {
                    design::role(p, Role::Secondary)
                }),
                r,
            );
        }
        let age = d
            .collection
            .as_ref()
            .map(|s| s.updated)
            .filter(|&t| t <= now)
            .map_or("unknown".into(), |t| format!("{}s", now - t));
        f.render_widget(
            Paragraph::new(vec![
                Line::raw(format!(
                    "Cycle collection | Age {age} | Bound {} | Unbound {} | Disabled {}",self.approved.counts[4],self.approved.counts[5],self.approved.counts[6]
                )),
                Line::raw(if d.collection.is_some() {"PPP/Hotspot session state; DHCP BOUND is a lease, not a reachability test. STALE is not offline."} else {"Collection unavailable; online/offline unknown without current session evidence"}),
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
    fn circuit_queue_mouse_action_is_manual_bound_to_selection_and_copyable() {
        let mut d = Data {
            target: "v3_pair".into(),
            rows: Arc::new(vec![
                CsvRow {
                    circuit_id: "c1".into(),
                    ..Default::default()
                },
                CsvRow {
                    circuit_id: "c2".into(),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        let mut v = DevicesView::default();
        v.on_data(&d);
        v.approved_data(&d);
        let mut term = Terminal::new(TestBackend::new(160, 40)).unwrap();
        term.draw(|f| {
            v.draw_approved(
                f,
                f.area(),
                &d,
                100,
                &Palette::new(crate::config::Theme::Dark),
            )
        })
        .unwrap();
        assert!(v.queue_details(&d).join("\n").contains("not requested"));
        let r = v.approved.actions[3];
        assert!(r.width > 0);
        let Input::Queue(request) = v.approved_mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: r.x,
                row: r.y,
                modifiers: KeyModifiers::NONE,
            },
            &d,
        ) else {
            panic!("click must request diagnostics")
        };
        assert_eq!(request.circuit, "c1");
        assert!(matches!(
            v.approved_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), &d),
            Input::Notice(_)
        ));
        v.cursor = 1;
        v.queue_done(crate::circuit_queue::Done {
            request,
            result: Err("source unavailable".into()),
        });
        let Input::Copy(text) =
            v.approved_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE), &d)
        else {
            panic!("copy")
        };
        assert!(
            text.contains("not requested for selected circuit")
                && !text.contains("source unavailable")
        );
        d.target = "v2_pair".into();
        assert!(matches!(
            v.approved_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), &d),
            Input::Notice(_)
        ));
        d.target = "v3_pair".into();
        d.rows = Arc::new(vec![CsvRow {
            circuit_id: "observed:1".into(),
            ..Default::default()
        }]);
        v.on_data(&d);
        v.approved_data(&d);
        assert!(matches!(
            v.approved_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), &d),
            Input::Notice(_)
        ));
    }
    #[test]
    fn subscriber_long_details_use_mouse_focus_wrapped_scroll_bounds_and_page_keys() {
        let rows = vec![CsvRow {
            circuit_id: "c1".into(),
            ipv4: "192.0.2.1".into(),
            ..Default::default()
        }];
        let d = Data {
            rows: Arc::new(rows),
            ..Default::default()
        };
        let mut v = DevicesView::default();
        v.on_data(&d);
        v.approved_data(&d);
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal
            .draw(|f| {
                v.draw_approved(
                    f,
                    f.area(),
                    &d,
                    100,
                    &Palette::new(crate::config::Theme::Dark),
                )
            })
            .unwrap();
        let r = v.approved.detail.area;
        assert!(v.approved.detail.max > 0);
        v.approved_mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: r.x,
                row: r.y,
                modifiers: KeyModifiers::NONE,
            },
            &d,
        );
        assert_eq!(v.approved.focus, Some(6));
        v.approved_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE), &d);
        assert!(v.approved.detail.offset > 0);
        assert_eq!(v.cursor, 0);
        v.approved_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE), &d);
        assert_eq!(v.approved.detail.offset, v.approved.detail.max);
        v.approved_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE), &d);
        assert!(v.approved.detail.offset < v.approved.detail.max);
    }

    #[test]
    fn subscriber_dpdk_copy_uses_selected_circuit_ip_and_stale_rates_are_hidden() {
        let rows = vec![CsvRow {
            circuit_id: "c1".into(),
            ipv4: "192.0.2.1".into(),
            ..Default::default()
        }];
        let report=serde_json::from_value(serde_json::json!({"version":1,"updated":100,"stale":false,"reason":null,
            "hosts":{"c1":{"192.0.2.1":{"actual":{"download_bytes":100,"upload_bytes":200},"down_mbps":1.0,"up_mbps":2.0,
            "rtt_ms":2.0,"rtt_down_ms":1.5,"rtt_up_ms":null,"retransmits":3,"retransmits_down":2,"retransmits_up":1,"seen_age_ms":100}}}})).unwrap();
        let d = Data {
            ip_owners: Arc::new(crate::subscriber_stats::owners(&rows)),
            rows: Arc::new(rows),
            subscriber_stats: Some(Arc::new(report)),
            ..Default::default()
        };
        let mut v = DevicesView::default();
        v.on_data(&d);
        v.approved_data(&d);
        v.approved.now = 100;
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        let Input::Copy(text) = v.approved_key(key, &d) else {
            panic!("copy")
        };
        assert!(
            text.contains("Delivered Mbps: down 1.000 | up 2.000")
                && text.contains("RTT sample median")
                && text.contains("TCP retransmits")
        );
        v.approved.now = 116;
        let Input::Copy(text) = v.approved_key(key, &d) else {
            panic!("copy")
        };
        assert!(text.contains("STALE") && text.contains("down unknown | up unknown"));
    }

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
