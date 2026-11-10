//! Approved Dashboard body. Values come from cached collection and analytics evidence.
use super::{
    data::Data,
    design::{self, Scroll},
    theme::{Palette, Role},
};
use ratatui::crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Chart, Dataset, GraphType, Paragraph},
};

#[derive(Default, Debug)]
pub struct Overview {
    pub scroll: Scroll,
    alert_scroll: Scroll,
    health_scroll: Scroll,
    window: usize,
    ranges: [Rect; 3],
    source: usize,
    sources: [Rect; 3],
    refresh: Rect,
}
impl Overview {
    #[cfg(test)]
    pub fn refresh_target(&self) -> Rect {
        self.refresh
    }
    pub fn clear(&mut self) {
        self.scroll.clear();
        self.alert_scroll.clear();
        self.health_scroll.clear();
        self.ranges = [Rect::default(); 3];
        self.sources = [Rect::default(); 3];
        self.refresh = Rect::default();
    }
    pub fn key(&mut self, k: KeyCode, d: &Data) {
        match k {
            KeyCode::Char('v') => self.source = (self.source + 1) % 3,
            KeyCode::Char('w') => {
                for step in 1..=3 {
                    let i = (self.window + step) % 3;
                    if self.enough(d, [300, 900, 3600][i]) {
                        self.window = i;
                        break;
                    }
                }
            }
            KeyCode::Char('j') => {
                self.health_scroll.offset = self
                    .health_scroll
                    .offset
                    .saturating_add(1)
                    .min(self.health_scroll.max)
            }
            KeyCode::Char('k') => {
                self.health_scroll.offset = self.health_scroll.offset.saturating_sub(1)
            }
            KeyCode::Down => {
                self.scroll.offset = self.scroll.offset.saturating_add(1).min(self.scroll.max)
            }
            KeyCode::Up => self.scroll.offset = self.scroll.offset.saturating_sub(1),
            KeyCode::PageDown => {
                self.scroll.offset = self.scroll.offset.saturating_add(10).min(self.scroll.max)
            }
            KeyCode::PageUp => self.scroll.offset = self.scroll.offset.saturating_sub(10),
            KeyCode::Home => self.scroll.offset = 0,
            KeyCode::End => self.scroll.offset = self.scroll.max,
            _ => {}
        }
    }
    fn history<'a>(
        &self,
        t: &'a crate::worker_util::Telemetry,
    ) -> &'a std::collections::VecDeque<crate::worker_util::traffic::Point> {
        match self.source {
            1 => &t.classification.mapped.history,
            2 => &t.classification.unmapped.history,
            _ => &t.traffic,
        }
    }
    fn enough(&self, d: &Data, seconds: u64) -> bool {
        d.worker_util.as_ref().is_some_and(|t| {
            self.history(t)
                .front()
                .zip(self.history(t).back())
                .is_some_and(|(a, b)| b.at.saturating_sub(a.at) >= seconds)
        })
    }
    pub fn mouse(&mut self, m: MouseEvent, d: &Data) -> bool {
        if m.kind == MouseEventKind::Down(MouseButton::Left) {
            for (i, r) in self.sources.iter().enumerate() {
                if r.contains((m.column, m.row).into()) {
                    self.source = i;
                }
            }
            for (i, r) in self.ranges.iter().enumerate() {
                if r.contains((m.column, m.row).into()) && self.enough(d, [300, 900, 3600][i]) {
                    self.window = i;
                }
            }
            if self.refresh.contains((m.column, m.row).into()) {
                return true;
            }
        }
        if !self.health_scroll.mouse(m) && !self.alert_scroll.mouse(m) {
            self.scroll.mouse(m);
        }
        false
    }
    pub fn draw(&mut self, f: &mut Frame, area: Rect, d: &Data, now: u64, p: &Palette) {
        let height = area.height.max(33);
        self.scroll.max = height.saturating_sub(area.height) as usize;
        self.scroll.offset = self.scroll.offset.min(self.scroll.max);
        if self.scroll.max == 0 {
            self.body(f, area, d, now, p);
        } else {
            // A bounded virtual body preserves every panel on small terminals.
            let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(
                area.width.saturating_sub(1),
                height,
            ))
            .expect("in-memory terminal");
            t.draw(|v| self.body(v, v.area(), d, now, p))
                .expect("in-memory draw");
            let b = t.backend().buffer();
            for y in 0..area.height {
                for x in 0..area.width.saturating_sub(1) {
                    f.buffer_mut()[(area.x + x, area.y + y)] =
                        b[(x, y + self.scroll.offset as u16)].clone();
                }
            }
            let map = |r: Rect| design::viewport_rect(r, area, self.scroll.offset);
            self.ranges = self.ranges.map(map);
            self.sources = self.sources.map(map);
            self.refresh = map(self.refresh);
            self.alert_scroll.area = map(self.alert_scroll.area);
            self.health_scroll.area = map(self.health_scroll.area);
        }
        self.scroll.draw(f, area, height as usize, p);
    }
    fn body(&mut self, f: &mut Frame, area: Rect, d: &Data, now: u64, p: &Palette) {
        f.render_widget(
            ratatui::widgets::Block::default().style(design::surface(p)),
            area,
        );
        let [heading, cards, main, wan, alerts] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(5),
            Constraint::Min(12),
            Constraint::Length(7),
            Constraint::Length(7),
        ])
        .areas(area);
        self.refresh = Rect::new(heading.right().saturating_sub(12), heading.y, 11, 1);
        f.render_widget(
            Paragraph::new("").block(design::panel("Network overview", p)),
            heading,
        );
        f.render_widget(
            Paragraph::new("[r Refresh]").style(design::accent(p)),
            self.refresh,
        );
        let regions = Layout::horizontal([Constraint::Ratio(1, 5); 5]).split(cards);
        let st = d.status.as_deref();
        let routers = st.and_then(|s| s["routers"].as_array());
        let count = |source: &str| super::presence::count(d, now, source, false);
        let reachable = routers.map_or("unknown".into(), |rs| {
            format!(
                "{} / {}",
                rs.iter().filter(|r| r["state"] == "online").count(),
                rs.len()
            )
        });
        let reload = st.and_then(|s| s["reload"].as_str()).unwrap_or("unknown");
        for (i, (title, value, note)) in [
            ("IPoE clients", count("dhcp"), "bound leases; latest cycle"),
            ("PPPoE online", count("ppp"), "latest collection"),
            (
                "PPPoE offline",
                super::presence::count(d, now, "ppp", true),
                "latest collection",
            ),
            ("Routers", reachable, "latest collection"),
            ("LibreQoS", reload.to_owned(), "last reload result"),
        ]
        .into_iter()
        .enumerate()
        {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(value),
                    Line::styled(
                        if title == "Routers" {
                            format!("Hotspot {} active", count("hotspot"))
                        } else {
                            note.into()
                        },
                        design::role(p, Role::Secondary),
                    ),
                ])
                .block(design::panel(title, p)),
                regions[i],
            );
        }
        let [chart, health] =
            Layout::horizontal([Constraint::Percentage(64), Constraint::Percentage(36)])
                .areas(main);
        self.chart(f, chart, d, now, p);
        let telemetry = d.worker_util.as_deref();
        let live = telemetry.is_some_and(|s| s.live(now));
        let mut lines: Vec<Line> = telemetry
            .map_or_else(
                || crate::worker_util::health::Health::default().measurements(now),
                |t| t.health.measurements(now),
            )
            .into_iter()
            .map(Line::raw)
            .collect();
        lines.extend(
            crate::worker_util::queue::Queue::measurements(
                telemetry.and_then(|t| t.queue.as_ref()),
                live,
            )
            .into_iter()
            .map(Line::raw),
        );
        if let Some(t) = telemetry {
            lines.extend(
                t.classification
                    .measurements(live, now)
                    .into_iter()
                    .map(Line::raw),
            );
            if let Some(w) = t.workers.iter().max_by_key(|w| w.busy_ppm) {
                for (label, ppm) in [("busy", w.busy_ppm), ("idle", 1000000 - w.busy_ppm)] {
                    let filled = (ppm / 50000) as usize;
                    lines.push(Line::from(vec![
                        Span::raw(format!(
                            "Worker {} {label} {:5.1}% ",
                            w.queue_id,
                            ppm as f64 / 10000.0
                        )),
                        Span::styled("█".repeat(filled), design::role(p, Role::Ok)),
                        Span::styled("░".repeat(20 - filled), design::role(p, Role::Secondary)),
                    ]));
                }
                lines.push(Line::raw(format!(
                    "Mean backlog {} packets",
                    w.backlog_mean.map_or("unknown".into(), |n| n.to_string())
                )));
            }
            if let Some(pressure) = &t.pressure {
                lines.extend(pressure.measurements().into_iter().map(Line::raw));
            }
            lines.push(Line::styled(
                if live {
                    "Snapshot LIVE"
                } else {
                    "Snapshot STALE"
                },
                design::role(p, if live { Role::Ok } else { Role::Warning }),
            ));
            lines.push(Line::raw(format!(
                "Sample age {}",
                if now >= t.updated && t.updated > 0 {
                    format!("{}s", now - t.updated)
                } else {
                    "unknown".into()
                }
            )));
        } else {
            lines.push(Line::raw("Telemetry unavailable"));
        }
        let block = design::panel("Dataplane health [j/k scroll]", p);
        let inner = block.inner(health);
        f.render_widget(block, health);
        let content = Rect {
            width: inner.width.saturating_sub(1),
            ..inner
        };
        let paragraph = Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false });
        let total = paragraph.line_count(content.width);
        self.health_scroll.max = total.saturating_sub(content.height as usize);
        self.health_scroll.offset = self.health_scroll.offset.min(self.health_scroll.max);
        f.render_widget(
            paragraph.scroll((self.health_scroll.offset as u16, 0)),
            content,
        );
        self.health_scroll.draw(f, inner, total, p);
        f.render_widget(
            Paragraph::new(vec![
                Line::raw("Upstream       State       Capacity       RX       TX       Data age"),
                Line::styled(
                    "WAN collection and assignment are planned.",
                    design::role(p, Role::Secondary),
                ),
                Line::raw("No WAN telemetry available."),
            ])
            .block(design::panel("WAN overview — planned view", p)),
            wan,
        );
        let mut a = Vec::new();
        match telemetry.filter(|_| live).and_then(|t| t.pressure.as_ref()) {
            Some(v) if !v.alerts().is_empty() => a.push(Line::styled(
                format!("[ALERT] {}", v.alerts().join("; ")),
                design::role(p, Role::Error),
            )),
            Some(v) if v.complete() => a.push(Line::styled(
                "[CLEAR] No active buffer/ring pressure warnings",
                design::role(p, Role::Ok),
            )),
            _ => a.push(Line::styled(
                "[UNKNOWN] Pressure telemetry unavailable or stale",
                design::role(p, Role::Warning),
            )),
        }
        match telemetry.map(|t| &t.health) {
            Some(health) if health.live(now) => {
                if let Some(summary) = &health.summary
                    && (summary.status != "healthy"
                        || summary
                            .degraded_subsystems
                            .as_ref()
                            .is_some_and(|v| !v.is_empty())
                        || summary.aqm_degraded_workers.is_some_and(|n| n > 0))
                {
                    a.push(Line::styled(format!("[WARN] Bridge health: {} | degraded subsystems: {} | AQM degraded workers: {}",summary.status,summary.degraded_subsystems.as_ref().map_or("unknown".into(),|v| if v.is_empty() {"none".into()} else {v.join(", ")}),summary.aqm_degraded_workers.map_or("unknown".into(),|n|n.to_string())),design::role(p,Role::Warning)));
                }
            }
            _ => a.push(Line::styled(
                "[UNKNOWN] Bridge health unavailable or stale",
                design::role(p, Role::Warning),
            )),
        }
        if let Some(st) = st {
            let text = |key: &str| st[key].as_str().unwrap_or("unknown");
            a.push(Line::raw(format!(
                "write {} | reload: {} | telegram: {}",
                st["files_changed"].as_bool().map_or("unknown", |v| if v {
                    "changed"
                } else {
                    "unchanged"
                }),
                text("reload"),
                text("telegram")
            )));
            if let Some(rs) = routers {
                for r in rs.iter().filter(|r| r["state"] != "online") {
                    a.push(Line::raw(format!(
                        "{:.42}: {}",
                        r["name"].as_str().unwrap_or("unknown"),
                        r["state"].as_str().unwrap_or("unknown")
                    )));
                }
            }
            for key in ["error", "config_error"] {
                if let Some(s) = st[key].as_str().filter(|s| !s.is_empty()) {
                    a.push(Line::styled(s.to_owned(), design::role(p, Role::Error)));
                }
            }
            if let Some(items) = st["report"]["alerts"].as_array() {
                a.extend(
                    items
                        .iter()
                        .filter_map(|s| s.as_str())
                        .map(|s| Line::raw(s.to_owned())),
                );
            }
        }
        a.extend(d.errors.iter().cloned().map(Line::raw));
        let block = design::panel("Operational alerts", p);
        let inner = block.inner(alerts);
        f.render_widget(block, alerts);
        let content = Rect {
            width: inner.width.saturating_sub(1),
            ..inner
        };
        let paragraph = Paragraph::new(a).wrap(ratatui::widgets::Wrap { trim: false });
        let total = paragraph.line_count(content.width);
        self.alert_scroll.max = total.saturating_sub(content.height as usize);
        self.alert_scroll.offset = self.alert_scroll.offset.min(self.alert_scroll.max);
        f.render_widget(
            paragraph.scroll((
                u16::try_from(self.alert_scroll.offset).unwrap_or(u16::MAX),
                0,
            )),
            content,
        );
        self.alert_scroll.draw(f, inner, total, p);
    }
    fn chart(&mut self, f: &mut Frame, area: Rect, d: &Data, now: u64, p: &Palette) {
        let inner = design::panel("LibreQoS DPDK Traffic", p).inner(area);
        f.render_widget(design::panel("LibreQoS DPDK Traffic", p), area);
        for (i, label) in ["5m", "15m", "1h"].into_iter().enumerate() {
            let r = Rect::new(inner.x + i as u16 * 6, inner.y, 5, 1);
            self.ranges[i] = r;
            f.render_widget(
                Paragraph::new(format!("[{label}]")).style(if i == self.window {
                    design::accent(p)
                } else {
                    design::role(p, Role::Secondary)
                }),
                r,
            );
        }
        for (i, label) in ["Actual", "Mapped", "Unmapped"].into_iter().enumerate() {
            let r = Rect::new(inner.x + i as u16 * 11, inner.y + 1, 10, 1).intersection(inner);
            self.sources[i] = r;
            f.render_widget(
                Paragraph::new(format!("[{label}]")).style(if i == self.source {
                    design::accent(p)
                } else {
                    design::role(p, Role::Secondary)
                }),
                r,
            );
        }
        let telemetry = d.worker_util.as_deref();
        let last = telemetry.and_then(|t| self.history(t).back());
        let source_known = telemetry.is_some_and(|t| match self.source {
            1 => t.classification.mapped.bytes.is_some(),
            2 => t.classification.unmapped.bytes.is_some(),
            _ => true,
        });
        let rate = |n| super::traffic_chart::rate(n, self.source != 0);
        let seconds = [300, 900, 3600][self.window];
        let prepared = super::traffic_chart::Plot::new(
            telemetry.into_iter().flat_map(|t| self.history(t).iter()),
            now,
            seconds,
        );
        let current = last.filter(|v| {
            source_known
                && telemetry.is_some_and(|t| t.live(now))
                && v.down_mbps.is_some()
                && v.up_mbps.is_some()
                && now >= v.at
                && now - v.at <= crate::worker_util::MAX_AGE
        });
        let state = if !source_known || last.is_none() {
            "UNAVAILABLE"
        } else if !telemetry.is_some_and(|t| t.live(now)) {
            "STALE (history only)"
        } else if current.is_some() {
            "LIVE"
        } else {
            "WARMING / GAP"
        };
        let down = current.and_then(|v| v.down_mbps);
        let up = current.and_then(|v| v.up_mbps);
        let scope = match self.source {
            1 => "mapped enqueued aggregate",
            2 => "unmapped enqueued aggregate",
            _ => "actual delivered aggregate",
        };
        let legend = Rect::new(inner.x, inner.y + 2, inner.width, 3);
        f.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(format!("Down {}  ", rate(down)), design::accent(p)),
                    Span::styled(format!("Up {}", rate(up)), design::role(p, Role::Ok)),
                ]),
                Line::styled(
                    format!(
                        "{state} | {scope} | age {} | {} history",
                        last.and_then(|v| now.checked_sub(v.at))
                            .map_or("unknown".into(), |age| format!("{age}s")),
                        if self.enough(d, [300, 900, 3600][self.window]) {
                            "ready"
                        } else {
                            "warming"
                        }
                    ),
                    design::role(p, Role::Secondary),
                ),
                Line::styled(
                    format!(
                        "Window peak D {} / U {} | {} rate samples",
                        rate(prepared.down_peak),
                        rate(prepared.up_peak),
                        prepared.count
                    ),
                    design::role(p, Role::Secondary),
                ),
            ]),
            legend,
        );
        let plot = Rect::new(
            inner.x,
            inner.y + 5,
            inner.width,
            inner.height.saturating_sub(5),
        );
        if plot.height < 3 {
            return;
        }
        let scale = super::traffic_chart::Scale::new(
            prepared
                .down_peak
                .unwrap_or(0.0)
                .max(prepared.up_peak.unwrap_or(0.0)),
            if self.source == 0 { 1.0 } else { 0.001 },
        );
        let down_segments = &prepared.down;
        let up_segments = &prepared.up;
        let sets: Vec<_> = down_segments
            .iter()
            .map(|s| {
                Dataset::default()
                    .marker(Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(design::accent(p))
                    .data(s)
            })
            .chain(up_segments.iter().map(|s| {
                Dataset::default()
                    .marker(Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(design::role(p, Role::Ok))
                    .data(s)
            }))
            .collect();
        let axis = Axis::default()
            .bounds([now.saturating_sub(seconds) as f64, now.max(1) as f64])
            .labels([format!("-{}m", seconds / 60), "now".into()]);
        f.render_widget(
            Chart::new(sets)
                .x_axis(axis)
                .y_axis(
                    Axis::default()
                        .bounds([0.0, scale.top])
                        .labels(["0".into(), scale.top_label]),
                )
                .style(design::surface(p)),
            plot,
        );
    }
}

#[cfg(test)]
mod queue_tests {
    use super::*;
    use crate::config::Theme;
    use ratatui::{Terminal, backend::TestBackend, crossterm::event::KeyModifiers};

    #[test]
    fn bridge_degradation_is_visible_in_operational_alerts_independent_of_snapshot() {
        use std::sync::Arc;
        let mut health = crate::worker_util::health::Health::default();
        for (now, up) in [(90, 1), (100, 10001)] {
            health.record(Ok(serde_json::from_value(serde_json::json!({"status":"degraded","version":"v3","uptime_ms":up,"workers":[],"degraded_subsystems":["stats_summary"]})).unwrap()),now);
        }
        let d = Data {
            worker_util: Some(Arc::new(crate::worker_util::Telemetry {
                health,
                ..Default::default()
            })),
            ..Default::default()
        };
        let mut term = Terminal::new(TestBackend::new(180, 40)).unwrap();
        let mut view = Overview::default();
        for (now, label) in [
            (100, "[WARN] Bridge health: degraded"),
            (116, "[UNKNOWN] Bridge health unavailable or stale"),
        ] {
            term.draw(|f| view.draw(f, f.area(), &d, now, &Palette::new(Theme::Dark)))
                .unwrap();
            let text = term
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            assert!(
                text.contains(label),
                "missing operational alert {label}: {text}"
            );
        }
    }
    #[test]
    fn chart_dynamic_values_retain_age_zero_missing_and_historical_peak_labels() {
        use std::sync::Arc;
        let mut report = crate::worker_util::Telemetry {
            updated: 100,
            stale: false,
            ..Default::default()
        };
        report
            .traffic
            .push_back(crate::worker_util::traffic::Point {
                at: 100,
                down_mbps: Some(0.025),
                up_mbps: Some(0.0),
            });
        let d = Data {
            worker_util: Some(Arc::new(report)),
            ..Default::default()
        };
        let mut view = Overview::default();
        let mut term = Terminal::new(TestBackend::new(160, 20)).unwrap();
        for (now, state, down) in [
            (100, "LIVE", "Down 25.0 kbps"),
            (107, "STALE (history only)", "Down unknown"),
        ] {
            term.draw(|f| view.chart(f, f.area(), &d, now, &Palette::new(Theme::Dark)))
                .unwrap();
            let text = term
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            for label in [state, down, "Window peak D 25.0 kbps", "1 rate samples"] {
                assert!(text.contains(label), "missing {label}: {text}");
            }
            assert!(text.contains(&format!("age {}s", now - 100)));
        }
        let mut report = (*d.worker_util.unwrap()).clone();
        report.traffic.back_mut().unwrap().down_mbps = None;
        let d = Data {
            worker_util: Some(Arc::new(report)),
            ..Default::default()
        };
        term.draw(|f| view.chart(f, f.area(), &d, 100, &Palette::new(Theme::Dark)))
            .unwrap();
        let text = term
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(text.contains("WARMING / GAP") && text.contains("Window peak D unknown"));
    }
    #[test]
    fn chart_maximum_history_redraw_cost_and_point_bounds() {
        use std::sync::Arc;
        let mut report = crate::worker_util::Telemetry {
            updated: 4000,
            stale: false,
            ..Default::default()
        };
        report.traffic = (0..crate::worker_util::traffic::MAX_POINTS)
            .map(|i| crate::worker_util::traffic::Point {
                at: 4000 - 3600 + i as u64 * 2,
                down_mbps: Some((i % 100) as f64),
                up_mbps: Some((i % 10) as f64),
            })
            .collect();
        report.classification.mapped.history = report.traffic.clone();
        report.classification.unmapped.history = report.traffic.clone();
        let d = Data {
            worker_util: Some(Arc::new(report)),
            ..Default::default()
        };
        let mut view = Overview {
            window: 2,
            ..Default::default()
        };
        let mut term = Terminal::new(TestBackend::new(240, 40)).unwrap();
        let start = std::time::Instant::now();
        for _ in 0..100 {
            term.draw(|f| view.chart(f, f.area(), &d, 4000, &Palette::new(Theme::Dark)))
                .unwrap();
        }
        eprintln!(
            "maximum traffic chart: 1801 points, 100 debug redraws {:?}",
            start.elapsed()
        );
        let text = term
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(text.contains("1801 rate samples"));
    }
    #[test]
    fn classified_chart_selects_actual_mapped_and_unmapped_without_mixing_units() {
        use std::sync::Arc;
        let report = serde_json::from_value(serde_json::json!({
            "version":1,"updated":100,"stale":false,"reason":null,
            "workers":[{"queue_id":0,"busy_ppm":125000}],
            "traffic":[{"at":100,"down_mbps":10.0,"up_mbps":1.0}],
            "classification":{
                "mapped":{"bytes":{"download_bytes":1000,"upload_bytes":500},"history":[{"at":100,"down_mbps":8.0,"up_mbps":4.0}]},
                "unmapped":{"bytes":{"download_bytes":0,"upload_bytes":0},"history":[{"at":100,"down_mbps":0.0,"up_mbps":0.0}]}
            }
        })).unwrap();
        let d = Data {
            worker_util: Some(Arc::new(report)),
            ..Data::default()
        };
        let p = Palette::new(Theme::Dark);
        let mut v = Overview::default();
        let mut term = Terminal::new(TestBackend::new(180, 40)).unwrap();
        let render = |term: &mut Terminal<TestBackend>, v: &mut Overview, now| {
            term.draw(|f| v.draw(f, f.area(), &d, now, &p)).unwrap();
            term.backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>()
        };
        assert!(render(&mut term, &mut v, 100).contains("actual delivered aggregate"));
        v.key(KeyCode::Char('v'), &d);
        let text = render(&mut term, &mut v, 100);
        assert!(
            text.contains("mapped enqueued aggregate") && text.contains("Down 8.000 Mbps"),
            "{text}"
        );
        v.key(KeyCode::Char('v'), &d);
        let text = render(&mut term, &mut v, 100);
        assert!(
            text.contains("unmapped enqueued aggregate") && text.contains("Down 0.000 Mbps"),
            "{text}"
        );
        let text = render(&mut term, &mut v, 107);
        assert!(
            text.contains("Down unknown") && text.contains("STALE (history only)"),
            "{text}"
        );
        v.key(KeyCode::Char('v'), &d);
        assert!(render(&mut term, &mut v, 100).contains("actual delivered aggregate"));
        let r = v.sources[2];
        v.mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: r.x,
                row: r.y,
                modifiers: KeyModifiers::NONE,
            },
            &d,
        );
        assert!(render(&mut term, &mut v, 100).contains("unmapped enqueued aggregate"));
        v.clear();
        assert_eq!(v.sources, [Rect::default(); 3]);
        for theme in [Theme::Dark, Theme::Light, Theme::Mono] {
            for (width, height) in [(40, 24), (80, 24), (120, 40)] {
                let mut term = Terminal::new(TestBackend::new(width, height)).unwrap();
                for _ in 0..3 {
                    term.draw(|f| v.draw(f, f.area(), &d, 100, &Palette::new(theme)))
                        .unwrap();
                    for source in v.sources {
                        if source.width > 0 && source.height > 0 {
                            assert!(source.right() <= width && source.bottom() <= height);
                        }
                    }
                    v.key(KeyCode::Char('v'), &d);
                }
            }
        }
    }

    #[test]
    fn health_panel_scrolls_wrapped_diagnostics_with_mouse_and_keyboard() {
        let data = Data::default();
        let mut view = Overview::default();
        let palette = Palette::new(Theme::Dark);
        let mut terminal = Terminal::new(TestBackend::new(40, 24)).unwrap();
        terminal
            .draw(|f| view.draw(f, f.area(), &data, 100, &palette))
            .unwrap();
        assert!(view.health_scroll.max > 0);
        let r = view.health_scroll.area;
        view.mouse(
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: r.x,
                row: r.y,
                modifiers: KeyModifiers::NONE,
            },
            &data,
        );
        assert!(view.health_scroll.offset > 0);
        let offset = view.health_scroll.offset;
        view.key(KeyCode::Char('k'), &data);
        assert_eq!(view.health_scroll.offset, offset - 1);
        view.key(KeyCode::Char('j'), &data);
        assert_eq!(view.health_scroll.offset, offset);
        terminal
            .draw(|f| view.draw(f, f.area(), &data, 100, &palette))
            .unwrap();
        view.clear();
        assert_eq!(view.health_scroll.area, Rect::default());
    }
}
