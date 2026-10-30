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

pub fn validate_assigned(
    settings: &Settings,
    subs: &[Subscriber],
    state: &crate::assignment::AssignmentState,
) -> Result<()> {
    let Some(config) = &settings.dynamic else {
        return Ok(());
    };
    crate::dynamic::validate_routes(state)?;
    let wans: HashMap<_, _> = config.wans.iter().map(|w| (w.id.as_str(), w)).collect();
    let policies: HashMap<_, _> = config.policies.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut sums = HashMap::<&str, (f64, f64)>::new();
    for sub in subs {
        let route = state
            .routes
            .get(&sub.id)
            .ok_or_else(|| anyhow::anyhow!("Subscriber routing assignment is missing"))?;
        ensure!(
            route.node == sub.node,
            "Subscriber route does not match exported parent"
        );
        let p = policies
            .get(route.policy.as_str())
            .ok_or_else(|| anyhow::anyhow!("Assignment policy no longer exists"))?;
        ensure!(
            p.selections.contains(&route.selection)
                && p.targets.contains(&crate::dynamic::Target {
                    node: route.node.clone(),
                    wan: route.wan.clone()
                }),
            "Subscriber route is outside its selected policy"
        );
        ensure!(
            wans.contains_key(route.wan.as_str()),
            "Subscriber routing WAN is unavailable"
        );
        let total = sums.entry(&route.wan).or_default();
        total.0 += sub.download_min;
        total.1 += sub.upload_min;
    }
    for (id, (download, upload)) in sums {
        let w = wans[id];
        ensure!(
            download.is_finite()
                && upload.is_finite()
                && download <= w.download + tolerance(w.download)
                && upload <= w.upload + tolerance(w.upload),
            "Aggregate subscriber minimums exceed routing WAN capacity at {id}"
        );
    }
    Ok(())
}
pub fn allocate_assigned(
    settings: &Settings,
    subs: &mut [Subscriber],
    state: &crate::assignment::AssignmentState,
) -> Result<Allocation> {
    let Some(config) = &settings.dynamic else {
        return allocate(settings, subs);
    };
    crate::dynamic::validate_routes(state)?;
    let mut planned = subs.to_vec();
    let mut result = allocate(settings, &mut planned)?;
    let mut members = HashMap::<&str, Vec<usize>>::new();
    for (i, sub) in planned.iter().enumerate() {
        let route = state
            .routes
            .get(&sub.id)
            .ok_or_else(|| anyhow::anyhow!("Subscriber routing assignment is missing"))?;
        members.entry(&route.wan).or_default().push(i);
    }
    for w in &config.wans {
        let Some(indices) = members.get(w.id.as_str()) else {
            continue;
        };
        for (d, capacity) in [w.download, w.upload].into_iter().enumerate() {
            let value = |s: &Subscriber| if d == 0 { s.download_min } else { s.upload_min };
            let floor = |s: &Subscriber| {
                settings
                    .speed_rules
                    .min_floor_mbps
                    .min(if d == 0 { s.download } else { s.upload })
            };
            let floors: f64 = indices.iter().map(|i| floor(&planned[*i])).sum();
            ensure!(
                floors.is_finite() && floors <= capacity + tolerance(capacity),
                "Subscriber minimum floor cannot fit routing WAN capacity at {}",
                w.id
            );
            let total: f64 = indices.iter().map(|i| value(&planned[*i])).sum();
            ensure!(total.is_finite(), "Routing WAN minimum total overflow");
            if total > capacity + tolerance(capacity) {
                let headroom: f64 = indices
                    .iter()
                    .map(|i| value(&planned[*i]) - floor(&planned[*i]))
                    .sum();
                ensure!(
                    headroom.is_finite() && headroom > 0.,
                    "Routing WAN minimum floor cannot fit"
                );
                let factor = ((capacity - floors).max(0.) / headroom).min(1.) * (1. - 1e-12);
                for &i in indices {
                    let s = &mut planned[i];
                    let v = floor(s) + (value(s) - floor(s)) * factor;
                    if d == 0 {
                        s.download_min = v
                    } else {
                        s.upload_min = v
                    };
                }
                result.constrained_directions += 1;
            }
        }
    }
    validate_assigned(settings, &planned, state)?;
    model::compile(settings, &planned)?;
    result.adjusted_subscribers = subs
        .iter()
        .zip(&planned)
        .filter(|(a, b)| a.download_min != b.download_min || a.upload_min != b.upload_min)
        .count();
    subs.clone_from_slice(&planned);
    Ok(result)
}
