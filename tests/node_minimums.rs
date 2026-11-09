use lqosync3::config::Config;
use lqosync3::model::State;
use lqosync3::sources::GroupKey;
use lqosync3::speed::Rate;
use lqosync3::topology::{GroupTotal, RouterGroups, build_nodes, to_network_json};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn config(extra: &str) -> Config {
    Config::from_toml_str(&format!(
        "{extra}\n[[routers]]\nname='R1'\naddress='192.0.2.1'\nusername='u'\npassword_file='/x'\n"
    ))
    .unwrap()
}

fn online() -> Vec<RouterGroups> {
    vec![RouterGroups {
        router: "R1".into(),
        ppp_enabled: true,
        per_plan_node: false,
        totals: BTreeMap::from([(
            GroupKey::Ppp,
            GroupTotal {
                count: 1,
                sum: Rate {
                    down: 100.0,
                    up: 40.0,
                },
                unit: Rate {
                    down: 100.0,
                    up: 40.0,
                },
            },
        )]),
    }]
}

fn network(cfg: &Config) -> anyhow::Result<Value> {
    let nodes = build_nodes(cfg, true, &online(), &BTreeMap::new(), &[])?;
    let tree = to_network_json(&nodes, None, &BTreeMap::new());
    lqosync3::validate::check(&[], &tree, cfg.network.mode)?;
    Ok(tree)
}

#[test]
fn node_minimums_survive_output_and_state_reload() {
    let c = config(
        "[network]\nroot_download_min_mbps=20\nroot_upload_min_mbps=10\n[nodes.ppp]\ndownload_min_mbps=8\nupload_min_mbps=4",
    );
    let nodes = build_nodes(&c, true, &online(), &BTreeMap::new(), &[]).unwrap();
    let state = State {
        nodes,
        ..State::default()
    };
    let restored = State::from_json(&state.to_json()).unwrap();
    let net = to_network_json(&restored.nodes, None, &BTreeMap::new());
    assert_eq!(net["R1"]["downloadBandwidthMbpsMin"].as_u64(), Some(20));
    assert_eq!(net["R1"]["uploadBandwidthMbpsMin"].as_u64(), Some(10));
    assert_eq!(
        net["R1"]["children"]["PPP-R1"]["downloadBandwidthMbpsMin"].as_u64(),
        Some(8)
    );
    assert_eq!(
        net["R1"]["children"]["PPP-R1"]["uploadBandwidthMbpsMin"].as_u64(),
        Some(4)
    );
}

#[test]
fn node_minimums_are_absent_by_default_and_old_state_still_loads() {
    let net = network(&config("")).unwrap();
    assert!(net["R1"].get("downloadBandwidthMbpsMin").is_none());
    assert!(
        net["R1"]["children"]["PPP-R1"]
            .get("uploadBandwidthMbpsMin")
            .is_none()
    );
    let old = r#"{"version":1,"circuits":{},"routers":{},"nodes":{"Old":{"id":"old","router":null,"parent":null,"type":"site","down":100.0,"up":100.0}}}"#;
    let restored = State::from_json(old).unwrap();
    let net = to_network_json(&restored.nodes, None, &BTreeMap::new());
    assert!(net["Old"].get("downloadBandwidthMbpsMin").is_none());
}

#[test]
fn node_minimums_preserve_seeded_operator_values() {
    let net = json!({"Manual":{"id":"manual","type":"site","downloadBandwidthMbps":100,"uploadBandwidthMbps":50,"downloadBandwidthMbpsMin":8,"uploadBandwidthMbpsMin":4,"children":{}}});
    let nodes = lqosync3::state::seed_nodes(&net);
    let actual = to_network_json(&nodes, Some(net), &BTreeMap::new());
    assert_eq!(actual["Manual"]["downloadBandwidthMbpsMin"], 8);
    assert_eq!(actual["Manual"]["uploadBandwidthMbpsMin"], 4);
}

#[test]
fn node_minimums_preserve_operator_values_with_state_from_before_the_upgrade() {
    let old = r#"{"version":1,"circuits":{},"routers":{},"nodes":{"Manual":{"id":"manual","router":null,"parent":null,"type":"site","down":100.0,"up":50.0,"seeded":true}}}"#;
    let restored = State::from_json(old).unwrap();
    let current =
        json!({"Manual":{"children":{},"downloadBandwidthMbpsMin":8,"uploadBandwidthMbpsMin":4}});
    let net = to_network_json(&restored.nodes, Some(current), &BTreeMap::new());
    assert_eq!(net["Manual"]["downloadBandwidthMbpsMin"], 8);
    assert_eq!(net["Manual"]["uploadBandwidthMbpsMin"], 4);
}

#[test]
fn node_minimums_reset_removes_generated_fields_in_preserve_mode() {
    let before = config("[network]\nroot_download_min_mbps=20\n[nodes.ppp]\ndownload_min_mbps=8");
    let old_nodes = build_nodes(&before, true, &online(), &BTreeMap::new(), &[]).unwrap();
    let previous = to_network_json(&old_nodes, None, &BTreeMap::new());
    let after = config("");
    let new_nodes = build_nodes(&after, true, &online(), &BTreeMap::new(), &[]).unwrap();
    let net = to_network_json(&new_nodes, Some(previous), &old_nodes);
    assert!(net["R1"].get("downloadBandwidthMbpsMin").is_none());
    assert!(
        net["R1"]["children"]["PPP-R1"]
            .get("downloadBandwidthMbpsMin")
            .is_none()
    );
}

#[test]
fn node_minimums_follow_manual_edits_in_preserve_mode() {
    let original = json!({"Manual":{"id":"manual","children":{},"downloadBandwidthMbps":100,"uploadBandwidthMbps":50,"downloadBandwidthMbpsMin":8}});
    let nodes = lqosync3::state::seed_nodes(&original);
    let mut current = original.clone();
    current["Manual"]["downloadBandwidthMbpsMin"] = 16.into();
    let net = to_network_json(&nodes, Some(current), &BTreeMap::new());
    assert_eq!(net["Manual"]["downloadBandwidthMbpsMin"], 16);
    let mut current = original;
    current["Manual"]
        .as_object_mut()
        .unwrap()
        .remove("downloadBandwidthMbpsMin");
    let net = to_network_json(&nodes, Some(current), &BTreeMap::new());
    assert!(net["Manual"].get("downloadBandwidthMbpsMin").is_none());
}

#[test]
fn node_minimums_reject_zero_fractional_negative_and_above_root_ceiling() {
    for value in ["0", "-1", "0.5", "inf", "nan", "641"] {
        let text = format!("[network]\nroot_download_min_mbps={value}");
        assert!(Config::from_toml_str(&text).is_err(), "accepted {value}");
    }
    assert!(Config::from_toml_str("[nodes.ppp]\nupload_min_mbps=0").is_err());
}

#[test]
fn node_minimums_reject_a_group_ceiling_that_shrinks_below_the_minimum() {
    let c = config("[nodes.ppp]\ndownload_min_mbps=60");
    assert!(
        network(&c).is_err(),
        "50 Mbps group must reject a 60 Mbps minimum"
    );
}

#[test]
fn node_minimums_show_up_in_diff_and_native_translation() {
    let old = json!({"N":{"id":"n","type":"site","downloadBandwidthMbps":100,"uploadBandwidthMbps":50,"children":{}}});
    let mut new = old.clone();
    new["N"]["downloadBandwidthMbpsMin"] = 8.into();
    let diff = lqosync3::diff::diff_pair("", "", &old, &new).unwrap();
    assert!(diff.contains("downloadBandwidthMbpsMin"), "{diff}");
    assert!(diff.contains("nodes: +0 -0 ~1"), "{diff}");
    let (native, _) = lqosync3::writer::v3network::render_libreqos_network(&[], &new).unwrap();
    let native: Value = serde_json::from_str(&native).unwrap();
    assert_eq!(native["topology"][0]["rate"]["commit_down_mbps"], 8.0);
    assert_eq!(native["topology"][0]["rate"]["commit_up_mbps"], 50.0);
}

#[test]
fn node_minimums_are_optional_integer_settings() {
    use lqosync3::tui::settings::meta::{self, Kind};
    for key in [
        "network.root_download_min_mbps",
        "network.root_upload_min_mbps",
        "nodes.ppp.download_min_mbps",
        "nodes.dhcp_plan.upload_min_mbps",
    ] {
        let m = meta::find(key).unwrap_or_else(|| panic!("missing editor key {key}"));
        assert_eq!(m.kind, Kind::Int);
        assert!(m.optional && m.default.is_none());
    }
}

/// Run with LQ3_BINDING_DIR=/opt/libreqos/src cargo test --test node_minimums -- --ignored --nocapture.
#[test]
#[ignore = "manual: needs the installed LibreQoS binding, python3, and strace"]
fn node_minimums_survive_the_installed_pair_compiler() {
    use lqosync3::model::{Columns, CsvRow};
    use lqosync3::writer::pair::{render_csv, render_network_json};
    use std::{fs, process::Command};

    let binding = std::env::var("LQ3_BINDING_DIR").expect("LQ3_BINDING_DIR");
    let scratch = tempfile::tempdir().unwrap();
    let dir = scratch.path();
    fs::create_dir(dir.join("src")).unwrap();
    fs::create_dir(dir.join("state")).unwrap();
    let c = config(
        "[network]\nroot_download_min_mbps=20\nroot_upload_min_mbps=10\n[nodes.ppp]\ndownload_min_mbps=8\nupload_min_mbps=4",
    );
    let net = network(&c).unwrap();
    let row = CsvRow {
        circuit_id: "TEST1".into(),
        circuit_name: "test".into(),
        device_id: "DEVICE1".into(),
        device_name: "test".into(),
        parent_node: "PPP-R1".into(),
        parent_node_id: net["R1"]["children"]["PPP-R1"]["id"]
            .as_str()
            .unwrap()
            .into(),
        ipv4: "192.0.2.2".into(),
        mac: "02:00:00:00:00:01".into(),
        download_min: "1".into(),
        upload_min: "1".into(),
        download_max: "10".into(),
        upload_max: "5".into(),
        ..Default::default()
    };
    fs::write(dir.join("src/network.json"), render_network_json(&net)).unwrap();
    fs::write(
        dir.join("src/ShapedDevices.csv"),
        render_csv(&[row], Columns::V3),
    )
    .unwrap();
    let example = fs::read_to_string(format!("{binding}/lqos.example")).unwrap();
    let text = lqosync3::probe::probe_conf(&example, dir, "full").unwrap();
    let mut conf: toml::Table = toml::from_str(&text).unwrap();
    conf["topology"]["source"] = "shaped_devices".into();
    fs::write(dir.join("lqos.conf"), toml::to_string(&conf).unwrap()).unwrap();
    fs::write(
        dir.join("compile.py"),
        r#"
import os, sys
sys.path.insert(0, os.environ['SIM_BINDING'])
import liblqos_python as m
assert m.write_compiled_topology_from_legacy_artifacts('shaped_devices', 'full')
"#,
    )
    .unwrap();
    let result = Command::new("timeout")
        .args(["60", "strace", "-f", "-qq", "-o"])
        .arg(dir.join("connect.log"))
        .args([
            "-e",
            "trace=connect",
            "-e",
            "inject=connect:error=EACCES",
            "python3",
            "-I",
        ])
        .arg(dir.join("compile.py"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("SIM_BINDING", &binding)
        .env("LQOS_CONFIG", dir.join("lqos.conf"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let canonical: Value = serde_json::from_str(
        &fs::read_to_string(dir.join("state/topology/topology_canonical_state.json")).unwrap(),
    )
    .unwrap();
    let tree = &canonical["compatibility_network_json"];
    assert_eq!(tree["R1"]["downloadBandwidthMbpsMin"], 20);
    assert_eq!(tree["R1"]["uploadBandwidthMbpsMin"], 10);
    assert_eq!(
        tree["R1"]["children"]["PPP-R1"]["downloadBandwidthMbpsMin"],
        8
    );
    assert_eq!(
        tree["R1"]["children"]["PPP-R1"]["uploadBandwidthMbpsMin"],
        4
    );
}
