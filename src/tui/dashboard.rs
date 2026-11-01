//! The Dashboard tab: the service, the last cycle, versions, the process, and the actions.

use super::data::Data;
use crate::tui::theme::{BoxKind, Grad, Palette, Role, clip};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table};
use serde_json::Value;
use std::collections::VecDeque;

/// Graph samples kept for this TUI session.
/// The CPU samples are in tenths of a percent, so 100% of one core is 1000.
const CPU_SCALE: u64 = 1000;

/// `YYYY-MM-DD hh:mm:ss` of a Unix time, in UTC (days to civil date, H. Hinnant).
pub fn fmt_utc(unix: u64) -> String {
    let (days, secs) = ((unix / 86_400) as i64, unix % 86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}

fn fmt_elapsed(s: u64) -> String {
    format!(
        "{}d {:02}:{:02}:{:02}",
        s / 86_400,
        s / 3600 % 24,
        s / 60 % 60,
        s % 60
    )
}

/// The text lines of the cards. A missing value shows `unknown`, never 0.
pub fn lines(d: &Data, now: u64) -> Vec<Line<'static>> {
    let null = Value::Null;
    let st = d.status.as_deref().unwrap_or(&null);
    let s = |k: &str| st[k].as_str().map_or("unknown".to_string(), str::to_string);
    // With no status.json, "no error" is not known.
    let none = if d.status.is_some() {
        "none"
    } else {
        "unknown"
    };
    let n = |v: &Value| v.as_u64().map_or("unknown".to_string(), |x| x.to_string());
    let mut out: Vec<String> = vec![
        "[service]".into(),
        format!(
            "service: {}",
            d.service.as_ref().map_or("unknown", |u| u.active.as_str())
        ),
        format!("mode: {}", s("mode")),
        format!("target: {}", s("target")),
        "[last cycle]".into(),
        format!(
            "time: {}",
            st["updated"].as_u64().map_or("unknown".into(), |u| format!(
                "{} UTC ({}s ago)",
                fmt_utc(u),
                now.saturating_sub(u)
            ))
        ),
        format!(
            "cycle time: {}",
            st["cycle_ms"]
                .as_u64()
                .map_or("unknown".into(), |m| format!("{m} ms"))
        ),
        format!("cycle seq: {}", n(&st["cycle_seq"])),
        format!("trigger: {}", s("trigger")),
        format!(
            "files changed: {}",
            st["files_changed"]
                .as_bool()
                .map_or("unknown".into(), |b| b.to_string())
        ),
        format!("reload: {}", s("reload")),
        format!("error: {}", st["error"].as_str().unwrap_or(none)),
        format!(
            "config error: {}",
            st["config_error"].as_str().unwrap_or(none)
        ),
        format!("telegram: {}", s("telegram")),
        "[routers]".into(),
    ];
    match st["routers"].as_array() {
        Some(rs) if !rs.is_empty() => {
            for r in rs {
                let rows: u64 = r["rows"]
                    .as_object()
                    .map_or(0, |m| m.values().filter_map(Value::as_u64).sum());
                let state = r["state"].as_str().unwrap_or("unknown");
                // A router that is not online has no row counts.
                let rows = if state == "online" {
                    format!("{rows} rows")
                } else {
                    "rows unknown".into()
                };
                let mut l = format!(
                    "{}: {state}, {rows}, {} ms",
                    r["name"].as_str().unwrap_or("?"),
                    n(&r["read_ms"])
                );
                if let Some(e) = r["error"].as_str() {
                    l.push_str(&format!(" ({e})"));
                }
                out.push(l);
            }
        }
        _ => out.push("routers: unknown".into()),
    }
    out.push("[counts]".into());
    let c = &st["report"]["counts"];
    for k in ["added", "updated", "removed", "held", "duplicates"] {
        out.push(format!("{k}: {}", n(&c[k])));
    }
    out.push(format!(
        "guard alerts: {}",
        st["report"]["alerts"]
            .as_array()
            .map_or("unknown".into(), |a| a.len().to_string())
    ));
    out.push("[versions]".into());
    out.push(format!(
        "LibreQoS: {}",
        d.versions.libreqos.as_deref().unwrap_or("unknown")
    ));
    out.push(format!(
        "lqosd: {}",
        d.versions
            .lqosd
            .as_ref()
            .map_or("unknown".into(), |(u, a)| format!("{u} {a}"))
    ));
    let tui = env!("CARGO_PKG_VERSION");
    let svc = st["program_version"].as_str();
    out.push(format!(
        "lqosync3 service: {} | TUI: {tui}",
        svc.unwrap_or("unknown")
    ));
    if svc.is_some_and(|v| v != tui) {
        out.push("the versions are different: the service needs a restart".into());
    }
    out.push(format!(
        "config: {}",
        match st["config_digest"].as_str().filter(|x| !x.is_empty()) {
            None => "unknown",
            Some(x) if x == crate::cli::config_digest(d.config_text.as_bytes()) => "loaded",
            Some(_) => "the file on disk is not loaded (press R to reload)",
        }
    ));
    out.push("[process]".into());
    match &d.proc {
        Some(p) => {
            out.push(format!("PID: {}", p.pid));
            out.push(format!("start: {} UTC", fmt_utc(p.start_unix)));
            out.push(format!("elapsed: {}", fmt_elapsed(p.elapsed_s)));
            out.push(format!("RSS: {:.1} MB", p.rss_kb as f64 / 1024.0));
            out.push(format!(
                "CPU: {}",
                p.cpu_pct.map_or("unknown".into(), |c| format!("{c:.1} %"))
            ));
        }
        None => {
            out.push("PID: unknown (the service is not running, or systemd is not readable)".into())
        }
    }
    out.into_iter().map(Line::from).collect()
}

/// Check the PID before a signal: not 0, not this process, and the name is `lqosync3`.
pub fn signal_target(
    main_pid: u32,
    own_pid: u32,
    proc_comm: impl Fn(u32) -> Option<String>,
) -> Result<u32, String> {
    if main_pid == 0 {
        return Err("the service has no MainPID (it is not running)".into());
    }
    if main_pid == own_pid {
        return Err("the MainPID is this TUI".into());
    }
    match proc_comm(main_pid) {
        Some(c) if c.trim() == "lqosync3" => Ok(main_pid),
        Some(c) => Err(format!("PID {main_pid} is \"{}\", not lqosync3", c.trim())),
        None => Err(format!("PID {main_pid} does not exist")),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    SyncNow,
    Reload,
}

/// A signal that the TUI sent, and what it waits for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    pub kind: Kind,
    /// Unix seconds of the send.
    pub at: u64,
    pub pid: u32,
    /// `cycle_seq` at the send.
    pub seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wait {
    Pending,
    Done(String),
    Restarted,
    TimedOut,
}

/// The result of an action, from `status.json`, not from the signal call.
/// `timeout` is `tui.apply_verify_timeout` in seconds.
pub fn wait_result(sent: &Sent, status: Option<&Value>, now: u64, timeout: u64) -> Wait {
    if let Some(st) = status {
        let pid = st["pid"].as_u64();
        if pid.is_some_and(|p| p != u64::from(sent.pid)) {
            return Wait::Restarted;
        }
        let trigger = st["trigger"].as_str();
        match sent.kind {
            Kind::SyncNow => {
                if pid.is_some()
                    && trigger == Some("sync_now")
                    && st["cycle_seq"].as_u64().is_some_and(|q| q > sent.seq)
                {
                    let c = &st["report"]["counts"];
                    let n = |k: &str| c[k].as_u64().unwrap_or(0);
                    return Wait::Done(match st["error"].as_str() {
                        Some(e) => format!("sync now: the cycle has an error: {e}"),
                        None => format!(
                            "sync now done: added {} updated {} removed {}",
                            n("added"),
                            n("updated"),
                            n("removed")
                        ),
                    });
                }
            }
            Kind::Reload => {
                if trigger == Some("reload") && st["updated"].as_u64().is_some_and(|u| u >= sent.at)
                {
                    return Wait::Done(match st["config_error"].as_str() {
                        Some(e) => format!("reload failed, the old config stays: {e}"),
                        None => format!(
                            "reload done: config {} is loaded",
                            st["config_digest"].as_str().unwrap_or("unknown")
                        ),
                    });
                }
            }
        }
    }
    if now >= sent.at + timeout {
        Wait::TimedOut
    } else {
        Wait::Pending
    }
}

/// Send `SIGUSR1` (sync now) or `SIGHUP` (reload) to the MainPID of the service.
/// `main_pid` comes from systemd at the send, not from the last loader sample (the service can restart).
pub fn send(kind: Kind, d: &Data, now: u64, main_pid: Option<u32>) -> Result<Sent, String> {
    let main_pid = main_pid.ok_or("cannot read the MainPID from systemd (systemctl show)")?;
    let pid = signal_target(main_pid, std::process::id(), |p| {
        std::fs::read_to_string(format!("/proc/{p}/comm")).ok()
    })?;
    let sig = match kind {
        Kind::SyncNow => libc::SIGUSR1,
        Kind::Reload => libc::SIGHUP,
    };
    // SAFETY: kill has no memory preconditions; the PID was checked above.
    if unsafe { libc::kill(pid as libc::pid_t, sig) } != 0 {
        return Err(format!(
            "cannot send the signal to PID {pid}: {}",
            std::io::Error::last_os_error()
        ));
    }
    let seq = d
        .status
        .as_ref()
        .and_then(|s| s["cycle_seq"].as_u64())
        .unwrap_or(0);
    Ok(Sent {
        kind,
        at: now,
        pid,
        seq,
    })
}

#[derive(Debug)]
pub struct DashboardView {
    /// CPU percent in tenths, and RSS in kB.
    pub cpu: VecDeque<u64>,
    pub rss: VecDeque<u64>,
    /// `tui.graph_samples`.
    pub samples: usize,
}

impl Default for DashboardView {
    fn default() -> Self {
        Self {
            cpu: VecDeque::new(),
            rss: VecDeque::new(),
            samples: crate::config::defaults().tui.graph_samples,
        }
    }
}

impl DashboardView {
    /// Add one graph sample for each new data.
    pub fn on_data(&mut self, d: &Data) {
        if let Some(p) = &d.proc {
            // An unknown CPU value (the first sample) is not drawn as 0.
            let cpu = p.cpu_pct.map(|c| (c * (CPU_SCALE as f64 / 100.0)) as u64);
            for (q, v) in [(&mut self.cpu, cpu), (&mut self.rss, Some(p.rss_kb))] {
                let Some(v) = v else { continue };
                while q.len() >= self.samples.max(1) {
                    q.pop_front();
                }
                q.push_back(v);
            }
        }
    }

    pub fn draw(&self, f: &mut Frame, area: Rect, d: &Data, now: u64, palette: &Palette) {
        let areas = card_areas(area);
        let titles = [
            "Service",
            "Last cycle",
            "Routers",
            "Counts",
            "Versions",
            "Process",
        ];
        let mut sections: [Vec<String>; 6] = std::array::from_fn(|_| Vec::new());
        let mut index = 0;
        for line in lines(d, now) {
            let text = line.to_string();
            match text.as_str() {
                "[service]" => index = 0,
                "[last cycle]" => index = 1,
                "[routers]" => index = 2,
                "[counts]" => index = 3,
                "[versions]" => index = 4,
                "[process]" => index = 5,
                _ => sections[index].push(text),
            }
        }
        // Present failures first without changing status router order.
        sections[2].sort_by_key(|s| !(s.contains(": failed") || s.contains(": offline")));
        let st = d.status.as_deref();
        let bad = |s: &String| {
            (s.starts_with("error:") || s.starts_with("config error:"))
                && !s.ends_with(": none")
                && !s.ends_with(": unknown")
        };
        sections[1].sort_by_key(|s| !bad(s));
        sections[4].sort_by_key(|s| !(s.contains("different") || s.contains("not loaded")));
        let n = |key: &str| {
            st.and_then(|s| s["report"]["counts"][key].as_u64())
                .map_or("unknown".into(), |v| v.to_string())
        };
        sections[3] = vec![
            format!("added {} | updated {}", n("added"), n("updated")),
            format!("removed {} | held {}", n("removed"), n("held")),
            format!("duplicates {}", n("duplicates")),
            sections[3]
                .last()
                .cloned()
                .unwrap_or_else(|| "guard alerts: unknown".into()),
        ];
        for i in 0..6 {
            let kind = [
                BoxKind::Mem,
                BoxKind::Proc,
                BoxKind::Net,
                BoxKind::Mem,
                BoxKind::Mem,
                BoxKind::Cpu,
            ][i];
            let mut block = palette.panel(kind, titles[i], false);
            if i == 5 {
                let context = d.proc.as_ref().map_or(" PID unknown ".into(), |p| {
                    format!(
                        " PID {}   elapsed {}   start {} UTC ",
                        p.pid,
                        fmt_elapsed(p.elapsed_s),
                        fmt_utc(p.start_unix)
                    )
                });
                let context = if d.proc.is_none() && (!self.cpu.is_empty() || !self.rss.is_empty())
                {
                    " PID unknown   history only ".into()
                } else {
                    context
                };
                block = block.title_bottom(Line::styled(
                    clip(&context, areas[i].width.saturating_sub(2) as usize),
                    palette.role(Role::Secondary),
                ));
            }
            let inner = block.inner(areas[i]);
            f.render_widget(block, areas[i]);
            if i == 5 {
                self.draw_process(f, inner, d, palette);
                continue;
            }
            if i == 2 && inner.width >= 55 {
                draw_routers(f, inner, d, palette);
                continue;
            }
            let mut content = sections[i].clone();
            if i == 0 && inner.height < 3 {
                content = vec![format!(
                    "service: {} | {}",
                    d.service.as_ref().map_or("unknown", |u| u.active.as_str()),
                    d.status
                        .as_ref()
                        .and_then(|s| s["mode"].as_str())
                        .unwrap_or("unknown")
                )];
            }
            if i == 3 && inner.height < 4 {
                let alerts = st
                    .and_then(|s| s["report"]["alerts"].as_array())
                    .map_or("unknown".into(), |a| a.len().to_string());
                content = vec![
                    format!("added {} | updated {}", n("added"), n("updated")),
                    format!("removed {} | held {}", n("removed"), n("held")),
                    format!("dup {} | guard alerts {alerts}", n("duplicates")),
                ];
            }
            if i == 4 && inner.height < content.len() as u16 {
                let loaded = d
                    .status
                    .as_ref()
                    .and_then(|s| s["config_digest"].as_str())
                    .filter(|s| !s.is_empty())
                    .map_or("unknown", |v| {
                        if v == crate::cli::config_digest(d.config_text.as_bytes()) {
                            "loaded"
                        } else {
                            "not loaded"
                        }
                    });
                let svc = d
                    .status
                    .as_ref()
                    .and_then(|s| s["program_version"].as_str())
                    .unwrap_or("unknown");
                let tui = env!("CARGO_PKG_VERSION");
                let version = if svc != "unknown" && svc != tui {
                    format!("version mismatch: {svc} / {tui}")
                } else {
                    format!("TUI {tui} | service {svc}")
                };
                content = vec![version, format!("config: {loaded}")];
            }
            if i == 1 && content.len() > inner.height as usize {
                let null = Value::Null;
                let st = d.status.as_deref().unwrap_or(&null);
                let text = |key: &str| st[key].as_str().unwrap_or("unknown");
                let mut compact = Vec::new();
                for key in ["error", "config_error"] {
                    if let Some(error) = st[key].as_str().filter(|s| !s.is_empty()) {
                        compact.push(format!(
                            "{}: {error}",
                            if key == "config_error" {
                                "config error"
                            } else {
                                key
                            }
                        ));
                    }
                }
                let write = st["files_changed"]
                    .as_bool()
                    .map_or("unknown", |b| if b { "changed" } else { "unchanged" });
                compact.push(format!("write: {write} | reload: {}", text("reload")));
                compact.push(format!(
                    "telegram: {} | trigger: {}",
                    text("telegram"),
                    text("trigger")
                ));
                let seq = st["cycle_seq"]
                    .as_u64()
                    .map_or("unknown".into(), |v| v.to_string());
                let ms = st["cycle_ms"]
                    .as_u64()
                    .map_or("unknown".into(), |v| v.to_string());
                compact.push(format!("cycle: {seq} | {ms} ms"));
                if compact.len() < inner.height as usize
                    && st["error"].as_str().is_none()
                    && st["config_error"].as_str().is_none()
                {
                    compact.push(format!(
                        "error: {}",
                        if d.status.is_none() {
                            "unknown"
                        } else {
                            "none"
                        }
                    ));
                }
                content = compact;
            }
            if i == 2 {
                for line in &mut content {
                    if Line::from(line.as_str()).width() > inner.width as usize
                        && let Some((name, rest)) = line.split_once(": ")
                    {
                        let state = rest.split(',').next().unwrap_or(rest);
                        let budget =
                            (inner.width as usize).saturating_sub(Line::from(state).width() + 3);
                        *line = format!("{}: {rest}", clip(name, budget));
                    }
                }
            }
            if i == 2 && content.len() > inner.height as usize && inner.height > 0 {
                let keep = inner.height.saturating_sub(1) as usize;
                let more = content.len() - keep;
                content.truncate(keep);
                content.push(format!("+{more} more"));
            }
            let rows = content
                .into_iter()
                .map(|text| {
                    let role = if text.contains(": offline") || bad(&text) {
                        Role::Error
                    } else if text.contains(": failed")
                        || text.contains("different")
                        || text.contains("mismatch")
                        || text.contains("not loaded")
                    {
                        Role::Warning
                    } else if text.contains(": online") || text.contains("service: active") {
                        Role::Ok
                    } else {
                        Role::Base
                    };
                    let text = clip(&text, inner.width as usize);
                    if let Some((label, value)) = text.split_once(": ") {
                        Line::from(vec![
                            Span::styled(format!("{label}: "), palette.role(Role::Secondary)),
                            Span::styled(value.to_string(), palette.role(role)),
                        ])
                    } else if i == 3 {
                        Line::from(
                            text.split_inclusive(' ')
                                .map(|part| {
                                    Span::styled(
                                        part.to_string(),
                                        palette.role(if part.trim().parse::<u64>().is_ok() {
                                            Role::Accent
                                        } else {
                                            Role::Secondary
                                        }),
                                    )
                                })
                                .collect::<Vec<_>>(),
                        )
                    } else {
                        Line::styled(text, palette.role(role))
                    }
                })
                .collect::<Vec<_>>();
            f.render_widget(Paragraph::new(rows), inner);
        }
    }

    fn draw_process(&self, f: &mut Frame, area: Rect, d: &Data, palette: &Palette) {
        if area.is_empty() {
            return;
        }
        let [cpu_area, divider, rss_area] = Layout::horizontal([
            Constraint::Percentage(60),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(area);
        f.render_widget(
            Paragraph::new(
                (0..divider.height)
                    .map(|_| Line::from("│"))
                    .collect::<Vec<_>>(),
            )
            .style(palette.role(Role::Secondary)),
            divider,
        );
        let cpu = d.proc.as_ref().and_then(|p| p.cpu_pct);
        let peak = self.rss.iter().copied().max().unwrap_or(0);
        for (i, box_area) in [cpu_area, rss_area].into_iter().enumerate() {
            let [heading, meter, trace, legend] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .areas(box_area);
            let (label, role, ratio, scale, data, range) = if i == 0 {
                let label = cpu.map_or("CPU % unknown".into(), |v| {
                    format!(
                        "CPU % {v:.1}{}",
                        if v > 100.0 { " >100% of one core" } else { "" }
                    )
                });
                (
                    label,
                    Role::Accent,
                    cpu.map_or(0.0, |v| fill_ratio(v, 100.0)),
                    CPU_SCALE,
                    &self.cpu,
                    "0 ─ 100% of one core".to_string(),
                )
            } else {
                let label = d.proc.as_ref().map_or("RSS unknown".into(), |p| {
                    format!(
                        "RSS {:.1}/{:.1} MiB",
                        p.rss_kb as f64 / 1024.0,
                        peak as f64 / 1024.0
                    )
                });
                (
                    label,
                    Role::Ok,
                    d.proc
                        .as_ref()
                        .map_or(0.0, |p| fill_ratio(p.rss_kb as f64, peak as f64)),
                    peak.max(1),
                    &self.rss,
                    "current / window peak".to_string(),
                )
            };
            f.render_widget(
                Paragraph::new(clip(&label, heading.width as usize)).style(palette.role(role)),
                heading,
            );
            let grad = if i == 0 { Grad::Cpu } else { Grad::Mem };
            f.render_widget(
                Paragraph::new(palette.meter(ratio, meter.width, grad)),
                meter,
            );
            draw_history(f, trace, data, scale, palette, grad);
            f.render_widget(
                Paragraph::new(clip(&range, legend.width as usize))
                    .style(palette.role(Role::Secondary)),
                legend,
            );
        }
    }
}

fn draw_history(
    f: &mut Frame,
    area: Rect,
    data: &VecDeque<u64>,
    scale: u64,
    palette: &Palette,
    grad: Grad,
) {
    if area.is_empty() {
        return;
    }
    if data.is_empty() {
        f.render_widget(
            Paragraph::new("no samples yet").style(palette.role(Role::Secondary)),
            area,
        );
        return;
    }
    let samples: Vec<u64> = data.iter().copied().collect();
    f.render_widget(
        Paragraph::new(palette.area_graph(&samples, scale, area.width, area.height, grad)),
        area,
    );
}

fn draw_routers(f: &mut Frame, area: Rect, d: &Data, palette: &Palette) {
    let Some(routers) = d
        .status
        .as_ref()
        .and_then(|s| s["routers"].as_array())
        .filter(|rs| !rs.is_empty())
    else {
        f.render_widget(Paragraph::new("routers: unknown"), area);
        return;
    };
    let mut routers = routers.iter().collect::<Vec<_>>();
    routers.sort_by_key(|r| !matches!(r["state"].as_str(), Some("failed" | "offline")));
    let error = routers.iter().find_map(|r| {
        r["error"]
            .as_str()
            .map(|e| format!("{}: {e}", r["name"].as_str().unwrap_or("?")))
    });
    let [body, note] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(u16::from(error.is_some())),
    ])
    .areas(area);
    let capacity = body.height.saturating_sub(1) as usize;
    let more = routers.len().saturating_sub(capacity);
    let keep = capacity.saturating_sub(usize::from(more > 0));
    let mut rows = routers
        .iter()
        .take(keep)
        .map(|r| {
            let state = r["state"].as_str().unwrap_or("unknown");
            let role = match state {
                "online" => Role::Ok,
                "offline" => Role::Error,
                "failed" => Role::Warning,
                _ => Role::Secondary,
            };
            let count = if state == "online" {
                r["rows"]
                    .as_object()
                    .map(|m| {
                        m.values()
                            .filter_map(Value::as_u64)
                            .sum::<u64>()
                            .to_string()
                    })
                    .unwrap_or("unknown".into())
            } else {
                "unknown".into()
            };
            let ms = r["read_ms"]
                .as_u64()
                .map_or("unknown".into(), |v| v.to_string());
            Row::new(vec![
                Cell::from(r["name"].as_str().unwrap_or("?")),
                Cell::from(state).style(palette.role(role)),
                Cell::from(Line::from(count).right_aligned()),
                Cell::from(Line::from(ms).right_aligned()),
            ])
        })
        .collect::<Vec<_>>();
    if more > 0 && capacity > 0 {
        rows.push(Row::new(vec![Cell::from(format!(
            "+{} more",
            routers.len() - keep
        ))]));
    }
    f.render_widget(
        Table::new(
            rows,
            [
                Constraint::Fill(1),
                Constraint::Length(11),
                Constraint::Length(7),
                Constraint::Length(9),
            ],
        )
        .header(Row::new(["Router", "State", "Rows", "Read ms"]).style(palette.role(Role::Accent)))
        .column_spacing(2),
        body,
    );
    if let Some(error) = error {
        f.render_widget(
            Paragraph::new(clip(&error, note.width as usize)).style(palette.role(Role::Warning)),
            note,
        );
    }
}

fn card_areas(area: Rect) -> [Rect; 6] {
    let graph_height = ((u32::from(area.height) * 2 / 5) as u16)
        .max(8)
        // The cards need about 18 rows; the graphs take the rest.
        .max(area.height.saturating_sub(18))
        .min(area.height);
    let [process, lower] =
        Layout::vertical([Constraint::Length(graph_height), Constraint::Min(0)]).areas(area);
    let [left, right] = Layout::horizontal([Constraint::Percentage(42), Constraint::Min(0)])
        .spacing(1)
        .areas(lower);
    let [service, counts, versions] = Layout::vertical([
        Constraint::Length(if lower.height >= 18 { 5 } else { 3 }),
        Constraint::Length(if lower.height >= 18 { 6 } else { 5 }),
        Constraint::Min(0),
    ])
    .areas(left);
    let [cycle, routers] = Layout::vertical([
        if lower.height >= 18 {
            Constraint::Length(11)
        } else {
            Constraint::Percentage(50)
        },
        Constraint::Min(0),
    ])
    .areas(right);
    [service, cycle, routers, counts, versions, process]
}

fn fill_ratio(value: f64, scale: f64) -> f64 {
    if !value.is_finite() || !scale.is_finite() || scale <= 0.0 {
        return 0.0;
    }
    (value / scale).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::proc::{ProcInfo, Unit};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use serde_json::json;
    use std::sync::Arc;

    fn data(status: Value) -> Data {
        Data {
            status: Some(Arc::new(status)),
            config_text: Arc::new("a".into()),
            ..Data::default()
        }
    }

    fn text(d: &Data) -> Vec<String> {
        lines(d, 2_000).into_iter().map(|l| l.to_string()).collect()
    }

    #[test]
    fn unknown_values_never_show_zero() {
        let d = data(
            json!({"version": 2, "mode": "apply", "target": "v2_pair", "updated": 1_000,
            "routers": [], "report": {"counts": {}}, "files_changed": false, "reload": "skipped"}),
        );
        let t = text(&d);
        for want in [
            "cycle seq: unknown",
            "trigger: unknown",
            "telegram: unknown",
            "cycle time: unknown",
            "PID: unknown",
            "config: unknown",
            "added: unknown",
        ] {
            assert!(t.iter().any(|l| l.contains(want)), "{want}: {t:#?}");
        }
        assert!(!t.iter().any(|l| l.ends_with(": 0")), "{t:#?}");
        let none = text(&Data::default());
        assert!(
            none.iter().any(|l| l.contains("mode: unknown")),
            "{none:#?}"
        );
    }

    #[test]
    fn digest_check_shows_not_loaded() {
        let d = data(json!({"config_digest": crate::cli::config_digest(b"b")}));
        assert!(
            text(&d)
                .iter()
                .any(|l| l.contains("the file on disk is not loaded"))
        );
        let d = data(json!({"config_digest": crate::cli::config_digest(b"a")}));
        assert!(text(&d).iter().any(|l| l == "config: loaded"));
    }

    #[test]
    fn version_difference_says_restart() {
        let d = data(json!({"program_version": "0.0.1"}));
        assert!(
            text(&d)
                .iter()
                .any(|l| l.contains("service needs a restart"))
        );
        let d = data(json!({"program_version": env!("CARGO_PKG_VERSION")}));
        assert!(!text(&d).iter().any(|l| l.contains("restart")));
    }

    #[test]
    fn process_values_show_with_units() {
        let mut d = data(json!({}));
        d.service = Some(Unit {
            load: "loaded".into(),
            active: "active".into(),
            main_pid: 42,
        });
        d.proc = Some(ProcInfo {
            pid: 42,
            start_unix: 1_700_000_000,
            elapsed_s: 90_061,
            rss_kb: 12_800,
            cpu_pct: Some(1.25),
        });
        let t = text(&d);
        for want in [
            "PID: 42",
            "start: 2023-11-14 22:13:20 UTC",
            "elapsed: 1d 01:01:01",
            "RSS: 12.5 MB",
            "CPU: 1.2 %",
            "service: active",
        ] {
            assert!(t.iter().any(|l| l.contains(want)), "{want}: {t:#?}");
        }
    }

    #[test]
    fn fmt_utc_known_values() {
        assert_eq!(fmt_utc(0), "1970-01-01 00:00:00");
        assert_eq!(fmt_utc(1_700_000_000), "2023-11-14 22:13:20");
        assert_eq!(fmt_utc(951_782_400), "2000-02-29 00:00:00");
    }

    #[test]
    fn signal_target_rejects_zero_own_pid_and_other_comm() {
        let comm = |p: u32| {
            Some(if p == 7 {
                "lqosync3".to_string()
            } else {
                "bash".to_string()
            })
        };
        assert_eq!(signal_target(7, 99, comm), Ok(7));
        assert!(signal_target(0, 99, comm).is_err());
        assert!(signal_target(7, 7, comm).is_err());
        assert!(signal_target(8, 99, comm).is_err());
        assert!(signal_target(7, 99, |_| None).is_err());
    }

    #[test]
    fn sync_now_waits_for_a_higher_cycle_seq_of_the_same_pid() {
        let sent = Sent {
            kind: Kind::SyncNow,
            at: 1_000,
            pid: 7,
            seq: 4,
        };
        let st = |pid: u32, seq: u64, trigger: &str| {
            json!({"pid": pid, "cycle_seq": seq, "trigger": trigger, "updated": 1_001,
            "report": {"counts": {"added": 1, "updated": 0, "removed": 0}}})
        };
        assert_eq!(
            wait_result(&sent, Some(&st(7, 4, "sync_now")), 1_001, 120),
            Wait::Pending
        );
        assert_eq!(
            wait_result(&sent, Some(&st(7, 5, "timer")), 1_001, 120),
            Wait::Pending
        );
        assert!(
            matches!(wait_result(&sent, Some(&st(7, 5, "sync_now")), 1_001, 120), Wait::Done(m) if m.contains("added 1"))
        );
        assert_eq!(
            wait_result(&sent, Some(&st(8, 1, "start")), 1_001, 120),
            Wait::Restarted
        );
        assert_eq!(wait_result(&sent, None, 1_200, 120), Wait::TimedOut);
    }

    #[test]
    fn reload_ignores_an_old_reload_status() {
        let sent = Sent {
            kind: Kind::Reload,
            at: 1_000,
            pid: 7,
            seq: 4,
        };
        let old = json!({"pid": 7, "trigger": "reload", "updated": 990, "config_digest": "x"});
        assert_eq!(wait_result(&sent, Some(&old), 1_001, 120), Wait::Pending);
        let ok = json!({"pid": 7, "trigger": "reload", "updated": 1_000, "config_digest": "abc"});
        assert!(
            matches!(wait_result(&sent, Some(&ok), 1_001, 120), Wait::Done(m) if m.contains("abc"))
        );
        let bad =
            json!({"pid": 7, "trigger": "reload", "updated": 1_002, "config_error": "line 3"});
        assert!(
            matches!(wait_result(&sent, Some(&bad), 1_003, 120), Wait::Done(m) if m.contains("old config stays"))
        );
    }

    #[test]
    fn send_uses_the_fresh_main_pid_not_the_cached_one() {
        let mut d = data(json!({}));
        // The cached sample is this process; the fresh systemd value says that the service stopped.
        d.service = Some(Unit {
            load: "loaded".into(),
            active: "active".into(),
            main_pid: std::process::id(),
        });
        let e = send(Kind::Reload, &d, 1_000, Some(0)).unwrap_err();
        assert!(e.contains("no MainPID"), "{e}");
        let e = send(Kind::Reload, &d, 1_000, None).unwrap_err();
        assert!(e.contains("systemd"), "{e}");
    }

    #[test]
    fn no_status_and_a_failed_router_show_unknown() {
        let t = text(&Data::default());
        for want in ["error: unknown", "config error: unknown"] {
            assert!(t.iter().any(|l| l == want), "{want}: {t:#?}");
        }
        let d = data(
            json!({"routers": [{"name": "R1", "state": "failed", "rows": {}, "read_ms": 5, "error": "trap"}]}),
        );
        let t = text(&d);
        assert!(
            t.iter().any(|l| l.starts_with("R1: failed, rows unknown")),
            "{t:#?}"
        );
    }

    #[test]
    fn dashboard_tab_renders() {
        let mut v = DashboardView::default();
        let mut d = data(
            json!({"mode": "apply", "routers": [{"name": "R1", "state": "online", "rows": {"/ppp/active": 3}, "read_ms": 12}]}),
        );
        d.proc = Some(ProcInfo {
            pid: 42,
            start_unix: 1,
            elapsed_s: 2,
            rss_kb: 1_000,
            cpu_pct: Some(3.0),
        });
        v.on_data(&d);
        v.on_data(&d);
        assert_eq!(v.cpu.len(), 2);
        let mut t = Terminal::new(TestBackend::new(120, 40)).unwrap();
        t.draw(|f| {
            v.draw(
                f,
                f.area(),
                &d,
                2_000,
                &Palette::new(crate::config::Theme::Dark),
            )
        })
        .unwrap();
        let buf = t.backend().buffer();
        let s: String = (0..40)
            .map(|y| (0..120).map(|x| buf[(x, y)].symbol()).collect::<String>() + "\n")
            .collect();
        for want in ["R1", "online", "Read ms", "CPU %", "RSS"] {
            assert!(s.contains(want), "{want}\n{s}");
        }
    }
    #[test]
    fn a_very_tall_area_does_not_overflow() {
        let cards = card_areas(Rect::new(0, 0, 1, 60_000));
        assert!(cards[5].height >= 24_000);
    }

    #[test]
    fn dashboard_card_geometry() {
        let cards = card_areas(Rect::new(0, 0, 80, 20));
        assert_eq!(cards[5], Rect::new(0, 0, 80, 8));
        assert_eq!(cards[0].y, 8);
        assert_eq!(cards[0].x, cards[3].x);
        assert!(cards[1].x > cards[0].x);
        for w in [80, 100, 119, 120] {
            let area = Rect::new(0, 0, w, 20);
            let cards = card_areas(area);
            for (i, a) in cards.iter().enumerate() {
                assert_eq!(a.intersection(area), *a);
                for b in cards.iter().skip(i + 1) {
                    assert!(a.intersection(*b).is_empty());
                }
            }
            assert!(cards[2].y > cards[5].y);
        }
    }
    #[test]
    fn the_graphs_keep_tui_graph_samples() {
        let mut v = DashboardView {
            samples: 3,
            ..DashboardView::default()
        };
        let mut d = Data::default();
        for i in 0..5 {
            d.proc = Some(ProcInfo {
                pid: 1,
                start_unix: 1,
                elapsed_s: 1,
                rss_kb: i,
                cpu_pct: Some(i as f64),
            });
            v.on_data(&d);
        }
        assert_eq!(v.rss, [2, 3, 4]);
        assert_eq!(v.cpu, [20, 30, 40]);
        assert_eq!(
            DashboardView::default().samples,
            crate::config::defaults().tui.graph_samples
        );
    }

    #[test]
    fn process_gauge_scales_and_unknowns() {
        assert_eq!(fill_ratio(25.0, 100.0), 0.25);
        assert_eq!(fill_ratio(150.0, 100.0), 1.0);
        for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(fill_ratio(1.0, scale), 0.0);
        }
        let mut v = DashboardView::default();
        let d = Data::default();
        v.on_data(&d);
        assert!(v.cpu.is_empty());
    }
    #[test]
    fn process_gauges_keep_values_and_both_traces() {
        let mut d = Data {
            proc: Some(ProcInfo {
                pid: 42,
                start_unix: 1,
                elapsed_s: 20,
                rss_kb: 1024,
                cpu_pct: Some(150.0),
            }),
            ..Data::default()
        };
        let mut v = DashboardView::default();
        v.on_data(&d);
        v.rss.push_back(2048);
        let mut t = Terminal::new(TestBackend::new(80, 20)).unwrap();
        let p = Palette::new(crate::config::Theme::Dark);
        t.draw(|f| v.draw(f, f.area(), &d, 30, &p)).unwrap();
        let b = t.backend().buffer();
        let process = card_areas(Rect::new(0, 0, 80, 20))[5];
        let text = (process.y..process.bottom())
            .map(|y| {
                (process.x..process.right())
                    .map(|x| b[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("150.0"));
        assert!(text.contains(">100%"));
        assert!(text.contains("1.0/2.0 MiB"));
        assert!(text.chars().any(|c| ('\u{2800}'..='\u{28ff}').contains(&c)));
        let inner = Rect::new(
            process.x + 1,
            process.y + 1,
            process.width.saturating_sub(2),
            process.height.saturating_sub(2),
        );
        let [cpu, _, rss] = Layout::horizontal([
            Constraint::Percentage(60),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(inner);
        for chart in [cpu, rss] {
            assert!((chart.y + 2..chart.bottom().saturating_sub(1)).any(|y| {
                (chart.x..chart.right()).any(|x| {
                    b[(x, y)]
                        .symbol()
                        .chars()
                        .any(|c| ('\u{2801}'..='\u{28ff}').contains(&c))
                })
            }));
        }
        assert_eq!(v.cpu.front(), Some(&1500));
        d.proc = None;
        t.draw(|f| v.draw(f, f.area(), &d, 30, &p)).unwrap();
        let b = t.backend().buffer();
        let text = (0..20)
            .map(|y| (0..80).map(|x| b[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("history only"));
        assert!(text.contains("CPU % unknown"));
    }
    #[test]
    #[ignore = "exports actual UI frames; set LQOSYNC_RENDER_DIR"]
    fn export_dashboard_frames() {
        use crate::tui::app::{App, Tab};
        let dir = std::path::PathBuf::from(
            std::env::var("LQOSYNC_RENDER_DIR").expect("set render output directory"),
        );
        std::fs::create_dir_all(&dir).unwrap();
        let config = "[[routers]]\nname=\"Fixture\"\naddress=\"192.0.2.1\"\nusername=\"fixture\"\npassword_file=\"/fixture/password\"\n";
        for theme in [
            crate::config::Theme::Dark,
            crate::config::Theme::Light,
            crate::config::Theme::Mono,
        ] {
            for (w, h) in [(80, 24), (120, 40), (160, 48)] {
                let mut app = App::new(false, config.into(), None);
                app.palette = Palette::new(theme);
                let mut d = data(
                    json!({"mode":"apply","target":"v2_pair","updated":1793505600,"next_scan_seconds":120,"cycle_ms":184,"cycle_seq":918,"trigger":"timer","files_changed":false,"reload":"skipped","telegram":"ok","program_version":env!("CARGO_PKG_VERSION"),"config_digest":crate::cli::config_digest(config.as_bytes()),
                    "report":{"counts":{"added":7,"updated":2,"removed":0,"held":1,"duplicates":0},"alerts":[]},
                    "routers":[{"name":"Core","state":"online","rows":{"ppp":63},"read_ms":22},{"name":"North","state":"online","rows":{"dhcp":41},"read_ms":18},{"name":"Backup","state":"failed","read_ms":500,"error":"timeout; cached subscribers retained"}]}),
                );
                d.service = Some(Unit {
                    load: "loaded".into(),
                    active: "active".into(),
                    main_pid: 4242,
                });
                d.proc = Some(ProcInfo {
                    pid: 4242,
                    start_unix: 1793504600,
                    elapsed_s: 1014,
                    rss_kb: 49152,
                    cpu_pct: Some(32.7),
                });
                d.config_text = Arc::new(config.into());
                d.versions.libreqos = Some("2.2".into());
                let rows: Vec<crate::model::CsvRow> = (0..40)
                    .map(|i| crate::model::CsvRow {
                        circuit_id: format!("c{i:03}"),
                        circuit_name: format!("client-{i:02}"),
                        device_id: format!("d{i:03}"),
                        device_name: format!("client-{i:02}"),
                        parent_node: ["Core-PPP", "North-DHCP", "Plan-50M"][i % 3].into(),
                        mac: format!("AA:BB:CC:00:00:{i:02X}"),
                        ipv4: format!("10.0.{}.{}", i / 20, i % 200 + 2),
                        download_min: "5".into(),
                        upload_min: "2".into(),
                        download_max: ["50", "100", "25"][i % 3].into(),
                        upload_max: ["20", "50", "10"][i % 3].into(),
                        ..Default::default()
                    })
                    .collect();
                d.row_search = Arc::new(rows.iter().map(|r| r.circuit_name.clone()).collect());
                d.flags = Arc::new((0..rows.len()).map(|i| (i % 7 == 0) as u8).collect());
                d.rows = Arc::new(rows);
                d.tree = Arc::new(json!({
                    "Core": {"downloadBandwidthMbps": 1000, "uploadBandwidthMbps": 1000, "children": {
                        "Core-PPP": {"downloadBandwidthMbps": 600, "uploadBandwidthMbps": 600},
                        "North-DHCP": {"downloadBandwidthMbps": 300, "uploadBandwidthMbps": 300}
                    }},
                    "Plan-50M": {"downloadBandwidthMbps": 200, "uploadBandwidthMbps": 200}
                }));
                app.set_data(d, 1793505614);
                app.journal.push_batch(
                    [
                        "Nov 01 10:00:00 host lqosync3[4242]: cycle 918 start trigger=timer",
                        "Nov 01 10:00:00 host lqosync3[4242]: router Core online rows=63",
                        "Nov 01 10:00:01 host lqosync3[4242]: warning: router Backup timeout; cached subscribers retained",
                        "Nov 01 10:00:01 host lqosync3[4242]: error: telegram send failed",
                        "Nov 01 10:00:01 host lqosync3[4242]: cycle 918 done 184 ms files unchanged",
                    ]
                    .map(|l| crate::tui::journal::JournalMsg::Line(l.into()))
                    .to_vec(),
                );
                app.dashboard.cpu = (0..120)
                    .map(|i| {
                        let wave = (i as f64 / 7.0).sin() * 180.0 + (i as f64 / 3.0).sin() * 70.0;
                        (320.0 + wave).max(0.0) as u64
                    })
                    .collect();
                app.dashboard.rss = (0..120)
                    .map(|i| (42000.0 + (i as f64 / 18.0).sin() * 6000.0) as u64)
                    .collect();
                *app.dashboard.cpu.back_mut().unwrap() = 327;
                *app.dashboard.rss.back_mut().unwrap() = 49152;
                for tab in Tab::ALL {
                    app.tab = tab;
                    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
                    terminal.draw(|f| app.draw(f)).unwrap();
                    let cells=terminal.backend().buffer().content.iter().map(|c|json!({"text":c.symbol(),"fg":format!("{:?}",c.fg),"bg":format!("{:?}",c.bg),"bold":c.modifier.contains(ratatui::style::Modifier::BOLD)})).collect::<Vec<_>>();
                    let path = dir.join(format!("{}-{:?}-{w}x{h}.json", tab.title(), theme));
                    std::fs::write(
                        path,
                        serde_json::to_vec(&json!({"width":w,"height":h,"cells":cells})).unwrap(),
                    )
                    .unwrap();
                }
            }
        }
    }
}
