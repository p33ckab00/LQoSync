//! A read-only table of actual DPDK worker utilization.

use super::data::Data;
use super::theme::{BoxKind, Palette, Role};
use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Layout, Rect},
    widgets::{Cell, Paragraph, Row, Table},
};

#[derive(Default)]
pub struct WorkersView {
    offset: usize,
}

impl WorkersView {
    pub fn on_key(&mut self, key: KeyEvent, data: &Data) {
        let count = data.worker_util.as_ref().map_or(0, |s| s.workers.len());
        self.offset = match key.code {
            KeyCode::Up => self.offset.saturating_sub(1),
            KeyCode::Down => self.offset.saturating_add(1),
            KeyCode::PageUp => self.offset.saturating_sub(10),
            KeyCode::PageDown => self.offset.saturating_add(10),
            KeyCode::Home => 0,
            KeyCode::End => count.saturating_sub(1),
            _ => self.offset,
        }
        .min(count.saturating_sub(1));
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect, data: &Data, now: u64, palette: &Palette) {
        let block = palette.panel(BoxKind::Cpu, "Workers", false);
        let inner = block.inner(area);
        f.render_widget(block, area);
        let v3 = data.worker_util.is_some()
            || data.target.starts_with("v3_")
            || data
                .status
                .as_ref()
                .and_then(|s| s["target"].as_str())
                .is_some_and(|t| t.starts_with("v3_"));
        if !v3 {
            f.render_widget(
                Paragraph::new("LibreQoS v3 worker telemetry required"),
                inner,
            );
            return;
        }
        let Some(report) = data.worker_util.as_deref() else {
            f.render_widget(
                Paragraph::new("workers unavailable: no telemetry cache")
                    .style(palette.role(Role::Warning)),
                inner,
            );
            return;
        };
        let [status, reason, table] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(inner);
        let live = report.live(now);
        let state = if report.workers.is_empty() {
            "unavailable"
        } else if live {
            "live"
        } else {
            "stale"
        };
        let age = if report.updated == 0 || now < report.updated {
            "unknown".into()
        } else {
            format!("{}s", now - report.updated)
        };
        f.render_widget(
            Paragraph::new(format!(
                "workers: {state} | age {age} | {} workers",
                report.workers.len()
            ))
            .style(palette.role(if live { Role::Ok } else { Role::Warning })),
            status,
        );
        let detail = if live {
            "Busy excludes empty polling; backlog is mean packets"
        } else {
            report
                .reason
                .as_deref()
                .unwrap_or("cache expired; last-known values below")
        };
        f.render_widget(
            Paragraph::new(detail).style(palette.role(Role::Secondary)),
            reason,
        );
        let visible = table.height.saturating_sub(1) as usize;
        self.offset = self
            .offset
            .min(report.workers.len().saturating_sub(visible.max(1)));
        let rows = report
            .workers
            .iter()
            .skip(self.offset)
            .take(visible)
            .enumerate()
            .map(|(i, w)| {
                let pct = |ppm: u64| format!("{:.1}%", ppm as f64 / 10000.0);
                Row::new([
                    Cell::from(w.queue_id.to_string()),
                    Cell::from(pct(w.busy_ppm)),
                    Cell::from(pct(1_000_000u64.saturating_sub(w.busy_ppm))),
                    Cell::from(w.backlog_mean.map_or("unknown".into(), |n| n.to_string())),
                ])
                .style(palette.row(i + self.offset, false))
            });
        let header =
            Row::new(["Worker", "Busy", "Idle", "Backlog"]).style(palette.role(Role::Accent));
        f.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Length(8),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Min(7),
                ],
            )
            .header(header),
            table,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Theme;
    use crate::worker_util::{Telemetry, WorkerRow};
    use ratatui::{Terminal, backend::TestBackend};
    use std::sync::Arc;

    fn render(data: &Data, now: u64, width: u16, height: u16, theme: Theme) -> String {
        let mut term = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut view = WorkersView::default();
        term.draw(|f| view.draw(f, f.area(), data, now, &Palette::new(theme)))
            .unwrap();
        term.backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    fn live() -> Data {
        Data {
            target: "v3_pair".into(),
            worker_util: Some(Arc::new(Telemetry {
                version: 1,
                updated: 100,
                stale: false,
                reason: None,
                workers: vec![WorkerRow {
                    queue_id: 0,
                    busy_ppm: 125000,
                    backlog_mean: Some(3),
                }],
            })),
            ..Data::default()
        }
    }

    #[test]
    fn worker_util_view_reports_busy_idle_and_backlog_with_explicit_freshness() {
        for theme in [Theme::Dark, Theme::Light, Theme::Mono] {
            let text = render(&live(), 101, 80, 15, theme);
            for want in ["live", "Busy", "Idle", "Backlog", "12.5%", "87.5%", "3"] {
                assert!(text.contains(want), "missing {want}: {text}");
            }
            let text = render(&live(), 120, 60, 12, theme);
            assert!(text.contains("stale") && text.contains("12.5%"), "{text}");
            render(&live(), 120, 25, 5, theme);
        }
    }

    #[test]
    fn worker_util_view_does_not_fabricate_zero_on_missing_data() {
        let mut data = live();
        data.worker_util = None;
        let text = render(&data, 101, 80, 15, Theme::Dark);
        assert!(text.contains("unavailable"), "{text}");
        assert!(!text.contains("0.0%"));
        data.target = "v2_pair".into();
        assert!(render(&data, 101, 80, 15, Theme::Dark).contains("LibreQoS v3"));
    }

    #[test]
    fn worker_util_view_can_scroll_to_the_last_worker() {
        let mut data = live();
        Arc::make_mut(data.worker_util.as_mut().unwrap()).workers = (60001..=60020)
            .map(|queue_id| WorkerRow {
                queue_id,
                busy_ppm: 100000,
                backlog_mean: None,
            })
            .collect();
        let mut view = WorkersView::default();
        view.on_key(
            KeyEvent::new(KeyCode::End, ratatui::crossterm::event::KeyModifiers::NONE),
            &data,
        );
        let mut term = Terminal::new(TestBackend::new(80, 12)).unwrap();
        term.draw(|f| view.draw(f, f.area(), &data, 101, &Palette::new(Theme::Dark)))
            .unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains("60020") && text.contains("unknown"), "{text}");
        assert!(!text.contains("60001"), "{text}");
    }
}
