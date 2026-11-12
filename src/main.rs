//! The lqosync3 program.

use lqosync3::cli::{self, Command, Level};
use lqosync3::config::Config;
use lqosync3::diff::diff_config;
use lqosync3::mikrotik::MikrotikConnector;
use lqosync3::notify::{Notifier, send_all};
use lqosync3::runner::{CycleSummary, Runner};
use lqosync3::status::Trigger;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::signal::unix::{SignalKind, signal};

/// Secret files must belong to root.
const SECRET_OWNER_UID: u32 = 0;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn exit_with(e: anyhow::Error) -> ! {
    eprintln!("error: {e:#}");
    std::process::exit(1);
}

fn print_summary(s: &CycleSummary) {
    for r in &s.status.routers {
        let rows: usize = r.rows.values().sum();
        match &r.error {
            Some(e) => println!("router {}: {} ({e})", r.name, r.state),
            None => println!(
                "router {}: {} ({rows} rows, {} ms)",
                r.name, r.state, r.read_ms
            ),
        }
    }
    let c = &s.status.report.counts;
    let n = |k: &str| c.get(k).copied().unwrap_or(0);
    println!(
        "added {} updated {} removed {} held {} duplicates {} | files changed: {} | reload: {}",
        n("added"),
        n("updated"),
        n("removed"),
        n("held"),
        n("duplicates"),
        s.changed,
        s.status.reload
    );
    for a in &s.status.report.alerts {
        println!("ALERT: {a}");
    }
    for w in &s.status.report.warnings {
        println!("warning: {w}");
    }
    if let Some(e) = &s.error {
        println!("ERROR: {e}");
    }
}

/// Send the Telegram message of the cycle, if it has an event. A failure is only a log line.
/// Returns the send result for `status.json`, or `None` if nothing was sent.
async fn notify(n: &mut Notifier, cfg: &Config, s: &CycleSummary) -> Option<String> {
    if s.status.target == "v3_pair" && s.status.mode == "apply" {
        let hub = lqosync3::system_health::Hub::default();
        let queued = queue_notify(n, cfg, s, &hub);
        let work = async {
            let mut outbox =
                lqosync3::notify::outbox::Outbox::new(Path::new(&cfg.runtime.state_dir), hub)?;
            loop {
                let did =
                    outbox
                        .step(&cfg.telegram, now(), |text| {
                            let tg = cfg.telegram.clone();
                            async move {
                                lqosync3::notify::send_receipt(&tg, &text, SECRET_OWNER_UID).await
                            }
                        })
                        .await?;
                if !did {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
            Ok::<_, anyhow::Error>(())
        };
        if let Ok(Err(error)) =
            tokio::time::timeout(Duration::from_secs(cfg.telegram.timeout.max(1)), work).await
        {
            eprintln!("telegram outbox: {error}");
        }
        return queued.or_else(|| {
            Some(
                if cfg.telegram.enabled {
                    "queued; run service resumes pending work"
                } else {
                    "disabled"
                }
                .into(),
            )
        });
    }
    // The notifier keeps its event memory also when Telegram is off.
    let text = n.message(&cfg.telegram, &s.status);
    if !cfg.telegram.enabled {
        return Some("disabled".into());
    }
    let text = text?;
    Some(
        match send_all(&cfg.telegram, &text, SECRET_OWNER_UID).await {
            Ok(()) => "ok".into(),
            Err(e) => {
                eprintln!("warning: telegram: {e}");
                e.to_string()
            }
        },
    )
}

/// Save notification work before changing deduplication memory. The service never awaits HTTP.
fn queue_notify(
    n: &mut Notifier,
    cfg: &Config,
    s: &CycleSummary,
    hub: &lqosync3::system_health::Hub,
) -> Option<String> {
    use lqosync3::system_health::{Domain, Observation, State};
    let old = n.clone();
    let mut status = s.status.clone();
    if status.target == "v3_pair" && status.mode == "apply" {
        status.files_changed = false;
    }
    let text = n.message(&cfg.telegram, &status);
    if !cfg.telegram.enabled {
        return Some("disabled".into());
    }
    let text = text?;
    match lqosync3::notify::outbox::enqueue(
        Path::new(&cfg.runtime.state_dir),
        &cfg.telegram,
        &text,
        now(),
    ) {
        Ok(()) => Some("queued".into()),
        Err(error) => {
            *n = old;
            let _ = hub.observe(Observation {
                domain: Domain::Notification,
                component: "telegram_event_store".into(),
                state: State::Degraded,
                reason: "notification_event_not_saved".into(),
                observed_at: now(),
                max_age: u64::MAX,
                source_epoch: "service".into(),
                last_success: None,
            });
            eprintln!("telegram queue: {error}");
            Some("queue storage failed".into())
        }
    }
}

async fn run(path: &Path, mut cfg: Config, mut digest: String) -> anyhow::Result<()> {
    let mut runner = Runner::new(
        cfg.clone(),
        Arc::new(MikrotikConnector {
            owner_uid: SECRET_OWNER_UID,
        }),
    );
    runner.lock()?;
    let worker_monitor =
        lqosync3::worker_util::Monitor::start_with_health(&cfg, runner.health.clone());
    runner.set_source(path, digest.clone());
    let apply_monitor = runner.pending_tracker().map(|tracker| {
        lqosync3::apply_evidence::Monitor::start(&cfg, &digest, tracker, runner.health.clone())
    });
    let telegram_worker = lqosync3::notify::outbox::Worker::start(
        Path::new(&cfg.runtime.state_dir),
        &cfg.telegram,
        runner.health.clone(),
        SECRET_OWNER_UID,
    );
    let (mut hup, mut usr1) = (
        signal(SignalKind::hangup())?,
        signal(SignalKind::user_defined1())?,
    );
    let (mut term, mut int) = (
        signal(SignalKind::terminate())?,
        signal(SignalKind::interrupt())?,
    );
    let mut force = false;
    let mut notifier = Notifier::default();
    loop {
        let summary = runner.cycle(now(), force).await;
        print_summary(&summary);
        if let Some(t) = queue_notify(&mut notifier, &runner.cfg, &summary, &runner.health) {
            runner.set_telegram(t);
        }
        force = false;
        runner.trigger = Trigger::Timer;
        let sleep =
            tokio::time::sleep(Duration::from_secs(summary.next_sleep(&runner.cfg.runtime)));
        tokio::select! {
            _ = sleep => {}
            _ = usr1.recv() => {
                force = true;
                runner.trigger = Trigger::SyncNow;
            }
            _ = hup.recv() => {
                match cli::reload(&mut cfg, &mut digest, path) {
                    Ok(()) => {
                        eprintln!("config reloaded from {}", path.display());
                        worker_monitor.reconfigure(&cfg);
                        telegram_worker.reconfigure(&cfg.telegram);
                        if let Some(monitor)=&apply_monitor {monitor.reconfigure(&cfg,&digest);}
                        runner.set_config(cfg.clone());
                        runner.set_source(path, digest.clone());
                    }
                    Err(e) => {
                        eprintln!("error: config reload failed, the old config stays: {e:#}");
                        runner.config_failed(format!("{e:#}"));
                    }
                }
                // The TUI waits for this status: write it now, not after the next cycle.
                runner.write_reload_status(now());
                runner.trigger = Trigger::Reload;
            }
            _ = term.recv() => break,
            _ = int.recv() => break,
        }
    }
    eprintln!("lqosync3 stopped");
    Ok(())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let parsed = cli::parse(&args).unwrap_or_else(|e| {
        eprintln!("{e:#}");
        std::process::exit(2)
    });
    if parsed.command == Command::Defaults {
        print!("{}", lqosync3::config::DEFAULTS_TOML);
        return;
    }
    if parsed.command == Command::Version {
        println!("lqosync3 {}", lqosync3::VERSION);
        return;
    }
    // The TUI starts also with a bad config file: it shows the error.
    if parsed.command == Command::Tui {
        lqosync3::tui::run(&parsed.config).unwrap_or_else(|e| exit_with(e));
        return;
    }
    let (cfg, digest) = cli::read_config(&parsed.config).unwrap_or_else(|e| exit_with(e));
    match parsed.command {
        Command::Check => {
            let mut lines = cli::check(&cfg, SECRET_OWNER_UID);
            lines.extend(cli::digest_line(Path::new(&cfg.runtime.state_dir), &digest));
            for l in &lines {
                let tag = match l.level {
                    Level::Ok => "OK   ",
                    Level::Warn => "WARN ",
                    Level::Error => "ERROR",
                };
                println!("{tag} {}", l.text);
            }
            if lines.iter().any(|l| l.level == Level::Error) {
                std::process::exit(1);
            }
        }
        Command::Once { force } => {
            let mut runner = Runner::new(
                cfg,
                Arc::new(MikrotikConnector {
                    owner_uid: SECRET_OWNER_UID,
                }),
            );
            runner.lock().unwrap_or_else(|e| exit_with(e));
            runner.set_source(&parsed.config, digest);
            runner.trigger = Trigger::Once;
            let s = runner.cycle(now(), force).await;
            print_summary(&s);
            if let Some(t) = notify(&mut Notifier::default(), &runner.cfg, &s).await {
                runner.set_telegram(t);
            }
            if s.error.is_some() {
                std::process::exit(1);
            }
        }
        Command::Detect => match lqosync3::detect::resolve(&cfg.libreqos) {
            Ok(d) => println!("target={} ({})", cli::target_name(d.target), d.reason),
            Err(e) => exit_with(e),
        },
        Command::Diff => print!("{}", diff_config(&cfg).unwrap_or_else(|e| exit_with(e))),
        Command::Run => run(&parsed.config, cfg, digest)
            .await
            .unwrap_or_else(|e| exit_with(e)),
        Command::Defaults | Command::Tui | Command::Version => unreachable!("handled above"),
    }
}
