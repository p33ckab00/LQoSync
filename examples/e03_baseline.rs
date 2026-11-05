//! Full E03 cycle against synthetic RouterOS replies and temporary LibreQoS paths.
//! No real router, production file, or LibreQoS daemon is contacted.

use lqosync3::config::{Config, RouterAccess, RouterCfg, RuntimeMode, Target};
use lqosync3::model::{RosRow, RouterSnapshot};
use lqosync3::routeros::{Connector, ReadError, RouterSession};
use lqosync3::runner::Runner;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

struct FakeConnector {
    snapshot: Arc<RouterSnapshot>,
    calls: Arc<AtomicUsize>,
}
struct FakeSession {
    snapshot: Arc<RouterSnapshot>,
    calls: Arc<AtomicUsize>,
}

impl Connector for FakeConnector {
    type Session = FakeSession;
    fn connect(
        &self,
        _: &RouterCfg,
        _: &RouterAccess,
    ) -> impl Future<Output = Result<FakeSession, ReadError>> + Send {
        std::future::ready(Ok(FakeSession {
            snapshot: self.snapshot.clone(),
            calls: self.calls.clone(),
        }))
    }
}

impl RouterSession for FakeSession {
    fn print(
        &mut self,
        path: &str,
        _: &str,
    ) -> impl Future<Output = Result<Vec<RosRow>, ReadError>> + Send {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let rows = match path {
            "/ppp/secret" => &self.snapshot.ppp_secret,
            "/ppp/active" => &self.snapshot.ppp_active,
            "/ppp/profile" => &self.snapshot.ppp_profile,
            "/ip/dhcp-server" => &self.snapshot.dhcp_server,
            "/ip/dhcp-server/lease" => &self.snapshot.dhcp_lease,
            _ => {
                return std::future::ready(Err(ReadError::Protocol(format!(
                    "unexpected table: {path}"
                ))));
            }
        };
        std::future::ready(Ok(rows.clone()))
    }
}

fn row(fields: &[(&str, String)]) -> RosRow {
    fields
        .iter()
        .map(|(name, value)| ((*name).to_string(), value.clone()))
        .collect()
}

fn snapshot(count: usize) -> RouterSnapshot {
    let mut snap = RouterSnapshot::default();
    snap.ppp_profile
        .push(row(&[("name", "Gold".into()), ("comment", "50M".into())]));
    snap.dhcp_server.push(row(&[("name", "LAN".into())]));
    let ppp = count * 7 / 10;
    for i in 0..count {
        let name = format!("user-{i:05}");
        let address = format!(
            "100.{}.{}.{}",
            if i < ppp { 64 } else { 66 },
            i / 254,
            i % 254 + 1
        );
        let mac = format!(
            "02:00:00:{:02X}:{:02X}:{:02X}",
            i >> 16,
            (i >> 8) & 255,
            i & 255
        );
        if i < ppp {
            snap.ppp_secret.push(row(&[
                ("name", name.clone()),
                ("profile", "Gold".into()),
                ("disabled", "false".into()),
            ]));
            snap.ppp_active.push(row(&[
                ("name", name),
                ("address", address),
                ("caller-id", mac),
            ]));
        } else {
            snap.dhcp_lease.push(row(&[
                ("server", "LAN".into()),
                ("host-name", name),
                ("address", address),
                ("mac-address", mac),
                ("status", "bound".into()),
            ]));
        }
    }
    snap
}

fn inode(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.ino())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    anyhow::ensure!(
        args.len() == 4,
        "usage: e03_baseline <1000|10000> <runs> <v2_pair|v3_pair|v3_network>"
    );
    let count: usize = args[1].parse()?;
    let runs: usize = args[2].parse()?;
    anyhow::ensure!(
        matches!(count, 1000 | 10000) && runs >= 2,
        "use 1000 or 10000 subscribers and at least two runs"
    );
    let target = match args[3].as_str() {
        "v2_pair" => Target::V2Pair,
        "v3_pair" => Target::V3Pair,
        "v3_network" => Target::V3Network,
        _ => anyhow::bail!("unknown target"),
    };
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let temp = std::env::temp_dir().join(format!("lqosync4-e03-{}-{stamp}", std::process::id()));
    let lqos = temp.join("lqos");
    let state = temp.join("state");
    std::fs::create_dir(&temp)?;
    std::fs::create_dir(&lqos)?;
    let mut cfg = Config::from_toml_str(include_str!(
        "../tests/scenarios/v3_root_promotion/config.toml"
    ))?;
    cfg.runtime.mode = RuntimeMode::Shadow;
    cfg.runtime.state_dir = state.display().to_string();
    cfg.libreqos.target = target;
    cfg.libreqos.directory = lqos.display().to_string();
    cfg.libreqos.state_directory = temp.join("lqstate").display().to_string();
    cfg.libreqos.dpkg_status = temp.join("no-dpkg-status").display().to_string();
    let calls = Arc::new(AtomicUsize::new(0));
    let connector = Arc::new(FakeConnector {
        snapshot: Arc::new(snapshot(count)),
        calls: calls.clone(),
    });
    let mut runner = Runner::new(cfg, connector);
    let names: &[&str] = if target == Target::V3Network {
        &["libreqos_network.json"]
    } else {
        &["network.json", "ShapedDevices.csv"]
    };
    let candidate = state.join("candidate");

    println!(
        "target,subscribers,run,elapsed_us,cycle_ms,api_print_calls,output_bytes,changed_files"
    );
    let result = async {
        for run in 0..runs {
            let before: Vec<_> = names
                .iter()
                .map(|name| inode(&candidate.join(name)))
                .collect();
            let calls_before = calls.load(Ordering::Relaxed);
            let started = Instant::now();
            let summary = runner.cycle(1_000 + run as u64, false).await;
            let elapsed = started.elapsed().as_micros();
            anyhow::ensure!(summary.error.is_none(), "cycle failed: {:?}", summary.error);
            if run == 0 {
                anyhow::ensure!(
                    summary.status.report.added.len() == count,
                    "first cycle did not include all {count} subscribers"
                );
            }
            if target == Target::V3Network {
                anyhow::ensure!(
                    summary.status.report.alerts.is_empty(),
                    "native render omitted a circuit"
                );
            }
            let after: Vec<_> = names
                .iter()
                .map(|name| inode(&candidate.join(name)))
                .collect();
            anyhow::ensure!(
                after.iter().all(Option::is_some),
                "candidate output missing"
            );
            let changed = before.iter().zip(&after).filter(|(a, b)| a != b).count();
            anyhow::ensure!(
                summary.changed == (changed > 0),
                "changed flag disagrees with file inodes"
            );
            if run > 0 {
                anyhow::ensure!(changed == 0, "unchanged cycle rewrote a shaping file");
            }
            let bytes: u64 = names
                .iter()
                .map(|name| std::fs::metadata(candidate.join(name)).map(|m| m.len()))
                .collect::<std::io::Result<Vec<_>>>()?
                .into_iter()
                .sum();
            let print_calls = calls.load(Ordering::Relaxed) - calls_before;
            anyhow::ensure!(print_calls == 5, "expected five fake RouterOS table reads");
            println!(
                "{},{count},{run},{elapsed},{},{print_calls},{bytes},{changed}",
                args[3], summary.status.cycle_ms
            );
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;
    drop(runner);
    std::fs::remove_dir_all(temp)?;
    result
}
