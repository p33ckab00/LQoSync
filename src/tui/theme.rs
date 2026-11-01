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
            (Theme::Dark, Color::Black) => Rgb(0, 0, 0),
            (Theme::Light, Color::Black) => Rgb(36, 55, 68),
            (_, Color::White) => Rgb(244, 246, 248),
            (Theme::Dark, Color::Gray) => Rgb(0xcc, 0xcc, 0xcc),
            (Theme::Light, Color::Gray) => Rgb(36, 55, 68),
            (Theme::Dark, Color::DarkGray) => Rgb(0x30, 0x30, 0x30),
            (Theme::Light, Color::DarkGray) => Rgb(113, 132, 139),
            (Theme::Dark, Color::Cyan) => Rgb(0xee, 0xee, 0xee),
            (Theme::Light, Color::Cyan) => Rgb(0, 106, 128),
            (Theme::Dark, Color::Green) => Rgb(0x77, 0xca, 0x9b),
            (Theme::Light, Color::Green) => Rgb(43, 116, 83),
            (Theme::Dark, Color::Yellow) => Rgb(0xcb, 0xc0, 0x6c),
            (Theme::Light, Color::Yellow) => Rgb(129, 87, 0),
            (Theme::Dark, Color::Red) => Rgb(0xdc, 0x4c, 0x4c),
            (Theme::Light, Color::Red) => Rgb(178, 57, 63),
            _ => color,
        }
    }

    pub fn role(&self, role: Role) -> Style {
        let color = match role {
            Role::Base => self.color(Color::Gray),
            Role::Secondary => match self.theme {
                Theme::Dark => Color::Rgb(0x80, 0x80, 0x80),
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
            Theme::Dark => Color::Rgb(0, 0, 0),
            Theme::Light => Color::Rgb(244, 246, 248),
            Theme::Mono => Color::Reset,
        })
    }

    pub fn selection(&self) -> Style {
        match self.theme {
            Theme::Dark => Style::new()
                .fg(Color::Rgb(0xee, 0xee, 0xee))
                .bg(Color::Rgb(0x6a, 0x2f, 0x2f)),
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
            Theme::Dark => base.bg(Color::Rgb(0x0e, 0x0e, 0x0e)),
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
                _ => (0x1a, 0x1a, 0x1a),
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
            spans.push(Span::styled(rest[start..stop].to_string(), self.hotkey()));
            rest = &rest[stop..];
        }
        spans.push(Span::styled(rest.to_string(), self.role(Role::Base)));
        Line::from(spans)
    }
}

/// The colour scales of the graphs and meters. The values are the btop++ Default theme
/// (`cpu_*`, `cached_*`, `free_*`, `used_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grad {
    Cpu,
    Mem,
    Free,
    Used,
}

/// The box kinds. Each kind has its own border colour, as the btop++ boxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxKind {
    Cpu,
    Mem,
    Net,
    Proc,
}

const SUPERSCRIPT: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];

fn hex(v: u32) -> Color {
    Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

fn lerp(a: u32, b: u32, t: f64) -> Color {
    let ch = |shift: u32| {
        let (x, y) = (((a >> shift) & 0xff) as f64, ((b >> shift) & 0xff) as f64);
        (x + (y - x) * t).round() as u8
    };
    Color::Rgb(ch(16), ch(8), ch(0))
}

impl Palette {
    /// A colour of a scale at `t` (0 = start, 0.5 = middle, 1 = end).
    pub fn gradient(&self, g: Grad, t: f64) -> Color {
        if self.theme == Theme::Mono {
            return Color::Reset;
        }
        let (start, mid, end) = match g {
            Grad::Cpu => (0x77ca9b, 0xcbc06c, 0xdc4c4c),
            Grad::Mem => (0x163350, 0x74e6fc, 0x26c5ff),
            Grad::Free => (0x384f21, 0xb5e685, 0xdcff85),
            Grad::Used => (0x592b26, 0xd9626d, 0xff4769),
        };
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if t <= 0.5 {
            lerp(start, mid, t * 2.0)
        } else {
            lerp(mid, end, (t - 0.5) * 2.0)
        }
    }

    /// The border colour of a box kind.
    pub fn box_color(&self, kind: BoxKind) -> Color {
        let (dark, light) = match kind {
            BoxKind::Cpu => (0x556d59, 0x3f6b46),
            BoxKind::Mem => (0x6c6c4b, 0x6b6b2f),
            BoxKind::Net => (0x5c588d, 0x4a4596),
            BoxKind::Proc => (0x805252, 0x8f4242),
        };
        match self.theme {
            Theme::Dark => hex(dark),
            Theme::Light => hex(light),
            Theme::Mono => Color::Reset,
        }
    }

    /// The colour of a hotkey (btop++ `hi_fg`).
    pub fn hotkey(&self) -> Style {
        match self.theme {
            Theme::Dark => Style::new().fg(hex(0xb54040)).add_modifier(Modifier::BOLD),
            Theme::Light => Style::new().fg(hex(0xa02828)).add_modifier(Modifier::BOLD),
            Theme::Mono => Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        }
    }

    /// The colour of a title (btop++ `title`).
    pub fn title_style(&self) -> Style {
        match self.theme {
            Theme::Dark => Style::new().fg(hex(0xeeeeee)),
            Theme::Light => Style::new().fg(hex(0x1c2b36)),
            Theme::Mono => Style::new(),
        }
        .add_modifier(Modifier::BOLD)
    }

    fn meter_bg(&self) -> Color {
        match self.theme {
            Theme::Dark => hex(0x404040),
            Theme::Light => hex(0xc8cdd2),
            Theme::Mono => Color::Reset,
        }
    }

    /// A box title in btop++ form: `┐¹name┌`. The brackets take the style of the box border.
    pub fn title(&self, name: &str, hotkey: Option<u8>) -> Line<'static> {
        let mut spans = vec![Span::raw("┐")];
        if let Some(k) = hotkey {
            spans.push(Span::styled(
                SUPERSCRIPT[(k % 10) as usize].to_string(),
                self.hotkey(),
            ));
        }
        spans.push(Span::styled(name.to_string(), self.title_style()));
        spans.push(Span::raw("┌"));
        Line::from(spans)
    }

    /// A box with rounded corners, the border colour of its kind, and a btop++ title.
    pub fn boxed(&self, kind: BoxKind, name: &str, hotkey: Option<u8>) -> Block<'static> {
        let border = Style::new().fg(self.box_color(kind));
        let mut title = self.title(name, hotkey);
        let last = title.spans.len() - 1;
        title.spans[0].style = border;
        title.spans[last].style = border;
        Block::default()
            .style(self.canvas())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border)
            .title(title)
    }

    /// A meter of `■` cells: the filled part takes the colours of the scale, the rest is dim.
    pub fn meter(&self, ratio: f64, width: u16, g: Grad) -> Line<'static> {
        let w = width as usize;
        let ratio = if ratio.is_finite() {
            ratio.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let filled = ((ratio * w as f64).round() as usize).min(w);
        let spans = (0..w)
            .map(|i| {
                if i < filled {
                    let t = i as f64 / (w.max(2) - 1) as f64;
                    Span::styled("■", Style::new().fg(self.gradient(g, t)))
                } else if self.theme == Theme::Mono {
                    Span::raw("·")
                } else {
                    Span::styled("■", Style::new().fg(self.meter_bg()))
                }
            })
            .collect::<Vec<_>>();
        Line::from(spans)
    }

    /// A filled braille area graph of the last samples (two samples for each cell), right-aligned.
    /// Each row takes the colour of its height on the scale.
    pub fn area_graph(
        &self,
        samples: &[u64],
        scale: u64,
        width: u16,
        height: u16,
        g: Grad,
    ) -> Vec<Line<'static>> {
        const LEFT: [u32; 4] = [0x01, 0x02, 0x04, 0x40];
        const RIGHT: [u32; 4] = [0x08, 0x10, 0x20, 0x80];
        let (w, h) = (width as usize, height as usize);
        let dots = h * 4;
        let take = samples.len().min(w * 2);
        let mut heights = vec![0usize; w * 2];
        for (k, v) in samples[samples.len() - take..].iter().enumerate() {
            heights[w * 2 - take + k] = if scale == 0 {
                0
            } else {
                ((((*v).min(scale)) as f64 / scale as f64) * dots as f64).round() as usize
            };
        }
        (0..h)
            .map(|r| {
                let rb = h - 1 - r;
                let text: String = (0..w)
                    .map(|c| {
                        let mut bits = 0;
                        for j in 0..4 {
                            let dot = rb * 4 + (3 - j);
                            if heights[2 * c] > dot {
                                bits |= LEFT[j];
                            }
                            if heights[2 * c + 1] > dot {
                                bits |= RIGHT[j];
                            }
                        }
                        if bits == 0 {
                            ' '
                        } else {
                            char::from_u32(0x2800 + bits).unwrap_or(' ')
                        }
                    })
                    .collect();
                let t = (rb as f64 + 0.5) / h.max(1) as f64;
                Line::styled(text, Style::new().fg(self.gradient(g, t)))
            })
            .collect()
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

    fn texts(lines: &[Line]) -> Vec<String> {
        lines.iter().map(|l| l.to_string()).collect()
    }

    #[test]
    fn btop_gradient_ends() {
        let p = Palette::new(Theme::Dark);
        assert_eq!(p.gradient(Grad::Cpu, 0.0), Color::Rgb(0x77, 0xca, 0x9b));
        assert_eq!(p.gradient(Grad::Cpu, 0.5), Color::Rgb(0xcb, 0xc0, 0x6c));
        assert_eq!(p.gradient(Grad::Cpu, 1.0), Color::Rgb(0xdc, 0x4c, 0x4c));
        assert_eq!(
            Palette::new(Theme::Mono).gradient(Grad::Cpu, 1.0),
            Color::Reset
        );
    }

    #[test]
    fn meter_fills_with_squares() {
        let p = Palette::new(Theme::Dark);
        let l = p.meter(0.5, 10, Grad::Cpu);
        assert_eq!(l.to_string(), "■■■■■■■■■■");
        assert_eq!(l.spans.len(), 10);
        assert_eq!(l.spans[9].style.fg, Some(Color::Rgb(0x40, 0x40, 0x40)));
        assert_ne!(l.spans[0].style.fg, Some(Color::Rgb(0x40, 0x40, 0x40)));
        assert_eq!(
            Palette::new(Theme::Mono)
                .meter(0.5, 4, Grad::Cpu)
                .to_string(),
            "■■··"
        );
    }

    #[test]
    fn area_graph_fills_from_the_bottom() {
        let p = Palette::new(Theme::Dark);
        assert_eq!(
            texts(&p.area_graph(&[10, 10, 10, 10], 10, 2, 2, Grad::Cpu)),
            ["⣿⣿", "⣿⣿"]
        );
        assert_eq!(
            texts(&p.area_graph(&[5, 5, 5, 5], 10, 2, 2, Grad::Cpu)),
            ["  ", "⣿⣿"]
        );
        assert_eq!(texts(&p.area_graph(&[0, 10], 10, 1, 1, Grad::Cpu)), ["⢸"]);
        // Fewer samples than columns: the graph is right-aligned.
        assert_eq!(
            texts(&p.area_graph(&[10, 10], 10, 3, 1, Grad::Cpu)),
            ["  ⣿"]
        );
    }

    #[test]
    fn a_box_title_has_the_btop_brackets_and_hotkey() {
        let p = Palette::new(Theme::Dark);
        let t = p.title("devices", Some(1));
        assert_eq!(t.to_string(), "┐¹devices┌");
        assert_eq!(p.title("node", None).to_string(), "┐node┌");
    }

    #[test]
    fn palette_roles_selection_and_mono() {
        // btop++ Default: titles #eeeeee, text #cccccc, selection #6a2f2f, black canvas.
        let d = Palette::new(Theme::Dark);
        assert_eq!(d.role(Role::Accent).fg, Some(Color::Rgb(0xee, 0xee, 0xee)));
        assert_eq!(d.role(Role::Base).fg, Some(Color::Rgb(0xcc, 0xcc, 0xcc)));
        assert_eq!(d.selection().bg, Some(Color::Rgb(0x6a, 0x2f, 0x2f)));
        assert_eq!(d.canvas().bg, Some(Color::Rgb(0, 0, 0)));
        assert_eq!(d.role(Role::Ok).fg, Some(Color::Rgb(0x77, 0xca, 0x9b)));
        assert_eq!(d.role(Role::Error).fg, Some(Color::Rgb(0xdc, 0x4c, 0x4c)));
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
                .any(|s| s.content == "[q]" && s.style.fg == Some(Color::Rgb(0xb5, 0x40, 0x40)))
        );
    }
}
