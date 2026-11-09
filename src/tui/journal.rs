//! The Journal tab: `journalctl -f` of the service, with a level filter, search, and line selection.

use crate::tui::theme::{BoxKind, Palette, Role};
use ratatui::Frame;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Constraint, Layout, Rect};
#[cfg(test)]
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::SyncSender;
mod approved;
pub use approved::JournalAction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalMsg {
    Line(String),
    Started,
    Stopped(String),
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
    tx: SyncSender<JournalMsg>,
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
    let _ = tx.send(JournalMsg::Started);
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
                let _ = tx.send(JournalMsg::Stopped(format!("{program} output stopped")));
            });
    }
    JournalHandle { child: Some(child) }
}

/// Message severity is inferred from an explicit prefix, not journal PRIORITY metadata.
fn severity(line: &str) -> Option<(&'static str, u8)> {
    let message = if let Some((_, s)) = line.split_once("]: ") {
        s
    } else if let Some(s) = line.strip_prefix("lqosync3: ") {
        s
    } else {
        line
    };
    let token = message
        .split_whitespace()
        .next()?
        .trim_end_matches(':')
        .to_ascii_lowercase();
    match token.as_str() {
        "error" | "err" => Some(("ERROR", 2)),
        "critical" | "fatal" => Some(("CRITICAL", 2)),
        "warning" | "warn" | "alert" => Some(("WARN", 1)),
        "info" => Some(("INFO", 0)),
        "debug" => Some(("DEBUG", 0)),
        "trace" => Some(("TRACE", 0)),
        _ => None,
    }
}
fn level_of(line: &str) -> u8 {
    severity(line).map_or(0, |(_, n)| n)
}

pub struct JournalView {
    approved: approved::JournalUi,
    pub unit: String,
    search: VecDeque<String>,
    last_received: Option<u64>,
    started: bool,
    stopped: bool,
    selection_expired: bool,
    discarded: u64,
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
            approved: Default::default(),
            unit: "unknown".into(),
            search: VecDeque::new(),
            last_received: None,
            started: false,
            stopped: false,
            selection_expired: false,
            discarded: 0,
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
        let keep = (!self.follow)
            .then(|| self.view.get(self.cursor).copied())
            .flatten();
        let anchor = (!self.follow)
            .then(|| self.anchor.and_then(|a| self.view.get(a).copied()))
            .flatten();
        let mut dropped = 0;
        for m in msgs {
            match m {
                JournalMsg::Line(l) => {
                    if self.lines.len() == self.max {
                        self.lines.pop_front();
                        self.search.pop_front();
                        dropped += 1;
                    }
                    self.last_received = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .ok()
                        .map(|d| d.as_secs());
                    self.started = true;
                    self.stopped = false;
                    self.search.push_back(l.to_lowercase());
                    self.lines.push_back((level_of(&l), l));
                }
                JournalMsg::Started => {
                    self.started = true;
                    self.stopped = false;
                    self.error = None;
                }
                JournalMsg::Stopped(reason) => {
                    self.stopped = true;
                    self.error = Some(reason);
                }
                JournalMsg::Error(e) => self.error = Some(e),
            }
        }
        self.discarded = self.discarded.saturating_add(dropped as u64);
        self.refilter();
        if let Some(old) = keep {
            if let Some(i) = old
                .checked_sub(dropped)
                .and_then(|n| self.view.iter().position(|&v| v == n))
            {
                self.cursor = i;
            } else {
                self.cursor = 0;
                self.selection_expired = true;
                self.anchor = None;
            }
            if let Some(old) = anchor.filter(|_| !self.selection_expired) {
                self.anchor = old
                    .checked_sub(dropped)
                    .and_then(|n| self.view.iter().position(|&v| v == n));
                if self.anchor.is_none() {
                    self.selection_expired = true;
                }
            }
        }
    }

    fn refilter(&mut self) {
        let q = self.query.to_lowercase();
        self.view = self
            .lines
            .iter()
            .enumerate()
            .filter(|(i, (lv, _))| {
                *lv >= self.level
                    && (q.is_empty() || self.search.get(*i).is_some_and(|s| s.contains(&q)))
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
        if self.selection_expired {
            return String::new();
        }
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
            self.anchor = None;
            self.selection_expired = false;
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
            self.selection_expired = false;
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
            KeyCode::Char('x') => self.clear_search(),
            KeyCode::Char('l') => {
                self.anchor = None;
                self.selection_expired = false;
                self.level = (self.level + 1) % 3;
                self.refilter();
            }
            KeyCode::Char('f') => {
                self.selection_expired = false;
                self.follow = !self.follow;
                self.refilter();
            }
            KeyCode::Esc => self.anchor = None,
            _ => {}
        }
    }

    pub fn clear_area(&mut self) {
        self.area = Rect::default();
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

    pub fn draw(&mut self, f: &mut Frame, area: Rect, palette: &Palette) {
        let [banner, body] = Layout::vertical([
            Constraint::Length(u16::from(self.error.is_some())),
            Constraint::Min(0),
        ])
        .areas(area);
        if let Some(e) = &self.error {
            f.render_widget(
                Paragraph::new(e.as_str()).style(palette.role(Role::Error)),
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
                styled_line(text, *lv, (sa..=sb).contains(&p), palette)
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
            Paragraph::new(lines).block(palette.panel(BoxKind::Proc, &title, true)),
            body,
        );
    }
}

fn styled_line<'a>(text: &'a str, level: u8, selected: bool, palette: &Palette) -> Line<'a> {
    use std::sync::OnceLock;
    static PREFIX: OnceLock<regex::Regex> = OnceLock::new();
    let prefix = PREFIX.get_or_init(|| {
        regex::Regex::new(
            r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})[ \t]+\S+[ \t]+",
        )
        .unwrap()
    });
    let role = match level {
        2 => Role::Error,
        1 => Role::Warning,
        _ => Role::Base,
    };
    let normal = if selected {
        palette.selection()
    } else {
        palette.role(role)
    };
    if let Some(m) = prefix.find(text).filter(|m| m.end() < text.len()) {
        let secondary = if selected {
            palette.selection()
        } else {
            palette.role(Role::Secondary)
        };
        Line::from(vec![
            Span::styled(&text[..m.end()], secondary),
            Span::styled(&text[m.end()..], normal),
        ])
    } else {
        Line::styled(text, normal)
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
    fn paused_selection_survives_rollover_or_expires_without_copying_new_records() {
        let mut v = view(4, &["a", "b", "c", "d"]);
        v.on_key(key(KeyCode::Home));
        v.on_key(key(KeyCode::Down));
        v.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT));
        v.push_batch(vec![JournalMsg::Line("e".into())]);
        assert_eq!(v.selection_text(), "b\nc");
        v.push_batch(vec![JournalMsg::Line("f".into())]);
        assert!(v.selection_expired);
        assert!(v.selection_text().is_empty());
        assert_eq!(v.lines.len(), v.search.len());
        v.on_key(key(KeyCode::End));
        assert_eq!(v.selection_text(), "f");
        assert!(!v.selection_expired);
    }

    #[test]
    fn explicit_prefix_severity_does_not_infer_from_arbitrary_message_words() {
        assert_eq!(
            severity("2026-11-09T10:00:00+0800 host lqosync3[1]: ERROR reload failed"),
            Some(("ERROR", 2))
        );
        assert_eq!(severity("info no error reported"), Some(("INFO", 0)));
        assert_eq!(severity("reload error_count=0"), None);
        assert_eq!(severity("unknown"), None);
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
        let (tx, rx) = std::sync::mpsc::sync_channel(2048);
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
        let (tx, rx) = std::sync::mpsc::sync_channel(2048);
        let h = spawn_journal(script.to_str().unwrap(), "lqosync3.service", 10, tx);
        let mut got = Vec::new();
        while got.len() < 3 {
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
    #[test]
    fn short_iso_metadata_is_secondary_and_copy_is_raw() {
        let p = Palette::new(crate::config::Theme::Dark);
        let raw = "2026-11-01T03:00:00+0000 hóst lqosync3[12]: méssage";
        let line = styled_line(raw, 0, false, &p);
        assert_eq!(line.to_string(), raw);
        assert_eq!(line.spans[0].style, p.role(Role::Secondary));
        assert!(
            !line
                .spans
                .last()
                .unwrap()
                .style
                .add_modifier
                .contains(Modifier::DIM)
        );
        for raw in ["-- Journal begins --", "bad timestamp host message"] {
            assert_eq!(styled_line(raw, 2, false, &p).to_string(), raw);
        }
        let line = styled_line(raw, 2, true, &p);
        assert!(line.spans.iter().all(|s| s.style == p.selection()));
        let mut v = view(10, &[raw]);
        v.cursor = 0;
        v.anchor = Some(0);
        assert_eq!(v.selection_text(), raw);
    }
}
