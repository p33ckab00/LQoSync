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
    widgets::{
        Paragraph,
        canvas::{Canvas, Line as CanvasLine},
    },
};

#[derive(Default, Debug)]
pub struct Overview {
    pub scroll: Scroll,
    alert_scroll: Scroll,
    health_scroll: Scroll,
    window: usize,
    ranges: [Rect; 3],
    traces: super::traffic_chart::Visibility,
    curves: super::traffic_chart::Cache,
    all_sources: Rect,
    focus_source: Rect,
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
        self.all_sources = Rect::default();
        self.focus_source = Rect::default();
    }
    pub fn key(&mut self, k: KeyCode, d: &Data) {
        match k {
            KeyCode::Char('v') => self.traces.cycle_focus(),
            KeyCode::Char('a') => self.traces.all(),
            KeyCode::Char('A') => self.traces.toggle(0),
            KeyCode::Char('M') => self.traces.toggle(1),
            KeyCode::Char('U') => self.traces.toggle(2),
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
    fn enough(&self, d: &Data, seconds: u64) -> bool {
        d.worker_util.as_ref().is_some_and(|t| {
            (0..3).any(|i| {
                self.traces.mask() & (1 << i) != 0
                    && super::traffic_chart::history(t, i)
                        .front()
                        .zip(super::traffic_chart::history(t, i).back())
                        .is_some_and(|(a, b)| b.at.saturating_sub(a.at) >= seconds)
            })
        })
    }
    pub fn mouse(&mut self, m: MouseEvent, d: &Data) -> bool {
        if m.kind == MouseEventKind::Down(MouseButton::Left) {
            for (i, r) in self.sources.iter().enumerate() {
                if r.contains((m.column, m.row).into()) {
                    self.traces.toggle(i);
                }
            }
            if self.all_sources.contains((m.column, m.row).into()) {
                self.traces.all();
            }
            if self.focus_source.contains((m.column, m.row).into()) {
                self.traces.cycle_focus();
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
        let height = area.height.max(35);
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
            self.all_sources = map(self.all_sources);
            self.focus_source = map(self.focus_source);
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
            Constraint::Min(14),
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
            d.nic_stats
                .as_deref()
                .unwrap_or(&crate::nic_stats::Report::default())
                .measurements(now, false)
                .into_iter()
                .map(Line::raw),
        );
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
        if let Some(report) = &d.system_health {
            let report = report.at(now);
            let mut unified = vec![Line::styled(
                format!("System health {:?}", report.overall),
                design::role(
                    p,
                    match report.overall {
                        crate::system_health::State::Healthy => Role::Ok,
                        crate::system_health::State::Failed => Role::Error,
                        _ => Role::Warning,
                    },
                ),
            )];
            if let Some(update) = &report.update {
                unified.push(Line::raw(format!(
                    "Update {:?} | {}",
                    update.phase, update.write_id
                )));
            }
            if let Some(proof) = &report.acceptance {
                unified.push(Line::raw(format!(
                    "Circuit slots native {} / generated {}",
                    proof
                        .circuit_capacity
                        .map_or("unknown".into(), |v| v.to_string()),
                    proof.desired_circuit_capacity
                )));
            }
            for c in &report.components {
                unified.push(Line::raw(format!(
                    "{} / {}: {:?} | {} | age {}s",
                    c.domain.label(),
                    c.component,
                    c.state,
                    c.reason,
                    now.saturating_sub(c.observed_at)
                )));
            }
            unified.push(Line::raw(""));
            unified.extend(lines);
            lines = unified;
        } else {
            lines.insert(
                0,
                Line::styled(
                    "System health unavailable",
                    design::role(p, Role::Secondary),
                ),
            );
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
        a.extend(
            d.nic_stats
                .as_deref()
                .unwrap_or(&crate::nic_stats::Report::default())
                .alerts(now)
                .into_iter()
                .map(|line| Line::styled(line, design::role(p, Role::Warning))),
        );
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
        use super::traffic_chart::{self as chart, NAMES};
        let block = design::panel(
            "LibreQoS DPDK Traffic | mirrored | independent D/U scales",
            p,
        );
        let inner = block.inner(area);
        f.render_widget(block, area);
        self.ranges = [Rect::default(); 3];
        self.sources = [Rect::default(); 3];
        self.all_sources = Rect::default();
        self.focus_source = Rect::default();
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        for (i, label) in ["5m", "15m", "1h"].into_iter().enumerate() {
            let r = Rect::new(inner.x + i as u16 * 6, inner.y, 5, 1).intersection(inner);
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
        self.all_sources = Rect::new(inner.x + 19, inner.y, 7, 1).intersection(inner);
        self.focus_source = Rect::new(inner.x + 28, inner.y, 24, 1).intersection(inner);
        f.render_widget(
            Paragraph::new("[a All]").style(design::accent(p)),
            self.all_sources,
        );
        f.render_widget(
            Paragraph::new(format!(
                "[v Focus: {}]",
                self.traces.focused().map_or(
                    if self.traces.mask() == 7 {
                        "All"
                    } else {
                        "Selection"
                    },
                    |i| NAMES[i]
                )
            ))
            .style(design::accent(p)),
            self.focus_source,
        );
        let seconds = [300, 900, 3600][self.window];
        let mask = self.traces.mask();
        let prepared = self.curves.prepare(d.worker_util.as_ref(), now, seconds);
        for i in 0..3 {
            let history = d.worker_util.as_ref().map(|t| chart::history(t, i));
            let last = history.and_then(|h| h.back());
            let known = d.worker_util.as_ref().is_some_and(|t| match i {
                1 => t.classification.mapped.bytes.is_some(),
                2 => t.classification.unmapped.bytes.is_some(),
                _ => true,
            });
            let current = last.filter(|v| {
                known
                    && d.worker_util.as_ref().is_some_and(|t| t.live(now))
                    && v.down_mbps.is_some()
                    && v.up_mbps.is_some()
                    && now >= v.at
                    && now - v.at <= crate::worker_util::MAX_AGE
            });
            let state = if !known || last.is_none() {
                "UNAVAILABLE"
            } else if !d.worker_util.as_ref().is_some_and(|t| t.live(now)) {
                "STALE"
            } else if current.is_none() {
                "WARMING/GAP"
            } else {
                "LIVE"
            };
            let age = last
                .and_then(|v| now.checked_sub(v.at))
                .map_or("?".into(), |n| format!("{n}s"));
            let down = chart::rate(current.and_then(|v| v.down_mbps), i != 0);
            let up = chart::rate(current.and_then(|v| v.up_mbps), i != 0);
            let on = mask & (1 << i) != 0;
            let check = if on { "x" } else { " " };
            let text = if inner.width >= 85 {
                format!(
                    "[{check}] {:<8} Down {down:>13}  Up {up:>13} | {state} age {age} | n={}",
                    NAMES[i], prepared[i].count
                )
            } else {
                format!(
                    "[{check}] {:<8} D {down:>10} U {up:>10} {state} {age}",
                    NAMES[i]
                )
            };
            let r = Rect::new(inner.x, inner.y + 1 + i as u16, inner.width, 1).intersection(inner);
            self.sources[i] = r;
            let style = chart::source_style(p, i);
            f.render_widget(
                Paragraph::new(text).style(if on {
                    style
                } else {
                    style.add_modifier(ratatui::style::Modifier::DIM)
                }),
                r,
            );
        }
        let scope = Rect::new(inner.x, inner.y + 4, inner.width, 1).intersection(inner);
        f.render_widget(
            Paragraph::new("Actual delivered; Mapped/Unmapped enqueued | D above / U below")
                .style(design::role(p, Role::Secondary)),
            scope,
        );
        let summary = Rect::new(inner.x, inner.y + 5, inner.width, 1).intersection(inner);
        let selected = (0..3).find(|i| mask & (1 << i) != 0);
        let text = selected.map_or(
            "No traces selected; click a legend row or press a".into(),
            |i| {
                format!(
                    "{} window peak D {} / U {} | {} rate samples",
                    NAMES[i],
                    chart::rate(prepared[i].down_peak, i != 0),
                    chart::rate(prepared[i].up_peak, i != 0),
                    prepared[i].count
                )
            },
        );
        f.render_widget(
            Paragraph::new(text).style(design::role(p, Role::Secondary)),
            summary,
        );
        let plot = Rect::new(
            inner.x,
            inner.y + 6,
            inner.width,
            inner.height.saturating_sub(6),
        );
        if plot.height < 3 {
            return;
        }
        let scales = [false, true].map(|upload| {
            let peak = (0..3)
                .filter(|i| mask & (1 << i) != 0)
                .map(|i| {
                    if upload {
                        prepared[i].up_peak
                    } else {
                        prepared[i].down_peak
                    }
                    .unwrap_or(0.0)
                })
                .fold(0.0, f64::max);
            chart::Scale::new(peak, 0.001)
        });
        let start = now.saturating_sub(seconds) as f64;
        let end = now.max(1) as f64;
        // Paint all segments on one canvas. A canvas per gap makes unstable histories expensive.
        let axis_width = (scales.iter().map(|s| s.top_label.len()).max().unwrap_or(0) + 3)
            .min((plot.width / 3).max(1) as usize) as u16;
        let canvas = Rect::new(
            plot.x + axis_width,
            plot.y,
            plot.width.saturating_sub(axis_width),
            plot.height.saturating_sub(2),
        );
        if canvas.width == 0 || canvas.height == 0 {
            return;
        }
        let axis_style = design::role(p, Role::Secondary);
        for (label, y) in [
            (format!("D {}", scales[0].top_label), canvas.y),
            ("0".into(), canvas.y + canvas.height / 2),
            (format!("U {}", scales[1].top_label), canvas.bottom() - 1),
        ] {
            f.render_widget(
                Paragraph::new(label)
                    .alignment(ratatui::layout::Alignment::Right)
                    .style(axis_style),
                Rect::new(plot.x, y, axis_width.saturating_sub(1), 1),
            );
        }
        for y in canvas.y..canvas.bottom() {
            f.render_widget(
                Paragraph::new(if y == canvas.y + canvas.height / 2 {
                    "┼"
                } else {
                    "│"
                })
                .style(axis_style),
                Rect::new(canvas.x - 1, y, 1, 1),
            );
        }
        f.render_widget(
            Paragraph::new(format!("└{}", "─".repeat(canvas.width as usize))).style(axis_style),
            Rect::new(canvas.x - 1, canvas.bottom(), canvas.width + 1, 1),
        );
        let ticks = canvas.bottom() + 1;
        f.render_widget(
            Paragraph::new(format!("-{}m", seconds / 60)).style(axis_style),
            Rect::new(canvas.x, ticks, canvas.width, 1),
        );
        f.render_widget(
            Paragraph::new("now")
                .alignment(ratatui::layout::Alignment::Right)
                .style(axis_style),
            Rect::new(canvas.x, ticks, canvas.width, 1),
        );
        f.render_widget(
            Canvas::default()
                .marker(Marker::Braille)
                .background_color(
                    design::surface(p)
                        .bg
                        .unwrap_or(ratatui::style::Color::Reset),
                )
                .x_bounds([start, end])
                // Each half uses its own rate ceiling; normalized coordinates keep zero centered.
                .y_bounds([-1.0, 1.0])
                .paint(|ctx| {
                    ctx.draw(&CanvasLine {
                        x1: start,
                        y1: 0.0,
                        x2: end,
                        y2: 0.0,
                        color: axis_style.fg.unwrap_or(ratatui::style::Color::Reset),
                    });
                    ctx.layer();
                    for i in [1, 0, 2].into_iter().filter(|i| mask & (1 << i) != 0) {
                        let color = chart::source_style(p, i)
                            .fg
                            .unwrap_or(ratatui::style::Color::Reset);
                        for (segments, scale) in [
                            (&prepared[i].down, &scales[0]),
                            (&prepared[i].up, &scales[1]),
                        ] {
                            for segment in segments {
                                ctx.draw(&chart::Segment {
                                    coords: segment,
                                    ceiling: scale.top,
                                    color,
                                });
                            }
                        }
                        // Isolate source colors without allocating a layer for every gap.
                        if prepared[i].count > 0 {
                            ctx.layer();
                        }
                    }
                }),
            canvas,
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
            (107, "STALE", "Down unknown"),
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
            let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            for label in [state, down, "window peak D 25.0 kbps", "1 rate samples"] {
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
        assert!(text.contains("WARMING/GAP") && text.contains("window peak D unknown"));
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
        assert_eq!(
            view.curves.builds, 1,
            "unchanged source curves must be reused"
        );
        eprintln!(
            "maximum mirrored chart: 3 x 1801 points, 100 cached debug redraws {:?}",
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
        let mut gapped = (**d.worker_util.as_ref().unwrap()).clone();
        for (i, point) in gapped.traffic.iter_mut().enumerate() {
            if i % 2 == 1 {
                point.down_mbps = None;
                point.up_mbps = None;
            }
        }
        gapped.classification.mapped.history = gapped.traffic.clone();
        gapped.classification.unmapped.history = gapped.traffic.clone();
        let gapped = Data {
            worker_util: Some(Arc::new(gapped)),
            ..Default::default()
        };
        let start = std::time::Instant::now();
        for _ in 0..100 {
            term.draw(|f| view.chart(f, f.area(), &gapped, 4000, &Palette::new(Theme::Dark)))
                .unwrap();
        }
        assert_eq!(view.curves.builds, 2, "gapped curves must also be cached");
        eprintln!(
            "maximum gapped mirrored chart: 3 x 901 segments, 100 cached debug redraws {:?}",
            start.elapsed()
        );
    }
    #[test]
    fn directional_ceilings_track_their_own_visible_window_peaks() {
        use crate::worker_util::{Telemetry, traffic::Point};
        use std::sync::Arc;
        let point = |at, down, up| Point {
            at,
            down_mbps: Some(down),
            up_mbps: Some(up),
        };
        let mut report = Telemetry::default();
        report
            .traffic
            .extend([point(100, 100.0, 1.0), point(400, 0.025, 0.01)]);
        report
            .classification
            .mapped
            .history
            .push_back(point(400, 1.0, 40.0));
        let mut d = Data {
            worker_util: Some(Arc::new(report)),
            ..Default::default()
        };
        let mut view = Overview::default();
        let mut term = Terminal::new(TestBackend::new(200, 20)).unwrap();
        let mut render = |view: &mut Overview, d: &Data, now| {
            term.draw(|f| view.chart(f, f.area(), d, now, &Palette::new(Theme::Dark)))
                .unwrap();
            term.backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>()
        };
        let check = |text: String, down: &str, up: &str| {
            assert!(text.contains(&format!("D {down}")), "{text}");
            assert!(text.contains(&format!("U {up}")), "{text}");
        };
        check(render(&mut view, &d, 400), "110.0 Mbps", "44.0 Mbps");
        // Expiring a download peak must not change the independent upload ceiling.
        check(render(&mut view, &d, 401), "1.1 Mbps", "44.0 Mbps");
        view.traces.toggle(1);
        check(render(&mut view, &d, 401), "28.0 kbps", "11.0 kbps");
        let mut report = (**d.worker_util.as_ref().unwrap()).clone();
        report.traffic.push_back(point(402, 11.0, 2.0));
        d.worker_util = Some(Arc::new(report));
        check(render(&mut view, &d, 402), "13.0 Mbps", "2.2 Mbps");
        check(render(&mut view, &d, 703), "1.0 kbps", "1.0 kbps");
    }
    #[test]
    fn combined_chart_defaults_to_all_sources_and_keeps_mouse_focus_and_resizing_safe() {
        use std::sync::Arc;
        let report=serde_json::from_value(serde_json::json!({"version":1,"updated":100,"stale":false,"reason":null,
            "workers":[{"queue_id":0,"busy_ppm":125000}],"traffic":[{"at":100,"down_mbps":10.0,"up_mbps":1.0}],
            "classification":{"mapped":{"bytes":{"download_bytes":1,"upload_bytes":1},"history":[{"at":100,"down_mbps":8.0,"up_mbps":4.0}]},"unmapped":{"bytes":{"download_bytes":0,"upload_bytes":0},"history":[{"at":100,"down_mbps":0.0,"up_mbps":0.0}]}}})).unwrap();
        let d = Data {
            worker_util: Some(Arc::new(report)),
            ..Default::default()
        };
        let p = Palette::new(Theme::Dark);
        let mut v = Overview::default();
        let mut term = Terminal::new(TestBackend::new(200, 40)).unwrap();
        let render = |term: &mut Terminal<TestBackend>, v: &mut Overview, now| {
            term.draw(|f| v.draw(f, f.area(), &d, now, &p)).unwrap();
            term.backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        };
        let text = render(&mut term, &mut v, 100);
        for label in [
            "[x] Actual Down 10.0 Mbps Up 1.0 Mbps",
            "[x] Mapped Down 8.000 Mbps Up 4.000 Mbps",
            "[x] Unmapped Down 0.000 Mbps Up 0.000 Mbps",
            "Actual delivered; Mapped/Unmapped enqueued",
            "D 11.0 Mbps",
            "U 4.4 Mbps",
        ] {
            assert!(text.contains(label), "missing {label}: {text}");
        }
        assert_eq!(v.traces.mask(), 7);
        for expected in [1, 2, 4, 7] {
            v.key(KeyCode::Char('v'), &d);
            assert_eq!(v.traces.mask(), expected);
        }
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
        assert_eq!(v.traces.mask(), 3);
        assert!(render(&mut term, &mut v, 100).contains("Focus: Selection"));
        v.key(KeyCode::Char('U'), &d);
        assert_eq!(v.traces.mask(), 7);
        v.key(KeyCode::Char('A'), &d);
        assert_eq!(v.traces.mask(), 6);
        v.key(KeyCode::Char('M'), &d);
        assert_eq!(v.traces.mask(), 4);
        v.key(KeyCode::Char('a'), &d);
        assert_eq!(v.traces.mask(), 7);
        let text = render(&mut term, &mut v, 107);
        assert!(
            text.contains("STALE")
                && text.contains("Down unknown")
                && text.contains("Actual window peak D 10.0 Mbps")
        );
        for i in 0..3 {
            if v.traces.mask() & (1 << i) != 0 {
                v.traces.toggle(i);
            }
        }
        assert!(render(&mut term, &mut v, 100).contains("No traces selected"));
        let r = v.all_sources;
        v.mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: r.x,
                row: r.y,
                modifiers: KeyModifiers::NONE,
            },
            &d,
        );
        assert_eq!(v.traces.mask(), 7);
        let r = v.focus_source;
        v.mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: r.x,
                row: r.y,
                modifiers: KeyModifiers::NONE,
            },
            &d,
        );
        assert_eq!(v.traces.mask(), 1);
        v.clear();
        assert_eq!(v.sources, [Rect::default(); 3]);
        assert_eq!(v.all_sources, Rect::default());
        for theme in [Theme::Dark, Theme::Light, Theme::Mono] {
            for (width, height) in [(40, 24), (80, 24), (120, 40), (240, 40)] {
                let mut term = Terminal::new(TestBackend::new(width, height)).unwrap();
                v.scroll.offset = 100;
                for _ in 0..4 {
                    term.draw(|f| v.draw(f, f.area(), &d, 100, &Palette::new(theme)))
                        .unwrap();
                    for r in v
                        .sources
                        .into_iter()
                        .chain(v.ranges)
                        .chain([v.all_sources, v.focus_source])
                    {
                        if r.width > 0 && r.height > 0 {
                            assert!(r.right() <= width && r.bottom() <= height);
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
