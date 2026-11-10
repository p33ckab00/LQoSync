//! The TUI state, the keys, the mouse, and the draw code.

use super::dashboard::{DashboardView, Kind, Sent, Wait, wait_result};
use super::data::Data;
use super::devices::DevicesView;
use super::journal::JournalView;
use super::network::NetworkView;
use super::osc52::MAX_COPY;
use super::settings::view::{PreviewJob, SAction, SettingsView};
use super::theme::{BoxKind, Palette, Role};
use super::workers::WorkersView;
use ratatui::Frame;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use std::sync::Arc;

/// Route identities. ALL defines the primary navigation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Devices,
    Network,
    Settings,
    Journal,
    Dashboard,
    Workers,
    Routers,
    LibreQoS,
}

impl Tab {
    pub const ALL: [Tab; 7] = [
        Tab::Dashboard,
        Tab::Devices,
        Tab::Routers,
        Tab::LibreQoS,
        Tab::Workers,
        Tab::Settings,
        Tab::Journal,
    ];

    fn navigation_index(self) -> usize {
        Self::ALL.iter().position(|&t| t == self).unwrap_or(1)
    }

    pub fn title(self) -> &'static str {
        match self {
            Tab::Devices => "Subscribers",
            Tab::Network => "Network",
            Tab::Settings => "Settings",
            Tab::Journal => "Journal",
            Tab::Dashboard => "Dashboard",
            Tab::Workers => "Workers",
            Tab::Routers => "Routers",
            Tab::LibreQoS => "LibreQoS",
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
    Refresh,
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
    pub palette: Palette,
    pub tab: Tab,
    pub read_only: bool,
    pub message: Option<String>,
    pub modal: Option<Modal>,
    /// The navigation rectangles of the last draw, for the mouse.
    pub tab_rects: Vec<Rect>,
    theme_rect: Rect,
    dialog_buttons: [Rect; 2],
    routers: super::routers::RoutersView,
    libreqos: super::libreqos::LibreView,
    pub data: Data,
    subscriber_data: super::presence::Projection,
    pub devices: DevicesView,
    pub network: NetworkView,
    pub journal: JournalView,
    pub dashboard: DashboardView,
    overview: super::overview::Overview,
    pub workers: WorkersView,
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
            palette: Palette::new(crate::config::Theme::Dark),
            tab: Tab::Dashboard,
            read_only,
            message: None,
            modal: None,
            tab_rects: Vec::new(),
            theme_rect: Rect::default(),
            dialog_buttons: [Rect::default(); 2],
            routers: Default::default(),
            libreqos: Default::default(),
            data: Data {
                config_text: Arc::new(config_text),
                ..Data::default()
            },
            subscriber_data: Default::default(),
            devices: DevicesView::default(),
            network: NetworkView::default(),
            journal: JournalView::new(2000),
            dashboard: DashboardView::default(),
            overview: Default::default(),
            workers: WorkersView::default(),
            pending: None,
            action_timeout: 120,
            osc52: true,
            select_mode: false,
            now: 0,
            config_error,
            settings,
        }
    }

    /// Advance display freshness even when the loader does not send data.
    pub fn on_tick(&mut self, now: u64) {
        self.now = now;
        self.devices.approved_tick(&self.subscriber_data.view, now);
    }

    /// New data from the loader. `now` is Unix seconds.
    pub fn set_data(&mut self, data: Data, now: u64) {
        self.data = data;
        self.now = now;
        self.subscriber_data.update(&self.data);
        self.devices.on_data(&self.subscriber_data.view);
        self.devices.approved_tick(&self.subscriber_data.view, now);
        self.devices.approved_data(&self.subscriber_data.view);
        self.routers.on_data(&self.data);
        self.libreqos.on_data(&self.data);
        self.settings.set_snapshot(self.data.snapshot.clone());
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
            KeyCode::Char(c @ '1'..='7') => self.tab = Tab::ALL[c as usize - '1' as usize],
            KeyCode::Tab
                if matches!(
                    self.tab,
                    Tab::Devices
                        | Tab::Routers
                        | Tab::LibreQoS
                        | Tab::Workers
                        | Tab::Settings
                        | Tab::Journal
                ) =>
            {
                return self.tab_key(k);
            }
            KeyCode::BackTab
                if matches!(
                    self.tab,
                    Tab::Devices
                        | Tab::Routers
                        | Tab::LibreQoS
                        | Tab::Workers
                        | Tab::Settings
                        | Tab::Journal
                ) =>
            {
                return self.tab_key(k);
            }
            KeyCode::Tab => self.tab = Tab::ALL[(self.tab.navigation_index() + 1) % Tab::ALL.len()],
            KeyCode::BackTab => {
                self.tab =
                    Tab::ALL[(self.tab.navigation_index() + Tab::ALL.len() - 1) % Tab::ALL.len()]
            }
            KeyCode::Char('r') if self.tab == Tab::Dashboard => return Action::Refresh,
            KeyCode::Char('T') => {
                self.palette = Palette::new(match self.palette.theme {
                    crate::config::Theme::Dark => crate::config::Theme::Light,
                    crate::config::Theme::Light => crate::config::Theme::Mono,
                    crate::config::Theme::Mono => crate::config::Theme::Dark,
                });
            }
            KeyCode::Esc if self.tab == Tab::Network => self.tab = Tab::Devices,
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
            || (self.tab == Tab::Routers && self.routers.editing)
            || (self.tab == Tab::Workers && self.workers.editing())
    }

    fn router_action(&mut self, action: super::routers::RouterAction) -> Action {
        match action {
            super::routers::RouterAction::Subscribers(name) => {
                self.devices.filter_router(name, &self.subscriber_data.view);
                self.tab = Tab::Devices;
            }
            super::routers::RouterAction::Settings(i) => {
                self.settings.select(&format!("routers.{i}.name"));
                self.tab = Tab::Settings;
            }
            super::routers::RouterAction::Copy(text) => return self.copy_text(text),
            _ => {}
        }
        Action::None
    }

    fn journal_action(&mut self, action: super::journal::JournalAction) -> Action {
        match action {
            super::journal::JournalAction::None => Action::None,
            super::journal::JournalAction::Key(code) => {
                self.on_key(KeyEvent::new(code, KeyModifiers::NONE))
            }
            super::journal::JournalAction::Clear => {
                self.journal.clear_search();
                Action::None
            }
            super::journal::JournalAction::Copy => self.copy(),
            super::journal::JournalAction::Select => {
                self.on_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE))
            }
        }
    }

    fn settings_key(&mut self, k: KeyEvent) -> Action {
        let action = self.settings.approved_key(k);
        self.settings_action(action)
    }
    fn settings_action(&mut self, action: SAction) -> Action {
        match action {
            SAction::None => Action::None,
            SAction::Preview(job) => Action::Preview(Box::new(job)),
            SAction::Apply => Action::ApplySettings,
        }
    }

    /// A key for the current tab.
    fn tab_key(&mut self, k: KeyEvent) -> Action {
        if self.tab == Tab::Devices {
            match self.devices.approved_key(k, &self.subscriber_data.view) {
                super::devices::SubscriberInput::Parent(node) => {
                    self.network.reveal(&self.data, &node);
                    self.tab = Tab::Network;
                }
                super::devices::SubscriberInput::Copy(text) => return self.copy_text(text),
                _ => {}
            }
        }
        if self.tab == Tab::Network {
            self.network.on_key(k, &self.data);
        }
        if self.tab == Tab::Dashboard {
            self.overview.key(k.code, &self.data);
        }
        if self.tab == Tab::Routers {
            let action = self.routers.on_key(k);
            return self.router_action(action);
        }
        if self.tab == Tab::LibreQoS
            && matches!(
                self.libreqos.key(k, &self.data),
                super::libreqos::LibreAction::Sync
            )
        {
            return self.on_key(KeyEvent::new(KeyCode::Char('S'), KeyModifiers::NONE));
        }
        if self.tab == Tab::Workers
            && let super::workers::WorkerAction::Copy(text) =
                self.workers.approved_key(k, &self.data, self.now)
        {
            return self.copy_text(text);
        }
        if self.tab == Tab::Journal {
            match k.code {
                KeyCode::Char('y' | 'c') if !self.journal.editing => return self.copy(),
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
                _ => self.journal.approved_key(k),
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
        self.copy_text(self.journal.selection_text())
    }

    fn copy_text(&mut self, text: String) -> Action {
        if text.is_empty() {
            self.message = Some("No retained selection to copy; choose an entry".into());
            return Action::None;
        }
        if !self.osc52 {
            self.message = Some(
                "OSC 52 is off (tui.osc52 = false); use terminal text selection (Shift+mouse)"
                    .into(),
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
        if self.modal.is_some() {
            if m.kind == MouseEventKind::Down(MouseButton::Left)
                && let Some(i) = self
                    .dialog_buttons
                    .iter()
                    .position(|r| r.contains((m.column, m.row).into()))
            {
                return self.on_key(KeyEvent::new(
                    KeyCode::Char(if i == 0 { 'y' } else { 'n' }),
                    KeyModifiers::NONE,
                ));
            }
            return Action::None;
        }
        if self.select_mode {
            return Action::None;
        }
        if self.tab == Tab::Settings && self.settings.capturing() {
            let action = self.settings.approved_mouse(m);
            return self.settings_action(action);
        }
        if self.capturing_search() {
            if self.tab == Tab::Journal {
                let action = self.journal.approved_mouse(m);
                return self.journal_action(action);
            }
            if self.tab == Tab::Workers {
                if let super::workers::WorkerAction::Copy(text) =
                    self.workers.approved_mouse(m, &self.data, self.now)
                {
                    return self.copy_text(text);
                }
                return Action::None;
            }
            if self.tab == Tab::Routers {
                let action = self.routers.on_mouse(m);
                return self.router_action(action);
            }
            if self.tab == Tab::Devices {
                match self.devices.approved_mouse(m, &self.subscriber_data.view) {
                    super::devices::SubscriberInput::Copy(text) => return self.copy_text(text),
                    super::devices::SubscriberInput::Parent(node) => {
                        self.network.reveal(&self.data, &node);
                        self.tab = Tab::Network;
                    }
                    _ => {}
                }
            }
            return Action::None;
        }
        if self.tab == Tab::Devices {
            match self.devices.approved_mouse(m, &self.subscriber_data.view) {
                super::devices::SubscriberInput::Parent(node) => {
                    self.network.reveal(&self.data, &node);
                    self.tab = Tab::Network;
                }
                super::devices::SubscriberInput::Copy(text) => return self.copy_text(text),
                _ => {}
            }
        }
        if m.kind == MouseEventKind::Down(MouseButton::Left)
            && self.theme_rect.contains((m.column, m.row).into())
        {
            return self.on_key(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::NONE));
        }
        if self.tab == Tab::Workers
            && let super::workers::WorkerAction::Copy(text) =
                self.workers.approved_mouse(m, &self.data, self.now)
        {
            return self.copy_text(text);
        }
        if self.tab == Tab::Settings {
            let action = self.settings.approved_mouse(m);
            let action = self.settings_action(action);
            if action != Action::None {
                return action;
            }
        }
        if self.tab == Tab::Dashboard && self.overview.mouse(m, &self.data) {
            return self.on_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        }
        if self.tab == Tab::Routers {
            let action = self.routers.on_mouse(m);
            let action = self.router_action(action);
            if action != Action::None {
                return action;
            }
        }
        if self.tab == Tab::LibreQoS
            && matches!(
                self.libreqos.mouse(m, &self.data),
                super::libreqos::LibreAction::Sync
            )
        {
            return self.on_key(KeyEvent::new(KeyCode::Char('S'), KeyModifiers::NONE));
        }
        if let MouseEventKind::Down(MouseButton::Left) = m.kind
            && let Some(i) = self
                .tab_rects
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
        {
            return self.on_key(KeyEvent::new(
                KeyCode::Char(char::from(b'1' + i as u8)),
                KeyModifiers::NONE,
            ));
        } else if self.tab == Tab::Journal {
            let action = self.journal.approved_mouse(m);
            return self.journal_action(action);
        }
        Action::None
    }

    pub fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        self.dialog_buttons = [Rect::default(); 2];
        self.journal.clear_approved_areas();
        self.settings.clear_approved_areas();
        self.workers.clear_areas();
        self.devices.clear_approved_areas();
        self.overview.clear();
        self.theme_rect = Rect::default();
        self.routers.clear_area();
        self.libreqos.clear_area();
        f.render_widget(Block::default().style(self.palette.canvas()), area);
        if area.width < 80 || area.height < 24 {
            self.tab_rects.clear();
            self.settings.areas = [Rect::default(); 3];
            self.journal.clear_area();
            f.render_widget(Paragraph::new("terminal too small: min 80x24"), area);
            return;
        }
        let keys = self.key_hint();
        let wide = super::shell::enabled(area);
        let content_width = if wide {
            area.width - super::shell::sidebar_width(area.width)
        } else {
            area.width
        };
        let hint_lines = wrap_hint(&keys, content_width);
        let settings_msg = (self.tab == Tab::Settings)
            .then_some(self.settings.message.as_ref())
            .flatten();
        let message = self
            .message
            .as_ref()
            .or(settings_msg)
            .or(self.config_error.as_ref())
            .or(self.data.errors.first())
            .cloned();
        let message_role = if self.message.is_some() || settings_msg.is_some() {
            Role::Base
        } else {
            Role::Error
        };
        let content = if wide {
            let shell = super::shell::draw(f, area, self);
            self.tab_rects = shell.navigation;
            self.theme_rect = shell.theme;
            shell.content
        } else {
            let head = self.header_lines(area.width);
            let [header, tabs, content] = Layout::vertical([
                Constraint::Length(head.len() as u16),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .areas(area);
            f.render_widget(Paragraph::new(head), header);
            self.draw_tabs(f, tabs);
            content
        };
        let [main, msg_area, footer] = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(u16::from(message.is_some())),
            Constraint::Length(hint_lines.len() as u16),
        ])
        .areas(content);
        if let Some(m) = message {
            f.render_widget(
                Paragraph::new(m).style(self.palette.role(message_role)),
                msg_area,
            );
        }
        match self.tab {
            Tab::Routers => self
                .routers
                .draw(f, main, &self.data, self.now, &self.palette),
            Tab::LibreQoS => self
                .libreqos
                .draw(f, main, &self.data, self.now, &self.palette),
            Tab::Devices => self.devices.draw_approved(
                f,
                main,
                &self.subscriber_data.view,
                self.now,
                &self.palette,
            ),
            Tab::Network => self.network.draw(f, main, &self.data, &self.palette),
            Tab::Journal => {
                self.journal
                    .draw_approved(f, main, self.now, self.select_mode, &self.palette)
            }
            Tab::Dashboard => self
                .overview
                .draw(f, main, &self.data, self.now, &self.palette),
            Tab::Settings => self.settings.draw_approved(f, main, &self.palette),
            Tab::Workers => {
                self.workers
                    .draw_approved(f, main, &self.data, self.now, &self.palette)
            }
        }
        f.render_widget(
            Paragraph::new(
                hint_lines
                    .iter()
                    .map(|l| {
                        Line::styled(l.clone(), super::design::role(&self.palette, Role::Base))
                    })
                    .collect::<Vec<_>>(),
            ),
            footer,
        );
        if self.modal == Some(Modal::Discard) {
            let r = center(main, 56, 5);
            f.render_widget(Clear, r);
            f.render_widget(
                Paragraph::new(format!(
                    "{} settings change(s) are not applied.\ny quits and discards them.",
                    self.settings_changes()
                ))
                .block(dialog(" quit ", Modal::Discard, &self.palette)),
                r,
            );
            self.draw_dialog_buttons(f, r);
        }
        if self.modal == Some(Modal::ConfirmSync) {
            let r = center(main, 50, 5);
            f.render_widget(Clear, r);
            f.render_widget(
                Paragraph::new(
                                        "Sync now starts a cycle and releases a guard hold.\ny sends SIGUSR1 to the service.",
                )
                .block(dialog(" sync now ", Modal::ConfirmSync, &self.palette)),
                r,
            );
            self.draw_dialog_buttons(f, r);
        }
    }

    fn draw_dialog_buttons(&mut self, f: &mut Frame, r: Rect) {
        let inner = r.inner(ratatui::layout::Margin::new(1, 1));
        let half = inner.width / 2;
        let row = inner.bottom().saturating_sub(1);
        self.dialog_buttons = [
            Rect::new(inner.x, row, half, 1),
            Rect::new(inner.x + half, row, inner.width - half, 1),
        ];
        for (i, label) in ["[y Confirm]", "[n Cancel]"].into_iter().enumerate() {
            f.render_widget(
                Paragraph::new(label).style(super::design::accent(&self.palette)),
                self.dialog_buttons[i],
            );
        }
    }

    fn key_hint(&self) -> String {
        if let Some(m) = self.modal {
            return modal_keys(m).into();
        }
        if self.tab == Tab::Settings && self.settings.capturing() {
            return self.settings.hint();
        }
        match self.tab {
            Tab::Devices if self.devices.editing => "[Enter] done  [Esc] clear  type to search",
            Tab::Devices => "[q] quit  [/] search  [o] sort  [i] reverse  [Enter] edit/focus  [e] details  [n] parent  [c] copy  [s/r/t] filters  [Left/Right] columns  [1-7] tabs  [S] sync  [R] reload",
            Tab::Network if self.network.editing => "[Enter] done  [Esc] clear  type to search",
            Tab::Network => "[q] quit  [Esc] subscribers  [Space] open/close  [e] expand all  [c] collapse all  [/] search  [1-7] tabs",
            Tab::Journal if self.journal.editing => "[Enter] done  [Esc] clear  type to search",
            Tab::Journal => "[q] quit  [Tab] focus  [l] level  [/] search  [x] clear  [f] follow  [e] details  [y/c] copy  [s] terminal select  [Shift+arrows] select  [Left/Right] columns",
            Tab::Settings => "[q] quit  [arrows] move  [Enter/Space] edit  [r] reset  [u] undo  [P] secret  [+] add  [-] remove  [v] preview  [A] apply",
            Tab::Routers if self.routers.editing => "[Enter] done  [Esc] clear  type to search",
            Tab::Routers => "[q] quit  [/] search  [t] state  [x] clear  [Tab] focus  [d] details  [s] subscribers  [e] Settings  [c] copy  [Left/Right] columns  [1-7] pages",
            Tab::LibreQoS => "[q] quit  [p] preview/back  [s] sync  [Tab] file  [i] issues  [Up/Down/PgUp/PgDn] select/scroll  [Left/Right] columns  [1-7] pages",
            Tab::Dashboard => "[q] quit  [r] refresh  [w] window  [v] traffic source  [j/k] health  [Up/Down] scroll  [S] sync  [R] reload  [1-7] pages",
            Tab::Workers if self.workers.editing() => "[Enter] done  [Esc] clear  type to search",
            Tab::Workers => "[q] quit  [/] worker search  [t] sample  [x] clear  [Tab] focus  [e/Enter] details  [c] copy  [Up/Down/PgUp/PgDn] select/scroll  [1-7] pages",
        }.into()
    }

    /// The header: the name and the state pills, then the target and the cycle times.
    /// One line when both parts fit in `width`, else two lines.
    fn header_lines(&self, width: u16) -> Vec<Line<'static>> {
        let p = &self.palette;
        let service = self
            .data
            .service
            .as_ref()
            .map_or("unknown", |u| u.active.as_str());
        let mode = self.status_str("mode");
        let service_role = match service {
            "active" => Role::Ok,
            "failed" => Role::Error,
            "unknown" => Role::Secondary,
            _ => Role::Warning,
        };
        let mode_role = match mode.as_str() {
            "apply" => Role::Ok,
            "shadow" => Role::Warning,
            _ => Role::Secondary,
        };
        let pill = |text: String, role: Role| {
            let s = p.badge(&text, role, false);
            Span::styled(text.clone(), s.style)
        };
        let mut left = vec![
            Span::styled(" lqosync3 ", p.title_style()),
            pill(
                format!(" service {} ", super::theme::clip(service, 18)),
                service_role,
            ),
            Span::raw(" "),
            pill(
                format!(" mode {} ", super::theme::clip(&mode, 12)),
                mode_role,
            ),
        ];
        if self.read_only {
            left.push(Span::raw(" "));
            left.push(pill(" READ-ONLY (not root) ".into(), Role::Warning));
        }
        let updated = self.status_u64("updated");
        let age = updated.map_or("unknown".into(), |u| {
            format!("{}s ago", self.now.saturating_sub(u))
        });
        let next = match (updated, self.status_u64("next_scan_seconds")) {
            (Some(u), Some(n)) => format!("{}s", u.saturating_add(n).saturating_sub(self.now)),
            _ => "unknown".into(),
        };
        let target = super::theme::clip(&self.status_str("target"), 20);
        let dim = p.role(Role::Secondary);
        let mut right = vec![Span::raw(" ")];
        for (i, (label, value)) in [
            ("target ", target),
            ("last cycle ", age),
            ("next scan ", next),
        ]
        .into_iter()
        .enumerate()
        {
            if i > 0 {
                right.push(Span::styled(" │ ", dim));
            }
            right.push(Span::styled(label, dim));
            right.push(Span::styled(value, p.role(Role::Base)));
        }
        right.push(Span::raw(" "));
        let (lw, rw) = (
            Line::from(left.clone()).width(),
            Line::from(right.clone()).width(),
        );
        if lw + rw <= width as usize {
            left.push(Span::raw(" ".repeat(width as usize - lw - rw)));
            left.extend(right);
            vec![Line::from(left)]
        } else {
            vec![Line::from(left), Line::from(right)]
        }
    }

    /// Compact navigation keeps all seven routes visible at the minimum width.
    fn draw_tabs(&mut self, f: &mut Frame, area: Rect) {
        self.tab_rects.clear();
        let mut x = area.x;
        for (i, tab) in Tab::ALL.into_iter().enumerate() {
            let name = match tab {
                Tab::Dashboard => "Dash",
                Tab::Devices => "Subs",
                _ => tab.title(),
            };
            let label = format!("{}{} ", i + 1, name);
            let width = (label.len() as u16).min(area.right().saturating_sub(x));
            let rect = Rect::new(x, area.y, width, 1);
            self.tab_rects.push(rect);
            let selected = self.tab == tab || (self.tab == Tab::Network && tab == Tab::Devices);
            let style = if selected {
                self.palette.selection()
            } else {
                self.palette.role(Role::Secondary)
            };
            f.render_widget(Paragraph::new(label).style(style), rect);
            x += width;
        }
    }
}

/// The key hint in lines of at most `width` cells. A break is only between two keys,
/// and no key is dropped.
fn wrap_hint(keys: &str, width: u16) -> Vec<String> {
    let mut lines = vec![String::new()];
    for part in keys.split("  ").map(str::trim).filter(|p| !p.is_empty()) {
        let last = lines.last_mut().unwrap();
        if !last.is_empty() && Line::from(format!("{last}  {part}")).width() > width as usize {
            lines.push(part.to_string());
        } else {
            if !last.is_empty() {
                last.push_str("  ");
            }
            last.push_str(part);
        }
    }
    lines
}

/// The keys of a dialog, for the footer.
fn modal_keys(m: Modal) -> &'static str {
    match m {
        Modal::ConfirmSync => "[y] sync now   [n / Esc] cancel",
        Modal::Discard => "[y] quit and discard   [n / Esc] stay",
    }
}

/// A dialog block with its keys on the bottom border, so they are always visible.
fn dialog<'a>(title: &'a str, m: Modal, palette: &Palette) -> Block<'a> {
    let bar = match m {
        Modal::ConfirmSync => " [y] sync now   [n / Esc] cancel ",
        Modal::Discard => " [y] quit and discard   [n / Esc] stay ",
    };
    palette
        .panel(BoxKind::Proc, title, true)
        .title_bottom(palette.keys(bar))
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

    #[test]
    fn collection_updates_subscriber_states_across_cycles_without_changing_shaping() {
        let config = "[[routers]]\nname='R1'\naddress='192.0.2.1'\nusername='u'\npassword_file='/fixture/password'\n[routers.pppoe]\nenabled=true\n[routers.hotspot]\nenabled=true\n[routers.dhcp]\nenabled=true\n";
        let cfg = crate::config::Config::from_toml_str(config).unwrap();
        let mut a = App::new(true, config.into(), None);
        for (now, user) in [(100, "alice"), (200, "bob")] {
            let snapshot=serde_json::from_value(serde_json::json!({"ppp_secret":[{"name":"alice"},{"name":"bob"}],"ppp_active":[{"name":user,"address":"192.0.2.10"}],"hotspot_active":[{"user":"guest","mac-address":"02:00:00:00:00:01"}],"dhcp_lease":[{"host-name":"lease","address":"192.0.2.12","mac-address":"02:00:00:00:00:02","status":"bound"}]})).unwrap();
            let collection = crate::collection::Report::collect(
                &cfg,
                &[("R1".into(), crate::cycle::RouterResult::Online(snapshot))],
                None,
                now,
            );
            let alice = collection.routers[0]
                .subscribers
                .iter()
                .find(|s| s.name == "alice")
                .unwrap()
                .circuit
                .clone();
            let d = Data {
                rows: Arc::new(vec![crate::model::CsvRow {
                    circuit_id: "shaped-alice".into(),
                    circuit_name: alice.clone(),
                    download_max: "100".into(),
                    ..Default::default()
                }]),
                row_search: Arc::new(vec![alice.clone()]),
                sources: Arc::new([(alice, ("ppp".into(), "R1".into()))].into_iter().collect()),
                collection: Some(Arc::new(collection)),
                config_text: Arc::new(config.into()),
                ..Default::default()
            };
            a.set_data(d, now);
            assert_eq!(
                a.data.rows.len(),
                1,
                "collection must never alter shaping preview"
            );
            assert_eq!(
                a.subscriber_data.view.rows.len(),
                4,
                "shaped record must match rather than duplicate"
            );
            a.tab = Tab::Devices;
            let text = draw(&mut a, 200, 40);
            assert!(
                text.contains("2 known"),
                "current online count after cycle {now}\n{text}"
            );
            assert!(
                text.contains("1 known"),
                "known offline account after cycle {now}\n{text}"
            );
            assert!(text.contains("Bound 1"));
        }
        a.devices
            .approved_key(key(KeyCode::Char('t')), &a.subscriber_data.view);
        assert_eq!(a.devices.index.len(), 2);
        a.on_tick(10000);
        assert!(
            a.devices.index.is_empty(),
            "timer expiry must remove live-state filter results"
        );
        assert_eq!(
            super::super::presence::count(&a.data, 10000, "ppp", false),
            "unknown"
        );
    }

    #[test]
    fn journal_approved_body_exposes_stream_filters_details_and_selection() {
        let mut a = app(true);
        a.tab = Tab::Journal;
        let text = draw(&mut a, 200, 48);
        for label in [
            "Log source",
            "Retained lines",
            "View",
            "Level [",
            "Follow",
            "Service journal",
            "Selected entry",
            "Selection and navigation",
            "Data status",
        ] {
            assert!(text.contains(label), "missing {label}");
        }
    }

    #[test]
    fn settings_approved_body_exposes_loaded_draft_review_and_apply() {
        let mut a = app(true);
        a.tab = Tab::Settings;
        let text = draw(&mut a, 200, 48);
        for label in [
            "Configuration",
            "Settings groups",
            "Loaded",
            "Draft",
            "Unit",
            "Review and apply",
            "Validate & preview",
            "Save & apply",
        ] {
            assert!(text.contains(label), "missing {label}");
        }
        assert!(text.contains("READ-ONLY") || text.contains("read-only"));
    }

    #[test]
    fn workers_approved_body_has_summary_filter_details_and_pressure() {
        let mut a = app(true);
        a.tab = Tab::Workers;
        let text = draw(&mut a, 200, 48);
        for label in [
            "Busiest worker",
            "Telemetry",
            "Worker ID",
            "Sample [",
            "Worker utilization",
            "Worker details",
            "Buffer/ring pressure",
        ] {
            assert!(text.contains(label), "missing {label}");
        }
        assert!(text.contains("unavailable"));
    }

    #[test]
    fn libreqos_sync_click_keeps_read_only_and_confirmation_gates() {
        let mut a = app(false);
        a.tab = Tab::LibreQoS;
        a.on_key(key(KeyCode::Char('s')));
        assert_eq!(a.modal, Some(Modal::ConfirmSync));
        assert_eq!(a.on_key(key(KeyCode::Char('n'))), Action::None);
        a.read_only = true;
        a.on_key(key(KeyCode::Char('s')));
        assert!(a.modal.is_none());
        assert!(a.message.as_deref().unwrap().contains("read-only"));
    }

    #[test]
    fn libreqos_body_preserves_preview_and_guarded_sync_actions() {
        let mut a = app(true);
        a.tab = Tab::LibreQoS;
        let text = draw(&mut a, 200, 48);
        for label in [
            "Shaping input",
            "Native compatibility",
            "Last sync",
            "Remote sync",
            "Source preview",
            "Data status",
        ] {
            assert!(text.contains(label), "missing {label}");
        }
        assert!(text.contains("NOT CONFIGURED"));
    }

    #[test]
    fn routers_approved_body_has_collection_cards_and_details() {
        let mut a = app(true);
        a.tab = Tab::Routers;
        let text = draw(&mut a, 200, 48);
        for label in [
            "Configured",
            "Reachable",
            "Failed reads",
            "Router list",
            "Router details",
            "Data status",
        ] {
            assert!(text.contains(label), "missing {label}");
        }
        assert!(text.contains("unknown"));
    }

    #[test]
    fn subscribers_approved_body_has_filters_details_and_unknown_state() {
        let mut a = app(true);
        a.tab = Tab::Devices;
        let text = draw(&mut a, 200, 48);
        for label in [
            "Total",
            "Online",
            "Offline",
            "Source [",
            "Router [",
            "State [",
            "Subscriber list",
            "Subscriber details",
            "Data status",
        ] {
            assert!(text.contains(label), "missing {label}");
        }
        assert!(!text.contains("996"));
    }

    #[test]
    fn dashboard_refresh_click_and_compact_scroll_use_guarded_actions() {
        let mut a = app(true);
        draw(&mut a, 80, 24);
        let r = a.overview.scroll.area;
        let wheel = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: r.x + 2,
            row: r.y + 2,
            modifiers: KeyModifiers::NONE,
        };
        a.on_mouse(wheel);
        assert!(a.overview.scroll.offset > 0);
        a.modal = Some(Modal::ConfirmSync);
        let before = a.overview.scroll.offset;
        a.on_mouse(wheel);
        assert_eq!(a.overview.scroll.offset, before);
        a.modal = None;
        assert_eq!(a.on_key(key(KeyCode::Char('r'))), Action::Refresh);
        draw(&mut a, 200, 48);
        let r = a.overview.refresh_target();
        assert_eq!(
            a.on_mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: r.x,
                row: r.y,
                modifiers: KeyModifiers::NONE
            }),
            Action::Refresh
        );
    }

    #[test]
    fn dashboard_body_matches_approved_panel_order_and_missing_data() {
        let mut a = app(true);
        let text = draw(&mut a, 200, 48);
        let mut last = 0;
        for label in [
            "Network overview",
            "IPoE clients",
            "LibreQoS DPDK Traffic",
            "WAN overview",
            "Operational alerts",
        ] {
            let pos = text
                .find(label)
                .unwrap_or_else(|| panic!("missing {label}"));
            assert!(pos >= last);
            last = pos;
        }
        assert!(text.contains("Dataplane health"));
        assert!(text.contains("unavailable"));
        assert!(!text.contains("812 Mbps"));
    }

    #[test]
    fn approved_shell_starts_on_dashboard_and_has_seven_routes() {
        let mut a = app(false);
        assert_eq!(a.tab.title(), "Dashboard");
        for (key_char, label) in [
            ('1', "Dashboard"),
            ('2', "Subscribers"),
            ('3', "Routers"),
            ('4', "LibreQoS"),
            ('5', "Workers"),
            ('6', "Settings"),
            ('7', "Journal"),
        ] {
            a.on_key(key(KeyCode::Char(key_char)));
            assert_eq!(a.tab.title(), label);
        }
        let text = draw(&mut a, 200, 40);
        for label in [
            "TERMINAL CONSOLE",
            "OPERATIONS",
            "MANAGEMENT",
            "Profiles",
            "planned",
            "WAN plans",
        ] {
            assert!(text.contains(label), "missing {label}: {text}");
        }
    }

    #[test]
    fn wide_header_labels_its_local_time_offset_and_unknown_startup_clock() {
        let mut a = app(true);
        assert!(draw(&mut a, 200, 40).contains("time unknown"));
        a.on_tick(1793505614);
        let text = draw(&mut a, 200, 40);
        assert!(
            regex::Regex::new(r"[0-9]{2}:[0-9]{2} [+-][0-9]{2}:[0-9]{2}")
                .unwrap()
                .is_match(text.lines().next().unwrap())
        );
    }

    #[test]
    fn modal_blocks_navigation_clicks_and_old_targets_do_not_survive_resize() {
        let mut a = app(false);
        draw(&mut a, 200, 40);
        let r = a.tab_rects[1];
        a.modal = Some(Modal::ConfirmSync);
        let old = a.tab;
        let click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: r.x + 1,
            row: r.y,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(a.on_mouse(click), Action::None);
        assert_eq!(a.tab, old);
        a.modal = None;
        draw(&mut a, 40, 10);
        a.on_mouse(click);
        assert_eq!(a.tab, old);
    }

    #[test]
    fn sidebar_clicks_and_theme_use_the_current_draw_and_preserve_editor_input() {
        let mut a = app(false);
        for index in 0..7 {
            draw(&mut a, 200, 40);
            let r = a.tab_rects[index];
            a.on_mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: r.x + 1,
                row: r.y,
                modifiers: KeyModifiers::NONE,
            });
            assert_eq!(
                a.tab.title(),
                [
                    "Dashboard",
                    "Subscribers",
                    "Routers",
                    "LibreQoS",
                    "Workers",
                    "Settings",
                    "Journal"
                ][index]
            );
        }
        let r = a.theme_rect;
        a.on_mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: r.x,
            row: r.y,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(a.palette.theme, crate::config::Theme::Light);
        a.tab = Tab::Settings;
        a.settings.select("runtime.id_length");
        a.on_key(key(KeyCode::Enter));
        draw(&mut a, 200, 40);
        let r = a.tab_rects[0];
        a.on_mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: r.x + 1,
            row: r.y,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(a.tab, Tab::Settings);
        assert!(a.settings.capturing());
        a.on_key(key(KeyCode::Esc));
        a.tab = Tab::Network;
        a.on_key(key(KeyCode::Esc));
        assert_eq!(a.tab, Tab::Devices);
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
    fn keys_1_to_7_select_the_tabs_and_q_quits() {
        let mut a = app(false);
        assert_eq!(a.tab, Tab::Dashboard);
        for (c, t) in ['1', '2', '3', '4', '5', '6', '7']
            .into_iter()
            .zip(Tab::ALL)
        {
            assert_eq!(a.on_key(key(KeyCode::Char(c))), Action::None);
            assert_eq!(a.tab, t);
        }
        a.on_key(key(KeyCode::Tab));
        assert_eq!(a.tab, Tab::Journal);
        a.on_key(key(KeyCode::Char('1')));
        assert_eq!(a.tab, Tab::Dashboard);
        a.on_key(key(KeyCode::BackTab));
        assert_eq!(a.tab, Tab::Journal);
        assert_eq!(a.on_key(key(KeyCode::Char('q'))), Action::Quit);
    }

    #[test]
    fn bridge_health_dashboard_retains_freshness_when_worker_snapshot_is_stale() {
        let mut a = app(true);
        let mut t = crate::worker_util::Telemetry::default();
        let report = |up| {
            serde_json::from_value(serde_json::json!({"status":"degraded", "version":"v3", "uptime_ms":up,"workers":[], "degraded_subsystems":["stats_summary"]})).unwrap()
        };
        t.health.record(Ok(report(1)), 90);
        t.health.record(Ok(report(10001)), 100);
        a.set_data(
            Data {
                worker_util: Some(Arc::new(t)),
                ..Data::default()
            },
            100,
        );
        let s = draw(&mut a, 240, 48);
        assert!(
            s.contains("Bridge health LIVE")
                && s.contains("Status: degraded")
                && s.contains("stats_summary")
        );
        a.on_tick(116);
        assert!(draw(&mut a, 240, 48).contains("Bridge health STALE (last known)"));
    }

    #[test]
    fn queue_totals_render_in_dashboard_and_workers_with_stale_delta_suppression() {
        let mut a = app(true);
        let telemetry: crate::worker_util::Telemetry = serde_json::from_value(serde_json::json!({
            "version":1,"updated":100,"stale":false,"reason":null,
            "workers":[{"queue_id":0,"busy_ppm":125000,"backlog_mean":3}],
            "queue":{"totals":{"drop_pkts":103,"aqm_marks":205,"backlog_bytes":4096},
                "drop_delta":3,"aqm_delta":5}
        }))
        .unwrap();
        a.set_data(
            Data {
                target: "v3_pair".into(),
                worker_util: Some(Arc::new(telemetry)),
                ..Data::default()
            },
            100,
        );
        for tab in [Tab::Dashboard, Tab::Workers] {
            a.tab = tab;
            let s = draw(&mut a, 240, 48);
            for text in [
                "Queues LIVE",
                "whole bridge",
                "Drops: 103",
                "AQM marks: 205",
                "New/sample: drops 3",
                "Backlog: 4096 bytes",
            ] {
                assert!(s.contains(text), "{tab:?} missing {text}: {s}");
            }
            a.on_tick(107);
            let s = draw(&mut a, 240, 48);
            assert!(s.contains("Queues STALE (last known)"), "{tab:?}: {s}");
            assert!(
                s.contains("Drops: 103") && s.contains("New/sample: drops unknown"),
                "{s}"
            );
            a.on_tick(100);
        }
    }

    #[test]
    fn worker_util_expires_on_timer_ticks_without_loader_updates() {
        let mut a = app(true);
        a.tab = Tab::Workers;
        a.set_data(
            Data {
                target: "v3_pair".into(),
                worker_util: Some(Arc::new(crate::worker_util::Telemetry {
                    version: 1,
                    updated: 100,
                    stale: false,
                    reason: None,
                    workers: vec![crate::worker_util::WorkerRow {
                        queue_id: 0,
                        busy_ppm: 125000,
                        backlog_mean: Some(3),
                        ..Default::default()
                    }],
                    pressure: None,
                    traffic: Default::default(),
                    queue: None,
                    classification: Default::default(),
                    health: Default::default(),
                })),
                ..Data::default()
            },
            100,
        );
        assert!(draw(&mut a, 160, 40).contains("Snapshot LIVE"));
        a.on_tick(107);
        assert!(draw(&mut a, 160, 40).contains("Snapshot STALE"));
    }

    #[test]
    fn a_click_on_a_tab_title_selects_it() {
        let mut a = app(false);
        draw(&mut a, 120, 30);
        let r = a.tab_rects[6];
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
    fn compact_navigation_keeps_all_routes_and_selected_parent_view() {
        let mut a = app(false);
        a.tab = Tab::Network;
        let mut t = Terminal::new(TestBackend::new(120, 30)).unwrap();
        t.draw(|f| a.draw(f)).unwrap();
        let buf = t.backend().buffer().clone();
        let r = a.tab_rects[1];
        let row: String = (0..120)
            .map(|x| buf[(x, r.y)].symbol().to_string())
            .collect();
        for want in [
            "1Dash",
            "2Subs",
            "3Routers",
            "4LibreQoS",
            "5Workers",
            "6Settings",
            "7Journal",
        ] {
            assert!(row.contains(want), "{want}: {row}");
        }
        // The active tab name takes the selection colours; the other names do not.
        assert_eq!(buf[(r.x + 2, r.y)].bg, a.palette.selection().bg.unwrap());
        let r0 = a.tab_rects[0];
        assert_ne!(buf[(r0.x + 2, r0.y)].bg, a.palette.selection().bg.unwrap());
    }

    #[test]
    fn a_long_hint_wraps_without_a_lost_key() {
        let lines = wrap_hint("[a] one   [b] two  [c] three  [d] four", 14);
        assert_eq!(lines, ["[a] one", "[b] two", "[c] three", "[d] four"]);
    }

    #[test]
    fn every_key_of_each_hint_shows_at_80_columns() {
        let mut a = app(false);
        let mut cases: Vec<(Tab, Option<Modal>)> = Tab::ALL.iter().map(|&t| (t, None)).collect();
        cases.push((Tab::Devices, Some(Modal::ConfirmSync)));
        cases.push((Tab::Devices, Some(Modal::Discard)));
        for (tab, modal) in cases {
            a.tab = tab;
            a.modal = modal;
            let hint = a.key_hint();
            let s = draw(&mut a, 80, 24);
            let footer: String = s.lines().skip(20).collect::<Vec<_>>().join("\n");
            let mut rest = hint.as_str();
            while let Some(i) = rest.find('[') {
                let j = i + rest[i..].find(']').unwrap() + 1;
                assert!(
                    footer.contains(&rest[i..j]),
                    "{tab:?} {}: {footer}",
                    &rest[i..j]
                );
                rest = &rest[j..];
            }
            assert!(
                !footer
                    .lines()
                    .any(|l| l.starts_with(' ') && !l.trim().is_empty()),
                "{footer}"
            );
        }
    }

    #[test]
    fn a_wide_header_is_one_line() {
        let mut a = app(true);
        let s = draw(&mut a, 160, 30);
        let first = s.lines().next().unwrap();
        assert!(first.contains("Dashboard") && first.contains("SYNC AGE unknown"));
        assert!(s.contains("READ-ONLY") && s.contains("TERMINAL CONSOLE"));
        let s = draw(&mut a, 80, 24);
        assert!(s.lines().nth(2).unwrap().contains("1Dash"));
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
        let top = |a: &mut App| {
            draw(a, 140, 24)
                .lines()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ")
        };
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
                ..Default::default()
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
    fn n_opens_the_parent_node_in_the_network_tab() {
        let mut a = app(false);
        a.tab = Tab::Devices;
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
        a.on_key(key(KeyCode::Char('n')));
        assert_eq!(
            (a.tab, a.network.selected_name()),
            (Tab::Network, Some("AP"))
        );
    }

    #[test]
    fn search_text_takes_the_global_keys() {
        let mut a = app(false);
        a.tab = Tab::Devices;
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
        a.on_key(key(KeyCode::Char('7')));
        a
    }

    #[test]
    fn journal_keyboard_focus_and_expansion_use_approved_controls() {
        let mut a = journal_app(&["info ready", "error failed"]);
        a.on_key(key(KeyCode::Tab));
        a.on_key(key(KeyCode::Enter));
        assert_eq!(a.journal.level, 1);
        a.on_key(key(KeyCode::Char('e')));
        let expanded = draw(&mut a, 200, 40);
        assert!(expanded.contains("Selected entry"));
        assert!(!expanded.contains("Service journal"));
        a.on_key(key(KeyCode::Esc));
        assert!(draw(&mut a, 200, 40).contains("Service journal"));
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
        assert!(draw(&mut a, 120, 24).contains("cannot read"));
    }

    #[test]
    fn loader_errors_show_in_the_footer() {
        let mut a = app(false);
        let d = Data {
            errors: vec!["cannot read /x/ShapedDevices.csv: not found".into()],
            ..Data::default()
        };
        a.set_data(d, 0);
        let s = draw(&mut a, 120, 24);
        assert!(s.contains("cannot read /x/ShapedDevices.csv"), "{s}");
    }

    #[test]
    fn settings_tab_shows_the_editor() {
        let mut a = app(false);
        a.on_key(key(KeyCode::Char('6')));
        let screen = draw(&mut a, 140, 30);
        assert!(screen.contains("Draft: 0 changes"), "{screen}");
        assert!(
            screen.contains("Loaded") && screen.contains("Draft") && screen.contains("apply"),
            "{screen}"
        );
    }

    #[test]
    fn settings_messages_show_in_the_footer() {
        let mut a = app(true);
        a.on_key(key(KeyCode::Char('6')));
        a.settings.select("safety.enabled");
        a.on_key(key(KeyCode::Char(' ')));
        let s = draw(&mut a, 140, 30);
        assert!(s.contains("read-only"), "{s}");
    }

    #[test]
    fn global_dialog_mouse_buttons_use_confirmation_gates() {
        let mut a = app(false);
        a.on_key(key(KeyCode::Char('S')));
        draw(&mut a, 200, 40);
        let click = |r: Rect| MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: r.x,
            row: r.y,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(a.on_mouse(click(a.dialog_buttons[1])), Action::None);
        assert!(a.modal.is_none());
        a.on_key(key(KeyCode::Char('S')));
        draw(&mut a, 200, 40);
        assert_eq!(a.on_mouse(click(a.dialog_buttons[0])), Action::SyncNow);
        a.on_key(key(KeyCode::Char('6')));
        a.settings.select("safety.enabled");
        a.on_key(key(KeyCode::Char(' ')));
        a.on_key(key(KeyCode::Char('q')));
        draw(&mut a, 200, 40);
        assert_eq!(a.modal, Some(Modal::Discard));
        assert_eq!(a.on_mouse(click(a.dialog_buttons[0])), Action::Quit);
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
        assert!(s.lines().last().unwrap().contains("[y] sync now"), "{s}");
        assert_eq!(a.on_key(key(KeyCode::Char('x'))), Action::None);
        assert_eq!(a.modal, Some(Modal::ConfirmSync));
        assert_eq!(a.on_key(key(KeyCode::Esc)), Action::None);
        assert!(a.modal.is_none());
        a.on_key(key(KeyCode::Char('6')));
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
        a.on_key(key(KeyCode::Char('6')));
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
        a.on_key(key(KeyCode::Char('6')));
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
        a.on_key(key(KeyCode::Char('6')));
        a.settings.select("safety.enabled");
        a.on_key(key(KeyCode::Char(' ')));
        assert!(matches!(
            a.on_key(key(KeyCode::Char('v'))),
            Action::Preview(_)
        ));
    }
    #[test]
    fn minimum_size_preserves_state_and_clears_mouse_targets() {
        let mut a = app(false);
        a.devices.query = "kept".into();
        a.tab = Tab::Settings;
        a.settings.select("runtime.id_length");
        a.on_key(key(KeyCode::Enter));
        let draft = a.settings.draft.as_ref().unwrap().text();
        let pending = Sent {
            kind: Kind::SyncNow,
            at: 1,
            pid: 7,
            seq: 1,
        };
        a.pending = Some(pending.clone());
        draw(&mut a, 120, 30);
        for (w, h) in [(79, 24), (80, 23), (1, 1), (0, 0)] {
            let text = draw(&mut a, w, h);
            if w > 40 {
                assert!(text.contains("terminal too small: min 80x24"));
            }
            assert!(a.tab_rects.is_empty());
            assert_eq!(a.settings.areas, [Rect::default(); 3]);
            assert_eq!(a.devices.query, "kept");
            assert!(a.settings.capturing());
            assert_eq!(a.settings.draft.as_ref().unwrap().text(), draft);
            assert_eq!(a.pending, Some(pending.clone()));
        }
        draw(&mut a, 80, 24);
        assert!(!a.tab_rects.is_empty());
    }
    #[test]
    fn shell_pills_at_minimum_size() {
        let mut a = app(true);
        let s = draw(&mut a, 80, 24);
        for text in [
            "READ-ONLY",
            "service unknown",
            "target unknown",
            "cycle unknown",
            "next scan unknown",
            "[q]",
        ] {
            assert!(s.contains(text), "{text}: {s}");
        }
    }
    #[test]
    fn all_tabs_themes_and_size_matrix() {
        use crate::config::Theme;
        for theme in [Theme::Dark, Theme::Light, Theme::Mono] {
            let mut a = app(false);
            a.palette = Palette::new(theme);
            for (w, h) in [(80, 24), (100, 30), (120, 40)] {
                for tab in Tab::ALL {
                    a.tab = tab;
                    let text = draw(&mut a, w, h);
                    assert!(text.contains("[q]"), "{theme:?} {tab:?}: {text}");
                    assert_eq!(a.tab_rects.len(), Tab::ALL.len());
                    assert!(text.contains("[q]"));
                }
            }
        }
    }
    #[test]
    fn minimum_dashboard_keeps_outcomes_and_long_router_states() {
        let mut a = app(false);
        a.tab = Tab::Dashboard;
        a.set_data(Data { status:Some(Arc::new(serde_json::json!({
            "updated":100,"cycle_ms":5,"cycle_seq":3,"trigger":"timer","files_changed":true,"reload":"failed","telegram":"failed",
            "routers":[{"name":"router-with-a-very-long-name-aaaaaaaaaaaaaaaaaa","state":"failed"},{"name":"router-with-a-very-long-name-bbbbbbbbbbbbbbbbbb","state":"offline"}]
        }))), ..Data::default() },110);
        draw(&mut a, 80, 24);
        a.on_key(key(KeyCode::End));
        let s = draw(&mut a, 80, 24);
        let router_text = s.clone();
        assert!(
            router_text.contains(": failed") && router_text.contains(": offline"),
            "{router_text}"
        );
        for text in [
            "write",
            "reload: failed",
            "telegram: failed",
            "failed",
            "offline",
        ] {
            assert!(s.contains(text), "{text}: {s}");
        }
    }
    #[test]
    fn light_popup_restores_canvas_after_clear() {
        let mut a = app(false);
        a.palette = Palette::new(crate::config::Theme::Light);
        a.on_key(key(KeyCode::Char('S')));
        let mut t = Terminal::new(TestBackend::new(120, 30)).unwrap();
        t.draw(|f| a.draw(f)).unwrap();
        assert_eq!(
            t.backend().buffer()[(60, 15)].bg,
            a.palette.canvas().bg.unwrap()
        );
    }
    #[test]
    fn action_messages_keep_normal_style() {
        let mut a = app(false);
        a.message = Some("fixture completed".into());
        let mut t = Terminal::new(TestBackend::new(120, 30)).unwrap();
        t.draw(|f| a.draw(f)).unwrap();
        let b = t.backend().buffer();
        let y = (0..30)
            .find(|&y| {
                (0..120)
                    .map(|x| b[(x, y)].symbol())
                    .collect::<String>()
                    .contains("fixture completed")
            })
            .unwrap();
        assert_eq!(b[(0, y)].fg, a.palette.role(Role::Base).fg.unwrap());
    }
}
