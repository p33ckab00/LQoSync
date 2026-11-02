//! Native probe: does the installed LibreQoS v3 compile a schema 2 `libreqos_network.json` with the
//! node kinds and rates? The probe compiles a synthetic topology in a scratch folder with the
//! LibreQoS Python binding. It does not touch the live files, the live state or `/etc/lqos.conf`.
//! LibreQoS 3.0.7 fails it: each node gets 1 Mbps
//! (docs/reports/2026-11-02-libreqos-native-node-rate-loss.md).

use crate::config::Libreqos;
use crate::detect::package_version;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;

/// The probe result in `<state_dir>`. The probe runs again when its key changes.
pub const CACHE: &str = "native-probe.json";
/// The binding in `libreqos.directory`.
pub const BINDING: &str = "liblqos_python.so";
/// A probe that could not run is tried again after this time.
pub const ERROR_RETRY_SECS: u64 = 3600;
const TIMEOUT: Duration = Duration::from_secs(60);

/// A synthetic node: (id, name, type, parent, down, up).
type Node = (
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
    f64,
    f64,
);
const NODES: [Node; 2] = [
    (
        "lqosync3-probe-site",
        "PROBE-SITE",
        "site",
        None,
        200.0,
        100.0,
    ),
    (
        "lqosync3-probe-node",
        "PROBE-NODE",
        "node",
        Some("lqosync3-probe-site"),
        50.0,
        25.0,
    ),
];

/// The result of one probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Probe {
    /// LibreQoS version, compile mode, and size and time of the binding.
    pub key: String,
    /// ready | blocked | error
    pub state: String,
    pub detail: String,
    pub at: u64,
}

impl Probe {
    pub fn ready(&self) -> bool {
        self.state == "ready"
    }

    /// `ready`, or the state and the reason.
    pub fn summary(&self) -> String {
        if self.ready() {
            "ready".into()
        } else {
            format!("{}: {}", self.state, self.detail)
        }
    }
}

/// `[topology] compile_mode` of a v3 `lqos.conf`, or `full` if it is not set.
pub fn compile_mode(lqos_conf: &str) -> anyhow::Result<String> {
    let t: toml::Table = toml::from_str(lqos_conf).context("TOML parse error")?;
    Ok(t.get("topology")
        .and_then(|v| v.get("compile_mode"))
        .and_then(|v| v.as_str())
        .unwrap_or("full")
        .to_string())
}

/// The probe key, or `None` if this is not LibreQoS v3 with the binding (no probe).
pub fn key(cfg: &Libreqos) -> Option<String> {
    let dpkg = std::fs::read_to_string(&cfg.dpkg_status).ok()?;
    let version = package_version(&dpkg, "libreqos")?;
    if !version.starts_with("3.") {
        return None;
    }
    let mode = std::fs::read_to_string(&cfg.v3_config)
        .ok()
        .and_then(|t| compile_mode(&t).ok())
        .unwrap_or_else(|| "full".into());
    let meta = std::fs::metadata(Path::new(&cfg.directory).join(BINDING)).ok()?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    Some(format!("{version}|{mode}|{}|{mtime}", meta.len()))
}

/// The synthetic `libreqos_network.json` (schema 2).
pub fn input() -> Value {
    let topology: Vec<Value> = NODES
        .iter()
        .map(|(id, name, kind, parent, d, u)| {
            json!({"id": id, "name": name, "type": kind, "parent": parent,
                   "rate": {"commit_down_mbps": d, "commit_up_mbps": u, "ceil_down_mbps": d, "ceil_up_mbps": u}})
        })
        .collect();
    json!({
        "schema_version": 2,
        "match": {"mode": "ip"},
        "topology": topology,
        "circuits": [{
            "id": "LQOSYNC3PROBE", "display_name": "lqosync3-probe", "parent": NODES[1].0,
            "plan": {"commit_down_mbps": 5.0, "commit_up_mbps": 2.5, "ceil_down_mbps": 50.0, "ceil_up_mbps": 25.0},
            "match": {"prefixes": ["10.255.0.2/32"]},
            "devices": [{"id": "LQOSYNC3PROBED", "name": "lqosync3-probe", "mac": "02:00:00:00:00:01",
                         "ipv4": ["10.255.0.2"], "ipv6": [], "comment": ""}],
        }],
    })
}

/// The probe `lqos.conf`: the LibreQoS example with the scratch folders and the native source.
pub fn probe_conf(example: &str, dir: &Path, mode: &str) -> anyhow::Result<String> {
    let mut t: toml::Table = toml::from_str(example).context("lqos.example: TOML parse error")?;
    let d = dir.display();
    t.insert("lqos_directory".into(), format!("{d}/src").into());
    t.insert("state_directory".into(), format!("{d}/state").into());
    let mut topo = toml::Table::new();
    topo.insert("source".into(), "libreqos_network".into());
    topo.insert("compile_mode".into(), mode.into());
    t.insert("topology".into(), topo.into());
    Ok(toml::to_string(&t)?)
}

/// `Ok` if the compiled canonical state keeps the rate of each synthetic node.
pub fn judge(canonical: &Value) -> Result<(), String> {
    fn find<'a>(tree: &'a Value, id: &str) -> Option<&'a Value> {
        tree.as_object()?.values().find_map(|n| {
            if n["id"].as_str() == Some(id) {
                Some(n)
            } else {
                find(&n["children"], id)
            }
        })
    }
    let tree = &canonical["compatibility_network_json"];
    for (id, name, _, _, d, u) in NODES {
        let n = find(tree, id).ok_or(format!("node {name} is missing in the compiled tree"))?;
        let (gd, gu) = (
            n["downloadBandwidthMbps"].as_f64(),
            n["uploadBandwidthMbps"].as_f64(),
        );
        if (gd, gu) != (Some(d), Some(u)) {
            let f = |x: Option<f64>| x.map_or("none".into(), |x| x.to_string());
            return Err(format!(
                "node {name} compiled to {}/{} Mbps, expected {d}/{u}",
                f(gd),
                f(gu)
            ));
        }
    }
    Ok(())
}

const CHILD: &str = r#"
import os, sys
sys.path.insert(0, os.environ["PROBE_LIB"])
import liblqos_python as m
text = open(os.environ["PROBE_INPUT"]).read()
ok = m.write_compiled_topology_from_network_json_payload("libreqos_network", os.environ["PROBE_MODE"], text)
print("PROBE_RESULT", bool(ok))
"#;

/// Compile the synthetic topology in `dir`. `Ok(Ok)` = ready, `Ok(Err)` = blocked, `Err` = error.
async fn compile(cfg: &Libreqos, dir: &Path, mode: &str) -> anyhow::Result<Result<(), String>> {
    let lib = Path::new(&cfg.directory);
    let example = std::fs::read_to_string(lib.join("lqos.example"))
        .with_context(|| format!("cannot read {}/lqos.example", lib.display()))?;
    std::fs::create_dir_all(dir.join("src"))?;
    std::fs::create_dir_all(dir.join("state"))?;
    let conf = dir.join("lqos.conf");
    std::fs::write(&conf, probe_conf(&example, dir, mode)?)?;
    let inp = dir.join("src/libreqos_network.json");
    std::fs::write(&inp, input().to_string())?;
    let child = tokio::process::Command::new("python3")
        .args(["-I", "-c", CHILD])
        .current_dir(dir)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LQOS_CONFIG", &conf)
        .env("PROBE_LIB", lib)
        .env("PROBE_INPUT", &inp)
        .env("PROBE_MODE", mode)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("cannot start python3")?;
    let out = tokio::time::timeout(TIMEOUT, child.wait_with_output())
        .await
        .context("the probe did not finish in 60 s")??;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let last_err = || {
        let e = String::from_utf8_lossy(&out.stderr);
        e.lines().last().unwrap_or_default().to_string()
    };
    if !stdout.contains("PROBE_RESULT") {
        // The binding raised an error: LibreQoS rejected the schema 2 file or cannot load.
        return Ok(Err(format!(
            "LibreQoS did not compile the schema 2 file: {}",
            last_err()
        )));
    }
    if !stdout.contains("PROBE_RESULT True") {
        return Ok(Err("LibreQoS rejected the schema 2 file".into()));
    }
    let path = dir.join("state/topology/topology_canonical_state.json");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("no compiled state {}", path.display()))?;
    Ok(judge(&serde_json::from_str(&text)?))
}

/// The cached result for this LibreQoS, without a new probe. `None`: not LibreQoS v3.
/// `Some(None)`: no result for this LibreQoS yet.
pub fn cached(cfg: &Libreqos, state_dir: &Path) -> Option<Option<Probe>> {
    let key = key(cfg)?;
    let p = std::fs::read_to_string(state_dir.join(CACHE))
        .ok()
        .and_then(|t| serde_json::from_str::<Probe>(&t).ok());
    Some(p.filter(|p| p.key == key))
}

/// The probe result for this LibreQoS, from the cache or a new probe. `None`: not LibreQoS v3.
/// The bool is true when this call found `ready` and the last result was not `ready`.
pub async fn status(cfg: &Libreqos, state_dir: &Path, now: u64) -> (Option<Probe>, bool) {
    let Some(key) = key(cfg) else {
        return (None, false);
    };
    let path = state_dir.join(CACHE);
    let cached: Option<Probe> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok());
    if let Some(c) = &cached
        && c.key == key
        && (c.state != "error" || now.saturating_sub(c.at) < ERROR_RETRY_SECS)
    {
        return (cached, false);
    }
    let mode = key.split('|').nth(1).unwrap_or("full").to_string();
    let dir = state_dir.join("native-probe");
    let _ = std::fs::remove_dir_all(&dir);
    let (state, detail) = match compile(cfg, &dir, &mode).await {
        Ok(Ok(())) => ("ready", String::new()),
        Ok(Err(why)) => ("blocked", why),
        Err(e) => ("error", format!("{e:#}")),
    };
    let _ = std::fs::remove_dir_all(&dir);
    let p = Probe {
        key,
        state: state.into(),
        detail,
        at: now,
    };
    if let Err(e) = crate::files::write_if_changed(
        &path,
        serde_json::to_string(&p).unwrap_or_default().as_bytes(),
    ) {
        eprintln!("warning: cannot write {}: {e:#}", path.display());
    }
    let new = p.ready() && !cached.is_some_and(|c| c.ready());
    (Some(p), new)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical(site: (f64, f64), node: (f64, f64)) -> Value {
        json!({"compatibility_network_json": {"PROBE-SITE": {
            "id": "lqosync3-probe-site", "downloadBandwidthMbps": site.0, "uploadBandwidthMbps": site.1,
            "children": {"PROBE-NODE": {"id": "lqosync3-probe-node",
                "downloadBandwidthMbps": node.0, "uploadBandwidthMbps": node.1, "children": {}}}}}})
    }

    #[test]
    fn judge_accepts_the_rates_and_rejects_1_mbps() {
        assert_eq!(judge(&canonical((200.0, 100.0), (50.0, 25.0))), Ok(()));
        let e = judge(&canonical((1.0, 1.0), (1.0, 1.0))).unwrap_err();
        assert!(e.contains("PROBE-SITE") && e.contains("1/1"), "{e}");
        let e = judge(&json!({"compatibility_network_json": {}})).unwrap_err();
        assert!(e.contains("missing"), "{e}");
    }

    #[test]
    fn input_is_schema_2_with_node_rates() {
        let v = input();
        assert_eq!(v["schema_version"], 2);
        assert_eq!(v["topology"][1]["rate"]["ceil_down_mbps"], 50.0);
        assert_eq!(v["topology"][1]["parent"], "lqosync3-probe-site");
        assert_eq!(v["circuits"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn probe_conf_points_only_to_the_scratch_folder() {
        let example = "lqos_directory = \"/opt/libreqos/src\"\nstate_directory = \"/opt/libreqos/state\"\n\
                       node_id = \"x\"\n[topology]\nsource = \"shaped_devices\"\ncompile_mode = \"ap_site\"\n[queues]\nsudo = false\n";
        let t = probe_conf(example, Path::new("/tmp/p"), "full").unwrap();
        let v: toml::Table = toml::from_str(&t).unwrap();
        assert_eq!(v["lqos_directory"].as_str(), Some("/tmp/p/src"));
        assert_eq!(v["state_directory"].as_str(), Some("/tmp/p/state"));
        assert_eq!(v["topology"]["source"].as_str(), Some("libreqos_network"));
        assert_eq!(v["topology"]["compile_mode"].as_str(), Some("full"));
        assert_eq!(v["queues"]["sudo"].as_bool(), Some(false));
        assert!(!t.contains("/opt/libreqos"), "{t}");
    }

    /// Uses the installed LibreQoS: `cargo test -- --ignored probe_on_this_host`.
    #[tokio::test]
    #[ignore]
    async fn probe_on_this_host() {
        let d = tempfile::tempdir().unwrap();
        let cfg = crate::config::defaults().libreqos.clone();
        let (p, new) = status(&cfg, d.path(), 1_000).await;
        let p = p.expect("LibreQoS v3 with the binding");
        eprintln!("{p:?} new={new}");
        assert_ne!(p.state, "error", "{}", p.detail);
        assert!(
            !d.path().join("native-probe").exists(),
            "the scratch folder is removed"
        );
        // A second call uses the cache.
        assert_eq!(status(&cfg, d.path(), 2_000).await, (Some(p), false));
    }

    #[test]
    fn compile_mode_defaults_to_full() {
        assert_eq!(
            compile_mode("[topology]\nsource = \"x\"\n").unwrap(),
            "full"
        );
        assert_eq!(
            compile_mode("[topology]\ncompile_mode = \"ap_site\"\n").unwrap(),
            "ap_site"
        );
    }

    #[test]
    fn key_needs_libreqos_v3_and_the_binding() {
        let d = tempfile::tempdir().unwrap();
        let dpkg = d.path().join("status");
        let conf = d.path().join("lqos.conf");
        std::fs::write(&conf, "[topology]\ncompile_mode = \"full\"\n").unwrap();
        let cfg = Libreqos {
            directory: d.path().display().to_string(),
            dpkg_status: dpkg.display().to_string(),
            v3_config: conf.display().to_string(),
            ..Libreqos::default()
        };
        std::fs::write(
            &dpkg,
            "Package: libreqos\nStatus: install ok installed\nVersion: 2.2.1\n",
        )
        .unwrap();
        assert_eq!(key(&cfg), None, "v2");
        std::fs::write(
            &dpkg,
            "Package: libreqos\nStatus: install ok installed\nVersion: 3.0.7.1\n",
        )
        .unwrap();
        assert_eq!(key(&cfg), None, "no binding");
        std::fs::write(d.path().join(BINDING), "x").unwrap();
        let k = key(&cfg).unwrap();
        assert!(k.starts_with("3.0.7.1|full|1|"), "{k}");
    }
}
