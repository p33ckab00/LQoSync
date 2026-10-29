use crate::model::{self, Plan, Settings, Subscriber};
use anyhow::{Result, ensure};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Default, Serialize)]
pub struct Allocation {
    pub adjusted_subscribers: usize,
    pub constrained_directions: usize,
}

// DFS ranges keep each subscriber once while each ancestor can inspect its contiguous subtree.
struct Ranges<'a> {
    children: BTreeMap<&'a str, Vec<&'a str>>,
    direct: HashMap<&'a str, Vec<usize>>,
    order: Vec<usize>,
    ranges: Vec<(&'a str, usize, usize)>,
}
impl<'a> Ranges<'a> {
    fn new(settings: &'a Settings, subs: &'a [Subscriber]) -> Self {
        let mut result = Self {
            children: BTreeMap::new(),
            direct: HashMap::new(),
            order: vec![],
            ranges: vec![],
        };
        for n in &settings.nodes {
            result.children.entry(&n.parent).or_default().push(&n.id);
        }
        for list in result.children.values_mut() {
            list.sort();
        }
        for (n, s) in subs.iter().enumerate() {
            result.direct.entry(&s.node).or_default().push(n);
        }
        for list in result.direct.values_mut() {
            list.sort_by(|a, b| subs[*a].id.cmp(&subs[*b].id));
        }
        result.visit("");
        result
    }
    fn visit(&mut self, parent: &'a str) {
        for node in self.children.get(parent).cloned().unwrap_or_default() {
            let slot = self.ranges.len();
            let start = self.order.len();
            self.ranges.push((node, start, start));
            self.order
                .extend(self.direct.get(node).into_iter().flatten().copied());
            self.visit(node);
            self.ranges[slot].2 = self.order.len();
        }
    }
}
fn tolerance(capacity: f64) -> f64 {
    capacity * 1e-10
}

pub(crate) fn validate(settings: &Settings, subs: &[Subscriber], plan: &Plan) -> Result<()> {
    let ranges = Ranges::new(settings, subs);
    for (id, start, end) in &ranges.ranges {
        let Some(cap) = plan.capacities.get(*id) else {
            continue;
        };
        let download: f64 = ranges.order[*start..*end]
            .iter()
            .map(|n| subs[*n].download_min)
            .sum();
        let upload: f64 = ranges.order[*start..*end]
            .iter()
            .map(|n| subs[*n].upload_min)
            .sum();
        ensure!(
            download.is_finite()
                && upload.is_finite()
                && download <= cap.download + tolerance(cap.download)
                && upload <= cap.upload + tolerance(cap.upload),
            "Aggregate subscriber minimums exceed shaping capacity at {id}"
        );
    }
    Ok(())
}

pub fn allocate(settings: &Settings, subscribers: &mut [Subscriber]) -> Result<Allocation> {
    let Plan { capacities, .. } = model::compile_unallocated(settings, subscribers)?;
    let ranges = Ranges::new(settings, subscribers);
    let floors: Vec<[f64; 2]> = subscribers
        .iter()
        .map(|s| {
            [
                settings.speed_rules.min_floor_mbps.min(s.download),
                settings.speed_rules.min_floor_mbps.min(s.upload),
            ]
        })
        .collect();
    let mut values: Vec<[f64; 2]> = subscribers
        .iter()
        .zip(&floors)
        .map(|(s, f)| [s.download_min.max(f[0]), s.upload_min.max(f[1])])
        .collect();
    let mut result = Allocation::default();
    for (id, start, end) in &ranges.ranges {
        let Some(cap) = capacities.get(*id) else {
            continue;
        };
        let members = &ranges.order[*start..*end];
        for (direction, capacity) in [cap.download, cap.upload].into_iter().enumerate() {
            let floor: f64 = members.iter().map(|n| floors[*n][direction]).sum();
            ensure!(
                floor.is_finite() && floor <= capacity + tolerance(capacity),
                "Subscriber minimum floor cannot fit shaping capacity at {id}"
            );
            let total: f64 = members.iter().map(|n| values[*n][direction]).sum();
            ensure!(
                total.is_finite(),
                "Aggregate subscriber minimums overflow at {id}"
            );
            if total > capacity + tolerance(capacity) {
                let headroom: f64 = members
                    .iter()
                    .map(|n| values[*n][direction] - floors[*n][direction])
                    .sum();
                ensure!(
                    headroom.is_finite() && headroom > 0.,
                    "Subscriber minimum floor cannot fit shaping capacity at {id}"
                );
                let factor = ((capacity - floor).max(0.) / headroom).min(1.) * (1. - 1e-12);
                for &n in members {
                    values[n][direction] = floors[n][direction]
                        + (values[n][direction] - floors[n][direction]) * factor;
                }
                result.constrained_directions += 1;
            }
        }
    }
    // Validate both directions before changing any caller-owned record.
    for (id, start, end) in &ranges.ranges {
        let Some(cap) = capacities.get(*id) else {
            continue;
        };
        for (d, c) in [cap.download, cap.upload].into_iter().enumerate() {
            let sum: f64 = ranges.order[*start..*end]
                .iter()
                .map(|n| values[*n][d])
                .sum();
            ensure!(
                sum.is_finite() && sum <= c + tolerance(c),
                "Allocated subscriber minimums exceed capacity at {id}"
            );
        }
    }
    for (s, v) in subscribers.iter_mut().zip(values) {
        if s.download_min != v[0] || s.upload_min != v[1] {
            result.adjusted_subscribers += 1;
        }
        s.download_min = v[0];
        s.upload_min = v[1];
    }
    Ok(result)
}
