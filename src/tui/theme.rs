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
        match (self.theme, color) {
            (Theme::Mono, _) => Color::Reset,
            (Theme::Light, Color::Cyan) => Color::Blue,
            (Theme::Light, Color::Gray) => Color::Black,
            _ => color,
        }
    }

    pub fn role(&self, role: Role) -> Style {
        let color = match role {
            Role::Base => Color::Gray,
            Role::Secondary => {
                if self.theme == Theme::Light {
                    Color::DarkGray
                } else {
                    Color::Gray
                }
            }
            Role::Accent => Color::Cyan,
            Role::Ok => Color::Green,
            Role::Warning => Color::Yellow,
            Role::Error => Color::Red,
        };
        let mut style = Style::new().fg(self.color(color));
        if matches!(role, Role::Secondary) {
            style = style.add_modifier(Modifier::DIM);
        }
        if matches!(role, Role::Accent | Role::Warning | Role::Error) {
            style = style.add_modifier(Modifier::BOLD);
        }
        style
    }

    pub fn canvas(&self) -> Style {
        self.role(Role::Base).bg(match self.theme {
            Theme::Dark => Color::Black,
            Theme::Light => Color::White,
            Theme::Mono => Color::Reset,
        })
    }

    pub fn selection(&self) -> Style {
        match self.theme {
            Theme::Dark => Style::new().fg(Color::Black).bg(Color::Cyan),
            Theme::Light => Style::new().fg(Color::White).bg(Color::Blue),
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
            Theme::Dark => base.bg(Color::DarkGray),
            Theme::Light => base.bg(Color::Gray),
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
            style = match role {
                Role::Error => style.fg(Color::White).bg(Color::Red),
                Role::Warning if self.theme == Theme::Light => {
                    style.fg(Color::Yellow).bg(Color::Black)
                }
                Role::Warning => style.fg(Color::Black).bg(Color::Yellow),
                Role::Ok => style.fg(Color::Black).bg(Color::Green),
                _ => style,
            };
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
            Some(Color::Cyan)
        );
        assert_eq!(
            Palette::new(Theme::Light).role(Role::Accent).fg,
            Some(Color::Blue)
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
            Some(Color::Black)
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
                .any(|s| s.content == "[q]" && s.style.fg == Some(Color::Cyan))
        );
    }
}
