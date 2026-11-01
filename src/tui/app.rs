//! The TUI state, the keys, the mouse, and the draw code.

use super::dashboard::{DashboardView, Kind, Sent, Wait, wait_result};
use super::data::Data;
use super::devices::DevicesView;
use super::journal::JournalView;
use super::network::NetworkView;
use super::osc52::MAX_COPY;
use super::settings::view::{PreviewJob, SAction, SettingsView};
use ratatui::Frame;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use std::sync::Arc;

/// The tabs, in the order of the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Devices,
    Network,
    Settings,
    Journal,
    Dashboard,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Devices,
        Tab::Network,
        Tab::Settings,
        Tab::Journal,
        Tab::Dashboard,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Devices => "1 ShapedDevices",
            Tab::Network => "2 Network",
            Tab::Settings => "3 Settings",
            Tab::Journal => "4 Journal",
            Tab::Dashboard => "5 Dashboard",
        }
    }
}

/// What the main loop must do after a key or a mouse event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    Quit,
    SyncNow,
    Reload,
    /// Copy this text with OSC 52.
    Copy(String),
    /// `true`: release the mouse to the terminal.
    SelectMode(bool),
    /// Run a settings preview in a worker thread.
    Preview(Box<PreviewJob>),
    /// Apply the settings draft, then send SIGHUP.
    ApplySettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modal {
    ConfirmSync,
    /// Quit with settings changes that are not applied.
    Discard,
}

/// The side panel goes to the right on a wide terminal, else under the main panel.
pub fn split_main(area: Rect) -> (Rect, Rect) {
    let [m, s] = if area.width >= 100 {
        Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)]).areas(area)
    } else {
        Layout::vertical([Constraint::Percentage(60), Constraint::Percentage(40)]).areas(area)
    };
    (m, s)
}

pub struct App {
    pub tab: Tab,
    pub read_only: bool,
    pub message: Option<String>,
    pub modal: Option<Modal>,
    /// The tab title rectangles of the last draw, for the mouse.
    pub tab_rects: Vec<Rect>,
    pub data: Data,
    pub devices: DevicesView,
    pub network: NetworkView,
    pub journal: JournalView,
    pub dashboard: DashboardView,
    /// The action that waits for its result in `status.json`.
    pub pending: Option<Sent>,
    /// `tui.apply_verify_timeout`.
    pub action_timeout: u64,
    /// `tui.osc52`.
    pub osc52: bool,
    /// The mouse belongs to the terminal (select mode).
    pub select_mode: bool,
    /// Unix seconds of the last data.
    pub now: u64,
    config_error: Option<String>,
    pub settings: SettingsView,
}

impl App {
    pub fn new(read_only: bool, config_text: String, config_error: Option<String>) -> Self {
        let settings = SettingsView::new(std::path::PathBuf::new(), &config_text, read_only);
        App {
            tab: Tab::Devices,
            read_only,
            message: None,
            modal: None,
            tab_rects: Vec::new(),
            data: Data {
                config_text: Arc::new(config_text),
                ..Data::default()
            },
            devices: DevicesView::default(),
            network: NetworkView::default(),
            journal: JournalView::new(2000),
            dashboard: DashboardView::default(),
            pending: None,
            action_timeout: 120,
            osc52: true,
            select_mode: false,
            now: 0,
            config_error,
            settings,
        }
    }

    /// New data from the loader. `now` is Unix seconds.
    pub fn set_data(&mut self, data: Data, now: u64) {
        self.data = data;
        self.now = now;
        self.devices.on_data(&self.data);
        self.settings
            .on_status(self.data.status.as_deref(), now, self.action_timeout);
        self.network.on_data(&self.data);
        self.dashboard.on_data(&self.data);
        if let Some(sent) = &self.pending {
            let done =
                match wait_result(sent, self.data.status.as_deref(), now, self.action_timeout) {
                    Wait::Pending => None,
                    Wait::Done(m) => Some(m),
                    Wait::Restarted => {
                        Some("the service restarted; the result of the action is not known".into())
                    }
                    Wait::TimedOut => Some(format!(
                        "no result from the service after {} s",
                        now.saturating_sub(sent.at)
                    )),
                };
            if let Some(m) = done {
                self.message = Some(m);
                self.pending = None;
            }
        }
    }

    /// A text field of `status.json`, or `unknown`.
    pub fn status_str(&self, key: &str) -> String {
        self.data
            .status
            .as_ref()
            .and_then(|s| s[key].as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".into())
    }

    fn settings_changes(&self) -> usize {
        self.settings
            .draft
            .as_ref()
            .map_or(0, |d| d.changed().len())
            + self.settings.secrets.len()
    }

    /// A number field of `status.json`.
    pub fn status_u64(&self, key: &str) -> Option<u64> {
        self.data.status.as_ref().and_then(|s| s[key].as_u64())
    }

    pub fn on_key(&mut self, k: KeyEvent) -> Action {
        // A message shows until the next key, so a later loader error can show.
        self.message = None;
        if let Some(m) = self.modal {
            self.modal = None;
            return match (m, k.code) {
                (Modal::ConfirmSync, KeyCode::Char('y')) => Action::SyncNow,
                (Modal::Discard, KeyCode::Char('y')) => Action::Quit,
                (_, KeyCode::Char('n') | KeyCode::Esc) => {
                    self.message = Some("cancelled: nothing changed".into());
                    Action::None
                }
                // Only y, n, and Esc act: another key does not close the dialog.
                (m, _) => {
                    self.modal = Some(m);
                    self.message = Some(modal_keys(m).replace("   ", ", or ").to_string());
                    Action::None
                }
            };
        }
        // An open settings input or modal has all keys.
        if self.tab == Tab::Settings && self.settings.capturing() {
            return self.settings_key(k);
        }
        let ctrl_c = k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c');
        if ctrl_c || (k.code == KeyCode::Char('q') && !self.capturing_search()) {
            if self.settings.dirty() {
                self.modal = Some(Modal::Discard);
                return Action::None;
            }
            return Action::Quit;
        }
        // A search box has all keys.
        if self.capturing_search() {
            return self.tab_key(k);
        }
        match k.code {
            KeyCode::Char(c @ '1'..='5') => self.tab = Tab::ALL[c as usize - '1' as usize],
            KeyCode::Tab => self.tab = Tab::ALL[(self.tab as usize + 1) % 5],
            KeyCode::BackTab => self.tab = Tab::ALL[(self.tab as usize + 4) % 5],
            KeyCode::Char('S') => {
                if self.blocked("sync now") {
                    return Action::None;
                }
                self.modal = Some(Modal::ConfirmSync);
            }
            KeyCode::Char('R') => {
                if !self.blocked("reload") {
                    return Action::Reload;
                }
            }
            _ => return self.tab_key(k),
        }
        Action::None
    }

    fn capturing_search(&self) -> bool {
        (self.tab == Tab::Devices && self.devices.editing)
            || (self.tab == Tab::Network && self.network.editing)
            || (self.tab == Tab::Journal && self.journal.editing)
    }

    fn settings_key(&mut self, k: KeyEvent) -> Action {
        match self.settings.on_key(k) {
            SAction::None => Action::None,
            SAction::Preview(job) => Action::Preview(Box::new(job)),
            SAction::Apply => Action::ApplySettings,
        }
    }

    /// A key for the current tab.
    fn tab_key(&mut self, k: KeyEvent) -> Action {
        if self.tab == Tab::Devices
            && let Some(node) = self.devices.on_key(k, &self.data)
        {
            self.network.reveal(&self.data, &node);
            self.tab = Tab::Network;
        }
        if self.tab == Tab::Network {
            self.network.on_key(k, &self.data);
        }
        if self.tab == Tab::Journal {
            match k.code {
                KeyCode::Char('y') if !self.journal.editing => return self.copy(),
                KeyCode::Char('s') if !self.journal.editing => {
                    self.select_mode = !self.select_mode;
                    self.message = Some(if self.select_mode {
                        "select mode: the mouse belongs to the terminal; press s again to end"
                            .into()
                    } else {
                        "select mode ended".into()
                    });
                    return Action::SelectMode(self.select_mode);
                }
                _ => self.journal.on_key(k),
            }
        }
        if self.tab == Tab::Settings {
            return self.settings_key(k);
        }
        Action::None
    }

    /// The main loop sent a signal (or could not).
    pub fn action_sent(&mut self, r: Result<Sent, String>) {
        match r {
            Ok(s) => {
                let sig = match s.kind {
                    Kind::SyncNow => "SIGUSR1 (sync now)",
                    Kind::Reload => "SIGHUP (reload)",
                };
                self.message = Some(format!(
                    "{sig} sent to PID {}; waiting for status.json",
                    s.pid
                ));
                self.pending = Some(s);
            }
            Err(e) => self.message = Some(format!("not sent: {e}")),
        }
    }

    /// Copy the selected journal lines with OSC 52.
    fn copy(&mut self) -> Action {
        let text = self.journal.selection_text();
        if !self.osc52 {
            self.message = Some(
                "OSC 52 is off (tui.osc52 = false); press s and select with the terminal".into(),
            );
            return Action::None;
        }
        if text.len() > MAX_COPY {
            self.message = Some(format!(
                "the selection is too large for OSC 52 ({} bytes, limit {MAX_COPY}); press s and select with the terminal",
                text.len()
            ));
            return Action::None;
        }
        self.message = Some(
            "sent to the terminal clipboard (not confirmed); if it did not arrive, press s and select with the terminal"
                .into(),
        );
        Action::Copy(text)
    }

    /// Read-only mode (not root) blocks an action. Shows the reason.
    fn blocked(&mut self, what: &str) -> bool {
        if self.read_only {
            self.message = Some(format!(
                "{what}: read-only (not root); start the TUI with sudo"
            ));
        }
        self.read_only
    }

    pub fn on_mouse(&mut self, m: MouseEvent) -> Action {
        if let MouseEventKind::Down(MouseButton::Left) = m.kind
            && let Some(i) = self
                .tab_rects
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
        {
            self.tab = Tab::ALL[i];
        } else if self.tab == Tab::Journal {
            self.journal.on_mouse(m);
        }
        Action::None
    }

    pub fn draw(&mut self, f: &mut Frame) {
        let [header, tabs, main, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(f.area());
        self.draw_header(f, header);
        self.draw_tabs(f, tabs);
        match self.tab {
            Tab::Devices => self.devices.draw(f, main, &self.data),
            Tab::Network => self.network.draw(f, main, &self.data),
            Tab::Journal => self.journal.draw(f, main),
            Tab::Dashboard => self.dashboard.draw(f, main, &self.data, self.now),
            Tab::Settings => self.settings.draw(f, main),
        }
        let tab_keys = match self.tab {
            Tab::Devices if self.devices.editing => "type to search  Enter done  Esc clear",
            Tab::Devices => {
                "/ search  o sort  i reverse  Enter parent node  q quit  1-5 tabs  S sync  R reload"
            }
            Tab::Network if self.network.editing => "type to search  Enter done  Esc clear",
            Tab::Network => {
                "Space open/close  e expand all  c collapse all  / search  q quit  1-5 tabs"
            }
            Tab::Journal if self.journal.editing => "type to search  Enter done  Esc clear",
            Tab::Journal => {
                "l level  / search  f follow  Shift+arrows or mouse select  y copy  s select mode  q quit"
            }
            Tab::Settings => {
                "arrows move  Enter/Space edit  r reset  u undo  P secret  + add  - remove  v preview  A apply  q quit"
            }
            Tab::Dashboard => "S sync now (asks first)  R reload the config  q quit  1-5/Tab tabs",
        };
        // An open dialog or input shows its own keys, so confirm and cancel are always visible.
        let keys = match self.modal {
            Some(m) => modal_keys(m).to_string(),
            None if self.tab == Tab::Settings && self.settings.capturing() => self.settings.hint(),
            None => tab_keys.to_string(),
        };
        // An action message first, then a loader error, then the keys.
        let settings_msg = (self.tab == Tab::Settings)
            .then_some(self.settings.message.as_ref())
            .flatten();
        let (text, color) = match (
            self.message.as_ref().or(settings_msg),
            self.data.errors.first(),
        ) {
            (Some(m), _) => (format!("{m}  |  {keys}"), Color::Gray),
            (None, Some(e)) => (format!("{e}  |  {keys}"), Color::Red),
            (None, None) => (keys.to_string(), Color::Gray),
        };
        f.render_widget(Paragraph::new(text).style(Style::new().fg(color)), footer);
        if self.modal == Some(Modal::Discard) {
            let r = center(main, 56, 5);
            f.render_widget(Clear, r);
            f.render_widget(
                Paragraph::new(format!(
                    "{} settings change(s) are not applied.\ny quits and discards them.",
                    self.settings_changes()
                ))
                .block(dialog(" quit ", Modal::Discard)),
                r,
            );
        }
        if self.modal == Some(Modal::ConfirmSync) {
            let r = center(main, 50, 5);
            f.render_widget(Clear, r);
            f.render_widget(
                Paragraph::new(
                                        "Sync now starts a cycle and releases a guard hold.\ny sends SIGUSR1 to the service.",
                )
                .block(dialog(" sync now ", Modal::ConfirmSync)),
                r,
            );
        }
    }

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let service = self
            .data
            .service
            .as_ref()
            .map_or("unknown", |u| u.active.as_str());
        let updated = self.status_u64("updated");
        let age = updated.map_or("unknown".into(), |u| {
            format!("{}s ago", self.now.saturating_sub(u))
        });
        let next = match (updated, self.status_u64("next_scan_seconds")) {
            (Some(u), Some(n)) => format!("{}s", (u + n).saturating_sub(self.now)),
            _ => "unknown".into(),
        };
        let text = format!(
            " | service {service} | mode {} | target {} | last cycle {age} | next scan {next} ",
            self.status_str("mode"),
            self.status_str("target")
        );
        // The most important parts first: a narrow terminal cuts the end.
        let mut spans = vec![Span::styled(
            " lqosync3 ",
            Style::new().add_modifier(Modifier::BOLD),
        )];
        if self.read_only {
            spans.push(Span::styled(
                " READ-ONLY (not root) ",
                Style::new().fg(Color::Black).bg(Color::Yellow),
            ));
        }
        if let Some(e) = &self.config_error {
            spans.push(Span::styled(
                format!(" config error: {e} "),
                Style::new().fg(Color::Red),
            ));
        }
        spans.push(Span::raw(text));
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    /// The titles are drawn as spans, so the stored rectangles match the screen.
    fn draw_tabs(&mut self, f: &mut Frame, area: Rect) {
        self.tab_rects.clear();
        let mut spans = Vec::new();
        let mut x = area.x;
        for t in Tab::ALL {
            let text = format!(" {} ", t.title());
            let w = (text.len() as u16).min(area.right().saturating_sub(x));
            self.tab_rects.push(Rect::new(x, area.y, w, 1));
            x = x.saturating_add(w + 1);
            let style = if t == self.tab {
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(Color::Cyan)
            };
            spans.push(Span::styled(text, style));
            spans.push(Span::raw(" "));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }
}

/// The keys of a dialog, for the footer.
fn modal_keys(m: Modal) -> &'static str {
    match m {
        Modal::ConfirmSync => "y sync now   n / Esc cancel",
        Modal::Discard => "y quit and discard   n / Esc stay",
    }
}

/// A dialog block with its keys on the bottom border, so they are always visible.
fn dialog(title: &str, m: Modal) -> Block<'_> {
    let bar = match m {
        Modal::ConfirmSync => " [y] sync now   [n / Esc] cancel ",
        Modal::Discard => " [y] quit and discard   [n / Esc] stay ",
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Yellow))
        .title(title)
        .title_bottom(Line::styled(
            bar,
            Style::new()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))
}

/// A `w` x `h` rectangle in the middle of `area`, cut to `area`.
fn center(area: Rect, w: u16, h: u16) -> Rect {
    let (w, h) = (w.min(area.width), h.min(area.height));
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::journal::JournalMsg;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    fn app(read_only: bool) -> App {
        App::new(read_only, "[runtime]\nmode = \"apply\"\n".into(), None)
    }

    fn draw(a: &mut App, w: u16, h: u16) -> String {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| a.draw(f)).unwrap();
        let buf = t.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..h {
            for x in 0..w {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn keys_1_to_5_select_the_tabs_and_q_quits() {
        let mut a = app(false);
        assert_eq!(a.tab, Tab::Devices);
        for (c, t) in ['1', '2', '3', '4', '5'].into_iter().zip(Tab::ALL) {
            assert_eq!(a.on_key(key(KeyCode::Char(c))), Action::None);
            assert_eq!(a.tab, t);
        }
        assert_eq!(a.on_key(key(KeyCode::Char('q'))), Action::Quit);
    }

    #[test]
    fn a_click_on_a_tab_title_selects_it() {
        let mut a = app(false);
        draw(&mut a, 120, 30);
        let r = a.tab_rects[3];
        let click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: r.x + 1,
            row: r.y,
            modifiers: KeyModifiers::NONE,
        };
        a.on_mouse(click);
        assert_eq!(a.tab, Tab::Journal);
    }

    #[test]
    fn read_only_blocks_sync_now() {
        let mut a = app(true);
        assert_eq!(a.on_key(key(KeyCode::Char('S'))), Action::None);
        assert!(a.modal.is_none());
        assert!(a.message.as_deref().unwrap().contains("read-only"));
        assert!(draw(&mut a, 120, 30).contains("READ-ONLY (not root)"));
        let mut a = app(false);
        a.on_key(key(KeyCode::Char('S')));
        assert_eq!(a.modal, Some(Modal::ConfirmSync));
        assert_eq!(a.on_key(key(KeyCode::Char('y'))), Action::SyncNow);
        assert!(a.modal.is_none());
    }

    #[test]
    fn draws_on_a_40x10_terminal_without_a_panic() {
        let mut a = app(false);
        for t in Tab::ALL {
            a.tab = t;
            draw(&mut a, 40, 10);
            draw(&mut a, 1, 1);
        }
    }

    #[test]
    fn narrow_terminal_puts_the_side_panel_under_the_main_panel() {
        let (m, s) = split_main(Rect::new(0, 0, 120, 30));
        assert!(s.x > m.x && s.y == m.y, "{m:?} {s:?}");
        let (m, s) = split_main(Rect::new(0, 0, 80, 30));
        assert!(s.y > m.y && s.x == m.x, "{m:?} {s:?}");
    }

    #[test]
    fn header_shows_unknown_without_status_and_the_values_with_it() {
        let mut a = app(false);
        let top = |a: &mut App| draw(a, 140, 10).lines().next().unwrap().to_string();
        let h = top(&mut a);
        assert!(
            h.contains("service unknown") && h.contains("mode unknown"),
            "{h}"
        );
        let d = Data {
            status: Some(Arc::new(serde_json::json!(
                {"mode": "apply", "target": "v2_pair", "updated": 1000, "next_scan_seconds": 120}
            ))),
            service: Some(crate::tui::proc::Unit {
                load: "loaded".into(),
                active: "active".into(),
                main_pid: 7,
            }),
            ..Data::default()
        };
        a.set_data(d, 1030);
        let h = top(&mut a);
        for want in [
            "service active",
            "mode apply",
            "target v2_pair",
            "last cycle 30s ago",
            "next scan 90s",
        ] {
            assert!(h.contains(want), "{want}: {h}");
        }
    }

    #[test]
    fn enter_opens_the_parent_node_in_the_network_tab() {
        let mut a = app(false);
        let row = crate::model::CsvRow {
            circuit_id: "c1".into(),
            circuit_name: "alice".into(),
            parent_node: "AP".into(),
            ..Default::default()
        };
        let d = Data {
            rows: Arc::new(vec![row]),
            row_search: Arc::new(vec!["alice".into()]),
            flags: Arc::new(vec![0]),
            tree: Arc::new(serde_json::json!({"Site": {"children": {"AP": {}}}})),
            ..Data::default()
        };
        a.set_data(d, 0);
        a.on_key(key(KeyCode::Enter));
        assert_eq!(
            (a.tab, a.network.selected_name()),
            (Tab::Network, Some("AP"))
        );
    }

    #[test]
    fn search_text_takes_the_global_keys() {
        let mut a = app(false);
        a.on_key(key(KeyCode::Char('/')));
        assert_eq!(a.on_key(key(KeyCode::Char('q'))), Action::None);
        a.on_key(key(KeyCode::Char('2')));
        assert_eq!((a.tab, a.devices.query.as_str()), (Tab::Devices, "q2"));
    }

    fn journal_app(lines: &[&str]) -> App {
        let mut a = app(false);
        a.journal.push_batch(
            lines
                .iter()
                .map(|l| JournalMsg::Line(l.to_string()))
                .collect(),
        );
        a.on_key(key(KeyCode::Char('4')));
        a
    }

    #[test]
    fn y_on_journal_copies_with_osc52() {
        let mut a = journal_app(&["one", "two"]);
        assert_eq!(
            a.on_key(key(KeyCode::Char('y'))),
            Action::Copy("two".into())
        );
        let m = a.message.clone().unwrap();
        assert!(
            m.contains("sent to the terminal clipboard (not confirmed)"),
            "{m}"
        );
        assert!(m.contains("press s and select with the terminal"), "{m}");
        a.osc52 = false;
        assert_eq!(a.on_key(key(KeyCode::Char('y'))), Action::None);
        assert!(a.message.as_deref().unwrap().contains("tui.osc52"));
    }

    #[test]
    fn a_too_large_copy_is_not_sent() {
        let big = "x".repeat(crate::tui::osc52::MAX_COPY + 1);
        let mut a = journal_app(&[&big]);
        assert_eq!(a.on_key(key(KeyCode::Char('y'))), Action::None);
        assert!(a.message.as_deref().unwrap().contains("too large"));
    }

    #[test]
    fn s_toggles_select_mode() {
        let mut a = journal_app(&["one"]);
        assert_eq!(a.on_key(key(KeyCode::Char('s'))), Action::SelectMode(true));
        assert_eq!(a.on_key(key(KeyCode::Char('s'))), Action::SelectMode(false));
    }

    #[test]
    fn action_result_comes_from_status() {
        use crate::tui::dashboard::{Kind, Sent};
        let mut a = app(false);
        a.action_sent(Ok(Sent {
            kind: Kind::SyncNow,
            at: 1_000,
            pid: 7,
            seq: 4,
        }));
        assert!(a.message.as_deref().unwrap().contains("waiting"));
        let st = |seq: u64| Data {
            status: Some(Arc::new(
                serde_json::json!({"pid": 7, "cycle_seq": seq, "trigger": "sync_now", "updated": 1_001}),
            )),
            ..Data::default()
        };
        a.set_data(st(4), 1_001);
        assert!(a.message.as_deref().unwrap().contains("waiting"));
        a.set_data(st(5), 1_002);
        assert!(
            a.message.as_deref().unwrap().contains("sync now done"),
            "{:?}",
            a.message
        );
        assert!(a.pending.is_none());
        a.action_sent(Err("PID 9 is \"bash\", not lqosync3".into()));
        assert!(a.message.as_deref().unwrap().contains("bash"));
    }

    #[test]
    fn a_key_press_clears_the_message_so_errors_show() {
        let mut a = app(true);
        a.on_key(key(KeyCode::Char('S')));
        assert!(a.message.is_some());
        a.set_data(
            Data {
                errors: vec!["cannot read /x/ShapedDevices.csv".into()],
                ..Data::default()
            },
            0,
        );
        a.on_key(key(KeyCode::Down));
        assert!(a.message.is_none());
        assert!(
            draw(&mut a, 120, 10)
                .lines()
                .last()
                .unwrap()
                .contains("cannot read")
        );
    }

    #[test]
    fn loader_errors_show_in_the_footer() {
        let mut a = app(false);
        let d = Data {
            errors: vec!["cannot read /x/ShapedDevices.csv: not found".into()],
            ..Data::default()
        };
        a.set_data(d, 0);
        let s = draw(&mut a, 120, 10);
        assert!(
            s.lines()
                .last()
                .unwrap()
                .contains("cannot read /x/ShapedDevices.csv"),
            "{s}"
        );
    }

    #[test]
    fn settings_tab_shows_the_editor() {
        let mut a = app(false);
        a.on_key(key(KeyCode::Char('3')));
        let screen = draw(&mut a, 140, 30);
        assert!(screen.contains("Settings: 0 changed keys"), "{screen}");
        assert!(screen.contains("mode = \"apply\""), "{screen}");
    }

    #[test]
    fn settings_messages_show_in_the_footer() {
        let mut a = app(true);
        a.on_key(key(KeyCode::Char('3')));
        a.settings.select("safety.enabled");
        a.on_key(key(KeyCode::Char(' ')));
        let s = draw(&mut a, 140, 30);
        assert!(s.lines().last().unwrap().contains("read-only"), "{s}");
    }

    #[test]
    fn app_dialogs_show_their_keys_and_only_y_n_esc_act() {
        let mut a = app(false);
        a.on_key(key(KeyCode::Char('S')));
        let s = draw(&mut a, 120, 30);
        assert!(
            s.contains("[y] sync now") && s.contains("[n / Esc] cancel"),
            "{s}"
        );
        assert!(s.lines().last().unwrap().contains("y sync now"), "{s}");
        assert_eq!(a.on_key(key(KeyCode::Char('x'))), Action::None);
        assert_eq!(a.modal, Some(Modal::ConfirmSync));
        assert_eq!(a.on_key(key(KeyCode::Esc)), Action::None);
        assert!(a.modal.is_none());
        a.on_key(key(KeyCode::Char('3')));
        a.settings.select("safety.enabled");
        a.on_key(key(KeyCode::Char(' ')));
        a.on_key(key(KeyCode::Char('q')));
        let s = draw(&mut a, 120, 30);
        assert!(
            s.contains("[y] quit and discard") && s.contains("[n / Esc] stay"),
            "{s}"
        );
    }

    #[test]
    fn quit_with_changes_asks_first() {
        let mut a = app(false);
        a.on_key(key(KeyCode::Char('3')));
        assert!(a.settings.select("safety.enabled"));
        a.on_key(key(KeyCode::Char(' ')));
        assert!(a.settings.dirty());
        assert_eq!(a.on_key(key(KeyCode::Char('q'))), Action::None);
        assert_eq!(a.modal, Some(Modal::Discard));
        assert_eq!(a.on_key(key(KeyCode::Char('n'))), Action::None);
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(a.on_key(ctrl_c), Action::None);
        assert_eq!(a.on_key(key(KeyCode::Char('y'))), Action::Quit);
    }

    #[test]
    fn an_open_field_takes_q_and_digits() {
        let mut a = app(false);
        a.on_key(key(KeyCode::Char('3')));
        assert!(a.settings.select("runtime.id_length"));
        a.on_key(key(KeyCode::Enter));
        assert!(a.settings.capturing());
        assert_eq!(a.on_key(key(KeyCode::Char('q'))), Action::None);
        a.on_key(key(KeyCode::Char('2')));
        a.on_key(key(KeyCode::Tab));
        assert_eq!(a.tab, Tab::Settings);
        assert!(a.settings.capturing());
    }

    #[test]
    fn settings_preview_and_apply_become_actions() {
        let mut a = app(false);
        a.on_key(key(KeyCode::Char('3')));
        a.settings.select("safety.enabled");
        a.on_key(key(KeyCode::Char(' ')));
        assert!(matches!(
            a.on_key(key(KeyCode::Char('v'))),
            Action::Preview(_)
        ));
    }
}
