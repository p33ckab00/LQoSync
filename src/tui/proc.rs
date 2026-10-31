//! Process data from `/proc`: CPU ticks, start time, and memory.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stat {
    pub comm: String,
    pub utime: u64,
    pub stime: u64,
    /// Clock ticks after the boot.
    pub starttime: u64,
}

/// Parse `/proc/<pid>/stat`. The name can hold spaces and `)`, so split after the last `)`.
pub fn parse_stat(text: &str) -> Option<Stat> {
    let open = text.find('(')?;
    let close = text.rfind(')')?;
    let f: Vec<&str> = text.get(close + 1..)?.split_whitespace().collect();
    // After the name: 0 state, 11 utime, 12 stime, 19 starttime.
    let n = |i: usize| f.get(i)?.parse::<u64>().ok();
    Some(Stat {
        comm: text.get(open + 1..close)?.to_string(),
        utime: n(11)?,
        stime: n(12)?,
        starttime: n(19)?,
    })
}

/// `VmRSS` of `/proc/<pid>/status`, in kB.
pub fn parse_rss_kb(status: &str) -> Option<u64> {
    status
        .lines()
        .find_map(|l| l.strip_prefix("VmRSS:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// `btime` of `/proc/stat`: the boot time in Unix seconds.
pub fn boot_time(proc_stat: &str) -> Option<u64> {
    proc_stat
        .lines()
        .find_map(|l| l.strip_prefix("btime "))?
        .trim()
        .parse()
        .ok()
}

/// The CPU percent of one core between two samples.
pub fn cpu_percent(prev_ticks: u64, ticks: u64, secs: f64, clk_tck: u64) -> f64 {
    if secs <= 0.0 || clk_tck == 0 {
        return 0.0;
    }
    ticks.saturating_sub(prev_ticks) as f64 / clk_tck as f64 / secs * 100.0
}

/// Clock ticks per second.
pub fn clk_tck() -> u64 {
    // SAFETY: sysconf has no preconditions.
    let t = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if t > 0 { t as u64 } else { 100 }
}

/// A systemd unit, from `systemctl show -p LoadState -p ActiveState -p MainPID`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Unit {
    pub load: String,
    pub active: String,
    /// 0 if the unit has no process.
    pub main_pid: u32,
}

pub fn parse_unit(show: &str) -> Unit {
    let mut u = Unit::default();
    for l in show.lines() {
        match l.split_once('=') {
            Some(("LoadState", v)) => u.load = v.to_string(),
            Some(("ActiveState", v)) => u.active = v.to_string(),
            Some(("MainPID", v)) => u.main_pid = v.parse().unwrap_or(0),
            _ => {}
        }
    }
    u
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcInfo {
    pub pid: u32,
    pub start_unix: u64,
    pub elapsed_s: u64,
    pub rss_kb: u64,
    /// `None` for the first sample of a process.
    pub cpu_pct: Option<f64>,
}

/// Samples one process. Keeps the last CPU ticks for the CPU percent.
#[derive(Debug, Default)]
pub struct Sampler {
    prev: Option<(u32, u64, Instant)>,
    btime: Option<u64>,
    clk: u64,
}

impl Sampler {
    /// `None` if the process does not exist or its name is not `comm` (a PID that has a new owner).
    /// `now` is Unix seconds; 0 means the clock of this host.
    pub fn sample(&mut self, pid: u32, comm: &str, now: u64) -> Option<ProcInfo> {
        let dir = format!("/proc/{pid}");
        if std::fs::read_to_string(format!("{dir}/comm")).ok()?.trim() != comm {
            return None;
        }
        let st = parse_stat(&std::fs::read_to_string(format!("{dir}/stat")).ok()?)?;
        let rss_kb = parse_rss_kb(&std::fs::read_to_string(format!("{dir}/status")).ok()?)?;
        if self.btime.is_none() {
            self.btime = boot_time(&std::fs::read_to_string("/proc/stat").ok()?);
            self.clk = clk_tck();
        }
        let ticks = st.utime + st.stime;
        let t = Instant::now();
        let cpu_pct = match self.prev {
            Some((p, prev, at)) if p == pid => Some(cpu_percent(
                prev,
                ticks,
                t.duration_since(at).as_secs_f64(),
                self.clk,
            )),
            _ => None,
        };
        self.prev = Some((pid, ticks, t));
        let start_unix = self.btime? + st.starttime / self.clk;
        let now = if now == 0 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs())
        } else {
            now
        };
        Some(ProcInfo {
            pid,
            start_unix,
            elapsed_s: now.saturating_sub(start_unix),
            rss_kb,
            cpu_pct,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stat_handles_spaces_and_parens_in_comm() {
        let mut f = vec!["S"; 1];
        f.extend(["1"; 10]); // ppid .. cmajflt
        let mut text = format!("123 (a) b) {}", f.join(" "));
        text.push_str(" 70 30 0 0 20 0 1 0 5000 999 5");
        let s = parse_stat(&text).unwrap();
        assert_eq!(s.comm, "a) b");
        assert_eq!((s.utime, s.stime, s.starttime), (70, 30, 5000));
        assert_eq!(parse_stat("no paren"), None);
    }

    #[test]
    fn rss_and_btime_parse() {
        assert_eq!(parse_rss_kb("Name:\tx\nVmRSS:\t   1234 kB\n"), Some(1234));
        assert_eq!(parse_rss_kb("Name:\tx\n"), None);
        assert_eq!(
            boot_time("cpu 1 2\nbtime 1700000000\n"),
            Some(1_700_000_000)
        );
    }

    #[test]
    fn cpu_percent_math() {
        assert_eq!(cpu_percent(0, 100, 1.0, 100), 100.0);
        assert_eq!(cpu_percent(100, 150, 2.0, 100), 25.0);
        assert_eq!(cpu_percent(100, 150, 0.0, 100), 0.0);
        assert!(clk_tck() > 0);
    }

    #[test]
    fn parse_unit_reads_main_pid() {
        let u = parse_unit("LoadState=loaded\nActiveState=active\nMainPID=4321\n");
        assert_eq!(
            (u.load.as_str(), u.active.as_str(), u.main_pid),
            ("loaded", "active", 4321)
        );
        assert_eq!(parse_unit("").main_pid, 0);
    }

    #[test]
    fn sample_reads_this_process() {
        let me = std::process::id();
        let comm = std::fs::read_to_string("/proc/self/comm").unwrap();
        let mut p = Sampler::default();
        let first = p.sample(me, comm.trim(), 0).unwrap();
        assert_eq!(first.pid, me);
        assert!(first.rss_kb > 0 && first.start_unix > 0);
        assert_eq!(first.cpu_pct, None);
        assert!(p.sample(me, comm.trim(), 0).unwrap().cpu_pct.is_some());
        assert!(p.sample(me, "lqosync3-other", 0).is_none());
    }
}
