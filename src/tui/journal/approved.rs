//! Approved Journal inspection. Display text is safe; copying preserves original retained records.
use super::*;
use crate::tui::design::{self, Scroll};
use ratatui::text::Line;

#[derive(Default)]
pub struct JournalUi {
    table: Scroll,
    detail: Scroll,
    body: Scroll,
    controls: [Rect; 4],
    actions: [Rect; 4],
    focus: usize,
    horizontal: u16,
    expanded: bool,
    hidden_rows: usize,
    detail_cache: Option<(u64, u16, Vec<Line<'static>>)>,
}
pub enum JournalAction {
    None,
    Key(KeyCode),
    Clear,
    Copy,
    Select,
}
fn visible(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { '�' } else { c })
        .collect()
}
fn parts(line: &str) -> (&str, &str) {
    let first = line.split_whitespace().next().unwrap_or("");
    let b = first.as_bytes();
    let iso = b.len() >= 19
        && b.get(4) == Some(&b'-')
        && b.get(7) == Some(&b'-')
        && b.get(10) == Some(&b'T')
        && b.get(13) == Some(&b':')
        && b.get(16) == Some(&b':')
        && [0..4, 5..7, 8..10, 11..13, 14..16, 17..19]
            .iter()
            .all(|r| b[r.clone()].iter().all(u8::is_ascii_digit));
    if !iso {
        return ("unknown", line);
    }
    let rest = line[first.len()..].trim_start();
    let message = rest
        .split_once("]: ")
        .map(|(_, m)| m)
        .or_else(|| {
            rest.split_once(": ")
                .filter(|(h, _)| h.contains(' '))
                .map(|(_, m)| m)
        })
        .unwrap_or(rest);
    (first, message)
}
impl JournalView {
    pub fn clear_search(&mut self) {
        self.query.clear();
        self.editing = false;
        self.anchor = None;
        self.selection_expired = false;
        self.refilter();
    }
    pub fn age(&self, now: u64) -> String {
        self.last_received
            .filter(|&t| now >= t)
            .map_or("unknown".into(), |t| format!("{}s", now - t))
    }
    fn stream(&self) -> &str {
        if self.stopped {
            "STOPPED"
        } else if self.error.is_some() {
            "ISSUE / UNAVAILABLE"
        } else if self.last_received.is_some() {
            "CONNECTED"
        } else if self.started {
            "WAITING"
        } else {
            "STARTING"
        }
    }
    pub fn clear_approved_areas(&mut self) {
        self.approved.table.clear();
        self.approved.detail.clear();
        self.approved.body.clear();
        self.approved.controls = [Rect::default(); 4];
        self.approved.actions = [Rect::default(); 4];
    }
    pub fn approved_key(&mut self, k: KeyEvent) {
        if self.editing {
            self.on_key(k);
            return;
        }
        match k.code {
            KeyCode::Tab => self.approved.focus = (self.approved.focus + 1) % 6,
            KeyCode::BackTab => self.approved.focus = (self.approved.focus + 5) % 6,
            KeyCode::Enter if self.approved.focus < 4 => {
                match self.approved.focus {
                    0 => self.editing = true,
                    1 => self.on_key(KeyEvent::new(KeyCode::Char('l'), k.modifiers)),
                    2 => self.on_key(KeyEvent::new(KeyCode::Char('f'), k.modifiers)),
                    _ => self.clear_search(),
                };
            }
            KeyCode::Enter | KeyCode::Char('e') => self.approved.expanded = !self.approved.expanded,
            KeyCode::Esc => {
                self.approved.expanded = false;
                self.on_key(k);
            }
            KeyCode::Left => self.approved.horizontal = self.approved.horizontal.saturating_sub(12),
            KeyCode::Right => {
                self.approved.horizontal = self.approved.horizontal.saturating_add(12)
            }
            KeyCode::Up if self.approved.focus == 5 => {
                self.approved.detail.offset = self.approved.detail.offset.saturating_sub(1)
            }
            KeyCode::Down if self.approved.focus == 5 => {
                self.approved.detail.offset = self
                    .approved
                    .detail
                    .offset
                    .saturating_add(1)
                    .min(self.approved.detail.max)
            }
            _ => self.on_key(k),
        }
    }
    pub fn approved_mouse(&mut self, m: MouseEvent) -> JournalAction {
        if m.kind == MouseEventKind::Down(MouseButton::Left) {
            if let Some(i) = self
                .approved
                .controls
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
            {
                self.approved.focus = i;
                if i == 0 {
                    self.editing = true;
                    return JournalAction::None;
                }
                self.editing = false;
                return match i {
                    1 => JournalAction::Key(KeyCode::Char('l')),
                    2 => JournalAction::Key(KeyCode::Char('f')),
                    _ => JournalAction::Clear,
                };
            }
            if let Some(i) = self
                .approved
                .actions
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
            {
                self.editing = false;
                return match i {
                    0 | 3 => JournalAction::Copy,
                    1 => {
                        self.follow = true;
                        self.refilter();
                        JournalAction::None
                    }
                    _ => JournalAction::Select,
                };
            }
        }
        if self.approved.table.mouse(m) {
            self.offset = self.approved.table.offset;
            self.cursor =
                (self.offset + self.approved.hidden_rows).min(self.view.len().saturating_sub(1));
            self.follow = false;
            self.anchor = None;
            self.selection_expired = false;
            return JournalAction::None;
        }
        let r = self.approved.table.area;
        if r.contains((m.column, m.row).into()) {
            let at = (self.offset + self.approved.hidden_rows + usize::from(m.row - r.y))
                .min(self.view.len().saturating_sub(1));
            match m.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    self.cursor = at;
                    self.anchor = Some(at);
                    self.follow = false;
                    self.editing = false;
                    self.selection_expired = false;
                    self.approved.focus = 4;
                }
                MouseEventKind::Drag(MouseButton::Left) if self.anchor.is_some() => {
                    self.cursor = at;
                    self.follow = false;
                }
                _ => {}
            }
        }
        if m.kind == MouseEventKind::Down(MouseButton::Left)
            && self.approved.detail.area.contains((m.column, m.row).into())
        {
            self.approved.focus = 5;
        }
        if !self.approved.detail.mouse(m) {
            self.approved.body.mouse(m);
        }
        JournalAction::None
    }
    pub fn draw_approved(
        &mut self,
        f: &mut Frame,
        area: Rect,
        now: u64,
        select_mode: bool,
        p: &Palette,
    ) {
        let height = area.height.max(28);
        self.approved.body.offset = self
            .approved
            .body
            .offset
            .min(height.saturating_sub(area.height) as usize);
        self.approved.hidden_rows = 0;
        if height > area.height {
            let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(
                area.width.saturating_sub(1),
                height,
            ))
            .expect("in-memory terminal");
            t.draw(|v| self.draw_body(v, v.area(), now, select_mode, p))
                .expect("in-memory draw");
            let b = t.backend().buffer();
            for y in 0..area.height {
                for x in 0..area.width.saturating_sub(1) {
                    f.buffer_mut()[(area.x + x, area.y + y)] =
                        b[(x, y + self.approved.body.offset as u16)].clone();
                }
            }
            let map = |r: Rect| design::viewport_rect(r, area, self.approved.body.offset);
            self.approved.controls = self.approved.controls.map(map);
            self.approved.actions = self.approved.actions.map(map);
            self.approved.hidden_rows = self
                .approved
                .body
                .offset
                .saturating_sub(self.approved.table.area.y as usize);
            self.approved.table.area = map(self.approved.table.area);
            self.approved.detail.area = map(self.approved.detail.area);
        } else {
            self.draw_body(f, area, now, select_mode, p);
        }
        self.approved.body.draw(f, area, height as usize, p);
    }
    fn draw_body(&mut self, f: &mut Frame, area: Rect, now: u64, select_mode: bool, p: &Palette) {
        f.render_widget(
            ratatui::widgets::Block::default().style(design::surface(p)),
            area,
        );
        let [cards, tools, body, selection, status] = Layout::vertical([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(14),
            Constraint::Length(3),
            Constraint::Length(4),
        ])
        .areas(area);
        let regions = Layout::horizontal([Constraint::Ratio(1, 3); 3]).split(cards);
        for (i, (title, value, note)) in [
            ("Log source", visible(&self.unit), "journalctl stream"),
            (
                "Retained lines",
                format!("{} / {}", self.lines.len(), self.max),
                "bounded local buffer",
            ),
            (
                "View",
                if self.follow { "FOLLOWING" } else { "PAUSED" }.into(),
                if self.selection_expired {
                    "previous selection expired"
                } else {
                    "selection retained while available"
                },
            ),
        ]
        .into_iter()
        .enumerate()
        {
            f.render_widget(
                Paragraph::new(vec![
                    Line::styled(
                        value,
                        design::role(
                            p,
                            if i < 2 || self.follow {
                                Role::Ok
                            } else {
                                Role::Warning
                            },
                        ),
                    ),
                    Line::styled(note, design::role(p, Role::Secondary)),
                ])
                .block(design::panel(title, p)),
                regions[i],
            );
        }
        let regions = Layout::horizontal([
            Constraint::Fill(4),
            Constraint::Fill(2),
            Constraint::Fill(2),
            Constraint::Length(16),
        ])
        .split(tools);
        for (i, label) in [
            format!(
                "Search: {}{}",
                visible(&self.query),
                if self.editing { "_" } else { "" }
            ),
            format!(
                "Level [{}]",
                ["All", "Warning+", "Error"][self.level as usize]
            ),
            format!("Follow {}", if self.follow { "ON" } else { "OFF" }),
            "Clear search".into(),
        ]
        .into_iter()
        .enumerate()
        {
            self.approved.controls[i] = regions[i];
            f.render_widget(
                Paragraph::new(label)
                    .style(if self.approved.focus == i {
                        design::accent(p)
                    } else {
                        design::surface(p)
                    })
                    .block(design::panel("", p)),
                regions[i],
            );
        }
        let [table, details] = if self.approved.expanded {
            [Rect::default(), body]
        } else if body.width < 100 {
            Layout::vertical([Constraint::Percentage(50), Constraint::Min(0)]).areas(body)
        } else {
            Layout::horizontal([Constraint::Percentage(70), Constraint::Percentage(30)]).areas(body)
        };
        if table.height > 0 {
            let block = design::panel(
                format!(
                    "Service journal | {} matches / {} retained",
                    self.view.len(),
                    self.lines.len()
                ),
                p,
            );
            let inner = block.inner(table);
            f.render_widget(block, table);
            let [header, rows, footer] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .areas(inner);
            f.render_widget(
                Paragraph::new("Time       Level      Message")
                    .style(design::accent(p))
                    .scroll((0, self.approved.horizontal)),
                header,
            );
            let h = rows.height as usize;
            if self.follow {
                self.offset = self.view.len().saturating_sub(h);
            } else if self.cursor < self.offset {
                self.offset = self.cursor;
            } else if h > 0 && self.cursor >= self.offset + h {
                self.offset = self.cursor + 1 - h;
            }
            let (a, b) = self.anchor.map_or((self.cursor, self.cursor), |a| {
                (a.min(self.cursor), a.max(self.cursor))
            });
            for (y, &index) in self.view.iter().skip(self.offset).take(h).enumerate() {
                let original = &self.lines[index].1;
                let (time, message) = parts(original);
                let time = time.get(11..19).unwrap_or(time);
                let level = severity(original).map_or("UNKNOWN", |(label, _)| label);
                let style = if (a..=b).contains(&(self.offset + y)) {
                    design::selected(p)
                } else {
                    design::surface(p)
                };
                f.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::raw(format!("{time:<10} ")),
                        Span::styled(
                            format!("{level:<10} "),
                            style.fg(design::role(
                                p,
                                match self.lines[index].0 {
                                    1 => Role::Warning,
                                    2 => Role::Error,
                                    _ => Role::Base,
                                },
                            )
                            .fg
                            .unwrap_or(ratatui::style::Color::Reset)),
                        ),
                        Span::raw(visible(message)),
                    ]))
                    .style(style)
                    .scroll((0, self.approved.horizontal)),
                    Rect::new(rows.x, rows.y + y as u16, rows.width, 1),
                );
            }
            self.approved.table.offset = self.offset;
            self.approved.table.draw(f, rows, self.view.len(), p);
            let selected = if self.view.is_empty() {
                0
            } else {
                b.saturating_sub(a) + 1
            };
            f.render_widget(
                Paragraph::new(format!(
                    "Selected {selected} | Follow {} | {}",
                    if self.follow { "ON" } else { "paused" },
                    if self.selection_expired {
                        "selection expired; choose an entry"
                    } else {
                        "copy preserves original text"
                    }
                )),
                footer,
            );
        }
        let block = design::panel("Selected entry", p);
        let inner = block.inner(details);
        f.render_widget(block, details);
        let [fields, actions, note, _] = Layout::vertical([
            Constraint::Length(inner.height.saturating_sub(4).min(12)),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .areas(inner);
        let selected = self.view.get(self.cursor).copied();
        let key = selected.map(|i| self.discarded.saturating_add(i as u64));
        if self
            .approved
            .detail_cache
            .as_ref()
            .is_none_or(|(old, width, _)| Some(*old) != key || *width != fields.width)
        {
            let lines = if let Some(index) = selected {
                let original = &self.lines[index].1;
                let (time, message) = parts(original);
                let mut lines = vec![
                    Line::raw(visible(time)),
                    Line::raw(format!("Unit {}", visible(&self.unit))),
                    Line::raw(format!(
                        "Level {} (message prefix)",
                        severity(original).map_or("UNKNOWN", |(s, _)| s)
                    )),
                    Line::raw(""),
                    Line::raw("Full message"),
                ];
                let mut current = String::new();
                let mut width = 0;
                let max = fields.width.saturating_sub(1).max(1) as usize;
                for ch in visible(message).chars() {
                    let cells = Line::from(ch.to_string()).width();
                    if !current.is_empty() && width + cells > max {
                        lines.push(Line::raw(std::mem::take(&mut current)));
                        width = 0;
                    }
                    current.push(ch);
                    width += cells;
                }
                lines.push(Line::raw(current));
                lines
            } else {
                vec![
                    Line::raw("No selected entry"),
                    Line::raw("Waiting for retained log records"),
                ]
            };
            self.approved.detail_cache = Some((key.unwrap_or(u64::MAX), fields.width, lines));
            self.approved.detail.offset = 0;
        }
        let lines = &self.approved.detail_cache.as_ref().unwrap().2;
        let total = lines.len();
        self.approved.detail.offset = self
            .approved
            .detail
            .offset
            .min(total.saturating_sub(fields.height as usize));
        f.render_widget(
            Paragraph::new(
                lines
                    .iter()
                    .skip(self.approved.detail.offset)
                    .take(fields.height as usize)
                    .cloned()
                    .collect::<Vec<_>>(),
            ),
            fields,
        );
        self.approved.detail.draw(f, fields, total, p);
        f.render_widget(
            Paragraph::new("\nInspection only\nActions do not restart services."),
            note,
        );
        let buttons = Layout::horizontal([Constraint::Percentage(50); 2]).split(actions);
        for (i, label) in ["[Copy selected]", "[Resume follow]"]
            .into_iter()
            .enumerate()
        {
            let r = buttons[i];
            self.approved.actions[i] = r;
            f.render_widget(Paragraph::new(label).style(design::accent(p)), r);
        }
        let block = design::panel("Selection and navigation", p);
        let inner = block.inner(selection);
        f.render_widget(block, selection);
        let width = inner.width / 3;
        self.approved.actions[2] = Rect::new(inner.x, inner.y, width, 1);
        self.approved.actions[3] = Rect::new(inner.x + width, inner.y, width, 1);
        f.render_widget(
            Paragraph::new(format!(
                "[Select text {}]",
                if select_mode { "ON; s exits" } else { "OFF" }
            ))
            .style(design::accent(p)),
            self.approved.actions[2],
        );
        f.render_widget(
            Paragraph::new("[Copy selected]").style(design::accent(p)),
            self.approved.actions[3],
        );
        f.render_widget(
            Paragraph::new("Shift+arrows extend | End latest"),
            Rect::new(inner.x + 2 * width, inner.y, inner.width - 2 * width, 1),
        );
        let warning = if self.selection_expired {
            "Selection expired; choose an entry before copy"
        } else {
            self.error.as_deref().unwrap_or("Search/level filters apply to retained lines; terminal selection releases mouse capture")
        };
        f.render_widget(
            Paragraph::new(vec![
                Line::raw(format!(
                    "Stream {} | Last received {} ago | Retained {} | Follow {}",
                    self.stream(),
                    self.age(now),
                    self.lines.len(),
                    if self.follow { "ON" } else { "PAUSED" }
                )),
                Line::styled(
                    visible(warning),
                    design::role(
                        p,
                        if self.error.is_some() || self.selection_expired {
                            Role::Warning
                        } else {
                            Role::Secondary
                        },
                    ),
                ),
            ])
            .block(design::panel("Data status", p)),
            status,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    fn mouse(kind: MouseEventKind, r: Rect) -> MouseEvent {
        MouseEvent {
            kind,
            column: r.x,
            row: r.y,
            modifiers: KeyModifiers::NONE,
        }
    }
    #[test]
    fn quiet_stream_remains_connected_until_stopped() {
        let mut v = JournalView::new(100);
        assert_eq!(v.stream(), "STARTING");
        v.push_batch(vec![JournalMsg::Started]);
        assert_eq!(v.stream(), "WAITING");
        v.push_batch(vec![JournalMsg::Line("info ready".into())]);
        assert_eq!(v.stream(), "CONNECTED");
        assert_ne!(v.age(u64::MAX), "unknown");
        assert_eq!(v.age(0), "unknown");
        v.push_batch(vec![JournalMsg::Stopped("EOF".into())]);
        assert_eq!(v.stream(), "STOPPED");
    }
    #[test]
    fn controls_wheel_drag_and_display_preserve_original_log_copy() {
        let mut v = JournalView::new(100);
        let original = "2026-11-09T10:00:00+0800 host lqosync3[1]: ERROR reload \x1b[31m failed";
        v.push_batch((0..60).map(|_| JournalMsg::Line(original.into())).collect());
        let mut t = Terminal::new(TestBackend::new(210, 35)).unwrap();
        let p = Palette::new(crate::config::Theme::Dark);
        t.draw(|f| v.draw_approved(f, f.area(), 1, false, &p))
            .unwrap();
        let rows = v.approved.table.area;
        v.approved_mouse(mouse(MouseEventKind::ScrollUp, rows));
        assert!(!v.follow);
        assert!(v.offset < 59);
        v.approved_mouse(mouse(MouseEventKind::Down(MouseButton::Left), rows));
        let mut end = rows;
        end.y += 1;
        v.approved_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), end));
        assert_eq!(v.selection_text(), format!("{original}\n{original}"));
        assert!(!visible(original).contains('\x1b'));
        assert_eq!(parts(original).0, "2026-11-09T10:00:00+0800");
        assert!(matches!(
            v.approved_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                v.approved.controls[1]
            )),
            JournalAction::Key(KeyCode::Char('l'))
        ));
        v.approved_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            v.approved.controls[0],
        ));
        assert!(v.editing);
        assert!(matches!(
            v.approved_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                v.approved.actions[0]
            )),
            JournalAction::Copy
        ));
        assert!(!v.editing);
    }
}
