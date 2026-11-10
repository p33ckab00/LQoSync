//! Read-only source verification. No host, flow, or subscriber identities are printed.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/shm/libreqos-analytics.sock".into());
    let mut last = None;
    for _ in 0..3 {
        let sample = lqosync3::worker_util::read_snapshot(std::path::Path::new(&path)).await?;
        println!(
            "{}",
            serde_json::json!({
                "captured_at_millis":sample.captured_at_millis,
                "producer_advanced":last.map(|at|sample.captured_at_millis > at),
                "actual_delivered":sample.actual,
                "mapped_enqueued":sample.mapped,
                "unmapped_enqueued":sample.unmapped,
            })
        );
        last = Some(sample.captured_at_millis);
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    Ok(())
}
