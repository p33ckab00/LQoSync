//! The Journal tab: `journalctl -f` of the service, with a level filter, search, and line selection.

use crate::tui::theme::Palette;
use ratatui::Frame;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalMsg {
    Line(String),
    /// A start error, a stderr line, or the end of the child.
    Error(String),
}

/// Kills the child at drop.
pub struct JournalHandle {
    child: Option<Child>,
}

impl Drop for JournalHandle {
    fn drop(&mut self) {
        if let Some(c) = &mut self.child {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

/// Start `<program> -u <unit> -n <lines> -o short-iso --no-pager -f`. The arguments are a list, not a shell text.
pub fn spawn_journal(
    program: &str,
    unit: &str,
    lines: usize,
    tx: Sender<JournalMsg>,
) -> JournalHandle {
    let child = Command::new(program)
        .args([
            "-u",
            unit,
            "-n",
            &lines.to_string(),
            "-o",
            "short-iso",
            "--no-pager",
            "-f",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        // journalctl prints the group hint on stderr. An inherited stderr would draw over the screen.
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(JournalMsg::Error(format!("cannot start {program}: {e}")));
            return JournalHandle { child: None };
        }
    };
    if let Some(err) = child.stderr.take() {
        let tx = tx.clone();
        let _ = std::thread::Builder::new()
            .name("tui-journal-err".into())
            .spawn(move || {
                for l in BufReader::new(err).lines().map_while(Result::ok) {
                    if tx.send(JournalMsg::Error(l)).is_err() {
                        return;
                    }
                }
            });
    }
    if let Some(out) = child.stdout.take() {
        let program = program.to_string();
        let _ = std::thread::Builder::new()
            .name("tui-journal".into())
            .spawn(move || {
                for l in BufReader::new(out).lines().map_while(Result::ok) {
                    if tx.send(JournalMsg::Line(l)).is_err() {
                        return;
                    }
                }
                let _ = tx.send(JournalMsg::Error(format!("{program} stopped")));
            });
    }
    JournalHandle { child: Some(child) }
}

/// 0 info, 1 warning, 2 error. The words of `main.rs` and of the service log.
fn level_of(l: &str) -> u8 {
    if l.contains("error:") || l.contains("ERROR") {
        2
    } else if l.contains("warning:") || l.contains("ALERT") {
        1
    } else {
        0
    }
}

pub struct JournalView {
    /// Level and text of each line, at most `max`.
    pub lines: VecDeque<(u8, String)>,
    max: usize,
    /// The lowest level shown: 0 all, 1 warning, 2 error.
    pub level: u8,
    pub query: String,
    pub editing: bool,
    /// Show the last line as new lines come.
    pub follow: bool,
    /// Indexes into `lines` after the filter.
    pub view: Vec<usize>,
    /// The cursor and the anchor of the selection, as positions in `view`.
    cursor: usize,
    anchor: Option<usize>,
    offset: usize,
    pub error: Option<String>,
    /// The line area of the last draw, for the mouse.
    area: Rect,
}

impl JournalView {
    pub fn new(max: usize) -> Self {
        JournalView {
            lines: VecDeque::with_capacity(max.min(100_000)),
            max: max.max(1),
            level: 0,
            query: String::new(),
            editing: false,
            follow: true,
            view: Vec::new(),
            cursor: 0,
            anchor: None,
            offset: 0,
            error: None,
            area: Rect::default(),
        }
    }

    /// Add the messages of one receive, then filter one time.
    pub fn push_batch(&mut self, msgs: Vec<JournalMsg>) {
        if msgs.is_empty() {
            return;
        }
        let mut dropped = 0;
        for m in msgs {
            match m {
                JournalMsg::Line(l) => {
                    if self.lines.len() == self.max {
                        self.lines.pop_front();
                        dropped += 1;
                    }
                    self.lines.push_back((level_of(&l), l));
                }
                JournalMsg::Error(e) => self.error = Some(e),
            }
        }
        let keep = (!self.follow)
            .then(|| self.view.get(self.cursor).copied())
            .flatten();
        self.refilter();
        // A paused view keeps the same line, also when old lines go out.
        if let Some(i) = keep {
            self.cursor = self
                .view
                .iter()
                .position(|&v| v + dropped >= i)
                .unwrap_or(0);
        }
    }

    fn refilter(&mut self) {
        let q = self.query.to_lowercase();
        self.view = self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, (lv, l))| {
                *lv >= self.level && (q.is_empty() || l.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .collect();
        if self.follow {
            self.cursor = self.view.len().saturating_sub(1);
            self.anchor = None;
        } else {
            self.cursor = self.cursor.min(self.view.len().saturating_sub(1));
        }
    }

    /// The selected lines, or the line under the cursor.
    pub fn selection_text(&self) -> String {
        let (a, b) = match self.anchor {
            Some(a) => (a.min(self.cursor), a.max(self.cursor)),
            None => (self.cursor, self.cursor),
        };
        self.view
            .get(a..=b.min(self.view.len().saturating_sub(1)))
            .unwrap_or_default()
            .iter()
            .map(|&i| self.lines[i].1.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn on_key(&mut self, k: KeyEvent) {
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
                _ => return,
            }
            self.refilter();
            return;
        }
        let last = self.view.len().saturating_sub(1);
        let shift = k.modifiers.contains(KeyModifiers::SHIFT);
        let moved = match k.code {
            KeyCode::Down => Some((self.cursor + 1).min(last)),
            KeyCode::Up => Some(self.cursor.saturating_sub(1)),
            KeyCode::PageDown => Some((self.cursor + 20).min(last)),
            KeyCode::PageUp => Some(self.cursor.saturating_sub(20)),
            KeyCode::Home => Some(0),
            KeyCode::End => Some(last),
            _ => None,
        };
        if let Some(to) = moved {
            if shift {
                self.anchor.get_or_insert(self.cursor);
            } else {
                self.anchor = None;
            }
            self.cursor = to;
            self.follow = to == last && !shift && k.code == KeyCode::End;
            return;
        }
        match k.code {
            KeyCode::Char('/') => self.editing = true,
            KeyCode::Char('l') => {
                self.level = (self.level + 1) % 3;
                self.refilter();
            }
            KeyCode::Char('f') => {
                self.follow = !self.follow;
                self.refilter();
            }
            KeyCode::Esc => self.anchor = None,
            _ => {}
        }
    }

    pub fn on_mouse(&mut self, m: MouseEvent) {
        let a = self.area;
        if m.row < a.y || m.row >= a.bottom() {
            return;
        }
        let at = (self.offset + (m.row - a.y) as usize).min(self.view.len().saturating_sub(1));
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.anchor = Some(at);
                self.cursor = at;
                self.follow = false;
            }
            MouseEventKind::Drag(MouseButton::Left) => self.cursor = at,
            MouseEventKind::ScrollDown => {
                self.cursor = (self.cursor + 3).min(self.view.len().saturating_sub(1))
            }
            MouseEventKind::ScrollUp => {
                self.cursor = self.cursor.saturating_sub(3);
                self.follow = false;
            }
            _ => {}
        }
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect, _palette: &Palette) {
        let [banner, body] = Layout::vertical([
            Constraint::Length(u16::from(self.error.is_some())),
            Constraint::Min(0),
        ])
        .areas(area);
        if let Some(e) = &self.error {
            f.render_widget(
                Paragraph::new(e.as_str()).style(Style::new().fg(Color::Red)),
                banner,
            );
        }
        let inner = Rect::new(
            body.x + 1,
            body.y + 1,
            body.width.saturating_sub(2),
            body.height.saturating_sub(2),
        );
        self.area = inner;
        let height = inner.height as usize;
        if self.follow {
            self.offset = self.view.len().saturating_sub(height);
        } else if self.cursor < self.offset {
            self.offset = self.cursor;
        } else if height > 0 && self.cursor >= self.offset + height {
            self.offset = self.cursor + 1 - height;
        }
        let (sa, sb) = match self.anchor {
            Some(a) => (a.min(self.cursor), a.max(self.cursor)),
            None => (self.cursor, self.cursor),
        };
        let end = (self.offset + height).min(self.view.len());
        let lines: Vec<Line> = (self.offset..end)
            .map(|p| {
                let (lv, text) = &self.lines[self.view[p]];
                let mut st = match lv {
                    2 => Style::new().fg(Color::Red),
                    1 => Style::new().fg(Color::Yellow),
                    _ => Style::new(),
                };
                if (sa..=sb).contains(&p) {
                    st = st.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                }
                Line::styled(text.as_str(), st)
            })
            .collect();
        let title = format!(
            " journal {} lines | level {} | {} | /{}{} ",
            self.view.len(),
            ["all", "warning", "error"][self.level as usize],
            if self.follow { "follow" } else { "paused" },
            self.query,
            if self.editing { "_" } else { "" }
        );
        f.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title)),
            body,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use std::time::Duration;

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    fn view(max: usize, lines: &[&str]) -> JournalView {
        let mut v = JournalView::new(max);
        v.push_batch(
            lines
                .iter()
                .map(|l| JournalMsg::Line(l.to_string()))
                .collect(),
        );
        v
    }

    fn shown(v: &JournalView) -> Vec<&str> {
        v.view.iter().map(|&i| v.lines[i].1.as_str()).collect()
    }

    #[test]
    fn buffer_keeps_the_last_n_lines() {
        let v = view(3, &["1", "2", "3", "4", "5"]);
        assert_eq!(shown(&v), ["3", "4", "5"]);
    }

    #[test]
    fn level_filter() {
        let mut v = view(
            10,
            &[
                "error: x",
                "warning: y",
                "info z",
                "ALERT: a",
                "lqosync3: ERROR: b",
            ],
        );
        v.on_key(key(KeyCode::Char('l')));
        assert_eq!(
            shown(&v),
            ["error: x", "warning: y", "ALERT: a", "lqosync3: ERROR: b"]
        );
        v.on_key(key(KeyCode::Char('l')));
        assert_eq!(shown(&v), ["error: x", "lqosync3: ERROR: b"]);
        v.on_key(key(KeyCode::Char('l')));
        assert_eq!(shown(&v).len(), 5);
    }

    #[test]
    fn search_filters_the_lines() {
        let mut v = view(10, &["router R1 online", "router R2 failed", "added 1"]);
        for c in "/router".chars() {
            v.on_key(key(KeyCode::Char(c)));
        }
        v.on_key(key(KeyCode::Enter));
        assert_eq!(shown(&v), ["router R1 online", "router R2 failed"]);
    }

    #[test]
    fn selection_text_is_the_selected_lines() {
        let mut v = view(10, &["a", "b", "c", "d", "e"]);
        assert_eq!(v.selection_text(), "e");
        v.on_key(key(KeyCode::Home));
        v.on_key(key(KeyCode::Down));
        assert_eq!(v.selection_text(), "b");
        assert!(!v.follow);
    }

    #[test]
    fn shift_down_extends_the_selection() {
        let mut v = view(10, &["a", "b", "c", "d", "e"]);
        v.on_key(key(KeyCode::Home));
        v.on_key(key(KeyCode::Down));
        v.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT));
        v.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT));
        assert_eq!(v.selection_text(), "b\nc\nd");
        v.on_key(key(KeyCode::Down));
        assert_eq!(v.selection_text(), "e");
    }

    #[test]
    fn mouse_drag_selects_lines() {
        let mut v = view(10, &["a", "b", "c", "d", "e"]);
        let mut t = Terminal::new(TestBackend::new(60, 12)).unwrap();
        t.draw(|f| v.draw(f, f.area(), &Palette::new(crate::config::Theme::Dark)))
            .unwrap();
        let at = |kind, row| MouseEvent {
            kind,
            column: 3,
            row,
            modifiers: KeyModifiers::NONE,
        };
        // The first line is under the border.
        v.on_mouse(at(MouseEventKind::Down(MouseButton::Left), 1));
        v.on_mouse(at(MouseEventKind::Drag(MouseButton::Left), 3));
        assert_eq!(v.selection_text(), "a\nb\nc");
    }

    #[test]
    fn journal_start_error_is_shown() {
        let (tx, rx) = std::sync::mpsc::channel();
        let _h = spawn_journal("/nonexistent/journalctl", "lqosync3.service", 10, tx);
        match rx.recv_timeout(Duration::from_secs(5)).unwrap() {
            JournalMsg::Error(e) => assert!(e.contains("cannot start"), "{e}"),
            m => panic!("{m:?}"),
        }
    }

    #[test]
    fn journal_stderr_is_shown() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("journalctl");
        std::fs::write(&script, "#!/bin/sh\necho line one\necho 'Hint: you are not in the adm group' >&2\nexec sleep 30\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let h = spawn_journal(script.to_str().unwrap(), "lqosync3.service", 10, tx);
        let mut got = Vec::new();
        while got.len() < 2 {
            got.push(rx.recv_timeout(Duration::from_secs(5)).unwrap());
        }
        assert!(
            got.iter()
                .any(|m| matches!(m, JournalMsg::Line(l) if l == "line one")),
            "{got:?}"
        );
        assert!(
            got.iter()
                .any(|m| matches!(m, JournalMsg::Error(e) if e.contains("adm"))),
            "{got:?}"
        );
        let start = std::time::Instant::now();
        drop(h);
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn journal_tab_renders() {
        let mut v = view(10, &["2026-10-31T10:00:00+0000 host lqosync3[1]: added 1"]);
        v.push_batch(vec![JournalMsg::Error("Hint: not in adm".into())]);
        let mut t = Terminal::new(TestBackend::new(100, 10)).unwrap();
        t.draw(|f| v.draw(f, f.area(), &Palette::new(crate::config::Theme::Dark)))
            .unwrap();
        let buf = t.backend().buffer();
        let s: String = (0..10)
            .map(|y| (0..100).map(|x| buf[(x, y)].symbol()).collect::<String>() + "\n")
            .collect();
        for want in ["added 1", "Hint: not in adm", "level all", "follow"] {
            assert!(s.contains(want), "{want}\n{s}");
        }
    }
}
