//! Transitional cached status views for the new navigation routes.

use super::{
    data::Data,
    theme::{Palette, Role},
};
use ratatui::crossterm::event::{KeyCode, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

#[derive(Default)]
pub struct StatusView {
    offset: usize,
    max: usize,
    area: Rect,
}

impl StatusView {
    pub fn clear_area(&mut self) {
        self.area = Rect::default();
    }

    pub fn on_key(&mut self, code: KeyCode) {
        self.offset = match code {
            KeyCode::Down => self.offset.saturating_add(1),
            KeyCode::Up => self.offset.saturating_sub(1),
            KeyCode::PageDown => self.offset.saturating_add(10),
            KeyCode::PageUp => self.offset.saturating_sub(10),
            KeyCode::Home => 0,
            KeyCode::End => self.max,
            _ => self.offset,
        }
        .min(self.max);
    }

    pub fn on_mouse(&mut self, event: MouseEvent) {
        if self.area.contains((event.column, event.row).into()) {
            match event.kind {
                MouseEventKind::ScrollDown => self.on_key(KeyCode::Down),
                MouseEventKind::ScrollUp => self.on_key(KeyCode::Up),
                _ => (),
            }
        }
    }

    pub fn draw(
        &mut self,
        f: &mut Frame,
        area: Rect,
        data: &Data,
        now: u64,
        routers: bool,
        palette: &Palette,
    ) {
        self.area = area;
        let title = if routers {
            "Routers — latest collection status"
        } else {
            "LibreQoS — sync status"
        };
        let block = Block::bordered()
            .borders(Borders::ALL)
            .title(title)
            .style(palette.canvas());
        let inner = block.inner(area);
        f.render_widget(block, area);
        let age = data
            .status
            .as_ref()
            .and_then(|s| s["updated"].as_u64())
            .filter(|&at| at <= now)
            .map_or("unknown".into(), |at| format!("{}s", now - at));
        let mut lines = vec![Line::styled(
            format!("Cached cycle snapshot | age {age}"),
            palette.role(Role::Secondary),
        )];
        if routers {
            if let Some(items) = data.status.as_ref().and_then(|s| s["routers"].as_array()) {
                lines.push(Line::raw(format!("{} routers in latest cycle; this is collection evidence, not a live connection test.",items.len())));
                self.max = items
                    .len()
                    .saturating_sub(inner.height.saturating_sub(2) as usize);
                self.offset = self.offset.min(self.max);
                for r in items
                    .iter()
                    .skip(self.offset)
                    .take(inner.height.saturating_sub(2) as usize)
                {
                    let name = r["name"].as_str().unwrap_or("unknown");
                    let state = r["state"].as_str().unwrap_or("unknown");
                    let ms = r["read_ms"]
                        .as_u64()
                        .map_or("unknown".into(), |n| format!("{n} ms"));
                    let error = r["error"].as_str().unwrap_or("");
                    lines.push(Line::raw(format!("{name} | {state} | {ms} | {error}")));
                }
            } else {
                self.max = 0;
                lines.push(Line::raw("Router status unavailable"));
            }
        } else {
            let all = super::dashboard::lines(data, now);
            self.max = all
                .len()
                .saturating_sub(inner.height.saturating_sub(1) as usize);
            self.offset = self.offset.min(self.max);
            lines.extend(
                all.into_iter()
                    .skip(self.offset)
                    .take(inner.height.saturating_sub(1) as usize),
            );
        }
        f.render_widget(Paragraph::new(lines), inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Theme;
    use ratatui::crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};
    use serde_json::json;
    use std::sync::Arc;

    fn draw(view: &mut StatusView, data: &Data) -> String {
        let mut term = Terminal::new(TestBackend::new(100, 10)).unwrap();
        term.draw(|f| view.draw(f, f.area(), data, 100, true, &Palette::new(Theme::Dark)))
            .unwrap();
        term.backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[test]
    fn cached_routers_scroll_only_under_the_pointer_and_clamp_at_both_ends() {
        let data = Data {
            status: Some(Arc::new(
                json!({"updated":98,"routers":(0..12).map(|n|json!({"name":format!("R{n:02}"),"state":"online","read_ms":2})).collect::<Vec<_>>()}),
            )),
            ..Default::default()
        };
        let mut view = StatusView::default();
        let text = draw(&mut view, &data);
        assert!(text.contains("R00") && text.contains("age 2s"));
        view.on_mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 10,
            row: 15,
            modifiers: KeyModifiers::NONE,
        });
        assert!(draw(&mut view, &data).contains("R00"));
        view.on_mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 10,
            row: 4,
            modifiers: KeyModifiers::NONE,
        });
        assert!(!draw(&mut view, &data).contains("R00"));
        view.on_key(KeyCode::End);
        assert!(draw(&mut view, &data).contains("R11"));
        for _ in 0..100 {
            view.on_key(KeyCode::Down);
        }
        assert!(draw(&mut view, &data).contains("R11"));
        view.clear_area();
        view.on_mouse(MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 10,
            row: 4,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(view.offset, view.max);
        assert!(draw(&mut view, &Data::default()).contains("unavailable"));
    }
}
