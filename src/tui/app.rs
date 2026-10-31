//! The TUI state, the keys, the mouse, and the draw code.

use ratatui::Frame;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modal {
    ConfirmSync,
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
    config_text: String,
    config_error: Option<String>,
    settings_offset: u16,
}

impl App {
    pub fn new(read_only: bool, config_text: String, config_error: Option<String>) -> Self {
        App {
            tab: Tab::Devices,
            read_only,
            message: None,
            modal: None,
            tab_rects: Vec::new(),
            config_text,
            config_error,
            settings_offset: 0,
        }
    }

    pub fn on_key(&mut self, k: KeyEvent) -> Action {
        if let Some(m) = self.modal {
            self.modal = None;
            return match (m, k.code) {
                (Modal::ConfirmSync, KeyCode::Char('y')) => Action::SyncNow,
                _ => {
                    self.message = Some("cancelled".into());
                    Action::None
                }
            };
        }
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            return Action::Quit;
        }
        match k.code {
            KeyCode::Char('q') => return Action::Quit,
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

    /// A key for the current tab.
    fn tab_key(&mut self, k: KeyEvent) -> Action {
        if self.tab == Tab::Settings {
            match k.code {
                KeyCode::Down => self.settings_offset = self.settings_offset.saturating_add(1),
                KeyCode::Up => self.settings_offset = self.settings_offset.saturating_sub(1),
                KeyCode::PageDown => self.settings_offset = self.settings_offset.saturating_add(20),
                KeyCode::PageUp => self.settings_offset = self.settings_offset.saturating_sub(20),
                _ => {}
            }
        }
        Action::None
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
            Tab::Settings => self.draw_settings(f, main),
            _ => f.render_widget(
                Paragraph::new("loading").block(Block::default().borders(Borders::ALL)),
                main,
            ),
        }
        let keys = "q quit  1-5/Tab tabs  S sync now  R reload";
        let text = match &self.message {
            Some(m) => format!("{m}  |  {keys}"),
            None => keys.into(),
        };
        f.render_widget(
            Paragraph::new(text).style(Style::new().fg(Color::Gray)),
            footer,
        );
        if self.modal == Some(Modal::ConfirmSync) {
            let r = center(main, 50, 5);
            f.render_widget(Clear, r);
            f.render_widget(
                Paragraph::new(
                    "Sync now releases a guard hold.\nPress y to send, another key to cancel.",
                )
                .block(Block::default().borders(Borders::ALL).title(" sync now ")),
                r,
            );
        }
    }

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let mut spans = vec![Span::styled(
            " lqosync3 ",
            Style::new().add_modifier(Modifier::BOLD),
        )];
        if let Some(e) = &self.config_error {
            spans.push(Span::styled(
                format!(" config error: {e} "),
                Style::new().fg(Color::Red),
            ));
        }
        if self.read_only {
            spans.push(Span::styled(
                " READ-ONLY (not root) ",
                Style::new().fg(Color::Black).bg(Color::Yellow),
            ));
        }
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

    fn draw_settings(&self, f: &mut Frame, area: Rect) {
        f.render_widget(
            Paragraph::new(self.config_text.as_str())
                .scroll((self.settings_offset, 0))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" config file (read-only; editor: Plan 4b) "),
                ),
            area,
        );
    }
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
    fn settings_tab_shows_the_config_text_read_only() {
        let mut a = app(false);
        a.on_key(key(KeyCode::Char('3')));
        let screen = draw(&mut a, 100, 20);
        assert!(screen.contains("mode = \"apply\""), "{screen}");
        assert!(screen.contains("editor: Plan 4b"), "{screen}");
    }
}
