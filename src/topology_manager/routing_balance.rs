//! The routing WAN of each subscriber (canvas flows design, section 4). It decides the
//! `via-WAN` list of a subscriber; shaping does not use it. Runtime state, not policy.

use super::catalog::AllocationMode;
use super::rates::RateBps;
use std::collections::BTreeMap;
use std::path::Path;

pub const FILE: &str = "routing-balance.json";

/// Subscriber ID to WAN node ID.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RoutingState {
    pub assigned: BTreeMap<String, String>,
}

impl RoutingState {
    /// The saved state of `dir`. A missing or damaged file gives an empty state.
    pub fn load(dir: &Path) -> RoutingState {
        std::fs::read(dir.join(FILE))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, dir: &Path) -> anyhow::Result<()> {
        crate::files::write_secret(&dir.join(FILE), &serde_json::to_vec(self)?)
    }
}

/// One subscriber that needs a routing WAN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Demand {
    pub subscriber_id: String,
    /// The WAN node IDs above the placement of the subscriber. Empty: no WAN link.
    pub linked: Vec<String>,
    pub mode: AllocationMode,
    /// The base rate, for Capacity and Quality.
    pub weight: RateBps,
}

/// One WAN node that can carry subscribers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WanCandidate {
    pub node_id: String,
    pub capacity: RateBps,
    /// The quality score in percent. `None`: no probe data (treated as 100).
    pub quality: Option<u64>,
    /// The WAN monitor allows subscribers on it.
    pub up: bool,
}

/// Give each subscriber one routing WAN. A subscriber keeps its previous WAN while that WAN is
/// up and allowed for it; the others go to the best WAN by mode, comparing the total load of
/// each WAN. With no WAN up, a subscriber keeps its previous WAN.
pub fn balance(demands: &[Demand], wans: &[WanCandidate], previous: &RoutingState) -> RoutingState {
    let mut order: Vec<&Demand> = demands.iter().collect();
    order.sort_by(|a, b| a.subscriber_id.cmp(&b.subscriber_id));
    let any_up = wans.iter().any(|w| w.up);
    // The WANs a subscriber may use now: its linked WANs that are up, else every WAN that is up.
    let allowed = |d: &Demand| -> Vec<usize> {
        let linked: Vec<usize> = (0..wans.len())
            .filter(|&i| wans[i].up && d.linked.contains(&wans[i].node_id))
            .collect();
        if linked.is_empty() {
            (0..wans.len()).filter(|&i| wans[i].up).collect()
        } else {
            linked
        }
    };
    let mut count = vec![0u64; wans.len()];
    let mut load = vec![0u128; wans.len()];
    let mut out = RoutingState::default();
    let mut pending = Vec::new();
    for d in order {
        let kept = previous.assigned.get(&d.subscriber_id);
        if !any_up {
            if let Some(w) = kept {
                out.assigned.insert(d.subscriber_id.clone(), w.clone());
            }
            continue;
        }
        match kept.and_then(|w| allowed(d).into_iter().find(|&i| &wans[i].node_id == w)) {
            Some(i) => {
                count[i] += 1;
                load[i] += u128::from(d.weight.down);
                out.assigned
                    .insert(d.subscriber_id.clone(), wans[i].node_id.clone());
            }
            None => pending.push(d),
        }
    }
    for d in pending {
        // The score is a ratio; compare a/b < c/d as a*d < c*b. Ties go to the first WAN.
        let score = |i: usize| -> (u128, u128) {
            let effective = u128::from(wans[i].capacity.down.max(1))
                * u128::from(match d.mode {
                    AllocationMode::Quality => wans[i].quality.unwrap_or(100).max(1),
                    _ => 100,
                });
            match d.mode {
                AllocationMode::Count => (u128::from(count[i]), 1),
                _ => (load[i] + u128::from(d.weight.down), effective),
            }
        };
        let Some(best) = allowed(d).into_iter().reduce(|a, b| {
            let ((an, ad), (bn, bd)) = (score(a), score(b));
            if bn * ad < an * bd { b } else { a }
        }) else {
            continue;
        };
        count[best] += 1;
        load[best] += u128::from(d.weight.down);
        out.assigned
            .insert(d.subscriber_id.clone(), wans[best].node_id.clone());
    }
    out
}

/// One placed subscriber: subscriber ID, placement node ID (empty: flat), max download Mbps.
pub type Placed = (String, String, f64);

/// The placed subscribers of a pair.
pub fn placed_of(pair: &super::publication::PreparedPair) -> Vec<Placed> {
    pair.view()
        .subscribers
        .iter()
        .map(|s| (s.id.clone(), s.node.clone(), s.max_mbps.0))
        .collect()
}

/// The placed subscribers of a cached preview (the JSON of `topology.preview.refresh`).
pub fn placed_from_preview(preview: &serde_json::Value) -> Vec<Placed> {
    preview["subscribers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| {
            Some((
                s["id"].as_str()?.to_string(),
                s["node"].as_str().unwrap_or_default().to_string(),
                s["max_mbps"][0].as_f64().unwrap_or(0.0),
            ))
        })
        .collect()
}

/// The demand of each placed subscriber. Linked: the WAN at the root above its placement node,
/// so routing follows the shaping WAN while that WAN is up.
pub fn demands(placed: &[Placed], catalog: &super::catalog::TopologyCatalog) -> Vec<Demand> {
    use super::layout::NodeKind;
    let index: std::collections::HashMap<&str, usize> = catalog
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    placed
        .iter()
        .map(|(id, node, mbps)| {
            let at = index.get(node.as_str()).copied();
            let mut root = at;
            while let Some(parent) = root.and_then(|i| catalog.nodes[i].parent.as_deref()) {
                root = index.get(parent).copied();
            }
            let linked = root
                .filter(|&i| catalog.nodes[i].kind == NodeKind::Wan)
                .map(|i| vec![catalog.nodes[i].id.clone()])
                .unwrap_or_default();
            let source = at
                .and_then(|i| catalog.nodes[i].source_id.as_deref())
                .and_then(|s| catalog.sources.iter().find(|x| x.id == s));
            let bps = (mbps.max(0.0) * 1_000_000.0) as u64;
            Demand {
                subscriber_id: id.clone(),
                linked,
                mode: catalog.mode_of(source),
                weight: RateBps { down: bps, up: bps },
            }
        })
        .collect()
}

/// Every WAN node of the catalog, with its capacity, quality, and WAN monitor state.
pub fn candidates(
    catalog: &super::catalog::TopologyCatalog,
    wan: &super::wan_health::WanSnapshot,
) -> Vec<WanCandidate> {
    use super::catalog::CapacityPolicy;
    use super::layout::NodeKind;
    catalog
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Wan)
        .map(|n| {
            let status = wan.0.get(&n.id);
            WanCandidate {
                node_id: n.id.clone(),
                capacity: match n.capacity {
                    CapacityPolicy::Fixed { base, .. } => base,
                    _ => RateBps { down: 0, up: 0 },
                },
                quality: status
                    .and_then(|s| s.quality)
                    .map(|q| q.numerator * 100 / q.denominator.max(1)),
                up: !status.is_some_and(|s| s.excluded()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wan(id: &str, mbps: u64, quality: Option<u64>, up: bool) -> WanCandidate {
        WanCandidate {
            node_id: id.into(),
            capacity: RateBps {
                down: mbps * 1_000_000,
                up: mbps * 1_000_000,
            },
            quality,
            up,
        }
    }

    fn demand(id: &str, linked: &[&str], mode: AllocationMode) -> Demand {
        Demand {
            subscriber_id: id.into(),
            linked: linked.iter().map(|s| s.to_string()).collect(),
            mode,
            weight: RateBps {
                down: 10_000_000,
                up: 10_000_000,
            },
        }
    }

    fn many(n: usize, mode: AllocationMode) -> Vec<Demand> {
        (0..n)
            .map(|i| demand(&format!("s{i:02}"), &[], mode))
            .collect()
    }

    fn count(s: &RoutingState, wan: &str) -> usize {
        s.assigned.values().filter(|w| *w == wan).count()
    }

    fn prev(pairs: &[(&str, &str)]) -> RoutingState {
        RoutingState {
            assigned: pairs
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
        }
    }

    #[test]
    fn a_subscriber_keeps_its_wan_while_it_is_up() {
        let wans = [wan("w1", 100, None, true), wan("w2", 100, None, true)];
        let d = many(4, AllocationMode::Count);
        let p = prev(&[("s00", "w1"), ("s01", "w1"), ("s02", "w1"), ("s03", "w1")]);
        assert_eq!(balance(&d, &wans, &p), p);
    }

    #[test]
    fn new_subscribers_fill_the_least_loaded_wan_by_count() {
        let wans = [wan("w1", 100, None, true), wan("w2", 100, None, true)];
        let p = prev(&[("s00", "w1"), ("s01", "w1")]);
        let s = balance(&many(6, AllocationMode::Count), &wans, &p);
        assert_eq!((count(&s, "w1"), count(&s, "w2")), (3, 3));
        assert_eq!(s.assigned["s00"], "w1");
    }

    #[test]
    fn capacity_mode_follows_the_wan_capacity_ratio() {
        let wans = [wan("w1", 300, None, true), wan("w2", 100, None, true)];
        let s = balance(
            &many(8, AllocationMode::Capacity),
            &wans,
            &RoutingState::default(),
        );
        assert_eq!((count(&s, "w1"), count(&s, "w2")), (6, 2));
    }

    #[test]
    fn quality_mode_uses_capacity_times_quality() {
        // Equal capacity; w2 has half the quality, so it gets about a third.
        let wans = [
            wan("w1", 100, Some(100), true),
            wan("w2", 100, Some(50), true),
        ];
        let s = balance(
            &many(9, AllocationMode::Quality),
            &wans,
            &RoutingState::default(),
        );
        assert_eq!((count(&s, "w1"), count(&s, "w2")), (6, 3));
    }

    #[test]
    fn linked_subscribers_use_only_their_linked_wans() {
        let wans = [wan("w1", 100, None, true), wan("w2", 100, None, true)];
        let d: Vec<Demand> = (0..4)
            .map(|i| demand(&format!("s{i}"), &["w2"], AllocationMode::Count))
            .collect();
        let s = balance(&d, &wans, &RoutingState::default());
        assert_eq!(count(&s, "w2"), 4);
    }

    #[test]
    fn a_linked_subscriber_moves_to_an_up_wan_when_its_wans_are_down() {
        let wans = [wan("w1", 100, None, false), wan("w2", 100, None, true)];
        let d = [demand("s0", &["w1"], AllocationMode::Count)];
        let s = balance(&d, &wans, &prev(&[("s0", "w1")]));
        assert_eq!(s.assigned["s0"], "w2");
    }

    #[test]
    fn nobody_moves_when_every_wan_is_down() {
        let wans = [wan("w1", 100, None, false), wan("w2", 100, None, false)];
        let d = [
            demand("s0", &[], AllocationMode::Count),
            demand("s1", &[], AllocationMode::Count),
        ];
        let s = balance(&d, &wans, &prev(&[("s0", "w1")]));
        assert_eq!(s, prev(&[("s0", "w1")]));
    }

    #[test]
    fn a_removed_wan_moves_its_subscribers() {
        let wans = [wan("w2", 100, None, true)];
        let d = many(2, AllocationMode::Count);
        let s = balance(&d, &wans, &prev(&[("s00", "gone"), ("s01", "w2")]));
        assert_eq!(s.assigned["s00"], "w2");
        assert!(!s.assigned.values().any(|w| w == "gone"));
    }

    #[test]
    fn a_missing_state_file_assigns_once_in_id_order() {
        let dir = tempfile::tempdir().unwrap();
        let wans = [wan("w1", 100, None, true), wan("w2", 100, None, true)];
        let mut d = many(4, AllocationMode::Count);
        d.reverse();
        let first = balance(&d, &wans, &RoutingState::load(dir.path()));
        assert_eq!(first.assigned["s00"], "w1");
        assert_eq!(first.assigned["s01"], "w2");
        // A second run with the saved state changes nothing.
        first.save(dir.path()).unwrap();
        assert_eq!(balance(&d, &wans, &RoutingState::load(dir.path())), first);
    }

    #[test]
    fn the_state_survives_a_save_and_a_load() {
        let dir = tempfile::tempdir().unwrap();
        let s = prev(&[("a", "w1")]);
        s.save(dir.path()).unwrap();
        assert_eq!(RoutingState::load(dir.path()), s);
        std::fs::write(dir.path().join(FILE), b"not json").unwrap();
        assert_eq!(RoutingState::load(dir.path()), RoutingState::default());
    }
}
