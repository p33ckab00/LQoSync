//! The settings preview: the cycle core two times on the same router reads, with the old and the new config.

use crate::config::{Config, NetworkMode, RuntimeMode, Target};
use crate::cycle::{CycleInput, CycleOutput, RouterResult, assemble};
use crate::model::{Columns, RouterSnapshot};
use crate::runner::{Inputs, cycle_inputs};
use crate::writer::pair::{render_csv, render_network_json};
use anyhow::{Context, bail};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Preview {
    /// No key that changes the output changed. The reason.
    Skipped(String),
    Ran(Ran),
    /// The preview could not run. The reason; never "0 changes".
    Failed(String),
}

#[derive(Debug)]
pub struct Ran {
    /// The circuit and node lines of `diff::diff_pair`.
    pub text: String,
    /// Circuits added, removed, changed; nodes added, removed, changed.
    pub counts: [usize; 6],
    pub warnings: Vec<String>,
    /// Circuits that the removal guard holds with the new config: the next cycle removes them.
    pub held: Vec<String>,
}

/// True if a changed key can change the LibreQoS files (spec 6.3).
pub fn output_keys_changed(changed: &[String]) -> bool {
    changed
        .iter()
        .any(|k| !(k.starts_with("telegram.") || k.starts_with("tui.") || k == "runtime.log_level"))
}

/// `taken` and the result of each router in `snapshot.json`.
pub fn snapshot_routers(text: &str) -> anyhow::Result<(u64, HashMap<String, RouterResult>)> {
    let v: Value = serde_json::from_str(text).context("snapshot.json is not valid JSON")?;
    let taken = v["taken"].as_u64().unwrap_or(0);
    let mut out = HashMap::new();
    for r in v["routers"].as_array().into_iter().flatten() {
        let name = r["name"].as_str().unwrap_or_default().to_string();
        let result = match r["state"].as_str() {
            Some("online") => RouterResult::Online(
                serde_json::from_value::<RouterSnapshot>(r["snapshot"].clone())
                    .with_context(|| format!("snapshot.json: router {name}"))?,
            ),
            Some("failed") => RouterResult::Failed,
            Some("unreachable") => RouterResult::Unreachable,
            Some("offline") => RouterResult::Offline,
            other => bail!("snapshot.json: router {name} has the state {other:?}"),
        };
        out.insert(name, result);
    }
    Ok((taken, out))
}

/// The routers of `cfg` in its order. A router with no snapshot keeps its subscribers (failed).
fn routers_for(cfg: &Config, snap: &HashMap<String, RouterResult>) -> Vec<(String, RouterResult)> {
    cfg.routers
        .iter()
        .map(|r| {
            (
                r.name.clone(),
                snap.get(&r.name).cloned().unwrap_or(RouterResult::Failed),
            )
        })
        .collect()
}

struct One {
    csv: String,
    network: Value,
    out: CycleOutput,
    alerts: Vec<String>,
}

fn run_one(
    cfg: &Config,
    inputs: &Inputs,
    snap: &HashMap<String, RouterResult>,
    now: u64,
) -> anyhow::Result<One> {
    let target = crate::detect::resolve(&cfg.libreqos)?.target;
    let v3 = target != Target::V2Pair;
    // A counter, not random IDs: the two runs then give the same IDs in the same order.
    let mut n = 0u32;
    let mut new_id = || {
        n += 1;
        format!("PV{n:06}")
    };
    let input = CycleInput {
        config: cfg,
        state: &inputs.state,
        static_rows: &inputs.live.static_rows,
        routers: routers_for(cfg, snap),
        current_network: inputs.live.network.clone(),
        v3,
        force: false,
        now,
    };
    let mut out = assemble(input, &mut new_id)?;
    let mut alerts = std::mem::take(&mut out.report.alerts);
    if target == Target::V3Network {
        alerts
            .extend(crate::writer::v3network::render_libreqos_network(&out.rows, &out.network)?.1);
    }
    // The IDs of a new circuit depend on the counter order: hide them, so they do not show as a change.
    for row in &mut out.rows {
        if !inputs.state.circuits.contains_key(&row.circuit_name) {
            row.circuit_id = "(new)".into();
            row.device_id = "(new)".into();
            if cfg.network.mode == NetworkMode::FlatPromotion {
                row.parent_node_id = "(new)".into();
            }
        }
    }
    let csv = render_csv(&out.rows, if v3 { Columns::V3 } else { Columns::V2 });
    let network: Value =
        serde_json::from_str(&render_network_json(&out.network)).unwrap_or(Value::Null);
    Ok(One {
        csv,
        network,
        out,
        alerts,
    })
}

fn counts(text: &str) -> [usize; 6] {
    let last = text.lines().last().unwrap_or_default();
    let nums: Vec<usize> = last
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|x| x.parse().ok())
        .collect();
    let mut out = [0; 6];
    for (o, n) in out.iter_mut().zip(nums) {
        *o = n;
    }
    out
}

/// Run the preview. `snapshot` is the text of `snapshot.json`.
pub fn preview(
    old: &Config,
    new: &Config,
    snapshot: Option<&str>,
    now: u64,
    max_age: u64,
) -> Preview {
    match run(old, new, snapshot, now, max_age) {
        Ok(r) => Preview::Ran(r),
        Err(e) => Preview::Failed(format!("{e:#}")),
    }
}

fn run(
    old: &Config,
    new: &Config,
    snapshot: Option<&str>,
    now: u64,
    max_age: u64,
) -> anyhow::Result<Ran> {
    // The files and the state of the current config, for both runs.
    let inputs = cycle_inputs(old, now)?;
    let mut warnings = Vec::new();
    let snap = match snapshot {
        None => {
            warnings.push("no snapshot.json: the effect of the routers is not known".to_string());
            HashMap::new()
        }
        Some(t) => {
            let (taken, m) = snapshot_routers(t)?;
            let age = now.saturating_sub(taken);
            if age > max_age {
                warnings.push(format!(
                    "snapshot.json is {age} s old (tui.preview_max_age = {max_age})"
                ));
            }
            m
        }
    };
    if snapshot.is_some() {
        for r in &new.routers {
            if !snap.contains_key(&r.name) {
                warnings.push(format!(
                    "effect of router {} is not known (no snapshot)",
                    r.name
                ));
                continue;
            }
            let state = match snap.get(&r.name) {
                Some(RouterResult::Failed) => Some("failed"),
                Some(RouterResult::Unreachable) => Some("unreachable"),
                Some(RouterResult::Offline) => Some("offline"),
                _ => None,
            };
            if let Some(state) = state {
                warnings.push(format!(
                    "router {} was {state} at the last cycle: the effect of the change on its subscribers is not shown",
                    r.name
                ));
            }
            let o = old.routers.iter().find(|x| x.name == r.name);
            for (src, on_new, on_old) in [
                ("pppoe", r.pppoe.enabled, o.is_some_and(|o| o.pppoe.enabled)),
                (
                    "hotspot",
                    r.hotspot.enabled,
                    o.is_some_and(|o| o.hotspot.enabled),
                ),
                ("dhcp", r.dhcp.enabled, o.is_some_and(|o| o.dhcp.enabled)),
            ] {
                if on_new && !on_old {
                    warnings.push(format!(
                        "effect of router {} {src} is not known (not read at the last cycle)",
                        r.name
                    ));
                }
            }
        }
    }
    let mut not_shown = Vec::new();
    if old.libreqos.directory != new.libreqos.directory {
        not_shown.push("libreqos.directory");
    }
    if old.network.preserve_network_config != new.network.preserve_network_config {
        not_shown.push("network.preserve_network_config");
    }
    if old.runtime.static_comment != new.runtime.static_comment {
        not_shown.push("runtime.static_comment");
    }
    let first_run = !std::path::Path::new(&old.runtime.state_dir)
        .join("state.json")
        .exists();
    if first_run && format!("{:?}", old.circuits) != format!("{:?}", new.circuits) {
        not_shown.push("circuits");
    }
    for k in not_shown {
        warnings.push(format!(
            "the preview reads the files of the current config; the effect of {k} is not shown"
        ));
    }
    if old.runtime.mode != RuntimeMode::Apply && new.runtime.mode == RuntimeMode::Apply {
        warnings.push("apply mode writes the live LibreQoS files; run lqosync3 diff first".into());
    }
    if old.libreqos.target != new.libreqos.target {
        warnings.push("libreqos.target changes; the column change is not shown".into());
    }

    let a = run_one(old, &inputs, &snap, now)?;
    let b = run_one(new, &inputs, &snap, now)?;
    let mut text = crate::diff::diff_pair(&a.csv, &b.csv, &a.network, &b.network)?;
    let counts = counts(&text);
    let held = b.out.report.held.clone();
    if !held.is_empty() {
        text.push_str(&format!(
            "held: removed after one held cycle: {} circuits ({})\n",
            held.len(),
            held.join(", ")
        ));
    }
    warnings.extend(b.alerts);
    Ok(Ran {
        text,
        counts,
        warnings,
        held,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CsvRow, Source, State, StoredCircuit};

    struct Env {
        dir: tempfile::TempDir,
        base: String,
    }

    fn env() -> Env {
        let dir = tempfile::tempdir().unwrap();
        let (lqos, state) = (dir.path().join("lqos"), dir.path().join("state"));
        std::fs::create_dir_all(&lqos).unwrap();
        std::fs::create_dir_all(&state).unwrap();
        let base = format!(
            "[runtime]\nstate_dir = \"{}\"\n[libreqos]\ntarget = \"v2_pair\"\ndirectory = \"{}\"\n[network]\nmode = \"root_promotion\"\n[safety]\nenabled = false\n",
            state.display(),
            lqos.display()
        );
        Env { dir, base }
    }

    const ROUTER: &str = "[[routers]]\nname = \"R1\"\naddress = \"192.0.2.1\"\nusername = \"u\"\npassword_file = \"/x\"\n[routers.pppoe]\nenabled = true\nper_plan_node = true\n";

    fn cfg(e: &Env, extra: &str, routers: &str) -> Config {
        Config::from_toml_str(&format!("{}{extra}\n{routers}", e.base)).unwrap()
    }

    fn snap(taken: u64, users: &[(&str, &str)]) -> String {
        let secret: Vec<_> = users
            .iter()
            .map(|(n, _)| serde_json::json!({"name": n, "profile": "HS"}))
            .collect();
        let active: Vec<_> = users
            .iter()
            .map(|(n, ip)| serde_json::json!({"name": n, "address": ip}))
            .collect();
        serde_json::json!({"version": 1, "taken": taken, "routers": [{"name": "R1", "state": "online", "read_at": taken, "read_ms": 5,
            "snapshot": {"ppp_secret": secret, "ppp_active": active, "ppp_profile": [{"name": "HS", "rate-limit": "5M/20M"}]}}]})
        .to_string()
    }

    fn ran(p: Preview) -> Ran {
        match p {
            Preview::Ran(r) => r,
            other => panic!("{other:?}"),
        }
    }

    const ALICE: &[(&str, &str)] = &[("alice", "100.64.0.2")];

    #[test]
    fn same_snapshot_same_config_gives_no_change() {
        let e = env();
        let c = cfg(&e, "", ROUTER);
        let r = ran(preview(&c, &c, Some(&snap(1_000, ALICE)), 1_000, 300));
        assert_eq!(r.counts, [0; 6], "{}", r.text);
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    }

    #[test]
    fn a_rate_change_gives_the_expected_circuit_changes() {
        let e = env();
        let (old, new) = (
            cfg(&e, "", ROUTER),
            cfg(
                &e,
                "[bandwidth_defaults]\nmin_rate_percentage = 0.25",
                ROUTER,
            ),
        );
        let r = ran(preview(&old, &new, Some(&snap(1_000, ALICE)), 1_000, 300));
        assert_eq!(r.counts, [0, 0, 1, 0, 0, 0], "{}", r.text);
        let line = r
            .text
            .lines()
            .find(|l| l.starts_with("~ circuit alice"))
            .unwrap();
        assert!(
            line.contains("Min Mbps")
                && !line.contains("Circuit ID")
                && !line.contains("Device ID"),
            "{line}"
        );
    }

    #[test]
    fn an_old_snapshot_gives_the_stale_warning() {
        let e = env();
        let c = cfg(&e, "", ROUTER);
        let r = ran(preview(&c, &c, Some(&snap(100, ALICE)), 1_000, 300));
        assert!(
            r.warnings
                .iter()
                .any(|w| w.contains("snapshot.json is 900 s old")),
            "{:?}",
            r.warnings
        );
        let r = ran(preview(&c, &c, None, 1_000, 300));
        assert!(
            r.warnings.iter().any(|w| w.contains("no snapshot.json")),
            "{:?}",
            r.warnings
        );
    }

    #[test]
    fn a_new_router_effect_is_not_known() {
        let e = env();
        let two = format!(
            "{ROUTER}{}",
            ROUTER
                .replace("\"R1\"", "\"R2\"")
                .replace("192.0.2.1", "192.0.2.2")
        );
        let r = ran(preview(
            &cfg(&e, "", ROUTER),
            &cfg(&e, "", &two),
            Some(&snap(1_000, ALICE)),
            1_000,
            300,
        ));
        assert!(
            r.warnings
                .iter()
                .any(|w| w.contains("effect of router R2 is not known")),
            "{:?}",
            r.warnings
        );
    }

    #[test]
    fn a_router_that_was_not_online_gives_a_warning() {
        let e = env();
        let c = cfg(&e, "", ROUTER);
        let text = snap(1_000, ALICE).replace("\"state\":\"online\"", "\"state\":\"unreachable\"");
        let r = ran(preview(&c, &c, Some(&text), 1_000, 300));
        assert!(
            r.warnings
                .iter()
                .any(|w| w.contains("router R1 was unreachable")),
            "{:?}",
            r.warnings
        );
    }

    #[test]
    fn a_source_turned_on_is_not_known() {
        let e = env();
        let on = format!("{ROUTER}[routers.hotspot]\nenabled = true\n");
        let r = ran(preview(
            &cfg(&e, "", ROUTER),
            &cfg(&e, "", &on),
            Some(&snap(1_000, ALICE)),
            1_000,
            300,
        ));
        assert!(
            r.warnings
                .iter()
                .any(|w| w.contains("router R1 hotspot is not known")),
            "{:?}",
            r.warnings
        );
    }

    #[test]
    fn a_failed_preview_is_failed_not_zero() {
        let e = env();
        std::fs::write(e.dir.path().join("lqos/network.json"), "{bad").unwrap();
        let c = Config::from_toml_str(&format!(
            "{}\n{ROUTER}",
            e.base.replace(
                "mode = \"root_promotion\"",
                "mode = \"root_promotion\"\npreserve_network_config = true"
            )
        ))
        .unwrap();
        match preview(&c, &c, Some(&snap(1_000, ALICE)), 1_000, 300) {
            Preview::Failed(why) => {
                assert!(why.contains("network.json") && !why.contains("+0"), "{why}")
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn held_circuits_show_as_removed_later() {
        let e = env();
        let mut st = State::default();
        for n in ["bob", "carol", "dave"] {
            let row = CsvRow {
                circuit_id: format!("c-{n}"),
                circuit_name: n.into(),
                device_id: format!("d-{n}"),
                device_name: n.into(),
                ipv4: "100.64.1.1".replace("1.1", &format!("1.{}", n.len())),
                download_min: "1".into(),
                upload_min: "1".into(),
                download_max: "10".into(),
                upload_max: "10".into(),
                comment: "PPP".into(),
                ..CsvRow::default()
            };
            st.circuits.insert(
                n.into(),
                StoredCircuit {
                    router: Some("R1".into()),
                    source: Source::Ppp,
                    last_seen: 900,
                    row,
                },
            );
        }
        std::fs::write(e.dir.path().join("state/state.json"), st.to_json()).unwrap();
        let c = Config::from_toml_str(&format!(
            "{}\n{ROUTER}",
            e.base
                .replace("enabled = false", "enabled = true\nmax_removed_count = 1")
        ))
        .unwrap();
        let r = ran(preview(&c, &c, Some(&snap(1_000, ALICE)), 1_000, 300));
        assert_eq!(r.held.len(), 3, "{r:?}");
        assert!(
            r.text.contains("held: removed after one held cycle"),
            "{}",
            r.text
        );
    }

    #[test]
    fn mode_change_to_apply_warns() {
        let e = env();
        let new = Config::from_toml_str(&format!(
            "{}\n{ROUTER}",
            e.base.replace("[runtime]", "[runtime]\nmode = \"apply\"")
        ))
        .unwrap();
        let r = ran(preview(
            &cfg(&e, "", ROUTER),
            &new,
            Some(&snap(1_000, ALICE)),
            1_000,
            300,
        ));
        assert!(
            r.warnings
                .iter()
                .any(|w| w.contains("run lqosync3 diff first")),
            "{:?}",
            r.warnings
        );
    }

    #[test]
    fn flat_promotion_new_circuits_show_no_false_change() {
        let e = env();
        let flat = |extra: &str| {
            Config::from_toml_str(&format!(
                "{}{extra}\n{ROUTER}",
                e.base.replace("root_promotion", "flat_promotion")
            ))
            .unwrap()
        };
        let users = &[("alice", "100.64.0.2"), ("bob", "100.64.0.3")];
        let r = ran(preview(
            &flat(""),
            &flat("[bandwidth_defaults]\nmin_rate_percentage = 0.25"),
            Some(&snap(1_000, users)),
            1_000,
            300,
        ));
        assert!(!r.text.contains("Parent Node ID"), "{}", r.text);
        assert_eq!(r.counts[2], 2, "{}", r.text);
    }

    #[test]
    fn output_keys_changed_skips_telegram_log_level_and_tui() {
        assert!(!output_keys_changed(&[
            "telegram.enabled".into(),
            "runtime.log_level".into(),
            "tui.osc52".into()
        ]));
        assert!(output_keys_changed(&[
            "telegram.enabled".into(),
            "runtime.mode".into()
        ]));
        assert!(!output_keys_changed(&[]));
    }

    #[test]
    fn cycle_inputs_writes_nothing() {
        let e = env();
        std::fs::write(
            e.dir.path().join("lqos/ShapedDevices.csv"),
            "Circuit ID,Circuit Name\n",
        )
        .unwrap();
        let list = || {
            let mut v: Vec<_> = walk(e.dir.path());
            v.sort();
            v
        };
        fn walk(p: &std::path::Path) -> Vec<String> {
            std::fs::read_dir(p)
                .unwrap()
                .flat_map(|x| {
                    let x = x.unwrap().path();
                    if x.is_dir() {
                        walk(&x)
                    } else {
                        vec![x.display().to_string()]
                    }
                })
                .collect()
        }
        let before = list();
        let c = cfg(&e, "", ROUTER);
        let _ = preview(&c, &c, Some(&snap(1_000, ALICE)), 1_000, 300);
        assert_eq!(before, list());
    }
}
