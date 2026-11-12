//! Isolated receipt-write benchmark. No live configuration, RouterOS request, or Telegram send.
use lqosync3::{
    model::{Columns, CsvRow},
    system_health::Hub,
    write_receipt::{Meta, Tracker},
};
use serde_json::json;
use std::time::Instant;
fn main() -> anyhow::Result<()> {
    for count in [1000, 10000] {
        let temp = tempfile::tempdir()?;
        let hub = Hub::default();
        let mut tracker = Tracker::load(temp.path())?;
        let rows: Vec<_> = (0..count)
            .map(|i| CsvRow {
                circuit_id: format!("c{i}"),
                device_id: format!("d{i}"),
                circuit_name: format!("Client {i}"),
                device_name: "Device".into(),
                parent_node: "P".into(),
                ipv4: std::net::Ipv4Addr::from(0x6440_0000 + i).to_string(),
                download_min: "1".into(),
                upload_min: "1".into(),
                download_max: "20".into(),
                upload_max: "10".into(),
                ..Default::default()
            })
            .collect();
        let pair = vec![
            (temp.path().join("network.json"), "{}".into()),
            (
                temp.path().join("ShapedDevices.csv"),
                lqosync3::writer::pair::render_csv(&rows, Columns::V3),
            ),
        ];
        let meta = || Meta {
            cycle_seq: 1,
            config_digest: "benchmark".into(),
            notice: None,
        };
        let start = Instant::now();
        let first = tracker.write(&pair, meta(), 10, &hub);
        let changed_us = start.elapsed().as_micros();
        anyhow::ensure!(
            first.changed && first.error.is_none() && first.tracking_error.is_none(),
            "changed-write benchmark failed"
        );
        let id = tracker.current().unwrap().write_id.clone();
        let mut unchanged = Vec::new();
        for _ in 0..5 {
            let start = Instant::now();
            let r = tracker.write(&pair, meta(), 11, &hub);
            unchanged.push(start.elapsed().as_micros());
            anyhow::ensure!(
                !r.changed && r.error.is_none(),
                "unchanged-write benchmark failed"
            );
        }
        unchanged.sort_unstable();
        anyhow::ensure!(
            tracker.current().unwrap().write_id == id,
            "unchanged cycle created a new operation"
        );
        println!(
            "{}",
            json!({"rows":count,"csv_bytes":pair[1].1.len(),"changed_receipt_path_us":changed_us,"unchanged_median_us":unchanged[2],"profile":if cfg!(debug_assertions){"debug"}else{"release"},"full_cycle_cost_included":false,"csv_render_cost_included":false})
        );
    }
    Ok(())
}
