//! Approved shaping-input screen. File preview and sync retain separate action paths.
use super::{
    data::Data,
    design::{self, Scroll},
    theme::{Palette, Role, clip},
};
use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::Paragraph,
};
use serde_json::Value;
use std::sync::Arc;

#[derive(Default)]
pub struct LibreView {
    file: usize,
    preview: Scroll,
    body: Scroll,
    horizontal: u16,
    cursor: usize,
    expanded: bool,
    hidden_rows: usize,
    controls: [Rect; 2],
    tabs: [Rect; 2],
    pages: [Rect; 2],
    issue_button: Rect,
    tree: Option<Arc<Value>>,
    json_lines: Vec<Line<'static>>,
    native: Option<Arc<Value>>,
    native_lines: Vec<Line<'static>>,
    status: Option<Arc<Value>>,
    issue: Option<(u64, Vec<String>)>,
    issue_details: bool,
}
pub enum LibreAction {
    None,
    Sync,
}
impl LibreView {
    pub fn clear_area(&mut self) {
        self.controls = [Rect::default(); 2];
        self.tabs = [Rect::default(); 2];
        self.pages = [Rect::default(); 2];
        self.issue_button = Rect::default();
        self.preview.clear();
        self.body.clear();
    }
    pub fn on_data(&mut self, d: &Data) {
        let changed = match (&self.native, &d.native_input) {
            (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
            (None, None) => false,
            _ => true,
        };
        if changed {
            self.native = d.native_input.clone();
            self.native_lines = d
                .native_input
                .as_ref()
                .map(|v| {
                    serde_json::to_string_pretty(&**v)
                        .unwrap_or_default()
                        .lines()
                        .map(|s| Line::raw(s.to_owned()))
                        .collect()
                })
                .unwrap_or_default();
        }

        if self.tree.as_ref().is_none_or(|t| !Arc::ptr_eq(t, &d.tree)) {
            self.tree = Some(d.tree.clone());
            self.json_lines = serde_json::to_string_pretty(&*d.tree)
                .unwrap_or_default()
                .lines()
                .map(|s| Line::raw(s.to_owned()))
                .collect();
        }
        if match (&self.status, &d.status) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        } {
            return;
        }
        self.status = d.status.clone();
        let Some(st) = d.status.as_deref() else {
            return;
        };
        if st["target"] != "v3_network" {
            return;
        }
        let at = st["updated"].as_u64().unwrap_or(0);
        if let Some(alerts) = st["report"]["alerts"].as_array() {
            let issues: Vec<_> = alerts
                .iter()
                .filter_map(|v| v.as_str())
                .filter(|s| s.starts_with("v3_network: circuit ") && s.contains("parent node"))
                .map(str::to_owned)
                .collect();
            if !issues.is_empty() {
                self.issue = Some((at, issues));
            } else if d.errors.is_empty()
                && st["error"].as_str().is_none_or(str::is_empty)
                && st["native"] == "ready"
                && self.issue.as_ref().is_some_and(|(old, _)| at > *old)
            {
                self.issue = None;
                self.issue_details = false;
            }
        }
    }
    pub fn key(&mut self, k: KeyEvent, d: &Data) -> LibreAction {
        if self.file == 0 && d.target != "v3_network" && !self.issue_details {
            let last = d.rows.len().saturating_sub(1);
            match k.code {
                KeyCode::Up => self.cursor = self.cursor.saturating_sub(1),
                KeyCode::Down => self.cursor = self.cursor.saturating_add(1).min(last),
                KeyCode::PageUp => self.cursor = self.cursor.saturating_sub(10),
                KeyCode::PageDown => self.cursor = self.cursor.saturating_add(10).min(last),
                KeyCode::Home => self.cursor = 0,
                KeyCode::End => self.cursor = last,
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
                return LibreAction::None;
            }
        }

        match k.code {
            KeyCode::Char('s') => return LibreAction::Sync,
            KeyCode::Char('p') => self.expanded = !self.expanded,
            KeyCode::Char('i') if d.target == "v3_network" && self.issue.is_some() => {
                self.issue_details = !self.issue_details;
                self.preview.offset = 0;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.file = 1 - self.file;
                self.preview.offset = 0;
                self.cursor = 0;
            }
            KeyCode::Esc => {
                self.expanded = false;
                self.issue_details = false;
            }
            KeyCode::Left => self.horizontal = self.horizontal.saturating_sub(8),
            KeyCode::Right => self.horizontal = self.horizontal.saturating_add(8).min(300),
            KeyCode::Down => {
                self.preview.offset = self.preview.offset.saturating_add(1).min(self.preview.max)
            }
            KeyCode::Up => self.preview.offset = self.preview.offset.saturating_sub(1),
            KeyCode::PageDown => {
                self.preview.offset = self.preview.offset.saturating_add(10).min(self.preview.max)
            }
            KeyCode::PageUp => self.preview.offset = self.preview.offset.saturating_sub(10),
            KeyCode::Home => self.preview.offset = 0,
            KeyCode::End => self.preview.offset = self.preview.max,
            _ => {}
        }
        LibreAction::None
    }
    pub fn mouse(&mut self, m: MouseEvent, d: &Data) -> LibreAction {
        if m.kind == MouseEventKind::Down(MouseButton::Left) {
            if self.controls[1].contains((m.column, m.row).into()) {
                return LibreAction::Sync;
            }
            if self.controls[0].contains((m.column, m.row).into()) {
                self.expanded = !self.expanded;
            }
            if self.issue_button.contains((m.column, m.row).into()) {
                self.issue_details = !self.issue_details;
                self.preview.offset = 0;
            }
            if let Some(i) = self
                .tabs
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
            {
                self.file = i;
                self.preview.offset = 0;
                self.cursor = 0;
                self.issue_details = false;
            }
            if let Some(i) = self
                .pages
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
            {
                return self.key(
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
            }
            if self.preview.area.contains((m.column, m.row).into())
                && m.column < self.preview.area.right().saturating_sub(1)
            {
                self.cursor = self.preview.offset
                    + self.hidden_rows
                    + usize::from(m.row - self.preview.area.y);
            }
        }
        if self.preview.mouse(m) {
            self.cursor = self.preview.offset;
        } else {
            self.body.mouse(m);
        }
        LibreAction::None
    }
    pub fn draw(&mut self, f: &mut Frame, area: Rect, d: &Data, now: u64, p: &Palette) {
        // Only the compact composition needs a bounded virtual canvas.
        self.hidden_rows = 0;
        let height = area.height.max(29);
        self.body.offset = self
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
                        b[(x, y + self.body.offset as u16)].clone();
                }
            }
            let map = |r: Rect| design::viewport_rect(r, area, self.body.offset);
            self.controls = self.controls.map(map);
            self.tabs = self.tabs.map(map);
            self.pages = self.pages.map(map);
            self.issue_button = map(self.issue_button);
            self.hidden_rows = self
                .body
                .offset
                .saturating_sub(self.preview.area.y as usize);
            self.preview.area = map(self.preview.area);
        } else {
            self.draw_body(f, area, d, now, p);
        }
        self.body.draw(f, area, height as usize, p);
    }
    fn draw_body(&mut self, f: &mut Frame, area: Rect, d: &Data, now: u64, p: &Palette) {
        f.render_widget(
            ratatui::widgets::Block::default().style(design::surface(p)),
            area,
        );
        let [toolbar, notice, cards, preview, status] = if self.expanded {
            [
                Rect::default(),
                Rect::default(),
                Rect::default(),
                Rect::new(area.x, area.y, area.width, area.height.saturating_sub(4)),
                Rect::new(area.x, area.bottom().saturating_sub(4), area.width, 4),
            ]
        } else {
            Layout::vertical([
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Length(8),
                Constraint::Min(10),
                Constraint::Length(4),
            ])
            .areas(area)
        };
        let st = d.status.as_deref();
        let text = |key: &str| {
            st.and_then(|s| s[key].as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("unknown")
        };
        if !self.expanded {
            f.render_widget(
                Paragraph::new("").block(design::panel("Shaping input", p)),
                toolbar,
            );
            self.controls = [
                Rect::new(toolbar.right().saturating_sub(26), toolbar.y + 1, 12, 1),
                Rect::new(toolbar.right().saturating_sub(13), toolbar.y + 1, 12, 1),
            ];
            for (i, label) in ["[p Preview]", "[s Sync now]"].into_iter().enumerate() {
                f.render_widget(
                    Paragraph::new(label).style(design::accent(p)),
                    self.controls[i],
                );
            }
            let native =
                d.target == "v3_network" || st.is_some_and(|s| s["target"] == "v3_network");
            let message = if native {
                self.issue.as_ref().map(|(at,rows)|format!("Missing parents: {} circuits | rows unknown | observed {} UTC | [i Details]",rows.len(),super::dashboard::fmt_utc(*at)))
            } else {
                None
            };
            f.render_widget(
                Paragraph::new(vec![
                    Line::styled(
                        format!("Native compatibility: {}", text("native")),
                        design::role(
                            p,
                            if text("native") == "ready" {
                                Role::Ok
                            } else {
                                Role::Warning
                            },
                        ),
                    ),
                    Line::raw(message.unwrap_or_else(|| {
                        format!(
                            "Active target {} | Native probe does not switch active sources",
                            text("target")
                        )
                    })),
                ])
                .block(design::panel("", p)),
                notice,
            );
            if native && self.issue.is_some() {
                self.issue_button = notice;
            }
            let [last, remote] =
                Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)])
                    .areas(cards);
            let at = st
                .and_then(|s| s["updated"].as_u64())
                .map_or("unknown".into(), super::shell::local_clock);
            let reload = text("reload");
            let input = if st.is_some_and(|s| s["error"].as_str().is_some_and(|e| !e.is_empty())) {
                "FAILED"
            } else if text("mode") == "shadow" {
                "CANDIDATE"
            } else if reload == "imported by LibreQoS" {
                "IMPORTED"
            } else if reload.starts_with("waiting for the LibreQoS import") {
                "IMPORT PENDING"
            } else if st.is_some_and(|s| s["error"].as_str().is_some_and(|e| !e.is_empty())) {
                "FAILED"
            } else {
                "LAST RESULT (see reload)"
            };
            f.render_widget(
                Paragraph::new(vec![
                    Line::raw(format!("Target {} | Mode {}", text("target"), text("mode"))),
                    Line::raw(format!("Last cycle {at}")),
                    Line::styled(format!("Input state {input}"), design::role(p, Role::Base)),
                    Line::raw(format!("Reload {reload}")),
                    Line::raw(format!("Rows {} | Imported time unknown", d.rows.len())),
                    Line::raw(format!(
                        "Written {}",
                        st.and_then(|s| s["files_changed"].as_bool())
                            .map_or("unknown", |b| if b {
                                "changed this cycle"
                            } else {
                                "unchanged this cycle"
                            })
                    )),
                ])
                .block(design::panel("Last sync", p)),
                last,
            );
            f.render_widget(
                Paragraph::new(vec![
                    Line::raw("Status  NOT CONFIGURED"),
                    Line::raw("Destination --"),
                    Line::raw("Last transfer --"),
                    Line::raw(""),
                    Line::raw("No remote integration configured"),
                ])
                .block(design::panel("Remote sync", p)),
                remote,
            );
        }
        if self.expanded {
            self.controls[0] = Rect::new(preview.right().saturating_sub(13), preview.y, 12, 1);
            f.render_widget(
                Paragraph::new("[p Back]").style(design::accent(p)),
                self.controls[0],
            );
        }
        let block = design::panel("Source preview — read-only", p);
        let inner = block.inner(preview);
        f.render_widget(block, preview);
        if self.expanded {
            f.render_widget(
                Paragraph::new("[p Back]").style(design::accent(p)),
                self.controls[0],
            );
        }
        let [tabs, header, rows, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(inner);
        let native = d.target == "v3_network";
        let labels = if native {
            [
                if d.native_file.is_empty() {
                    "libreqos_network.json"
                } else {
                    &d.native_file
                },
                "Compatibility tree",
            ]
        } else {
            ["ShapedDevices.csv", "network.json"]
        };
        for (i, label) in labels.into_iter().enumerate() {
            let x = tabs.x + i as u16 * 28;
            let r = Rect::new(x, tabs.y, 27.min(tabs.right().saturating_sub(x)), 1);
            self.tabs[i] = r;
            f.render_widget(
                Paragraph::new(format!("[{label}]")).style(if i == self.file {
                    design::accent(p)
                } else {
                    design::surface(p)
                }),
                r,
            );
        }
        let json = self.file == 1 || native;
        let json_lines = if native && self.file == 0 {
            &self.native_lines
        } else {
            &self.json_lines
        };
        let count = if self.issue_details {
            self.issue.as_ref().map_or(0, |(_, v)| v.len())
        } else if json {
            json_lines.len()
        } else {
            d.rows.len()
        };
        if !json && !self.issue_details {
            if self.cursor < self.preview.offset {
                self.preview.offset = self.cursor;
            } else if rows.height > 0 && self.cursor >= self.preview.offset + rows.height as usize {
                self.preview.offset = self.cursor + 1 - rows.height as usize;
            }
        }
        self.preview.offset = self
            .preview
            .offset
            .min(count.saturating_sub(rows.height as usize));
        f.render_widget(Paragraph::new(if self.issue_details{"Native missing-parent details"}else if json {if native&&self.file==0 {"Original native input (unavailable if no lines)"}else{"Compatibility tree derived from loaded input"}}else{"Circuit ID          Name                  Parent              IPv4             Down/Up Mbps"}).style(design::accent(p)).scroll((0,self.horizontal)),header);
        for y in 0..rows.height as usize {
            let i = self.preview.offset + y;
            if i >= count {
                break;
            }
            let line = if self.issue_details {
                Line::raw(self.issue.as_ref().unwrap().1[i].clone())
            } else if json {
                json_lines[i].clone()
            } else {
                let r = &d.rows[i];
                Line::raw(format!(
                    "{:<19} {:<21} {:<19} {:<16} {}/{}",
                    clip(&r.circuit_id, 19),
                    clip(&r.circuit_name, 21),
                    clip(&r.parent_node, 19),
                    clip(&r.ipv4, 16),
                    r.download_max,
                    r.upload_max
                ))
            };
            f.render_widget(
                Paragraph::new(line).scroll((0, self.horizontal)).style(
                    if !json && i == self.cursor {
                        design::selected(p)
                    } else {
                        design::surface(p)
                    },
                ),
                Rect::new(rows.x, rows.y + y as u16, rows.width, 1),
            );
        }
        self.preview.draw(f, rows, count, p);
        f.render_widget(
            Paragraph::new(format!(
                "Showing {}-{} / {}",
                self.preview.offset + usize::from(count > 0),
                (self.preview.offset + rows.height as usize).min(count),
                count
            )),
            footer,
        );
        if !json && !self.issue_details && footer.width > 80 {
            let id = d
                .rows
                .get(self.cursor)
                .map_or("none", |r| r.circuit_id.as_str());
            f.render_widget(
                Paragraph::new(format!("Selected: {}", clip(id, 20))),
                Rect::new(footer.right() - 30, footer.y, 30, 1),
            );
        }
        self.pages = [
            Rect::new(footer.x + footer.width / 2, footer.y, 6, 1),
            Rect::new(footer.x + footer.width / 2 + 7, footer.y, 6, 1),
        ];
        for (i, label) in ["[Prev]", "[Next]"].into_iter().enumerate() {
            f.render_widget(
                Paragraph::new(label).style(design::accent(p)),
                self.pages[i],
            );
        }
        let age = st
            .and_then(|s| s["updated"].as_u64())
            .filter(|&at| at > 0 && at <= now)
            .map_or("unknown".into(), |t| format!("{}s", now - t));
        f.render_widget(
            Paragraph::new(vec![
                Line::raw(format!(
                    "Cycle age {age} | Source: cached shaping input | {}",
                    if d.errors.is_empty() && !d.target.is_empty() {
                        "loaded"
                    } else {
                        "read errors; retained data"
                    }
                )),
                Line::raw("Preview is read-only. Sync now requires confirmation."),
            ])
            .block(design::panel("Data status", p)),
            status,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_missing_parent_evidence_survives_errors_until_new_valid_recovery() {
        let mut view = LibreView::default();
        let report = |at, alerts| {
            Arc::new(
                serde_json::json!({"updated":at,"target":"v3_network","native":"ready","report":{"alerts":alerts}}),
            )
        };
        let mut d = Data {
            target: "v3_network".into(),
            status: Some(report(
                10,
                vec![
                    "v3_network: circuit A is not written: it has no parent node, and LibreQoS v3 requires one",
                ],
            )),
            native_input: Some(Arc::new(serde_json::json!({"circuits":[],"topology":[]}))),
            ..Default::default()
        };
        view.on_data(&d);
        assert_eq!(view.issue.as_ref().unwrap().1.len(), 1);
        assert!(!view.native_lines.is_empty());
        d.errors.push("cannot read input".into());
        d.status = Some(report(11, vec![]));
        view.on_data(&d);
        assert!(view.issue.is_some());
        d.errors.clear();
        d.status = Some(report(12, vec![]));
        view.on_data(&d);
        assert!(view.issue.is_none());
    }
}
