//! Visual tokens and scrolling shared by the approved screen bodies.
use super::theme::{Palette, Role};
use crate::config::Theme;
use ratatui::crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
};

pub fn panel<'a>(title: impl Into<ratatui::text::Line<'a>>, p: &Palette) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(surface(p))
        .title_style(surface(p).add_modifier(Modifier::BOLD))
        .border_style(if p.theme == Theme::Dark {
            Style::new().fg(Color::Rgb(43, 46, 50))
        } else {
            p.role(Role::Secondary)
        })
}
pub fn surface(p: &Palette) -> Style {
    match p.theme {
        Theme::Dark => Style::new()
            .fg(Color::Rgb(241, 243, 245))
            .bg(Color::Rgb(11, 12, 14)),
        _ => p.canvas(),
    }
}
pub fn role(p: &Palette, role: Role) -> Style {
    if p.theme != Theme::Dark {
        return p.role(role);
    }
    surface(p).fg(match role {
        Role::Base => Color::Rgb(241, 243, 245),
        Role::Secondary => Color::Rgb(178, 184, 191),
        Role::Ok => Color::Rgb(43, 211, 145),
        Role::Warning => Color::Rgb(255, 208, 119),
        Role::Error => Color::Rgb(255, 133, 138),
        Role::Accent => Color::Rgb(128, 170, 255),
    })
}

pub fn selected(p: &Palette) -> Style {
    if p.theme == Theme::Dark {
        surface(p).bg(Color::Rgb(44, 64, 82))
    } else {
        p.selection()
    }
}
/// Convert a virtual-body target into the visible terminal viewport.
pub fn viewport_rect(r: Rect, area: Rect, offset: usize) -> Rect {
    let clipped = r.intersection(Rect::new(0, offset as u16, area.width, area.height));
    if clipped.width == 0 || clipped.height == 0 {
        Rect::default()
    } else {
        Rect::new(
            area.x + clipped.x,
            area.y + clipped.y - offset as u16,
            clipped.width,
            clipped.height,
        )
    }
}

pub fn accent(p: &Palette) -> Style {
    if p.theme == Theme::Dark {
        surface(p).fg(Color::Rgb(128, 170, 255))
    } else {
        p.role(Role::Accent)
    }
}

#[derive(Default, Debug)]
pub struct Scroll {
    pub offset: usize,
    pub max: usize,
    pub area: Rect,
    dragging: bool,
    previous_area: Rect,
}
impl Scroll {
    pub fn clear(&mut self) {
        self.area = Rect::default();
    }
    pub fn mouse(&mut self, m: MouseEvent) -> bool {
        let inside = self.area.contains((m.column, m.row).into());
        let track = self.area.width > 0 && m.column == self.area.right() - 1;
        match m.kind {
            MouseEventKind::Up(MouseButton::Left) if self.dragging => {
                self.dragging = false;
            }
            MouseEventKind::ScrollDown if inside => {
                self.offset = self.offset.saturating_add(3).min(self.max)
            }
            MouseEventKind::ScrollUp if inside => self.offset = self.offset.saturating_sub(3),
            MouseEventKind::Down(MouseButton::Left) if inside && track && self.max > 0 => {
                self.dragging = true;
                self.at(m.row);
            }
            MouseEventKind::Drag(MouseButton::Left) if self.dragging => self.at(m.row),
            _ => return false,
        }
        true
    }
    fn at(&mut self, row: u16) {
        self.offset = usize::from(row.saturating_sub(self.area.y))
            .min(self.area.height.saturating_sub(1) as usize)
            * self.max
            / usize::from(self.area.height.saturating_sub(1).max(1));
    }
    pub fn draw(&mut self, f: &mut ratatui::Frame, area: Rect, total: usize, p: &Palette) {
        if self.previous_area != area {
            self.dragging = false;
        }
        self.previous_area = area;
        self.area = area;
        self.max = total.saturating_sub(area.height as usize);
        self.offset = self.offset.min(self.max);
        if self.max == 0 || area.height == 0 || area.width == 0 {
            return;
        }
        let thumb = self.offset * area.height.saturating_sub(1) as usize / self.max;
        for i in 0..area.height {
            f.buffer_mut()[(area.right() - 1, area.y + i)]
                .set_symbol(if i as usize == thumb { "█" } else { "│" })
                .set_style(accent(p));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partially_visible_targets_keep_only_the_visible_rectangle() {
        assert_eq!(
            viewport_rect(Rect::new(2, 5, 20, 10), Rect::new(30, 4, 80, 18), 8),
            Rect::new(32, 4, 20, 7)
        );
        assert_eq!(
            viewport_rect(Rect::new(2, 1, 20, 2), Rect::new(30, 4, 80, 18), 8),
            Rect::default()
        );
    }

    #[test]
    fn scrollbar_wheel_track_drag_and_bounds() {
        use ratatui::crossterm::event::KeyModifiers;
        let mut s = Scroll {
            area: Rect::new(10, 10, 20, 10),
            max: 100,
            ..Default::default()
        };
        let m = |kind, x, y| MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        };
        s.mouse(m(MouseEventKind::ScrollDown, 0, 0));
        assert_eq!(s.offset, 0);
        s.mouse(m(MouseEventKind::ScrollDown, 11, 11));
        assert_eq!(s.offset, 3);
        s.mouse(m(MouseEventKind::Down(MouseButton::Left), 29, 19));
        assert_eq!(s.offset, 100);
        s.mouse(m(MouseEventKind::Drag(MouseButton::Left), 29, 10));
        assert_eq!(s.offset, 0);
        s.clear();
        s.dragging = false;
        s.mouse(m(MouseEventKind::Drag(MouseButton::Left), 29, 19));
        assert_eq!(s.offset, 0);
    }
}
