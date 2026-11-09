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
    window: usize,
    ranges: [Rect; 3],
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
        self.ranges = [Rect::default(); 3];
        self.refresh = Rect::default();
    }
    pub fn key(&mut self, k: KeyCode, d: &Data) {
        match k {
            KeyCode::Char('w') => {
                for step in 1..=3 {
                    let i = (self.window + step) % 3;
                    if Self::enough(d, [300, 900, 3600][i]) {
                        self.window = i;
                        break;
                    }
                }
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
    fn enough(d: &Data, seconds: u64) -> bool {
        d.worker_util.as_ref().is_some_and(|t| {
            t.traffic
                .front()
                .zip(t.traffic.back())
                .is_some_and(|(a, b)| b.at.saturating_sub(a.at) >= seconds)
        })
    }
    pub fn mouse(&mut self, m: MouseEvent, d: &Data) -> bool {
        if m.kind == MouseEventKind::Down(MouseButton::Left) {
            for (i, r) in self.ranges.iter().enumerate() {
                if r.contains((m.column, m.row).into()) && Self::enough(d, [300, 900, 3600][i]) {
                    self.window = i;
                }
            }
            if self.refresh.contains((m.column, m.row).into()) {
                return true;
            }
        }
        if !self.alert_scroll.mouse(m) {
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
            let map = |r: Rect| {
                let y = i32::from(r.y) - self.scroll.offset as i32;
                if y >= 0 && y < i32::from(area.height) {
                    Rect::new(
                        area.x + r.x,
                        area.y + y as u16,
                        r.width,
                        r.height.min(area.height - y as u16),
                    )
                } else {
                    Rect::default()
                }
            };
            self.ranges = self.ranges.map(map);
            self.refresh = map(self.refresh);
            self.alert_scroll.area = map(self.alert_scroll.area);
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
        let count = |source: &str| {
            routers
                .filter(|r| !r.is_empty() && r.iter().all(|v| v["state"] == "online"))
                .and_then(|r| {
                    r.iter()
                        .map(|v| v["rows"][source].as_u64())
                        .try_fold(0u64, |a, n| a.checked_add(n?))
                })
                .map_or("unknown".into(), |n| n.to_string())
        };
        let reachable = routers.map_or("unknown".into(), |rs| {
            format!(
                "{} / {}",
                rs.iter().filter(|r| r["state"] == "online").count(),
                rs.len()
            )
        });
        let reload = st.and_then(|s| s["reload"].as_str()).unwrap_or("unknown");
        for (i, (title, value, note)) in [
            ("IPoE clients", count("dhcp"), "latest collection"),
            ("PPPoE online", count("ppp"), "latest collection"),
            ("PPPoE offline", "unknown".into(), "no session evidence"),
            ("Routers", reachable, "latest collection"),
            ("LibreQoS", reload.to_owned(), "last reload result"),
        ]
        .into_iter()
        .enumerate()
        {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(value),
                    Line::styled(note, design::role(p, Role::Secondary)),
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
        let mut lines = Vec::new();
        if let Some(t) = telemetry {
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
        f.render_widget(
            Paragraph::new(lines)
                .wrap(ratatui::widgets::Wrap { trim: false })
                .block(design::panel("Dataplane health", p)),
            health,
        );
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
        self.alert_scroll.max = a.len().saturating_sub(inner.height as usize);
        self.alert_scroll.offset = self.alert_scroll.offset.min(self.alert_scroll.max);
        let total = a.len();
        f.render_widget(
            Paragraph::new(
                a.into_iter()
                    .skip(self.alert_scroll.offset)
                    .collect::<Vec<_>>(),
            ),
            inner,
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
        let telemetry = d.worker_util.as_deref();
        let last = telemetry.and_then(|t| t.traffic.back());
        let rate = |n: Option<f64>| n.map_or("unknown".into(), |n| format!("{n:.1} Mbps"));
        let state = if telemetry.is_some_and(|t| t.live(now))
            && last.is_some_and(|v| {
                v.down_mbps.is_some() && now >= v.at && now - v.at <= crate::worker_util::MAX_AGE
            }) {
            "LIVE"
        } else {
            "unavailable / stale"
        };
        let down = last.and_then(|v| v.down_mbps);
        let up = last.and_then(|v| v.up_mbps);
        let legend = Rect::new(inner.x, inner.y + 1, inner.width, 2);
        f.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(format!("Down {}  ", rate(down)), design::accent(p)),
                    Span::styled(format!("Up {}", rate(up)), design::role(p, Role::Ok)),
                ]),
                Line::styled(
                    format!(
                        "{state} | actual delivered aggregate | {} history",
                        if Self::enough(d, [300, 900, 3600][self.window]) {
                            "ready"
                        } else {
                            "warming"
                        }
                    ),
                    design::role(p, Role::Secondary),
                ),
            ]),
            legend,
        );
        let plot = Rect::new(
            inner.x,
            inner.y + 3,
            inner.width,
            inner.height.saturating_sub(3),
        );
        if plot.height < 3 {
            return;
        }
        let seconds = [300, 900, 3600][self.window];
        let mut down_segments: Vec<Vec<(f64, f64)>> = vec![Vec::new()];
        let mut up_segments = down_segments.clone();
        if let Some(t) = telemetry {
            for v in t
                .traffic
                .iter()
                .filter(|v| v.at <= now && now - v.at <= seconds)
            {
                if let (Some(down), Some(up)) = (v.down_mbps, v.up_mbps) {
                    down_segments.last_mut().unwrap().push((v.at as f64, down));
                    up_segments.last_mut().unwrap().push((v.at as f64, up));
                } else if !down_segments.last().unwrap().is_empty() {
                    down_segments.push(Vec::new());
                    up_segments.push(Vec::new());
                }
            }
        }
        let peak = down_segments
            .iter()
            .chain(&up_segments)
            .flatten()
            .map(|v| v.1)
            .fold(1.0, f64::max)
            * 1.1;
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
                        .bounds([0.0, peak])
                        .labels(["0 M".to_owned(), format!("{peak:.0} M")]),
                )
                .style(design::surface(p)),
            plot,
        );
    }
}
