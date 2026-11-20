//! Recent changes observed by this browser, not a backend job or audit log.
use web_contract::Overview;

pub const LABELS: [&str; 5] = [
    "Service health",
    "Runtime mode",
    "Output target",
    "Publication phase",
    "Last cycle",
];
pub const ACTIVITY_SECONDS: u64 = 120;
pub fn values(view: &Overview) -> [String; 5] {
    [
        view.health.clone(),
        view.mode.clone(),
        view.target.clone(),
        view.publication.clone(),
        view.cycle_ms
            .map(|ms| format!("{ms} ms"))
            .unwrap_or("Unknown".into()),
    ]
}
#[derive(Clone)]
pub struct Activity {
    pub id: u64,
    pub at: u64,
    pub text: String,
    pub state: String,
}
struct Snapshot {
    values: [String; 5],
    components: Vec<(String, String)>,
    observed_at: Option<u64>,
    stale: bool,
}
#[derive(Default)]
pub struct Feed {
    previous: Option<Snapshot>,
    pub events: Vec<Activity>,
    next_id: u64,
}
impl Feed {
    pub fn expire(&mut self, now: u64) {
        self.events
            .retain(|e| now.saturating_sub(e.at) < ACTIVITY_SECONDS);
    }
    fn push(&mut self, now: u64, text: String, state: String) {
        self.next_id += 1;
        self.events.push(Activity {
            id: self.next_id,
            at: now,
            text,
            state,
        });
        if self.events.len() > 8 {
            self.events.remove(0);
        }
    }
    pub fn observe(&mut self, view: &Overview, now: u64) {
        self.expire(now);
        // A connection's empty warm-up cache is not an activity baseline.
        if view.observed_at.is_none() {
            return;
        }
        let next = Snapshot {
            values: values(view),
            observed_at: view.observed_at,
            stale: view.stale,
            components: view
                .components
                .iter()
                .map(|c| (c.name.clone(), c.state.clone()))
                .collect(),
        };
        if let Some(old) = self.previous.take() {
            for (i, label) in LABELS.iter().enumerate().take(4) {
                if old.values[i] != next.values[i] {
                    self.push(
                        now,
                        format!("{label}: {} → {}", old.values[i], next.values[i]),
                        next.values[i].clone(),
                    );
                }
            }
            for (index, (name, state)) in next.components.iter().enumerate() {
                if !old
                    .components
                    .get(index)
                    .is_some_and(|(n, s)| n == name && s == state)
                {
                    self.push(
                        now,
                        format!("{}: {state}", name.replace('_', " ")),
                        state.clone(),
                    );
                }
            }
            if old.stale != next.stale {
                self.push(
                    now,
                    format!("Evidence: {}", if next.stale { "stale" } else { "current" }),
                    "UNKNOWN".into(),
                );
            }
            if next.observed_at > old.observed_at {
                self.push(
                    now,
                    format!("Status snapshot updated · Last cycle {}", next.values[4]),
                    String::new(),
                );
            }
        }
        self.previous = Some(next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use web_contract::Component;
    fn overview() -> Overview {
        Overview {
            observed_at: Some(10),
            health: "HEALTHY".into(),
            mode: "apply".into(),
            publication: "IMPORT_CONFIRMED".into(),
            cycle_ms: Some(91),
            ..Default::default()
        }
    }
    #[test]
    fn baseline_and_repeated_or_traffic_only_updates_have_no_activity() {
        let mut feed = Feed::default();
        let mut v = overview();
        feed.observe(&v, 10);
        feed.observe(&v, 11);
        v.traffic.push(Default::default());
        feed.observe(&v, 12);
        assert!(feed.events.is_empty());
        assert_eq!(values(&v)[3], "IMPORT_CONFIRMED");
        assert_eq!(values(&v)[4], "91 ms");
    }
    #[test]
    fn state_changes_appear_once_and_expire_without_source_updates() {
        let mut feed = Feed::default();
        let mut v = overview();
        feed.observe(&v, 10);
        v.components.push(Component {
            name: "collector".into(),
            state: "FAILED".into(),
            observed_at: 11,
        });
        v.publication = "APPLIED".into();
        feed.observe(&v, 11);
        assert_eq!(feed.events.len(), 2);
        assert!(
            feed.events
                .iter()
                .any(|e| e.text == "collector: FAILED" && e.state == "FAILED")
        );
        feed.observe(&v, 12);
        assert_eq!(feed.events.len(), 2);
        feed.expire(131);
        assert!(feed.events.is_empty());
        feed.observe(&v, 132);
        assert!(feed.events.is_empty());
    }
    #[test]
    fn snapshot_activity_does_not_replay_and_the_buffer_is_bounded() {
        let mut feed = Feed::default();
        let mut v = overview();
        feed.observe(&v, 10);
        for at in 11..40 {
            v.observed_at = Some(at);
            feed.observe(&v, at);
        }
        assert_eq!(feed.events.len(), 8);
        let last = feed.events.last().unwrap().id;
        feed.observe(&v, 40);
        assert_eq!(feed.events.last().unwrap().id, last);
    }
}
