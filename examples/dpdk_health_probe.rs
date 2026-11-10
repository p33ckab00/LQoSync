//! Read-only health verification. Aggregate fields only; no subscriber or flow identities.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/shm/libreqos-analytics.sock".into());
    let mut health = lqosync3::worker_util::health::Health::default();
    for i in 0..3 {
        let report = lqosync3::worker_util::read_health(std::path::Path::new(&path)).await?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        health.record(Ok(report), now);
        println!("{}", serde_json::to_string(&health)?);
        if i < 2 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    Ok(())
}
