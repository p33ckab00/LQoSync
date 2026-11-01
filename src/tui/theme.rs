//! Shared terminal styles. All state meanings also have text labels.

use crate::config::Theme;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders};

#[derive(Debug, Clone, Copy)]
pub enum Role {
    Base,
    Secondary,
    Accent,
    Ok,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub theme: Theme,
}

impl Palette {
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }

    pub fn color(&self, color: Color) -> Color {
        use Color::Rgb;
        match (self.theme, color) {
            (Theme::Mono, _) => Color::Reset,
            (Theme::Dark, Color::Black) => Rgb(11, 16, 20),
            (Theme::Light, Color::Black) => Rgb(36, 55, 68),
            (_, Color::White) => Rgb(244, 246, 248),
            (Theme::Dark, Color::Gray) => Rgb(213, 221, 229),
            (Theme::Light, Color::Gray) => Rgb(36, 55, 68),
            (Theme::Dark, Color::DarkGray) => Rgb(54, 83, 96),
            (Theme::Light, Color::DarkGray) => Rgb(113, 132, 139),
            (Theme::Dark, Color::Cyan) => Rgb(89, 199, 213),
            (Theme::Light, Color::Cyan) => Rgb(0, 106, 128),
            (Theme::Dark, Color::Green) => Rgb(139, 200, 140),
            (Theme::Light, Color::Green) => Rgb(43, 116, 83),
            (Theme::Dark, Color::Yellow) => Rgb(224, 182, 109),
            (Theme::Light, Color::Yellow) => Rgb(129, 87, 0),
            (Theme::Dark, Color::Red) => Rgb(231, 118, 118),
            (Theme::Light, Color::Red) => Rgb(178, 57, 63),
            _ => color,
        }
    }

    pub fn role(&self, role: Role) -> Style {
        let color = match role {
            Role::Base => self.color(Color::Gray),
            Role::Secondary => match self.theme {
                Theme::Dark => Color::Rgb(133, 147, 159),
                Theme::Light => Color::Rgb(96, 117, 131),
                Theme::Mono => Color::Reset,
            },
            Role::Accent => self.color(Color::Cyan),
            Role::Ok => self.color(Color::Green),
            Role::Warning => self.color(Color::Yellow),
            Role::Error => self.color(Color::Red),
        };
        let mut style = Style::new().fg(color);
        if self.theme == Theme::Mono && matches!(role, Role::Secondary) {
            style = style.add_modifier(Modifier::DIM);
        }
        if matches!(role, Role::Accent | Role::Warning | Role::Error) {
            style = style.add_modifier(Modifier::BOLD);
        }
        style
    }

    pub fn canvas(&self) -> Style {
        self.role(Role::Base).bg(match self.theme {
            Theme::Dark => Color::Rgb(11, 16, 20),
            Theme::Light => Color::Rgb(244, 246, 248),
            Theme::Mono => Color::Reset,
        })
    }

    pub fn selection(&self) -> Style {
        match self.theme {
            Theme::Dark => Style::new()
                .fg(Color::Rgb(213, 239, 243))
                .bg(Color::Rgb(23, 58, 69)),
            Theme::Light => Style::new()
                .fg(Color::Rgb(22, 72, 88))
                .bg(Color::Rgb(215, 234, 240)),
            Theme::Mono => Style::new()
                .fg(Color::Reset)
                .bg(Color::Reset)
                .add_modifier(Modifier::REVERSED),
        }
        .add_modifier(Modifier::BOLD)
        .remove_modifier(Modifier::DIM)
    }

    pub fn row(&self, index: usize, selected: bool) -> Style {
        if selected {
            return self.selection();
        }
        let base = self.canvas();
        if index.is_multiple_of(2) {
            return base;
        }
        match self.theme {
            Theme::Dark => base.bg(Color::Rgb(17, 27, 34)),
            Theme::Light => base.bg(Color::Rgb(233, 239, 243)),
            Theme::Mono => base.add_modifier(Modifier::BOLD),
        }
    }

    pub fn panel<'a>(&self, title: Line<'a>, focused: bool) -> Block<'a> {
        Block::default()
            .style(self.canvas())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if focused {
                self.role(Role::Accent)
            } else {
                Style::new().fg(self.color(Color::DarkGray))
            })
            .title(title.style(self.role(Role::Accent)))
    }

    pub fn badge<'a>(&self, text: &'a str, role: Role, selected: bool) -> Span<'a> {
        let mut style = self
            .role(role)
            .remove_modifier(Modifier::DIM)
            .add_modifier(Modifier::BOLD);
        if selected {
            style = self.selection();
        } else if self.theme != Theme::Mono {
            let rgb = match (self.theme, role) {
                (Theme::Dark, Role::Ok) => (18, 39, 33),
                (Theme::Dark, Role::Warning) => (48, 37, 20),
                (Theme::Dark, Role::Error) => (50, 24, 29),
                (Theme::Light, Role::Ok) => (224, 239, 231),
                (Theme::Light, Role::Warning) => (248, 237, 211),
                (Theme::Light, Role::Error) => (249, 228, 229),
                (Theme::Light, _) => (233, 239, 243),
                _ => (17, 27, 34),
            };
            style = style.bg(Color::Rgb(rgb.0, rgb.1, rgb.2));
        }
        Span::styled(text, style)
    }

    pub fn keys(&self, hint: &str) -> Line<'static> {
        let mut spans = Vec::new();
        let mut rest = hint;
        while let Some(start) = rest.find('[') {
            let Some(end) = rest[start..].find(']') else {
                break;
            };
            spans.push(Span::styled(
                rest[..start].to_string(),
                self.role(Role::Base),
            ));
            let stop = start + end + 1;
            spans.push(Span::styled(
                rest[start..stop].to_string(),
                self.role(Role::Accent),
            ));
            rest = &rest[stop..];
        }
        spans.push(Span::styled(rest.to_string(), self.role(Role::Base)));
        Line::from(spans)
    }
}

/// Limit text by terminal cells, with an ellipsis for omitted text.
pub fn clip(text: &str, width: usize) -> String {
    if Line::from(text).width() <= width {
        return text.into();
    }
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0;
    for c in text.chars() {
        let n = Line::from(c.to_string()).width();
        if used + n >= width {
            break;
        }
        out.push(c);
        used += n;
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn palette_roles_selection_and_mono() {
        assert_eq!(
            Palette::new(Theme::Dark).role(Role::Accent).fg,
            Some(Color::Rgb(89, 199, 213))
        );
        assert_eq!(
            Palette::new(Theme::Light).role(Role::Accent).fg,
            Some(Color::Rgb(0, 106, 128))
        );
        for theme in [Theme::Dark, Theme::Light, Theme::Mono] {
            let p = Palette::new(theme);
            assert_eq!(p.badge("dup IP", Role::Error, true).style, p.selection());
        }
        let p = Palette::new(Theme::Mono);
        assert_eq!(
            p.badge("warning", Role::Warning, false).style.fg,
            Some(Color::Reset)
        );
        let p = Palette::new(Theme::Light);
        assert_eq!(
            p.badge("warning", Role::Warning, false).style.bg,
            Some(Color::Rgb(248, 237, 211))
        );
    }
    #[test]
    fn keys_preserve_text_and_highlight_brackets() {
        let p = Palette::new(Theme::Dark);
        let raw = "[/] search  [o] sort  [q] quit";
        let line = p.keys(raw);
        assert_eq!(line.to_string(), raw);
        assert!(
            line.spans
                .iter()
                .any(|s| s.content == "[q]" && s.style.fg == Some(Color::Rgb(89, 199, 213)))
        );
    }
}
