//! The Dashboard tab: the service, the last cycle, versions, the process, and the actions.

use super::app::split_main;
use super::data::Data;
use crate::tui::theme::Palette;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Sparkline};
use serde_json::Value;
use std::collections::VecDeque;

/// Graph samples kept for this TUI session.
const SAMPLES: usize = 120;

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

#[derive(Debug, Default)]
pub struct DashboardView {
    /// CPU percent in tenths, and RSS in kB.
    pub cpu: VecDeque<u64>,
    pub rss: VecDeque<u64>,
}

impl DashboardView {
    /// Add one graph sample for each new data.
    pub fn on_data(&mut self, d: &Data) {
        if let Some(p) = &d.proc {
            // An unknown CPU value (the first sample) is not drawn as 0.
            let cpu = p.cpu_pct.map(|c| (c * 10.0) as u64);
            for (q, v) in [(&mut self.cpu, cpu), (&mut self.rss, Some(p.rss_kb))] {
                let Some(v) = v else { continue };
                if q.len() == SAMPLES {
                    q.pop_front();
                }
                q.push_back(v);
            }
        }
    }

    pub fn draw(&self, f: &mut Frame, area: Rect, d: &Data, now: u64, _palette: &Palette) {
        let (main, side) = split_main(area);
        let text = lines(d, now);
        // Two columns of cards when the panel is wide.
        if main.width >= 80 {
            let [a, b] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(main);
            let cut = text
                .iter()
                .position(|l| l.to_string() == "[counts]")
                .unwrap_or(text.len());
            let (l, r) = text.split_at(cut);
            f.render_widget(
                Paragraph::new(l.to_vec())
                    .block(Block::default().borders(Borders::ALL).title(" dashboard ")),
                a,
            );
            f.render_widget(
                Paragraph::new(r.to_vec()).block(Block::default().borders(Borders::ALL)),
                b,
            );
        } else {
            f.render_widget(
                Paragraph::new(text)
                    .block(Block::default().borders(Borders::ALL).title(" dashboard ")),
                main,
            );
        }
        let [c, m] =
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(side);
        let cpu: Vec<u64> = self.cpu.iter().copied().collect();
        let rss: Vec<u64> = self.rss.iter().copied().collect();
        f.render_widget(
            Sparkline::default().data(&cpu).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" CPU % (this session) "),
            ),
            c,
        );
        f.render_widget(
            Sparkline::default().data(&rss).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" RSS (this session) "),
            ),
            m,
        );
    }
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
        for want in ["R1: online, 3 rows, 12 ms", "CPU %", "RSS"] {
            assert!(s.contains(want), "{want}\n{s}");
        }
    }
}
