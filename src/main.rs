//! The lqosync3 program.

use lqosync3::cli::{self, Command, Level};
use lqosync3::config::Config;
use lqosync3::diff::diff_pair;
use lqosync3::mikrotik::MikrotikConnector;
use lqosync3::runner::{CycleSummary, Runner};
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

fn read_or_empty(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

fn diff(cfg: &Config) -> anyhow::Result<String> {
    let cand = Path::new(&cfg.runtime.state_dir).join("candidate");
    let live = Path::new(&cfg.libreqos.directory);
    let json = |p: &Path| -> anyhow::Result<serde_json::Value> {
        let t = read_or_empty(p);
        Ok(if t.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(&t)?
        })
    };
    diff_pair(
        &read_or_empty(&live.join("ShapedDevices.csv")),
        &read_or_empty(&cand.join("ShapedDevices.csv")),
        &json(&live.join("network.json"))?,
        &json(&cand.join("network.json"))?,
    )
}

async fn run(path: &Path, mut cfg: Config) -> anyhow::Result<()> {
    let mut runner = Runner::new(
        cfg.clone(),
        Arc::new(MikrotikConnector {
            owner_uid: SECRET_OWNER_UID,
        }),
    );
    runner.lock()?;
    let (mut hup, mut usr1) = (
        signal(SignalKind::hangup())?,
        signal(SignalKind::user_defined1())?,
    );
    let (mut term, mut int) = (
        signal(SignalKind::terminate())?,
        signal(SignalKind::interrupt())?,
    );
    let mut force = false;
    loop {
        let summary = runner.cycle(now(), force).await;
        print_summary(&summary);
        force = false;
        let sleep =
            tokio::time::sleep(Duration::from_secs(summary.next_sleep(&runner.cfg.runtime)));
        tokio::select! {
            _ = sleep => {}
            _ = usr1.recv() => force = true,
            _ = hup.recv() => match cli::reload(&mut cfg, path) {
                Ok(()) => {
                    eprintln!("config reloaded from {}", path.display());
                    runner.set_config(cfg.clone());
                }
                Err(e) => eprintln!("error: config reload failed, the old config stays: {e:#}"),
            },
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
    let cfg = cli::load_config(&parsed.config).unwrap_or_else(|e| exit_with(e));
    match parsed.command {
        Command::Check => {
            let lines = cli::check(&cfg, SECRET_OWNER_UID);
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
            let s = runner.cycle(now(), force).await;
            print_summary(&s);
            if s.error.is_some() {
                std::process::exit(1);
            }
        }
        Command::Diff => print!("{}", diff(&cfg).unwrap_or_else(|e| exit_with(e))),
        Command::Run => run(&parsed.config, cfg)
            .await
            .unwrap_or_else(|e| exit_with(e)),
        Command::Defaults => unreachable!("handled above"),
    }
}
