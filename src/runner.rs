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
    /// The instance lock. Taken at the first cycle and held while the runner lives.
    lock: Option<std::fs::File>,
    /// The error of the last config reload, until a good reload.
    config_error: Option<String>,
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
            lock: None,
            config_error: None,
        }
    }

    /// Take the instance lock on `state_dir` and hold it while the runner lives.
    pub fn lock(&mut self) -> anyhow::Result<()> {
        if self.lock.is_none() {
            self.lock = Some(files::lock(Path::new(&self.cfg.runtime.state_dir))?);
        }
        Ok(())
    }

    /// Use a new config. The sessions close, because an address or a password can change.
    pub fn set_config(&mut self, cfg: Config) {
        self.cfg = cfg;
        self.sessions.clear();
        self.config_error = None;
    }

    /// A config reload failed. The old config stays, and `status.json` shows the error.
    pub fn config_failed(&mut self, error: String) {
        self.config_error = Some(error);
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
            config_error: self.config_error.clone(),
        };
        // Without the lock, another process owns the files: write nothing, not even the status.
        if let Err(e) = self.lock() {
            let error = Some(format!("{e:#}"));
            status.error = error.clone();
            return CycleSummary {
                changed: false,
                error,
                status,
            };
        }
        let result = self.cycle_inner(now, force, &state_dir, &mut status).await;
        let (changed, error) = match result {
            Ok((changed, None)) => (changed, None),
            Ok((changed, Some(reload))) => {
                (changed, Some(format!("LibreQoS reload failed: {reload}")))
            }
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
    ) -> anyhow::Result<(bool, Option<String>)> {
        if matches!(self.cfg.libreqos.target, Target::V3Pair | Target::V3Network) {
            anyhow::bail!(
                "LibreQoS v3 targets come in Plan 3; use target = \"v2_pair\" or \"auto\""
            );
        }
        let lqos_dir = Path::new(&self.cfg.libreqos.directory).to_path_buf();
        let first_run = !state_dir.join("state.json").exists();
        let live = files::read_live(
            &lqos_dir,
            &self.cfg.runtime.static_comment,
            first_run || self.cfg.network.preserve_network_config,
        )?;
        let state = files::load_state(state_dir, &live, &self.cfg.circuits, now)?;

        // Read all routers at the same time. Each task owns its session and gives it back.
        let mut tasks = Vec::new();
        for router in self.cfg.routers.clone() {
            let mut session = self.sessions.remove(&router.name).flatten();
            let (conn, access) = (self.connector.clone(), self.cfg.router_access.clone());
            let name = router.name.clone();
            tasks.push((
                name,
                tokio::spawn(async move {
                    let t = Instant::now();
                    let out = read_router(&*conn, &mut session, &router, &access).await;
                    (out, session, t.elapsed().as_millis() as u64)
                }),
            ));
        }
        let mut outcomes = HashMap::new();
        for (name, task) in tasks {
            // A task that panics fails only its own router.
            let (out, session, ms) = task.await.unwrap_or_else(|e| {
                (
                    Err(crate::routeros::ReadError::Protocol(format!(
                        "the read task stopped: {e}"
                    ))),
                    None,
                    0,
                )
            });
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
        // The nodes first: each Parent Node of the new CSV is then already in network.json.
        files::write_if_changed(&net_path, net.as_bytes())?;
        files::write_if_changed(&csv_path, csv.as_bytes())?;
        // A failed reload stays pending until it works, also when the files do not change again.
        let pending = state_dir.join("reload-pending");
        let mut reload_error = None;
        if self.cfg.runtime.mode == RuntimeMode::Apply && (changed || pending.exists()) {
            let l = &self.cfg.libreqos;
            match files::run_reload(&l.v2_command, &l.v2_args, Duration::from_secs(l.v2_timeout))
                .await
            {
                Ok(()) => {
                    status.reload = "ok".into();
                    if pending.exists() {
                        std::fs::remove_file(&pending)?;
                    }
                }
                Err(e) => {
                    status.reload = e.clone();
                    files::write_if_changed(&pending, e.as_bytes())?;
                    reload_error = Some(e);
                }
            }
        }
        Ok((changed, reload_error))
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
        panicking: Mutex<HashSet<String>>,
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
            let boom = self.0.panicking.lock().unwrap().contains(&r.name);
            async move {
                assert!(!boom, "test panic in a router task");
                Ok(s)
            }
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

    #[tokio::test]
    async fn a_panicking_router_task_fails_only_that_router() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut c = cfg(&e, "shadow", "");
        let mut r2 = c.routers[0].clone();
        r2.name = "R2".into();
        c.routers.push(r2);
        e.fake.panicking.lock().unwrap().insert("R2".into());
        let s = runner(&e, c).cycle(1_000, false).await;
        assert_eq!(s.error, None);
        let states: Vec<_> = s
            .status
            .routers
            .iter()
            .map(|r| (r.name.as_str(), r.state))
            .collect();
        assert_eq!(states, [("R1", "online"), ("R2", "failed")]);
    }

    #[tokio::test]
    async fn failed_reload_is_retried_until_it_works() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut c = cfg(&e, "apply", "");
        c.libreqos.v2_command = "/bin/false".into();
        c.libreqos.v2_args = vec![];
        let mut r = runner(&e, c.clone());
        let s1 = r.cycle(1_000, false).await;
        assert!(
            s1.error.as_deref().unwrap_or_default().contains("reload"),
            "{:?}",
            s1.error
        );
        let s2 = r.cycle(1_025, false).await;
        assert!(!s2.changed);
        assert!(
            s2.error.as_deref().unwrap_or_default().contains("reload"),
            "the reload must run again: {:?}",
            s2.error
        );
        c.libreqos.v2_command = "/bin/true".into();
        r.set_config(c);
        let s3 = r.cycle(1_050, false).await;
        assert_eq!((s3.error, s3.status.reload.as_str()), (None, "ok"));
        let s4 = r.cycle(1_075, false).await;
        assert_eq!(s4.status.reload, "skipped");
    }

    #[tokio::test]
    async fn a_held_lock_stops_the_cycle_without_touching_status() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let _other = crate::files::lock(&e.state).unwrap();
        let s = runner(&e, cfg(&e, "shadow", "")).cycle(1_000, false).await;
        assert!(
            s.error
                .as_deref()
                .unwrap_or_default()
                .contains("another lqosync3")
        );
        assert!(!e.state.join("status.json").exists());
    }

    #[tokio::test]
    async fn a_config_reload_error_shows_in_status_until_a_good_reload() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let c = cfg(&e, "shadow", "");
        let mut r = runner(&e, c.clone());
        r.config_failed("bad key".into());
        let s = r.cycle(1_000, false).await;
        assert_eq!(s.error, None);
        let st: serde_json::Value =
            serde_json::from_str(&read(&e.state.join("status.json"))).unwrap();
        assert_eq!(st["config_error"], "bad key");
        r.set_config(c);
        let s = r.cycle(1_025, false).await;
        assert_eq!(s.status.config_error, None);
    }

    #[tokio::test]
    async fn network_json_is_written_before_the_csv() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        // A write of network.json that fails must leave the CSV as it was.
        std::fs::create_dir_all(e.state.join("candidate/network.json")).unwrap();
        let s = runner(&e, cfg(&e, "shadow", "")).cycle(1_000, false).await;
        assert!(s.error.is_some());
        assert!(!e.state.join("candidate/ShapedDevices.csv").exists());
    }
}
