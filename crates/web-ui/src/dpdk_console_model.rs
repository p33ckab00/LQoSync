//! Values, tones, and detail text for the DPDK health band of the worker ring. Pure functions, so native tests cover them.
use crate::worker_ring_model::{compact, grouped};
use web_contract::dpdk::Console;

/// A console topic. Hover or focus on any instrument of a topic highlights the whole topic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Topic {
    Buffer,
    Rings,
    Queue,
    Rollup,
    Bridge,
}
/// The color role of a value. `Neutral` is unknown or not applicable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    Good,
    Warn,
    Info,
    Violet,
    Neutral,
}
impl Tone {
    pub(crate) fn class(self) -> &'static str {
        match self {
            Tone::Good => "tone-good",
            Tone::Warn => "tone-warn",
            Tone::Info => "tone-info",
            Tone::Violet => "tone-violet",
            Tone::Neutral => "tone-neutral",
        }
    }
}

pub(crate) fn count(n: Option<u64>) -> String {
    n.map(grouped).unwrap_or("—".into())
}
/// Free share of the buffer pool, 0 to 100. Unknown without a positive total.
pub(crate) fn pool_free_pct(c: &Console) -> Option<f64> {
    c.pool_total
        .zip(c.pool_free)
        .filter(|(t, f)| *t > 0 && f <= t)
        .map(|(t, f)| f as f64 * 100.0 / t as f64)
}
/// Occupancy of the handoff rings. Unknown when there is no capacity: N/A is not 0 %.
pub(crate) fn ring_pct(c: &Console) -> Option<f64> {
    c.ring_capacity
        .zip(c.ring_used)
        .filter(|(cap, used)| *cap > 0 && used <= cap)
        .map(|(cap, used)| used as f64 * 100.0 / cap as f64)
}
/// A duration from milliseconds: `19h 22m`, `3d 4h`, or `42s`.
pub(crate) fn uptime(ms: Option<u64>) -> String {
    let Some(ms) = ms else {
        return "—".into();
    };
    let s = ms / 1000;
    let (d, h, m) = (s / 86_400, s / 3600 % 24, s / 60 % 60);
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {}s", s % 60)
    } else {
        format!("{s}s")
    }
}
pub(crate) fn per_sample_tone(n: Option<u64>) -> Tone {
    match n {
        Some(0) => Tone::Good,
        Some(_) => Tone::Warn,
        None => Tone::Neutral,
    }
}
pub(crate) fn pressure_tone(c: &Console) -> Tone {
    match c.pressure.as_deref() {
        Some("CLEAR") => Tone::Good,
        Some("ALERT") => Tone::Warn,
        _ => Tone::Neutral,
    }
}
pub(crate) fn pool_tone(c: &Console) -> Tone {
    match pool_free_pct(c) {
        Some(p) if p > 10.0 => Tone::Good,
        Some(_) => Tone::Warn,
        None => Tone::Neutral,
    }
}
pub(crate) fn status_tone(c: &Console) -> Tone {
    match c.status.as_deref().map(str::to_ascii_lowercase).as_deref() {
        Some("healthy" | "ok") => Tone::Good,
        Some(_) => Tone::Warn,
        None => Tone::Neutral,
    }
}
/// One band of the outer health ring.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Segment {
    /// Short text on the band.
    pub label: &'static str,
    /// Full name for the chip, the focus text, and assistive technology.
    pub name: &'static str,
    pub value: String,
    pub tone: Tone,
    pub topic: Topic,
}
fn seg(
    label: &'static str,
    name: &'static str,
    value: String,
    tone: Tone,
    topic: Topic,
) -> Segment {
    Segment {
        label,
        name,
        value,
        tone,
        topic,
    }
}
/// The nine health bands, clockwise from the top: bridge health first, then analytics and classification.
pub(crate) fn segments(c: &Console) -> [Segment; 9] {
    let rollup_tone = match (c.missing_rollups, c.oldest_rollup_ms) {
        (Some(m), _) if m > 0 => Tone::Warn,
        (_, Some(_)) => Tone::Violet,
        _ => Tone::Neutral,
    };
    let rings_tone = match ring_pct(c) {
        Some(p) if p >= 75.0 => Tone::Warn,
        Some(_) => Tone::Info,
        None => Tone::Neutral,
    };
    let class_tone = match c.classification_empty {
        Some(false) => Tone::Good,
        Some(true) => Tone::Warn,
        None => Tone::Neutral,
    };
    [
        seg(
            "BRIDGE",
            "Bridge status",
            c.status
                .as_deref()
                .unwrap_or("unknown")
                .to_ascii_lowercase(),
            status_tone(c),
            Topic::Bridge,
        ),
        seg(
            "ROLLUP",
            "Oldest rollup",
            c.oldest_rollup_ms
                .map(|m| format!("{} ms", grouped(m)))
                .unwrap_or("—".into()),
            rollup_tone,
            Topic::Rollup,
        ),
        seg(
            "CLASSIFY",
            "Classification",
            match c.classification_empty {
                Some(false) => "populated",
                Some(true) => "empty",
                None => "—",
            }
            .into(),
            class_tone,
            Topic::Bridge,
        ),
        seg(
            "PRESSURE",
            "Buffer and ring pressure",
            c.pressure.clone().unwrap_or("UNKNOWN".into()),
            pressure_tone(c),
            Topic::Buffer,
        ),
        seg(
            "POOL",
            "Buffer pool free",
            pool_free_pct(c)
                .map(|p| format!("{p:.2}%"))
                .unwrap_or("—".into()),
            pool_tone(c),
            Topic::Buffer,
        ),
        seg(
            "RINGS",
            "Handoff ring occupancy",
            ring_pct(c)
                .map(|p| format!("{p:.1}%"))
                .unwrap_or("N/A".into()),
            rings_tone,
            Topic::Rings,
        ),
        seg(
            "BACKLOG",
            "Queue backlog",
            c.backlog_bytes
                .map(|b| format!("{}B", compact(b)))
                .unwrap_or("—".into()),
            if c.backlog_bytes.is_some() {
                Tone::Info
            } else {
                Tone::Neutral
            },
            Topic::Queue,
        ),
        seg(
            "DROPS",
            "Queue drops per sample",
            count(c.queue_drops_per_sample),
            per_sample_tone(c.queue_drops_per_sample),
            Topic::Queue,
        ),
        seg(
            "AQM",
            "AQM marks per sample",
            count(c.aqm_marks_per_sample),
            per_sample_tone(c.aqm_marks_per_sample),
            Topic::Queue,
        ),
    ]
}
fn age(n: Option<u64>) -> String {
    n.map(|n| format!("{n}s")).unwrap_or("unknown".into())
}
/// Title and three lines for the detail panel of a topic.
pub(crate) fn details(
    topic: Topic,
    c: &Console,
    analytics_age: Option<u64>,
    bridge_age: Option<u64>,
) -> [String; 4] {
    match topic {
        Topic::Buffer => [
            "Buffer pool".into(),
            format!(
                "Free {} · Used {} · Total {}",
                count(c.pool_free),
                count(
                    c.pool_total
                        .zip(c.pool_free)
                        .map(|(t, f)| t.saturating_sub(f))
                ),
                count(c.pool_total)
            ),
            format!(
                "RX exhaustion {}/sample · Pool drops {}/sample",
                count(c.rx_nombuf_per_sample),
                count(c.pool_drops_per_sample)
            ),
            format!(
                "Pressure {} · Analytics age {}",
                c.pressure.as_deref().unwrap_or("UNKNOWN"),
                age(analytics_age)
            ),
        ],
        Topic::Rings => [
            "Handoff rings".into(),
            format!(
                "{} rings · {} above 75 % · {} slots used",
                count(c.ring_count),
                count(c.rings_over_75),
                count(c.ring_used)
            ),
            format!(
                "Capacity {} · Occupancy {}",
                count(c.ring_capacity),
                ring_pct(c)
                    .map(|p| format!("{p:.2} %"))
                    .unwrap_or("N/A (not 0 %)".into())
            ),
            "Occupancy is the total over all rings.".into(),
        ],
        Topic::Queue => [
            "Queue and AQM".into(),
            format!(
                "Backlog {} bytes · Drops {}/sample",
                count(c.backlog_bytes),
                count(c.queue_drops_per_sample)
            ),
            format!(
                "AQM marks {}/sample · Queue drops total {}",
                count(c.aqm_marks_per_sample),
                count(c.queue_drops_total)
            ),
            format!("AQM marks total {}", count(c.aqm_marks_total)),
        ],
        Topic::Rollup => [
            "Rollup freshness".into(),
            format!(
                "Oldest rollup {} ms · Missing {}",
                count(c.oldest_rollup_ms),
                count(c.missing_rollups)
            ),
            format!(
                "Bridge age {} · Analytics age {}",
                age(bridge_age),
                age(analytics_age)
            ),
            "A rollup is a per-worker summary that the bridge publishes.".into(),
        ],
        Topic::Bridge => [
            "Bridge health".into(),
            format!(
                "Status {} · Version {}",
                c.status.as_deref().unwrap_or("unknown"),
                c.version.as_deref().unwrap_or("unknown")
            ),
            format!(
                "Uptime {} · Workers {}",
                uptime(c.uptime_ms),
                count(c.workers)
            ),
            format!(
                "AQM degraded {} · Classification {}{}",
                count(c.aqm_degraded_workers),
                match c.classification_empty {
                    Some(true) => "empty",
                    Some(false) => "populated",
                    None => "unknown",
                },
                if c.degraded.is_empty() {
                    String::new()
                } else {
                    format!(" · Degraded: {}", c.degraded.join(", "))
                }
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn console() -> Console {
        Console {
            pool_free: Some(1_159_923),
            pool_total: Some(1_171_958),
            ring_count: Some(0),
            ring_used: Some(0),
            ring_capacity: Some(0),
            rings_over_75: Some(0),
            rx_nombuf_per_sample: Some(0),
            pool_drops_per_sample: Some(0),
            pressure: Some("CLEAR".into()),
            queue_drops_per_sample: Some(25),
            status: Some("healthy".into()),
            uptime_ms: Some(69_751_882),
            ..Default::default()
        }
    }
    #[test]
    fn shares_are_unknown_without_a_positive_total() {
        let c = console();
        assert!((pool_free_pct(&c).unwrap() - 98.973).abs() < 0.001);
        // Zero ring capacity is N/A, not 0 %.
        assert_eq!(ring_pct(&c), None);
        assert_eq!(pool_free_pct(&Console::default()), None);
    }
    #[test]
    fn tones_keep_zero_good_and_unknown_neutral() {
        let c = console();
        assert_eq!(per_sample_tone(Some(0)), Tone::Good);
        assert_eq!(per_sample_tone(c.queue_drops_per_sample), Tone::Warn);
        assert_eq!(per_sample_tone(None), Tone::Neutral);
        assert_eq!(pressure_tone(&c), Tone::Good);
        assert_eq!(pool_tone(&c), Tone::Good);
        assert_eq!(status_tone(&c), Tone::Good);
        assert_eq!(status_tone(&Console::default()), Tone::Neutral);
    }
    #[test]
    fn every_tone_has_a_class_and_every_topic_a_title() {
        let classes = [
            Tone::Good,
            Tone::Warn,
            Tone::Info,
            Tone::Violet,
            Tone::Neutral,
        ]
        .map(Tone::class);
        assert_eq!(
            classes,
            [
                "tone-good",
                "tone-warn",
                "tone-info",
                "tone-violet",
                "tone-neutral"
            ]
        );
        let titles = [
            Topic::Buffer,
            Topic::Rings,
            Topic::Queue,
            Topic::Rollup,
            Topic::Bridge,
        ]
        .map(|t| details(t, &Console::default(), None, None)[0].clone());
        assert_eq!(
            titles,
            [
                "Buffer pool",
                "Handoff rings",
                "Queue and AQM",
                "Rollup freshness",
                "Bridge health"
            ]
        );
    }
    #[test]
    fn segments_show_the_reference_values_and_keep_unknown_neutral() {
        let mut c = console();
        c.backlog_bytes = Some(221_224);
        c.oldest_rollup_ms = Some(735);
        c.missing_rollups = Some(0);
        c.classification_empty = Some(false);
        let s = segments(&c);
        let values: Vec<_> = s.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(
            values,
            [
                "healthy",
                "735 ms",
                "populated",
                "CLEAR",
                "98.97%",
                "N/A",
                "221KB",
                "25",
                "—"
            ]
        );
        assert_eq!(s[5].tone, Tone::Neutral, "N/A is not 0 %");
        assert_eq!(s[7].tone, Tone::Warn);
        assert_eq!(s[8].tone, Tone::Neutral);
        c.missing_rollups = Some(1);
        assert_eq!(segments(&c)[1].tone, Tone::Warn);
        assert!(
            segments(&Console::default())
                .iter()
                .all(|s| s.tone == Tone::Neutral)
        );
    }
    #[test]
    fn uptime_and_details_read_plainly() {
        assert_eq!(uptime(Some(69_751_882)), "19h 22m");
        assert_eq!(uptime(Some(3 * 86_400_000 + 4 * 3_600_000)), "3d 4h");
        assert_eq!(uptime(Some(42_000)), "42s");
        assert_eq!(uptime(None), "—");
        let d = details(Topic::Rings, &console(), Some(0), Some(7));
        assert_eq!(d[0], "Handoff rings");
        assert!(d[2].contains("N/A (not 0 %)"), "{}", d[2]);
        let d = details(Topic::Buffer, &console(), Some(0), Some(7));
        assert_eq!(d[1], "Free 1,159,923 · Used 12,035 · Total 1,171,958");
    }
}
