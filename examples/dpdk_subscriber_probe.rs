//! Read-only source verification. Output has counts only, without subscriber identifiers.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/shm/libreqos-analytics.sock".into());
    let csv_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/opt/libreqos/src/ShapedDevices.csv".into());
    let mut csv = csv::Reader::from_path(csv_path)?;
    let rows: Vec<lqosync3::model::CsvRow> = csv.deserialize().collect::<Result<_, _>>()?;
    let mut previous = None;
    for i in 0..3 {
        let (mapping, snapshot) =
            lqosync3::subscriber_stats::sample(std::path::Path::new(&path)).await?;
        let hosts = snapshot.hosts.as_ref().unwrap();
        let matches = |offset: usize| {
            hosts
                .iter()
                .filter(|h| {
                    let Some(name) = usize::try_from(h.id.circuit_id)
                        .ok()
                        .and_then(|slot| slot.checked_sub(offset))
                        .and_then(|slot| mapping.names.get(slot))
                    else {
                        return false;
                    };
                    rows.iter().any(|r| {
                        r.circuit_id == *name
                            && r.ipv4.split(',').chain(r.ipv6.split(',')).any(|ip| {
                                ip.trim()
                                    .split('/')
                                    .next()
                                    .and_then(|v| v.parse::<std::net::IpAddr>().ok())
                                    == Some(h.id.subscriber_ip)
                            })
                    })
                })
                .count()
        };
        println!(
            "{}",
            serde_json::json!({"captured_at_millis":snapshot.captured_at_millis,
              "producer_advanced":previous.map(|p|snapshot.captured_at_millis>p),"host_rows":hosts.len(),"limit":lqosync3::subscriber_stats::LIMIT,
              "slot_names":mapping.names.len(),"first_slot_empty":mapping.names.first().is_some_and(String::is_empty),
              "direct_slot_matches":matches(0),"offset_one_matches":matches(1),
              "min_host_slot":hosts.iter().map(|h|h.id.circuit_id).min(),"max_host_slot":hosts.iter().map(|h|h.id.circuit_id).max(),
              "actual_fields":hosts.iter().filter(|h|h.actual.is_some()).count(),
              "pooled_rtt_fields":hosts.iter().filter(|h|h.rtt_samples_us.is_some()).count(),
              "directional_rtt_fields":hosts.iter().filter(|h|h.rtt_samples_down_us.is_some()&&h.rtt_samples_up_us.is_some()).count(),
              "rtt_samples":hosts.iter().map(|h|h.rtt_samples_us.as_ref().map_or(0,Vec::len)).sum::<usize>(),
              "pooled_retransmit_fields":hosts.iter().filter(|h|h.tcp_retransmits.is_some()).count(),
              "directional_retransmit_fields":hosts.iter().filter(|h|h.tcp_retransmits_down.is_some()&&h.tcp_retransmits_up.is_some()).count()
            })
        );
        previous = Some(snapshot.captured_at_millis);
        if i < 2 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    Ok(())
}
