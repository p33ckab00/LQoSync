//! Scenario tests: config, router snapshot, and previous CSV in; expected files out.
//!
//! Set `LQOSYNC3_BLESS=1` to write the expected files from the current output.
//! Check each blessed number against the formulas in the case `config.toml` before a commit.

use lqosync3::config::Config;
use lqosync3::cycle::{CycleInput, CycleOutput, RouterResult, assemble};
use lqosync3::model::{Columns, CsvRow, RouterSnapshot, State};
use lqosync3::state::{read_csv, seed_state, split_static};
use lqosync3::writer::pair::{render_csv, render_network_json};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Snapshot {
    routers: BTreeMap<String, RouterSnapshot>,
    #[serde(default)]
    results: BTreeMap<String, String>,
}

fn results(cfg: &Config, snap: Snapshot) -> Vec<(String, RouterResult)> {
    let mut routers = snap.routers;
    cfg.routers
        .iter()
        .map(|r| {
            let res = match snap.results.get(&r.name).map(String::as_str) {
                Some("failed") => RouterResult::Failed,
                Some("unreachable") => RouterResult::Unreachable,
                Some("offline") => RouterResult::Offline,
                _ => RouterResult::Online(routers.remove(&r.name).unwrap_or_default()),
            };
            (r.name.clone(), res)
        })
        .collect()
}

fn cycle(
    cfg: &Config,
    state: &State,
    statics: &[CsvRow],
    snap: Snapshot,
    v3: bool,
    n: &mut u32,
) -> CycleOutput {
    let mut ids = || {
        *n += 1;
        format!("N{n:04}")
    };
    let input = CycleInput {
        config: cfg,
        state,
        static_rows: statics,
        routers: results(cfg, snap),
        current_network: None,
        v3,
        force: false,
        now: 1_000,
    };
    assemble(input, &mut ids).unwrap()
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn report_json(out: &CycleOutput) -> String {
    let r = &out.report;
    let v = serde_json::json!({
        "added": r.added, "removed": r.removed, "held": r.held, "alerts": r.alerts.len(),
    });
    render_network_json(&v)
}

#[test]
fn scenarios_match_expected_output() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/scenarios");
    let bless = std::env::var_os("LQOSYNC3_BLESS").is_some();
    let mut cases: Vec<PathBuf> = fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    cases.sort();
    assert!(cases.len() >= 9, "expected nine scenarios");
    for dir in cases {
        let case = dir.file_name().unwrap().to_string_lossy().to_string();
        let cfg = Config::from_toml_str(&read(&dir.join("config.toml"))).unwrap();
        let (statics, other) = split_static(
            read_csv(&read(&dir.join("previous.csv"))).unwrap(),
            &cfg.runtime.static_comment,
        );
        let v3 = case.starts_with("v3_");
        let mut n = 0;
        let snap: Snapshot = serde_json::from_str(&read(&dir.join("snapshot.json"))).unwrap();
        let mut out = cycle(&cfg, &seed_state(&other, 1_000), &statics, snap, v3, &mut n);
        let second = dir.join("snapshot2.json");
        if second.exists() {
            let snap: Snapshot = serde_json::from_str(&read(&second)).unwrap();
            out = cycle(&cfg, &out.state.clone(), &statics, snap, v3, &mut n);
        }
        let cols = if v3 { Columns::V3 } else { Columns::V2 };
        let got = [
            ("ShapedDevices.csv", render_csv(&out.rows, cols)),
            ("network.json", render_network_json(&out.network)),
            ("report.json", report_json(&out)),
        ];
        let scratch = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/scenario-out")
            .join(&case);
        fs::create_dir_all(&scratch).unwrap();
        for (file, text) in &got {
            fs::write(scratch.join(file), text).unwrap();
            let want = dir.join("expected").join(file);
            if bless {
                fs::create_dir_all(want.parent().unwrap()).unwrap();
                fs::write(&want, text).unwrap();
            } else {
                assert_eq!(text, &read(&want), "scenario {case}: {file}");
            }
        }
    }
}
