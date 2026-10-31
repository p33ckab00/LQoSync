//! One full cycle: lock, live files, state, parallel router reads, the core, safe writes, reload, status.

use crate::config::{Config, Runtime, RuntimeMode, Target};
use crate::cycle::{CycleInput, RouterResult, assemble};
use crate::files;
use crate::model::{Columns, RouterMemo};
use crate::routeros::{Connector, TABLES, read_router};
use crate::routerstate::classify;
use crate::status::{ReportSummary, RouterStatus, Status};
use crate::writer::pair::{render_csv, render_network_json};
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A random ID of `len` characters from `A-Z0-9`.
pub fn random_id(len: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut bytes = vec![0u8; len];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .expect("/dev/urandom is readable");
    bytes
        .iter()
        .map(|b| CHARS[*b as usize % CHARS.len()] as char)
        .collect()
}

/// The result of one cycle.
#[derive(Debug, Clone)]
pub struct CycleSummary {
    /// A LibreQoS or candidate file changed.
    pub changed: bool,
    pub error: Option<String>,
    pub status: Status,
}

impl CycleSummary {
    /// Seconds until the next cycle.
    pub fn next_sleep(&self, rt: &Runtime) -> u64 {
        if self.error.is_some() {
            rt.error_retry_interval
        } else if self.changed {
            rt.scan_interval_active
        } else {
            rt.scan_interval_idle
        }
    }
}

/// Runs cycles. The router sessions stay open between cycles.
pub struct Runner<C: Connector + 'static> {
    pub cfg: Config,
    connector: Arc<C>,
    sessions: HashMap<String, Option<C::Session>>,
}

fn state_name(r: &RouterResult) -> &'static str {
    match r {
        RouterResult::Online(_) => "online",
        RouterResult::Failed => "failed",
        RouterResult::Unreachable => "unreachable",
        RouterResult::Offline => "offline",
    }
}

impl<C> Runner<C>
where
    C: Connector + 'static,
    C::Session: 'static,
{
    pub fn new(cfg: Config, connector: Arc<C>) -> Self {
        Runner {
            cfg,
            connector,
            sessions: HashMap::new(),
        }
    }

    /// Run one cycle. `force` releases a guard hold (a manual sync).
    pub async fn cycle(&mut self, now: u64, force: bool) -> CycleSummary {
        let start = Instant::now();
        let state_dir = Path::new(&self.cfg.runtime.state_dir).to_path_buf();
        let mut status = Status {
            version: 1,
            updated: now,
            mode: match self.cfg.runtime.mode {
                RuntimeMode::Shadow => "shadow".into(),
                RuntimeMode::Apply => "apply".into(),
            },
            target: "v2_pair".into(),
            cycle_ms: 0,
            next_scan_seconds: 0,
            routers: Vec::new(),
            report: ReportSummary::default(),
            files_changed: false,
            reload: "skipped".into(),
            error: None,
        };
        let result = self.cycle_inner(now, force, &state_dir, &mut status).await;
        let (changed, error) = match result {
            Ok(changed) => (changed, None),
            Err(e) => (false, Some(format!("{e:#}"))),
        };
        status.files_changed = changed;
        status.error = error.clone();
        status.cycle_ms = start.elapsed().as_millis() as u64;
        let mut summary = CycleSummary {
            changed,
            error,
            status,
        };
        summary.status.next_scan_seconds = summary.next_sleep(&self.cfg.runtime);
        if let Err(e) = files::write_if_changed(
            &state_dir.join("status.json"),
            summary.status.to_json().as_bytes(),
        ) {
            eprintln!("error: cannot write status.json: {e:#}");
        }
        summary
    }

    async fn cycle_inner(
        &mut self,
        now: u64,
        force: bool,
        state_dir: &Path,
        status: &mut Status,
    ) -> anyhow::Result<bool> {
        if matches!(self.cfg.libreqos.target, Target::V3Pair | Target::V3Network) {
            anyhow::bail!(
                "LibreQoS v3 targets come in Plan 3; use target = \"v2_pair\" or \"auto\""
            );
        }
        let _lock = files::lock(state_dir)?;
        let lqos_dir = Path::new(&self.cfg.libreqos.directory).to_path_buf();
        let first_run = !state_dir.join("state.json").exists();
        let live = files::read_live(
            &lqos_dir,
            &self.cfg.runtime.static_comment,
            first_run || self.cfg.network.preserve_network_config,
        )?;
        let state = files::load_state(state_dir, &live, &self.cfg.circuits, now)?;

        // Read all routers at the same time. Each task owns its session and gives it back.
        let mut tasks = tokio::task::JoinSet::new();
        for router in self.cfg.routers.clone() {
            let mut session = self.sessions.remove(&router.name).flatten();
            let (conn, access) = (self.connector.clone(), self.cfg.router_access.clone());
            tasks.spawn(async move {
                let t = Instant::now();
                let out = read_router(&*conn, &mut session, &router, &access).await;
                (router.name, out, session, t.elapsed().as_millis() as u64)
            });
        }
        let mut outcomes = HashMap::new();
        while let Some(done) = tasks.join_next().await {
            let (name, out, session, ms) = done?;
            self.sessions.insert(name.clone(), session);
            outcomes.insert(name, (out, ms));
        }
        let mut routers = Vec::new();
        let mut memos = BTreeMap::new();
        for router in &self.cfg.routers {
            let (out, ms) = outcomes
                .remove(&router.name)
                .expect("one outcome for each router");
            let memo = state
                .routers
                .get(&router.name)
                .cloned()
                .unwrap_or(RouterMemo {
                    unreachable_since: None,
                });
            let (result, memo, error) = classify(out, &memo, now, &self.cfg.router_access);
            let mut rows = BTreeMap::new();
            if let RouterResult::Online(snap) = &result {
                let tables = [
                    &snap.ppp_secret,
                    &snap.ppp_active,
                    &snap.ppp_profile,
                    &snap.hotspot_active,
                    &snap.dhcp_server,
                    &snap.dhcp_lease,
                ];
                for ((path, _), t) in TABLES.iter().zip(tables) {
                    rows.insert(path.to_string(), t.len());
                }
            }
            status.routers.push(RouterStatus {
                name: router.name.clone(),
                state: state_name(&result),
                error,
                rows,
                read_ms: ms,
            });
            memos.insert(router.name.clone(), memo);
            routers.push((router.name.clone(), result));
        }

        let id_len = self.cfg.runtime.id_length;
        let input = CycleInput {
            config: &self.cfg,
            state: &state,
            static_rows: &live.static_rows,
            routers,
            current_network: live.network.clone(),
            v3: false,
            force,
            now,
        };
        let mut out = assemble(input, &mut || random_id(id_len))?;
        status.report = ReportSummary::from_report(&out.report);
        out.state.routers = memos;

        // The state first: if the process stops here, the next cycle gives the same files.
        files::write_if_changed(
            &state_dir.join("state.json"),
            out.state.to_json().as_bytes(),
        )?;
        let csv = render_csv(&out.rows, Columns::V2);
        let net = render_network_json(&out.network);
        let target_dir = match self.cfg.runtime.mode {
            RuntimeMode::Shadow => state_dir.join("candidate"),
            RuntimeMode::Apply => lqos_dir.clone(),
        };
        let (csv_path, net_path) = (
            target_dir.join("ShapedDevices.csv"),
            target_dir.join("network.json"),
        );
        let differs = |p: &Path, b: &str| std::fs::read(p).map_or(true, |old| old != b.as_bytes());
        let changed = differs(&csv_path, &csv) || differs(&net_path, &net);
        if changed && self.cfg.runtime.mode == RuntimeMode::Apply {
            // shortcut: the backup folder name is Unix seconds (12 digits, so it sorts); a UTC date name needs a date crate.
            files::backup(
                &state_dir.join("applied"),
                &[csv_path.as_path(), net_path.as_path()],
                &format!("{now:012}"),
                self.cfg.runtime.keep_backups,
            )?;
        }
        files::write_if_changed(&csv_path, csv.as_bytes())?;
        files::write_if_changed(&net_path, net.as_bytes())?;
        if changed && self.cfg.runtime.mode == RuntimeMode::Apply {
            let l = &self.cfg.libreqos;
            status.reload = match files::run_reload(
                &l.v2_command,
                &l.v2_args,
                Duration::from_secs(l.v2_timeout),
            )
            .await
            {
                Ok(()) => "ok".into(),
                Err(e) => e,
            };
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, RouterAccess, RouterCfg};
    use crate::model::RosRow;
    use crate::routeros::{Connector, ReadError, RouterSession};
    use std::collections::{BTreeMap, HashSet};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    /// Rows for each router and path. A router in `failing` gives a trap on each read.
    #[derive(Default)]
    struct Fake {
        rows: Mutex<BTreeMap<String, BTreeMap<String, Vec<RosRow>>>>,
        failing: Mutex<HashSet<String>>,
    }
    struct FakeSession {
        router: String,
        fake: Arc<Fake>,
    }
    #[derive(Clone)]
    struct FakeConnector(Arc<Fake>);

    impl Connector for FakeConnector {
        type Session = FakeSession;
        fn connect(
            &self,
            r: &RouterCfg,
            _a: &RouterAccess,
        ) -> impl Future<Output = Result<FakeSession, ReadError>> + Send {
            let s = FakeSession {
                router: r.name.clone(),
                fake: self.0.clone(),
            };
            async move { Ok(s) }
        }
    }
    impl RouterSession for FakeSession {
        fn print(
            &mut self,
            path: &str,
            _p: &str,
        ) -> impl Future<Output = Result<Vec<RosRow>, ReadError>> + Send {
            let fail = self.fake.failing.lock().unwrap().contains(&self.router);
            let rows = self
                .fake
                .rows
                .lock()
                .unwrap()
                .get(&self.router)
                .and_then(|m| m.get(path))
                .cloned()
                .unwrap_or_default();
            async move {
                if fail {
                    Err(ReadError::Trap("down".into()))
                } else {
                    Ok(rows)
                }
            }
        }
    }

    fn row(kv: &[(&str, &str)]) -> RosRow {
        kv.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    struct Env {
        _dir: tempfile::TempDir,
        state: PathBuf,
        lqos: PathBuf,
        fake: Arc<Fake>,
    }

    fn env() -> Env {
        let dir = tempfile::tempdir().unwrap();
        let (state, lqos) = (dir.path().join("state"), dir.path().join("lqos"));
        std::fs::create_dir_all(&lqos).unwrap();
        let fake = Arc::new(Fake::default());
        Env {
            _dir: dir,
            state,
            lqos,
            fake,
        }
    }

    fn users(e: &Env, names: &[(&str, &str)]) {
        let mut m = BTreeMap::new();
        m.insert(
            "/ppp/secret".to_string(),
            names
                .iter()
                .map(|(n, _)| row(&[("name", n), ("profile", "HS")]))
                .collect(),
        );
        m.insert(
            "/ppp/active".to_string(),
            names
                .iter()
                .map(|(n, ip)| row(&[("name", n), ("address", ip)]))
                .collect(),
        );
        m.insert(
            "/ppp/profile".to_string(),
            vec![row(&[("name", "HS"), ("rate-limit", "5M/20M")])],
        );
        e.fake.rows.lock().unwrap().insert("R1".into(), m);
    }

    fn cfg(e: &Env, mode: &str, extra: &str) -> Config {
        Config::from_toml_str(&format!(
            "[runtime]\nmode=\"{mode}\"\nstate_dir=\"{}\"\n[libreqos]\ndirectory=\"{}\"\nv2_command=\"/bin/sh\"\nv2_args=[\"-c\",\"touch {}/reloaded\"]\n\
             [network]\nmode=\"root_promotion\"\n[safety]\nenabled=false\n{extra}\n\
             [[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n[routers.pppoe]\nenabled=true\nper_plan_node=true\n",
            e.state.display(),
            e.lqos.display(),
            e.lqos.display()
        ))
        .unwrap()
    }

    fn runner(e: &Env, c: Config) -> Runner<FakeConnector> {
        Runner::new(c, Arc::new(FakeConnector(e.fake.clone())))
    }

    fn read(p: &Path) -> String {
        std::fs::read_to_string(p).unwrap_or_default()
    }

    #[tokio::test]
    async fn shadow_writes_candidates_only() {
        let e = env();
        std::fs::write(
            e.lqos.join("ShapedDevices.csv"),
            "Circuit ID,Circuit Name\n",
        )
        .unwrap();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut r = runner(&e, cfg(&e, "shadow", ""));
        let s = r.cycle(1_000, false).await;
        assert_eq!(s.error, None);
        assert!(s.changed);
        assert!(read(&e.state.join("candidate/ShapedDevices.csv")).contains("alice"));
        assert_eq!(
            read(&e.lqos.join("ShapedDevices.csv")),
            "Circuit ID,Circuit Name\n"
        );
        let st: serde_json::Value =
            serde_json::from_str(&read(&e.state.join("status.json"))).unwrap();
        assert_eq!(st["routers"][0]["state"], "online");
        assert_eq!(st["reload"], "skipped");
        assert!(!e.lqos.join("reloaded").exists());
    }

    #[tokio::test]
    async fn apply_backs_up_writes_and_reloads() {
        let e = env();
        std::fs::write(
            e.lqos.join("ShapedDevices.csv"),
            "Circuit ID,Circuit Name\n",
        )
        .unwrap();
        users(&e, &[("alice", "100.64.0.2")]);
        let s = runner(&e, cfg(&e, "apply", "")).cycle(1_000, false).await;
        assert_eq!((s.error, s.status.reload.as_str()), (None, "ok"));
        assert!(read(&e.lqos.join("ShapedDevices.csv")).contains("alice"));
        assert!(read(&e.lqos.join("network.json")).contains("HS-R1"));
        assert!(e.lqos.join("reloaded").exists());
        let backups: Vec<_> = std::fs::read_dir(e.state.join("applied"))
            .unwrap()
            .collect();
        assert_eq!(backups.len(), 1);
    }

    #[tokio::test]
    async fn second_cycle_without_change_writes_nothing() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut r = runner(&e, cfg(&e, "apply", ""));
        r.cycle(1_000, false).await;
        std::fs::remove_file(e.lqos.join("reloaded")).unwrap();
        let s = r.cycle(1_025, false).await;
        assert_eq!((s.changed, s.status.reload.as_str()), (false, "skipped"));
        assert!(!e.lqos.join("reloaded").exists());
        assert_eq!(
            s.next_sleep(&r.cfg.runtime),
            r.cfg.runtime.scan_interval_idle
        );
    }

    #[tokio::test]
    async fn failed_router_keeps_rows_and_status_says_failed() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut r = runner(
            &e,
            cfg(
                &e,
                "shadow",
                "[router_access]\nread_retries=0\nretry_delay=0\n",
            ),
        );
        r.cycle(1_000, false).await;
        e.fake.failing.lock().unwrap().insert("R1".into());
        let s = r.cycle(1_025, false).await;
        assert_eq!(s.error, None);
        assert!(read(&e.state.join("candidate/ShapedDevices.csv")).contains("alice"));
        assert_eq!(s.status.routers[0].state, "failed");
        assert!(
            s.status.routers[0]
                .error
                .as_deref()
                .unwrap_or_default()
                .contains("down")
        );
    }

    #[tokio::test]
    async fn assemble_error_writes_nothing() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        e.fake.rows.lock().unwrap().get_mut("R1").unwrap().insert(
            "/ip/hotspot/active".into(),
            vec![row(&[("user", "g"), ("address", "100.65.0.2")])],
        );
        let c = cfg(&e, "shadow", "");
        let mut c2 = c.clone();
        c2.routers[0].hotspot.enabled = true;
        let s = runner(&e, c2).cycle(1_000, false).await;
        assert!(
            s.error.as_deref().unwrap_or_default().contains("HS-R1"),
            "{:?}",
            s.error
        );
        assert!(!e.state.join("candidate/ShapedDevices.csv").exists());
        let st: serde_json::Value =
            serde_json::from_str(&read(&e.state.join("status.json"))).unwrap();
        assert!(st["error"].as_str().unwrap().contains("HS-R1"));
    }

    #[tokio::test]
    async fn restart_after_state_write_gives_same_files() {
        let a = env();
        users(&a, &[("alice", "100.64.0.2"), ("bob", "100.64.0.3")]);
        runner(&a, cfg(&a, "shadow", "")).cycle(1_000, false).await;
        // A crash after the state write: only state.json is there, not the LibreQoS files.
        let b = env();
        users(&b, &[("alice", "100.64.0.2"), ("bob", "100.64.0.3")]);
        std::fs::create_dir_all(&b.state).unwrap();
        std::fs::copy(a.state.join("state.json"), b.state.join("state.json")).unwrap();
        runner(&b, cfg(&b, "shadow", "")).cycle(1_025, false).await;
        assert_eq!(
            read(&a.state.join("candidate/ShapedDevices.csv")),
            read(&b.state.join("candidate/ShapedDevices.csv"))
        );
        assert_eq!(
            read(&a.state.join("candidate/network.json")),
            read(&b.state.join("candidate/network.json"))
        );
    }

    #[test]
    fn random_ids_have_the_length_and_charset() {
        let id = random_id(8);
        assert!(
            id.len() == 8
                && id
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()),
            "{id}"
        );
        assert_ne!(random_id(8), random_id(8));
    }
}
