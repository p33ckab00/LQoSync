//! The loader worker: it reads the files, the service state, and the process, outside the draw loop.

use super::proc::{ProcInfo, Sampler, Unit, parse_unit};
use crate::config::{Config, RuntimeMode};
use crate::model::{CsvRow, State};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant, SystemTime};

/// The row has an IPv4 address that another row also has.
pub const FLAG_DUP_IP: u8 = 1;
/// The Parent Node of the row is not in the node tree.
pub const FLAG_NO_PARENT: u8 = 2;
/// LibreQoS rejects a rate of the row (`validate::rate_problem`).
pub const FLAG_RATE: u8 = 4;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Versions {
    pub libreqos: Option<String>,
    /// The lqosd unit and its ActiveState.
    pub lqosd: Option<(String, String)>,
}

/// One load of all TUI inputs. The large parts are shared: a clone is cheap.
#[derive(Debug, Clone, Default)]
pub struct Data {
    pub status: Option<Arc<Value>>,
    pub service: Option<Unit>,
    pub target: String,
    pub rows: Arc<Vec<CsvRow>>,
    /// Lowercase search text of each row.
    pub row_search: Arc<Vec<String>>,
    pub flags: Arc<Vec<u8>>,
    /// Circuit name to (source, router), from `state.json`.
    pub sources: Arc<HashMap<String, (String, String)>>,
    pub tree: Arc<Value>,
    /// Parent node name to the row indexes.
    pub node_circuits: Arc<HashMap<String, Vec<u32>>>,
    pub config_text: Arc<String>,
    pub versions: Versions,
    pub proc: Option<ProcInfo>,
    pub errors: Vec<String>,
    /// The text of `snapshot.json` (mode 0600: `None` if this process cannot read it).
    pub snapshot: Option<Arc<String>>,
}

type Stamp = Option<(SystemTime, u64)>;

fn stamp(p: &Path) -> Stamp {
    let m = std::fs::metadata(p).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// `<systemctl> show -p <prop>... <unit>`. Stderr is dropped, so it does not draw over the screen.
pub fn systemctl_show(systemctl: &str, unit: &str, props: &[&str]) -> Option<Unit> {
    let mut cmd = std::process::Command::new(systemctl);
    cmd.arg("show");
    for p in props {
        cmd.args(["-p", p]);
    }
    let out = cmd
        .arg(unit)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| parse_unit(&String::from_utf8_lossy(&out.stdout)))
}

/// Reads the inputs. Parses a file only when its modification time or size changed.
pub struct Loader {
    cfg: Config,
    config_path: PathBuf,
    pub(crate) systemctl: String,
    pub(crate) max_bytes: u64,
    data: Data,
    status_text: String,
    input_stamps: Vec<Stamp>,
    config_stamp: Stamp,
    snapshot_stamp: Stamp,
    versions_at: Option<Instant>,
    sampler: Sampler,
}

impl Loader {
    pub fn new(cfg: Config, config_path: PathBuf) -> Self {
        let max_bytes = cfg.tui.max_file_mb * 1024 * 1024;
        Loader {
            cfg,
            config_path,
            systemctl: "systemctl".into(),
            max_bytes,
            data: Data::default(),
            status_text: String::new(),
            input_stamps: Vec::new(),
            config_stamp: None,
            snapshot_stamp: None,
            versions_at: None,
            sampler: Sampler::default(),
        }
    }

    fn show(&self, unit: &str, props: &[&str]) -> Option<Unit> {
        systemctl_show(&self.systemctl, unit, props)
    }

    /// Read a file that is not larger than `max_file_mb`.
    fn read_limited(&self, p: &Path, errors: &mut Vec<String>) -> Option<String> {
        let len = match std::fs::metadata(p) {
            Ok(m) => m.len(),
            Err(e) => {
                errors.push(format!("cannot read {}: {e}", p.display()));
                return None;
            }
        };
        if len > self.max_bytes {
            errors.push(format!(
                "{} is larger than tui.max_file_mb ({} MB); the screen shows the last data",
                p.display(),
                self.cfg.tui.max_file_mb
            ));
            return None;
        }
        match std::fs::read_to_string(p) {
            Ok(t) => Some(t),
            Err(e) => {
                errors.push(format!("cannot read {}: {e}", p.display()));
                None
            }
        }
    }

    /// One refresh.
    pub fn load(&mut self) -> Data {
        let mut errors = Vec::new();
        let state_dir = PathBuf::from(&self.cfg.runtime.state_dir);

        let text = std::fs::read_to_string(state_dir.join("status.json")).unwrap_or_default();
        if text != self.status_text {
            self.data.status = serde_json::from_str(&text).ok().map(Arc::new);
            self.status_text = text;
        }
        let tui = self.cfg.tui.clone();
        self.data.service = self.show(
            &tui.service_unit,
            &[
                "LoadState",
                "ActiveState",
                "MainPID",
                "IOReadBytes",
                "IOWriteBytes",
                "IPIngressBytes",
                "IPEgressBytes",
            ],
        );
        self.data.proc = match &self.data.service {
            Some(u) if u.main_pid != 0 => self.sampler.sample(u.main_pid, "lqosync3", 0),
            _ => None,
        };
        if self
            .versions_at
            .is_none_or(|t| t.elapsed() >= Duration::from_secs(60))
        {
            self.versions_at = Some(Instant::now());
            let libreqos = std::fs::read_to_string(&self.cfg.libreqos.dpkg_status)
                .ok()
                .and_then(|t| crate::detect::package_version(&t, "libreqos"));
            let lqosd = tui.lqosd_units.iter().find_map(|u| {
                let s = self.show(u, &["LoadState", "ActiveState"])?;
                (s.load == "loaded").then(|| (u.clone(), s.active))
            });
            self.data.versions = Versions { libreqos, lqosd };
        }

        let snap_path = state_dir.join("snapshot.json");
        let ss = stamp(&snap_path);
        if ss != self.snapshot_stamp {
            self.snapshot_stamp = ss;
            self.data.snapshot = std::fs::read_to_string(&snap_path).ok().map(Arc::new);
        }
        let cs = stamp(&self.config_path);
        if cs != self.config_stamp {
            self.config_stamp = cs;
            self.data.config_text =
                Arc::new(std::fs::read_to_string(&self.config_path).unwrap_or_default());
        }

        let status = self.data.status.clone();
        let field = |k: &str| {
            status
                .as_ref()
                .and_then(|s| s[k].as_str().map(str::to_string))
        };
        let target = field("target").unwrap_or_else(|| {
            crate::detect::resolve(&self.cfg.libreqos)
                .map(|d| crate::cli::target_name(d.target).to_string())
                .unwrap_or_else(|e| {
                    errors.push(format!("target: {e:#}"));
                    "v2_pair".into()
                })
        });
        let shadow = match field("mode").as_deref() {
            Some(m) => m == "shadow",
            None => self.cfg.runtime.mode == RuntimeMode::Shadow,
        };
        let dir = if shadow {
            state_dir.join("candidate")
        } else {
            PathBuf::from(&self.cfg.libreqos.directory)
        };
        let v3net = target == "v3_network";
        let inputs: Vec<PathBuf> = if v3net {
            vec![dir.join(&self.cfg.libreqos.v3_network_file)]
        } else {
            vec![dir.join("ShapedDevices.csv"), dir.join("network.json")]
        };
        let state_path = state_dir.join("state.json");
        let stamps: Vec<Stamp> = inputs
            .iter()
            .chain([&state_path])
            .map(|p| stamp(p))
            .collect();
        if stamps != self.input_stamps || target != self.data.target {
            let parsed = if v3net {
                self.read_limited(&inputs[0], &mut errors).and_then(|t| {
                    let v: Value = serde_json::from_str(&t)
                        .map_err(|e| errors.push(format!("{}: {e}", inputs[0].display())))
                        .ok()?;
                    rows_from_libreqos_network(&v)
                        .map_err(|e| errors.push(format!("{}: {e:#}", inputs[0].display())))
                        .ok()
                })
            } else {
                let csv = self.read_limited(&inputs[0], &mut errors);
                // No network.json is normal for a flat network.
                // A network.json that exists but cannot be read or parsed keeps the last data.
                let tree = if inputs[1].exists() {
                    self.read_limited(&inputs[1], &mut errors).and_then(|t| {
                        serde_json::from_str::<Value>(&t)
                            .map_err(|e| errors.push(format!("{}: {e}", inputs[1].display())))
                            .ok()
                    })
                } else {
                    Some(Value::Null)
                };
                csv.and_then(|c| {
                    crate::state::read_csv(&c)
                        .map_err(|e| errors.push(format!("{}: {e:#}", inputs[0].display())))
                        .ok()
                })
                .zip(tree)
            };
            // A failed read keeps the old data and the old stamps, so the next refresh tries again.
            if let Some((rows, tree)) = parsed {
                self.input_stamps = stamps;
                self.data.target = target;
                let state: Option<State> = std::fs::read_to_string(&state_path)
                    .ok()
                    .and_then(|t| serde_json::from_str(&t).ok());
                self.data.sources = Arc::new(sources(state.as_ref()));
                self.set_rows(rows, tree);
            }
        }
        self.data.errors = errors;
        self.data.clone()
    }

    fn set_rows(&mut self, rows: Vec<CsvRow>, tree: Value) {
        let mut names = std::collections::HashSet::new();
        collect_names(&tree, &mut names);
        let mut ip_count: HashMap<&str, u32> = HashMap::new();
        for r in &rows {
            for ip in r.ipv4.split(',').map(str::trim).filter(|x| !x.is_empty()) {
                *ip_count.entry(ip).or_default() += 1;
            }
        }
        let mut node_circuits: HashMap<String, Vec<u32>> = HashMap::new();
        let mut search = Vec::with_capacity(rows.len());
        let mut flags = Vec::with_capacity(rows.len());
        for (i, r) in rows.iter().enumerate() {
            search.push(
                format!(
                    "{} {} {} {} {} {}",
                    r.circuit_name, r.circuit_id, r.device_name, r.mac, r.ipv4, r.comment
                )
                .to_lowercase(),
            );
            let mut f = 0;
            if r.ipv4
                .split(',')
                .map(str::trim)
                .any(|ip| ip_count.get(ip) > Some(&1))
            {
                f |= FLAG_DUP_IP;
            }
            if !r.parent_node.is_empty() && !names.contains(r.parent_node.as_str()) {
                f |= FLAG_NO_PARENT;
            }
            if crate::validate::rate_problem(r).is_some() {
                f |= FLAG_RATE;
            }
            flags.push(f);
            node_circuits
                .entry(r.parent_node.clone())
                .or_default()
                .push(i as u32);
        }
        self.data.row_search = Arc::new(search);
        self.data.flags = Arc::new(flags);
        self.data.node_circuits = Arc::new(node_circuits);
        self.data.tree = Arc::new(tree);
        self.data.rows = Arc::new(rows);
    }
}

fn collect_names<'a>(tree: &'a Value, out: &mut std::collections::HashSet<&'a str>) {
    if let Value::Object(m) = tree {
        for (name, node) in m {
            out.insert(name);
            collect_names(&node["children"], out);
        }
    }
}

fn sources(state: Option<&State>) -> HashMap<String, (String, String)> {
    state.map_or_else(HashMap::new, |s| {
        s.circuits
            .iter()
            .map(|(name, c)| {
                let source = serde_json::to_value(c.source)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default();
                (name.clone(), (source, c.router.clone().unwrap_or_default()))
            })
            .collect()
    })
}

/// Turn `libreqos_network.json` back into rows and a `network.json` style tree, for the screens.
pub fn rows_from_libreqos_network(doc: &Value) -> anyhow::Result<(Vec<CsvRow>, Value)> {
    let empty = Vec::new();
    let topology = doc["topology"].as_array().unwrap_or(&empty);
    let mut names: HashMap<&str, &str> = HashMap::new();
    let mut kids: HashMap<Option<&str>, Vec<&Value>> = HashMap::new();
    for n in topology {
        let id = n["id"].as_str().unwrap_or_default();
        names.insert(id, n["name"].as_str().unwrap_or(id));
        kids.entry(n["parent"].as_str()).or_default().push(n);
    }
    fn build(
        parent: Option<&str>,
        kids: &HashMap<Option<&str>, Vec<&Value>>,
        depth: u32,
    ) -> Map<String, Value> {
        let mut out = Map::new();
        // A loop in the parent links stops at 64 levels.
        if depth > 64 {
            return out;
        }
        for n in kids.get(&parent).into_iter().flatten() {
            let id = n["id"].as_str().unwrap_or_default();
            let mut node = json!({"id": id, "type": n["type"]});
            if let Some(r) = n.get("rate") {
                node["downloadBandwidthMbps"] = r["ceil_down_mbps"].clone();
                node["uploadBandwidthMbps"] = r["ceil_up_mbps"].clone();
            }
            let children = build(Some(id), kids, depth + 1);
            if !children.is_empty() {
                node["children"] = Value::Object(children);
            }
            out.insert(n["name"].as_str().unwrap_or(id).to_string(), node);
        }
        out
    }
    let tree = Value::Object(build(None, &kids, 0));

    let s = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let num = |v: &Value| v.as_f64().map(|x| x.to_string()).unwrap_or_default();
    let join = |v: &Value| {
        v.as_array()
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default()
    };
    let mut rows = Vec::new();
    for c in doc["circuits"].as_array().unwrap_or(&empty) {
        let parent = c["parent"].as_str().unwrap_or_default();
        let plan = &c["plan"];
        for d in c["devices"].as_array().unwrap_or(&empty) {
            rows.push(CsvRow {
                circuit_id: s(&c["id"]),
                circuit_name: s(&c["display_name"]),
                device_id: s(&d["id"]),
                device_name: s(&d["name"]),
                parent_node: names.get(parent).unwrap_or(&parent).to_string(),
                parent_node_id: parent.to_string(),
                mac: s(&d["mac"]),
                ipv4: join(&d["ipv4"]),
                ipv6: join(&d["ipv6"]),
                download_min: num(&plan["commit_down_mbps"]),
                upload_min: num(&plan["commit_up_mbps"]),
                download_max: num(&plan["ceil_down_mbps"]),
                upload_max: num(&plan["ceil_up_mbps"]),
                comment: s(&d["comment"]),
                ..CsvRow::default()
            });
        }
    }
    Ok((rows, tree))
}

/// Start the loader thread (`tui-loader`). It sends one `Data` each `tui.refresh_ms`.
/// It stops when the receiver is gone.
pub fn spawn_loader(
    cfg: &Config,
    config_path: PathBuf,
    tx: Sender<Data>,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    let refresh = Duration::from_millis(cfg.tui.refresh_ms);
    let mut l = Loader::new(cfg.clone(), config_path);
    std::thread::Builder::new()
        .name("tui-loader".into())
        .spawn(move || {
            while tx.send(l.load()).is_ok() {
                std::thread::sleep(refresh);
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::v3network::render_libreqos_network;
    use serde_json::json;

    struct Env {
        _dir: tempfile::TempDir,
        lqos: PathBuf,
        state: PathBuf,
        config: PathBuf,
    }

    fn env() -> (Env, Loader) {
        let dir = tempfile::tempdir().unwrap();
        let (lqos, state) = (dir.path().join("lqos"), dir.path().join("state"));
        std::fs::create_dir_all(&lqos).unwrap();
        std::fs::create_dir_all(&state).unwrap();
        let toml = format!(
            "[runtime]\nmode=\"apply\"\nstate_dir=\"{}\"\n[libreqos]\ntarget=\"v2_pair\"\ndirectory=\"{}\"\ndpkg_status=\"/nonexistent\"\n\
             [[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n",
            state.display(),
            lqos.display()
        );
        let config = dir.path().join("lqosync3.toml");
        std::fs::write(&config, &toml).unwrap();
        let cfg = Config::from_toml_str(&toml).unwrap();
        let mut l = Loader::new(cfg, config.clone());
        l.systemctl = "/nonexistent/systemctl".into();
        (
            Env {
                _dir: dir,
                lqos,
                state,
                config,
            },
            l,
        )
    }

    const HEAD: &str = "Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,MAC,IPv4,IPv6,Download Min Mbps,Upload Min Mbps,Download Max Mbps,Upload Max Mbps,Comment\n";

    fn pair(e: &Env, rows: &str) {
        std::fs::write(e.lqos.join("ShapedDevices.csv"), format!("{HEAD}{rows}")).unwrap();
        std::fs::write(
            e.lqos.join("network.json"),
            r#"{"Site":{"downloadBandwidthMbps":100,"uploadBandwidthMbps":100,"children":{"AP":{"downloadBandwidthMbps":50,"uploadBandwidthMbps":50}}}}"#,
        )
        .unwrap();
    }

    #[test]
    fn loader_reads_the_pair_and_the_sources() {
        let (e, mut l) = env();
        pair(&e, "c1,alice,d1,alice,AP,aa:bb,10.0.0.2,,1,1,10,10,x\n");
        std::fs::write(
            e.state.join("state.json"),
            r#"{"version":1,"circuits":{"alice":{"router":"R1","source":"ppp","last_seen":1,"row":{}}},"nodes":{},"routers":{}}"#,
        )
        .unwrap();
        let d = l.load();
        assert_eq!(d.rows.len(), 1);
        assert_eq!(d.row_search[0], "alice c1 alice aa:bb 10.0.0.2 x");
        assert_eq!(d.sources["alice"], ("ppp".to_string(), "R1".to_string()));
        assert_eq!(d.node_circuits["AP"], [0]);
        assert!(d.tree["Site"]["children"]["AP"].is_object());
        assert!(d.config_text.contains("[runtime]"));
        assert!(d.status.is_none() && d.service.is_none() && d.proc.is_none());
        assert_eq!(d.target, "v2_pair");
        let _ = &e.config;
    }

    #[test]
    fn a_missing_csv_is_an_error() {
        let (_e, mut l) = env();
        let d = l.load();
        assert!(
            d.errors.iter().any(|x| x.contains("ShapedDevices.csv")),
            "{:?}",
            d.errors
        );
    }

    #[test]
    fn loader_reads_the_snapshot_text() {
        let (e, mut l) = env();
        assert!(l.load().snapshot.is_none());
        std::fs::write(
            e.state.join("snapshot.json"),
            r#"{"version":1,"taken":5,"routers":[]}"#,
        )
        .unwrap();
        let (a, b) = (l.load(), l.load());
        assert!(a.snapshot.as_deref().unwrap().contains("taken"));
        assert!(Arc::ptr_eq(
            a.snapshot.as_ref().unwrap(),
            b.snapshot.as_ref().unwrap()
        ));
    }

    #[test]
    fn loader_skips_unchanged_files() {
        let (e, mut l) = env();
        pair(&e, "c1,alice,d1,alice,AP,aa:bb,10.0.0.2,,1,1,10,10,\n");
        let (a, b) = (l.load(), l.load());
        assert!(Arc::ptr_eq(&a.rows, &b.rows));
        assert!(Arc::ptr_eq(&a.config_text, &b.config_text));
        pair(
            &e,
            "c1,alice,d1,alice,AP,aa:bb,10.0.0.2,,1,1,10,10,\nc2,bob,d2,bob,AP,,10.0.0.3,,1,1,10,10,\n",
        );
        let c = l.load();
        assert!(!Arc::ptr_eq(&a.rows, &c.rows));
        assert_eq!(c.rows.len(), 2);
    }

    #[test]
    fn a_bad_network_json_keeps_the_last_data_and_shows_an_error() {
        let (e, mut l) = env();
        pair(&e, "c1,alice,d1,alice,AP,,10.0.0.2,,1,1,10,10,\n");
        let a = l.load();
        std::fs::write(e.lqos.join("network.json"), "{bad").unwrap();
        for _ in 0..2 {
            let b = l.load();
            assert!(Arc::ptr_eq(&a.tree, &b.tree) && Arc::ptr_eq(&a.rows, &b.rows));
            assert!(
                b.errors.iter().any(|x| x.contains("network.json")),
                "{:?}",
                b.errors
            );
        }
    }

    #[test]
    fn too_large_file_keeps_last_data() {
        let (e, mut l) = env();
        pair(&e, "c1,alice,d1,alice,AP,,10.0.0.2,,1,1,10,10,\n");
        let a = l.load();
        l.max_bytes = 200;
        pair(
            &e,
            &"c1,alice,d1,alice,AP,,10.0.0.2,,1,1,10,10,\n".repeat(10),
        );
        let b = l.load();
        assert!(Arc::ptr_eq(&a.rows, &b.rows));
        assert!(
            b.errors.iter().any(|x| x.contains("max_file_mb")),
            "{:?}",
            b.errors
        );
    }

    #[test]
    fn flags_mark_duplicate_ip_missing_parent_and_low_rate() {
        let (e, mut l) = env();
        pair(
            &e,
            "c1,a,d1,a,AP,,10.0.0.2,,1,1,10,10,\nc2,b,d2,b,AP,,10.0.0.2,,1,1,10,10,\nc3,c,d3,c,Gone,,10.0.0.4,,1,1,10,10,\nc4,d,d4,d,AP,,10.0.0.5,,1,1,0.1,10,\n",
        );
        let d = l.load();
        assert_eq!(d.flags[0], FLAG_DUP_IP);
        assert_eq!(d.flags[1], FLAG_DUP_IP);
        assert_eq!(d.flags[2], FLAG_NO_PARENT);
        assert_eq!(d.flags[3], FLAG_RATE);
    }

    #[test]
    fn status_target_selects_the_v3_network_file() {
        let (e, mut l) = env();
        std::fs::write(
            e.state.join("status.json"),
            r#"{"target":"v3_network","mode":"apply"}"#,
        )
        .unwrap();
        let rows = vec![CsvRow {
            circuit_id: "c1".into(),
            circuit_name: "alice".into(),
            device_id: "d1".into(),
            device_name: "alice".into(),
            parent_node: "Site".into(),
            ipv4: "10.0.0.2".into(),
            download_min: "1".into(),
            upload_min: "1".into(),
            download_max: "10".into(),
            upload_max: "10".into(),
            ..CsvRow::default()
        }];
        let tree =
            json!({"Site": {"id": "s1", "downloadBandwidthMbps": 100, "uploadBandwidthMbps": 100}});
        let (text, _) = render_libreqos_network(&rows, &tree).unwrap();
        std::fs::write(e.lqos.join("libreqos_network.json"), text).unwrap();
        let d = l.load();
        assert_eq!(d.target, "v3_network");
        assert_eq!(d.rows[0].circuit_name, "alice");
        assert_eq!(d.status.unwrap()["mode"], "apply");
    }

    #[test]
    fn rows_from_libreqos_network_round_trip() {
        let row = |cid: &str, name: &str, did: &str, parent: &str, ip: &str| CsvRow {
            circuit_id: cid.into(),
            circuit_name: name.into(),
            device_id: did.into(),
            device_name: format!("{name}-{did}"),
            parent_node: parent.into(),
            mac: "aa:bb:cc:dd:ee:ff".into(),
            ipv4: ip.into(),
            ipv6: "2001:db8::1".into(),
            download_min: "5".into(),
            upload_min: "2.5".into(),
            download_max: "50".into(),
            upload_max: "25".into(),
            comment: "note".into(),
            ..CsvRow::default()
        };
        let input = vec![
            row("c1", "alice", "d1", "AP", "10.0.0.2"),
            row("c1", "alice", "d2", "AP", "10.0.0.3,10.0.0.4"),
            row("c2", "bob", "d3", "Site", "10.0.0.5"),
        ];
        let tree = json!({"Site": {"id": "s1", "type": "site", "downloadBandwidthMbps": 100, "uploadBandwidthMbps": 100,
            "children": {"AP": {"id": "a1", "type": "ap", "downloadBandwidthMbps": 50, "uploadBandwidthMbps": 50}}}});
        let (text, alerts) = render_libreqos_network(&input, &tree).unwrap();
        assert!(alerts.is_empty(), "{alerts:?}");
        let (rows, back) =
            rows_from_libreqos_network(&serde_json::from_str(&text).unwrap()).unwrap();
        assert_eq!(rows.len(), 3);
        let num = |s: &str| s.parse::<f64>().unwrap();
        for (a, b) in input.iter().zip(&rows) {
            let text = |r: &CsvRow| {
                (
                    r.circuit_id.clone(),
                    r.circuit_name.clone(),
                    r.device_id.clone(),
                    r.device_name.clone(),
                    r.mac.clone(),
                    r.ipv4.clone(),
                    r.ipv6.clone(),
                    r.parent_node.clone(),
                    r.comment.clone(),
                )
            };
            assert_eq!(text(a), text(b));
            for (x, y) in [
                (&a.download_min, &b.download_min),
                (&a.upload_min, &b.upload_min),
                (&a.download_max, &b.download_max),
                (&a.upload_max, &b.upload_max),
            ] {
                assert_eq!(num(x), num(y));
            }
        }
        assert_eq!(back["Site"]["id"], "s1");
        assert_eq!(back["Site"]["children"]["AP"]["type"], "ap");
        assert_eq!(
            back["Site"]["children"]["AP"]["downloadBandwidthMbps"],
            50.0
        );
    }
}
