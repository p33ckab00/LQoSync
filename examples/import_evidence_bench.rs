//! Local comparison benchmark and optional read-only live import check. No identifiers in output.
use lqosync3::{import_evidence::ExpectedImport, model::CsvRow};
use serde_json::{Value, json};
use std::time::Instant;
fn main() -> anyhow::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--live") {
        let csv = std::fs::read_to_string("/opt/libreqos/src/ShapedDevices.csv")?;
        let network = std::fs::read_to_string("/opt/libreqos/src/network.json")?;
        let imported: Value = serde_json::from_str(&std::fs::read_to_string(
            "/opt/libreqos/state/topology/topology_import.json",
        )?)?;
        let rows = lqosync3::state::read_csv(&csv)?;
        let started = Instant::now();
        let expected = ExpectedImport::new(&network, &rows)?;
        println!(
            "{}",
            json!({"rows":rows.len(),"import_matches_pair":expected.matches(&imported),
            "prepare_and_compare_us":started.elapsed().as_micros(),"profile":if cfg!(debug_assertions){"debug"}else{"release"}})
        );
        return Ok(());
    }
    for count in [1000, 10000, 50000] {
        let rows: Vec<_> = (0..count)
            .map(|i| CsvRow {
                circuit_id: format!("c{i}"),
                device_id: format!("d{i}"),
                circuit_name: format!("Circuit {i}"),
                device_name: "Device".into(),
                parent_node: "P".into(),
                ipv4: std::net::Ipv4Addr::from(0x6440_0000 + i).to_string(),
                download_min: "1.1".into(),
                upload_min: "1.1".into(),
                download_max: "20".into(),
                upload_max: "10".into(),
                ..CsvRow::default()
            })
            .collect();
        let devices:Vec<_>=rows.iter().map(|r| json!({
            "circuit_id":r.circuit_id,"device_id":r.device_id,"circuit_name":r.circuit_name,"device_name":r.device_name,
            "parent_node":"P","mac":"","ipv4":[[r.ipv4,32]],"ipv6":[],
            "download_min_mbps":f64::from(1.1f32),"upload_min_mbps":f64::from(1.1f32),"download_max_mbps":20,"upload_max_mbps":10
        })).collect();
        let imported =
            json!({"imported":{"compatibility_network_json":{},"shaped_devices":devices}});
        let start = Instant::now();
        let expected = ExpectedImport::new("{}", &rows)?;
        let prepare_us = start.elapsed().as_micros();
        let mut cached = Vec::new();
        let mut rebuilt = Vec::new();
        for _ in 0..5 {
            let start = Instant::now();
            assert!(expected.matches(&imported));
            cached.push(start.elapsed().as_micros());
            let start = Instant::now();
            assert!(ExpectedImport::new("{}", &rows)?.matches(&imported));
            rebuilt.push(start.elapsed().as_micros());
        }
        cached.sort_unstable();
        rebuilt.sort_unstable();
        println!(
            "{}",
            json!({"rows":count,"prepare_us":prepare_us,"reuse_median_us":cached[2],
            "rebuild_median_us":rebuilt[2],"input_json_bytes":imported.to_string().len(),"iterations":5,
            "profile":if cfg!(debug_assertions){"debug"}else{"release"},"json_parse_cost_included":false})
        );
    }
    Ok(())
}
