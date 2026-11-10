//! Approved worker inspection layout, with bounded visible-row rendering.
use super::*;
use crate::tui::design::{self, Scroll};
use ratatui::{
    crossterm::event::{MouseButton, MouseEvent, MouseEventKind},
    text::{Line, Span},
};
use std::sync::Arc;

#[derive(Default)]
pub struct WorkerUi {
    pub editing: bool,
    query: String,
    sample: usize,
    filtered: Vec<usize>,
    cursor: usize,
    focus: usize,
    cached: Option<(Arc<crate::worker_util::Telemetry>, bool)>,
    table: Scroll,
    detail: Scroll,
    body: Scroll,
    controls: [Rect; 3],
    actions: [Rect; 2],
    expanded: bool,
    initialized: bool,
    hidden_rows: usize,
}
pub enum WorkerAction {
    None,
    Copy(String),
}
impl WorkersView {
    pub fn editing(&self) -> bool {
        self.approved.editing
    }
    pub fn clear_areas(&mut self) {
        self.approved.table.clear();
        self.approved.detail.clear();
        self.approved.body.clear();
        self.approved.controls = [Rect::default(); 3];
        self.approved.actions = [Rect::default(); 2];
    }
    fn refresh(&mut self, d: &Data, now: u64) {
        let Some(report) = d.worker_util.as_ref() else {
            self.approved.cached = None;
            self.approved.filtered.clear();
            self.approved.cursor = 0;
            return;
        };
        if !self.approved.initialized {
            self.approved.focus = 3;
            self.approved.initialized = true;
        }
        let live = report.live(now);
        if self
            .approved
            .cached
            .as_ref()
            .is_some_and(|(r, l)| Arc::ptr_eq(r, report) && *l == live)
        {
            return;
        }
        let selected = self
            .approved
            .cached
            .as_ref()
            .and_then(|(r, _)| {
                self.approved
                    .filtered
                    .get(self.approved.cursor)
                    .and_then(|&i| r.workers.get(i))
            })
            .map(|w| w.queue_id);
        self.approved.cached = Some((report.clone(), live));
        let q = &self.approved.query;
        self.approved.filtered = report
            .workers
            .iter()
            .enumerate()
            .filter(|(_, w)| {
                (q.is_empty() || w.queue_id.to_string().contains(q))
                    && (self.approved.sample == 0
                        || self.approved.sample == 1 && live
                        || self.approved.sample == 2 && !live)
            })
            .map(|(i, _)| i)
            .collect();
        self.approved
            .filtered
            .sort_unstable_by_key(|&i| report.workers[i].queue_id);
        self.approved.cursor = selected
            .and_then(|id| {
                self.approved
                    .filtered
                    .iter()
                    .position(|&i| report.workers[i].queue_id == id)
            })
            .unwrap_or(
                self.approved
                    .cursor
                    .min(self.approved.filtered.len().saturating_sub(1)),
            );
    }
    fn worker<'a>(&self, d: &'a Data) -> Option<&'a crate::worker_util::WorkerRow> {
        self.approved
            .filtered
            .get(self.approved.cursor)
            .and_then(|&i| d.worker_util.as_ref()?.workers.get(i))
    }
    fn control(&mut self, i: usize) {
        match i {
            0 => self.approved.editing = true,
            1 => self.approved.sample = (self.approved.sample + 1) % 3,
            _ => {
                self.approved.query.clear();
                self.approved.editing = false;
                self.approved.sample = 0;
            }
        }
        self.approved.cached = None;
    }
    pub fn approved_key(&mut self, k: KeyEvent, d: &Data, now: u64) -> WorkerAction {
        self.refresh(d, now);
        if self.approved.editing {
            match k.code {
                KeyCode::Enter => self.approved.editing = false,
                KeyCode::Esc => {
                    self.approved.editing = false;
                    self.approved.query.clear();
                }
                KeyCode::Backspace => {
                    self.approved.query.pop();
                }
                KeyCode::Char(c) => self.approved.query.push(c),
                _ => return WorkerAction::None,
            };
            self.approved.cached = None;
            self.refresh(d, now);
            return WorkerAction::None;
        }
        let last = self.approved.filtered.len().saturating_sub(1);
        match k.code {
            KeyCode::Char('/') => self.control(0),
            KeyCode::Char('t') => self.control(1),
            KeyCode::Char('x') => self.control(2),
            KeyCode::Tab => self.approved.focus = (self.approved.focus + 1) % 5,
            KeyCode::BackTab => self.approved.focus = (self.approved.focus + 4) % 5,
            KeyCode::Enter if self.approved.focus < 3 => self.control(self.approved.focus),
            KeyCode::Enter | KeyCode::Char('e') => self.approved.expanded = !self.approved.expanded,
            KeyCode::Esc => self.approved.expanded = false,
            KeyCode::Char('c') => {
                if let Some(w) = self.worker(d) {
                    let mut text = format!(
                        "Worker {}\nBusy {:.1}%\nIdle {:.1}%\nBacklog {} packets\nSnapshot {}",
                        w.queue_id,
                        w.busy_ppm as f64 / 10000.0,
                        (1000000 - w.busy_ppm) as f64 / 10000.0,
                        w.backlog_mean.map_or("unknown".into(), |n| n.to_string()),
                        if d.worker_util.as_ref().is_some_and(|t| t.live(now)) {
                            "LIVE"
                        } else {
                            "STALE"
                        }
                    );
                    text.push('\n');
                    text.push_str(
                        &w.extended_measurements(
                            d.worker_util.as_ref().is_some_and(|t| t.live(now)),
                        )
                        .join("\n"),
                    );
                    text.push('\n');
                    text.push_str(
                        &crate::worker_util::queue::Queue::measurements(
                            d.worker_util.as_ref().and_then(|t| t.queue.as_ref()),
                            d.worker_util.as_ref().is_some_and(|t| t.live(now)),
                        )
                        .join("\n"),
                    );
                    if let Some(t) = &d.worker_util {
                        text.push('\n');
                        text.push_str(&t.classification.measurements(t.live(now), now).join("\n"));
                        text.push('\n');
                        text.push_str(&t.health.measurements(now).join("\n"));
                        if let Some(reason) = &t.reason {
                            text.push_str(&format!("\nTelemetry note: {reason}"));
                        }
                    }
                    return WorkerAction::Copy(text);
                }
                let health = d.worker_util.as_ref().map_or_else(
                    || crate::worker_util::health::Health::default().measurements(now),
                    |t| t.health.measurements(now),
                );
                return WorkerAction::Copy(format!(
                    "Worker details unavailable\n{}",
                    health.join("\n")
                ));
            }
            KeyCode::Up if self.approved.focus == 4 => {
                self.approved.detail.offset = self.approved.detail.offset.saturating_sub(1)
            }
            KeyCode::Down if self.approved.focus == 4 => {
                self.approved.detail.offset = self
                    .approved
                    .detail
                    .offset
                    .saturating_add(1)
                    .min(self.approved.detail.max)
            }
            KeyCode::PageUp if self.approved.focus == 4 => {
                self.approved.detail.offset = self
                    .approved
                    .detail
                    .offset
                    .saturating_sub(self.approved.detail.area.height.max(1) as usize)
            }
            KeyCode::PageDown if self.approved.focus == 4 => {
                self.approved.detail.offset = self
                    .approved
                    .detail
                    .offset
                    .saturating_add(self.approved.detail.area.height.max(1) as usize)
                    .min(self.approved.detail.max)
            }
            KeyCode::Home if self.approved.focus == 4 => self.approved.detail.offset = 0,
            KeyCode::End if self.approved.focus == 4 => {
                self.approved.detail.offset = self.approved.detail.max
            }
            KeyCode::Up => self.approved.cursor = self.approved.cursor.saturating_sub(1),
            KeyCode::Down => {
                self.approved.cursor = self.approved.cursor.saturating_add(1).min(last)
            }
            KeyCode::PageUp => self.approved.cursor = self.approved.cursor.saturating_sub(10),
            KeyCode::PageDown => {
                self.approved.cursor = self.approved.cursor.saturating_add(10).min(last)
            }
            KeyCode::Home => self.approved.cursor = 0,
            KeyCode::End => self.approved.cursor = last,
            _ => {}
        }
        self.refresh(d, now);
        WorkerAction::None
    }
    pub fn approved_mouse(&mut self, m: MouseEvent, d: &Data, now: u64) -> WorkerAction {
        if m.kind == MouseEventKind::Down(MouseButton::Left) {
            if let Some(i) = self
                .approved
                .controls
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
            {
                self.control(i);
                self.refresh(d, now);
                return WorkerAction::None;
            }
            if let Some(i) = self
                .approved
                .actions
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
            {
                return self.approved_key(
                    KeyEvent::new(KeyCode::Char(['e', 'c'][i]), m.modifiers),
                    d,
                    now,
                );
            }
            let detail = self.approved.detail.area;
            if detail.contains((m.column, m.row).into())
                && m.column < detail.right().saturating_sub(1)
            {
                self.approved.focus = 4;
                self.approved.editing = false;
            }
            let r = self.approved.table.area;
            if r.contains((m.column, m.row).into()) && m.column < r.right().saturating_sub(1) {
                self.approved.cursor = (self.approved.table.offset
                    + self.approved.hidden_rows
                    + usize::from(m.row - r.y))
                .min(self.approved.filtered.len().saturating_sub(1));
                self.approved.focus = 3;
                self.approved.editing = false;
            }
        }
        if self.approved.table.mouse(m) {
            self.approved.cursor = self
                .approved
                .table
                .offset
                .min(self.approved.filtered.len().saturating_sub(1));
        } else if !self.approved.detail.mouse(m) {
            self.approved.body.mouse(m);
        }
        WorkerAction::None
    }
    pub fn draw_approved(&mut self, f: &mut Frame, area: Rect, d: &Data, now: u64, p: &Palette) {
        self.refresh(d, now);
        self.approved.hidden_rows = 0;
        let height = area.height.max(28);
        self.approved.body.offset = self
            .approved
            .body
            .offset
            .min(height.saturating_sub(area.height) as usize);
        if height > area.height {
            let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(
                area.width.saturating_sub(1),
                height,
            ))
            .expect("in-memory terminal");
            t.draw(|v| self.draw_body(v, v.area(), d, now, p))
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
            self.draw_body(f, area, d, now, p);
        }
        self.approved.body.draw(f, area, height as usize, p);
    }
    fn draw_body(&mut self, f: &mut Frame, area: Rect, d: &Data, now: u64, p: &Palette) {
        f.render_widget(
            ratatui::widgets::Block::default().style(design::surface(p)),
            area,
        );
        let report = d.worker_util.as_deref();
        let live = report.is_some_and(|r| r.live(now));
        let state = if live {
            "LIVE"
        } else if report.is_some_and(|r| !r.workers.is_empty()) {
            "STALE"
        } else {
            "unavailable"
        };
        let age = report
            .filter(|r| r.updated > 0 && r.updated <= now)
            .map_or("unknown".into(), |r| format!("{}s", now - r.updated));
        let [cards, tools, body, pressure, status] = Layout::vertical([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(11),
            Constraint::Length(6),
            Constraint::Length(4),
        ])
        .areas(area);
        let busiest = report.and_then(|r| r.workers.iter().max_by_key(|w| w.busy_ppm));
        let regions = Layout::horizontal([Constraint::Ratio(1, 3); 3]).split(cards);
        let values = [
            (
                "Workers",
                report.map_or("unknown".into(), |r| r.workers.len().to_string()),
                "reported by bridge".into(),
            ),
            (
                "Busiest worker",
                busiest.map_or("unknown".into(), |w| {
                    format!("{:.1}%", w.busy_ppm as f64 / 10000.0)
                }),
                busiest.map_or("no worker evidence".into(), |w| {
                    format!("worker {}", w.queue_id)
                }),
            ),
            ("Telemetry", state.into(), format!("snapshot age {age}")),
        ];
        for (i, (title, value, note)) in values.into_iter().enumerate() {
            f.render_widget(
                Paragraph::new(vec![
                    Line::styled(
                        value,
                        design::role(p, if live { Role::Ok } else { Role::Warning }),
                    ),
                    Line::styled(note, design::role(p, Role::Secondary)),
                ])
                .block(design::panel(title, p)),
                regions[i],
            );
        }
        let regions = Layout::horizontal([
            Constraint::Fill(3),
            Constraint::Fill(2),
            Constraint::Length(9),
        ])
        .split(tools);
        for (i, label) in [
            format!(
                "Worker ID: {}{}",
                self.approved.query,
                if self.approved.editing { "_" } else { "" }
            ),
            format!(
                "Sample [{}]",
                ["All", "Live", "Stale"][self.approved.sample]
            ),
            "Clear".into(),
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
        let [table, detail] = if self.approved.expanded {
            [Rect::default(), body]
        } else if body.width < 100 {
            Layout::vertical([Constraint::Percentage(50), Constraint::Min(0)]).areas(body)
        } else {
            Layout::horizontal([Constraint::Percentage(68), Constraint::Percentage(32)]).areas(body)
        };
        if table.height > 0 {
            let block = design::panel("Worker utilization — busy excludes empty polling", p);
            let inner = block.inner(table);
            f.render_widget(block, table);
            let [header, rows, footer] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .areas(inner);
            f.render_widget(
                Paragraph::new("Worker      Busy        Idle        Mean backlog     Sample")
                    .style(design::accent(p)),
                header,
            );
            let h = rows.height as usize;
            if self.approved.cursor < self.approved.table.offset {
                self.approved.table.offset = self.approved.cursor;
            } else if h > 0 && self.approved.cursor >= self.approved.table.offset + h {
                self.approved.table.offset = self.approved.cursor + 1 - h;
            }
            if let Some(report) = report {
                for (y, &i) in self
                    .approved
                    .filtered
                    .iter()
                    .skip(self.approved.table.offset)
                    .take(h)
                    .enumerate()
                {
                    let w = &report.workers[i];
                    f.render_widget(
                        Paragraph::new(format!(
                            "{:<11} {:<11} {:<11} {:<16} {state}",
                            w.queue_id,
                            format!("{:.1}%", w.busy_ppm as f64 / 10000.0),
                            format!("{:.1}%", (1000000 - w.busy_ppm) as f64 / 10000.0),
                            w.backlog_mean
                                .map_or("unknown".into(), |n| format!("{n} packets"))
                        ))
                        .style(
                            if self.approved.table.offset + y == self.approved.cursor {
                                design::selected(p)
                            } else {
                                design::surface(p)
                            },
                        ),
                        Rect::new(rows.x, rows.y + y as u16, rows.width, 1),
                    );
                }
            }
            self.approved
                .table
                .draw(f, rows, self.approved.filtered.len(), p);
            f.render_widget(
                Paragraph::new(format!(
                    "Workers {}-{} / {}",
                    self.approved.table.offset + usize::from(!self.approved.filtered.is_empty()),
                    (self.approved.table.offset + h).min(self.approved.filtered.len()),
                    self.approved.filtered.len()
                )),
                footer,
            );
        }
        let title = self.worker(d).map_or("Worker details".into(), |w| {
            format!("Worker details | ID {} | {state}", w.queue_id)
        });
        let block = design::panel(title, p);
        let inner = block.inner(detail);
        f.render_widget(block, detail);
        let [fields, actions] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).areas(inner);
        let mut lines = Vec::new();
        if let Some(w) = self.worker(d) {
            lines.push(Line::raw(format!("Worker {}", w.queue_id)));
            for (label, ppm) in [("Busy", w.busy_ppm), ("Idle", 1000000 - w.busy_ppm)] {
                let n = (ppm / 50000) as usize;
                lines.push(Line::from(vec![
                    Span::raw(format!("{label} {:5.1}% ", ppm as f64 / 10000.0)),
                    Span::styled("█".repeat(n), design::role(p, Role::Ok)),
                    Span::styled("░".repeat(20 - n), design::role(p, Role::Secondary)),
                ]));
            }
            lines.extend([
                Line::raw(format!(
                    "Backlog {} packets",
                    w.backlog_mean.map_or("unknown".into(), |n| n.to_string())
                )),
                Line::raw(format!("Sample age {age} | {state}")),
            ]);
            lines.extend(w.extended_measurements(live).into_iter().map(Line::raw));
        } else {
            lines.push(Line::raw("Worker details unavailable"));
        }
        lines.extend(
            report
                .map_or_else(
                    || crate::worker_util::health::Health::default().measurements(now),
                    |r| r.health.measurements(now),
                )
                .into_iter()
                .map(Line::raw),
        );
        lines.extend(
            crate::worker_util::queue::Queue::measurements(
                report.and_then(|r| r.queue.as_ref()),
                live,
            )
            .into_iter()
            .map(Line::raw),
        );
        if let Some(t) = report {
            lines.extend(
                t.classification
                    .measurements(live, now)
                    .into_iter()
                    .map(Line::raw),
            );
        }
        lines.extend([
            Line::raw("New = increments per valid sample."),
            Line::raw("Source Analytics snapshot | read-only"),
            Line::raw("Busy is forwarding utilization, not OS CPU usage."),
        ]);
        let content = Rect {
            width: fields.width.saturating_sub(1),
            ..fields
        };
        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        let total = paragraph.line_count(content.width);
        self.approved.detail.offset = self
            .approved
            .detail
            .offset
            .min(total.saturating_sub(content.height as usize));
        f.render_widget(
            paragraph.scroll((self.approved.detail.offset as u16, 0)),
            content,
        );
        self.approved.detail.draw(f, fields, total, p);
        for (i, label) in ["[e/Enter] Expand details", "[c] Copy details"]
            .into_iter()
            .enumerate()
        {
            let r = Rect::new(actions.x, actions.y + i as u16, actions.width, 1);
            self.approved.actions[i] = r;
            f.render_widget(Paragraph::new(label).style(design::accent(p)), r);
        }
        let pressure_report = report.and_then(|r| r.pressure.as_ref());
        let alerts = pressure_report.map_or_else(Vec::new, |v| v.alerts());
        let pressure_state = if !live {
            "unavailable (stale telemetry)".into()
        } else if !alerts.is_empty() {
            format!("ALERT: {}", alerts.join(", "))
        } else if pressure_report.is_some_and(|v| v.complete()) {
            "CLEAR".into()
        } else {
            "unavailable (diagnostics or baseline missing)".into()
        };
        let mut measurements = vec![Line::styled(
            pressure_state,
            design::role(
                p,
                if live && alerts.is_empty() && pressure_report.is_some_and(|v| v.complete()) {
                    Role::Ok
                } else {
                    Role::Warning
                },
            ),
        )];
        if let Some(v) = pressure_report {
            measurements.extend(v.measurements().into_iter().map(Line::raw));
        }
        measurements.push(Line::styled(
            "New events are increments since the previous valid sample.",
            design::role(p, Role::Secondary),
        ));
        f.render_widget(
            Paragraph::new(measurements).block(design::panel("Buffer/ring pressure", p)),
            pressure,
        );
        f.render_widget(
            Paragraph::new(vec![
                Line::raw(format!(
                    "Snapshot {state} | age {age} | Source: LibreQoS DPDK analytics"
                )),
                Line::raw(
                    report
                        .and_then(|r| r.reason.as_deref())
                        .unwrap_or("Worker and queue configuration is unchanged"),
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
    use ratatui::crossterm::event::KeyModifiers;
    #[test]
    fn bridge_health_copy_uses_independent_health_freshness_and_preserves_source_status() {
        let mut d = super::super::tests::live();
        let h = &mut Arc::make_mut(d.worker_util.as_mut().unwrap()).health;
        let report = |up| {
            serde_json::from_value(serde_json::json!({"status":"degraded","version":"v3","uptime_ms":up,
            "workers":[],"degraded_subsystems":["stats_collector"],"classification_empty":true,"topology_tier":"last_known_good"})).unwrap()
        };
        h.record(Ok(report(1)), 90);
        h.record(Ok(report(10001)), 100);
        let mut view = WorkersView::default();
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        let WorkerAction::Copy(text) = view.approved_key(key, &d, 100) else {
            panic!("copy")
        };
        assert!(
            text.contains("Bridge health LIVE")
                && text.contains("Status: degraded")
                && text.contains("stats_collector")
        );
        let WorkerAction::Copy(text) = view.approved_key(key, &d, 107) else {
            panic!("copy")
        };
        assert!(text.contains("Bridge health LIVE") && text.contains("Worker metrics unavailable"));
        let WorkerAction::Copy(text) = view.approved_key(key, &d, 116) else {
            panic!("copy")
        };
        assert!(
            text.contains("Bridge health STALE (last known)") && text.contains("stats_collector")
        );
        Arc::make_mut(d.worker_util.as_mut().unwrap())
            .workers
            .clear();
        let WorkerAction::Copy(text) = view.approved_key(key, &d, 100) else {
            panic!("health copy without workers")
        };
        assert!(
            text.contains("Worker details unavailable")
                && text.contains("Bridge health LIVE")
                && text.contains("stats_collector")
        );
    }

    #[test]
    fn worker_detail_mouse_focus_and_page_keys_scroll_without_changing_worker() {
        let d = super::super::tests::live();
        let mut view = WorkersView::default();
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 30)).unwrap();
        term.draw(|f| {
            view.draw_approved(
                f,
                f.area(),
                &d,
                100,
                &Palette::new(crate::config::Theme::Dark),
            )
        })
        .unwrap();
        assert!(view.approved.detail.max > 0);
        let r = view.approved.detail.area;
        view.approved_mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: r.x,
                row: r.y,
                modifiers: KeyModifiers::NONE,
            },
            &d,
            100,
        );
        assert_eq!(view.approved.focus, 4);
        let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
        view.approved_key(key(KeyCode::End), &d, 100);
        assert_eq!(view.approved.detail.offset, view.approved.detail.max);
        assert_eq!(view.approved.cursor, 0);
        view.approved_key(key(KeyCode::PageUp), &d, 100);
        assert!(view.approved.detail.offset < view.approved.detail.max);
        view.approved_key(key(KeyCode::Home), &d, 100);
        assert_eq!(view.approved.detail.offset, 0);
        view.approved_key(key(KeyCode::PageDown), &d, 100);
        assert!(view.approved.detail.offset > 0);
    }

    #[test]
    fn extended_worker_details_and_copy_keep_units_phase_names_and_stale_labels() {
        let mut d = super::super::tests::live();
        Arc::make_mut(d.worker_util.as_mut().unwrap()).workers[0] =
            serde_json::from_value(serde_json::json!({
                "queue_id":0,"busy_ppm":125000,"backlog_mean":3,"idle_ppm":875000,
                "laps_per_s":100000,"busy_laps":42,"cycles_per_busy_lap":250,
                "phase_cycles":[1,2,3,4,5,6,7,8,9,10]
            }))
            .unwrap();
        let mut view = WorkersView::default();
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        let WorkerAction::Copy(text) = view.approved_key(key, &d, 100) else {
            panic!("expected copy")
        };
        for expected in [
            "Reported idle: 87.5%",
            "Laps/s (reported): 100000",
            "Busy laps (reported): 42",
            "Cycles/busy lap: 250",
            "Phase TSC cycles (cumulative)",
            "empty_rx: 10",
            "not packets/s",
        ] {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
        let WorkerAction::Copy(text) = view.approved_key(key, &d, 107) else {
            panic!("expected copy")
        };
        assert!(
            text.contains("Worker metrics STALE (last known)") && text.contains("empty_rx: 10")
        );
    }

    #[test]
    fn classified_copy_preserves_scope_and_current_data_age() {
        let mut d = super::super::tests::live();
        Arc::make_mut(d.worker_util.as_mut().unwrap()).classification = serde_json::from_value(serde_json::json!({
            "mapped":{"bytes":{"download_bytes":1000,"upload_bytes":500},"history":[{"at":100,"down_mbps":1.0,"up_mbps":0.5}]}
        })).unwrap();
        let mut view = WorkersView::default();
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        let WorkerAction::Copy(text) = view.approved_key(key, &d, 100) else {
            panic!("expected copy")
        };
        assert!(
            text.contains("Classification | whole bridge | enqueued")
                && text.contains("Mapped LIVE: Down 1.000 Mbps")
                && text.contains("Unmapped unavailable"),
            "{text}"
        );
        let WorkerAction::Copy(text) = view.approved_key(key, &d, 107) else {
            panic!("expected copy")
        };
        assert!(
            text.contains("Mapped STALE (last known): Down unknown")
                && text.contains("Mapped bytes D/U: 1000 / 500")
                && !text.contains("1.000 Mbps"),
            "{text}"
        );
    }

    #[test]
    fn queue_copy_preserves_scope_and_suppresses_stale_increments() {
        let mut d = super::super::tests::live();
        Arc::make_mut(d.worker_util.as_mut().unwrap()).queue = Some(serde_json::from_value(serde_json::json!({
            "totals":{"drop_pkts":10,"aqm_marks":20,"backlog_bytes":30},"drop_delta":1,"aqm_delta":2
        })).unwrap());
        let mut view = WorkersView::default();
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        let WorkerAction::Copy(text) = view.approved_key(key, &d, 100) else {
            panic!("expected copy")
        };
        assert!(
            text.contains("whole bridge")
                && text.contains("Drops: 10")
                && text.contains("New/sample: drops 1"),
            "{text}"
        );
        let WorkerAction::Copy(text) = view.approved_key(key, &d, 107) else {
            panic!("expected copy")
        };
        assert!(
            text.contains("STALE (last known)") && text.contains("New/sample: drops unknown"),
            "{text}"
        );
    }

    #[test]
    fn sample_filters_expire_on_ticks_and_copy_only_selected_worker_evidence() {
        let d = super::super::tests::live();
        let mut view = WorkersView::default();
        let key = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        view.refresh(&d, 100);
        assert_eq!(view.approved.filtered.len(), 1);
        assert!(
            matches!(view.approved_key(key('c'),&d,100),WorkerAction::Copy(s) if s.contains("12.5%")&&s.contains("LIVE"))
        );
        view.approved_key(key('t'), &d, 100);
        assert_eq!(view.approved.filtered.len(), 1);
        view.refresh(&d, 107);
        assert!(
            view.approved.filtered.is_empty(),
            "live-only filter must expire without a loader event"
        );
        view.approved_key(key('t'), &d, 107);
        assert_eq!(view.approved.filtered.len(), 1);
        assert!(
            matches!(view.approved_key(key('c'),&d,107),WorkerAction::Copy(s) if s.contains("STALE"))
        );
        view.approved_key(key('x'), &d, 107);
        view.approved_key(key('/'), &d, 107);
        view.approved_key(key('9'), &d, 107);
        assert!(view.approved.filtered.is_empty());
    }
}
