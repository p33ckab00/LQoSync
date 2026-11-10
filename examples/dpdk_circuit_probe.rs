//! Read-only queue endpoint verification. No subscriber names, IDs, or IPs are printed.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let socket = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/shm/libreqos-analytics.sock".into());
    let csv_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/opt/libreqos/src/ShapedDevices.csv".into());
    let mut csv = csv::Reader::from_path(csv_path)?;
    let circuit = csv
        .deserialize::<lqosync3::model::CsvRow>()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no shaping record"))??
        .circuit_id;
    for i in 0..3 {
        let q = lqosync3::circuit_queue::fetch(std::path::Path::new(&socket), &circuit).await?;
        let row = &q.reply.rows[0];
        let directions:Vec<_>=[&row.download,&row.upload].into_iter().map(|d|d.as_ref().map(|d|serde_json::json!({
            "tin_count":d.tins.as_ref().map(Vec::len),"reported_memory_used":d.memory_used,
            "queue_depth_pkts":d.queue_depth_pkts,"drop_pkts":d.drop_pkts,"drop_pkts_queue_full":d.drop_pkts_queue_full,"aqm_marks":d.aqm_marks,
            "tin_fields_present":d.tins.as_ref().is_some_and(|t|t.iter().all(|t|t.sent_bytes.is_some() && t.backlog_bytes.is_some() && t.sojourn_p99_ns.is_some()))
        }))).collect();
        println!(
            "{}",
            serde_json::json!({"single_slot_match":q.slot>0,"rows":q.reply.rows.len(),"directions":directions,
            "rtt_bins":row.rtt_histogram.as_ref().map(Vec::len),"rtt_samples":row.rtt_histogram.as_ref().map(|h|h.iter().sum::<u64>()),
            "sampled_at_unix_ms":q.reply.sampled_at_unix_ms,"sample_epoch_unix_ms":q.reply.sample_epoch_unix_ms,
            "sample_seq":q.reply.sample_seq,"elapsed_ms":q.reply.elapsed_ms,"cadence_ms":q.reply.cadence_ms,
            "actor_gap":q.reply.actor_gap,"counter_reset":q.reply.counter_reset})
        );
        if i < 2 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    Ok(())
}
