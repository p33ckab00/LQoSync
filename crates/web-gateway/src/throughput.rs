//! Live throughput by placement: the dataplane host sample (`subscriber-telemetry.json`) joined
//! by address to the placed subscribers of the cached topology view (`topology.json`). It reads
//! two service caches only: no router, bridge, or service call.
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::net::IpAddr;
use std::path::Path;
use web_contract::topology::{FlowThroughput, NodeThroughput, TopologyThroughput, TopologyView};

pub const TELEMETRY: &str = "subscriber-telemetry.json";
/// The service host sample has at most this many rows (`subscriber_stats::LIMIT`).
const SAMPLE_LIMIT: usize = 512;
/// The service treats an older sample as stale (`subscriber_stats::MAX_AGE`).
const MAX_AGE: u64 = 15;

/// The fields of the service host sample that this view reads.
#[derive(Debug, Default, Deserialize)]
pub struct Telemetry {
    pub version: u32,
    pub updated: u64,
    pub stale: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub hosts: BTreeMap<String, BTreeMap<IpAddr, HostRate>>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct HostRate {
    #[serde(default)]
    pub down_mbps: Option<f64>,
    #[serde(default)]
    pub up_mbps: Option<f64>,
    #[serde(default)]
    pub rtt_ms: Option<f64>,
}

fn bps(mbps: Option<f64>) -> u64 {
    mbps.filter(|v| v.is_finite() && *v >= 0.0)
        .map_or(0, |v| (v * 1_000_000.0).round() as u64)
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let n = v.len();
    Some((v[(n - 1) / 2] + v[n / 2]) / 2.0)
}

#[derive(Default)]
struct Sum {
    down: u64,
    up: u64,
    hosts: u64,
    rtt: Vec<f64>,
}

/// Join one host sample to one topology view. A node sum includes its subtree; a flow is the
/// subtree sum of its child node, keyed like the canvas flows.
pub fn compute(view: &TopologyView, telemetry: Option<&Telemetry>, now: u64) -> TopologyThroughput {
    let Some(t) = telemetry.filter(|t| t.version == 1) else {
        return TopologyThroughput {
            stale: true,
            reason: Some("no dataplane host sample".into()),
            ..Default::default()
        };
    };
    let live = !t.stale && t.updated > 0 && now >= t.updated && now - t.updated <= MAX_AGE;
    let owner: HashMap<IpAddr, &str> = view
        .subscribers
        .iter()
        .filter_map(|s| s.active.as_ref())
        .flat_map(|a| {
            a.ips
                .iter()
                .filter_map(|ip| Some((ip.parse().ok()?, a.node.as_str())))
        })
        .collect();
    let parent: HashMap<&str, &str> = view
        .nodes
        .iter()
        .filter_map(|n| Some((n.id.as_str(), n.parent.as_deref()?)))
        .collect();
    let mut sums: HashMap<&str, Sum> = HashMap::new();
    let (mut unmapped, mut sampled) = (0u64, 0usize);
    for (ip, rate) in t.hosts.values().flat_map(|ips| ips.iter()) {
        sampled += 1;
        let Some(node) = owner.get(ip) else {
            unmapped += 1;
            continue;
        };
        let (down, up) = (bps(rate.down_mbps), bps(rate.up_mbps));
        let mut at = Some(*node);
        // A parent cycle cannot loop: at most one step for each node.
        for _ in 0..=view.nodes.len() {
            let Some(id) = at else { break };
            let s = sums.entry(id).or_default();
            s.down = s.down.saturating_add(down);
            s.up = s.up.saturating_add(up);
            s.hosts += 1;
            s.rtt
                .extend(rate.rtt_ms.filter(|v| v.is_finite() && *v >= 0.0));
            at = parent.get(id).copied();
        }
    }
    // The canvas card of a node; an older cache without one uses the node ID.
    let card: HashMap<&str, &str> = view
        .nodes
        .iter()
        .map(|n| {
            (
                n.id.as_str(),
                if n.card_id.is_empty() {
                    n.id.as_str()
                } else {
                    n.card_id.as_str()
                },
            )
        })
        .collect();
    let mut lines: BTreeMap<(String, String), (u64, u64)> = BTreeMap::new();
    for n in &view.nodes {
        let (Some(from), Some(s)) = (
            n.parent.as_deref().and_then(|p| card.get(p)),
            sums.get(n.id.as_str()),
        ) else {
            continue;
        };
        let line = lines
            .entry((from.to_string(), card[n.id.as_str()].to_string()))
            .or_default();
        line.0 = line.0.saturating_add(s.down);
        line.1 = line.1.saturating_add(s.up);
    }
    let placement_current = view.observed.as_ref().is_some_and(|o| !o.stale);
    let mut nodes: Vec<NodeThroughput> = view
        .nodes
        .iter()
        .filter_map(|n| {
            let s = sums.remove(n.id.as_str())?;
            Some(NodeThroughput {
                node_id: n.id.clone(),
                down_bps: s.down,
                up_bps: s.up,
                hosts: s.hosts,
                rtt_ms: median(s.rtt),
            })
        })
        .collect();
    nodes.sort_by(|a, b| a.node_id.cmp(&b.node_id));
    TopologyThroughput {
        updated: t.updated,
        stale: !live,
        complete: live && placement_current && unmapped == 0 && sampled < SAMPLE_LIMIT,
        reason: if live {
            None
        } else {
            Some(
                t.reason
                    .clone()
                    .unwrap_or_else(|| "the host sample is old".into()),
            )
        },
        nodes,
        flows: lines
            .into_iter()
            .map(|((from, to), (down, up))| FlowThroughput {
                from,
                to,
                wan: None,
                down_bps: down,
                up_bps: up,
            })
            .collect(),
        unmapped_hosts: unmapped,
    }
}

/// The broker answer: `None` when the service has written no topology view yet.
pub fn load(root: &Path, now: u64) -> Option<TopologyThroughput> {
    let view = crate::topology::load(root)?;
    let telemetry: Option<Telemetry> = crate::overview::read(root, TELEMETRY);
    Some(compute(&view, telemetry.as_ref(), now))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn view(stale_placement: bool) -> TopologyView {
        let node = |id: &str, parent: Option<&str>, kind: &str, card: &str| {
            json!({"id": id, "name": id, "parent": parent, "kind": kind, "source_id": null,
                   "capacity": {"mode": "automatic"}, "alias": false, "calculated_mbps": null,
                   "exported_mbps": null, "members": null, "card_id": card})
        };
        let sub = |id: &str, node: &str, ip: &str| {
            json!({"id": id, "identities": [], "plan_id": null, "pin": null, "assignment": null,
                   "active": {"node": node, "ips": [ip], "min_mbps": [1.0, 1.0], "max_mbps": [10.0, 10.0], "fallback": false}})
        };
        serde_json::from_value(json!({
            "version": 1, "updated": 1, "fresh_for": 60, "snapshot_id": "s", "registry_revision": "2",
            "mode": "apply", "writer": "topology", "enrolled": true,
            "observed": {"revision": "2", "collected_at": 1, "generation": 1, "state": "ok", "stale": stale_placement},
            "nodes": [
                node("wan-1", None, "wan", ""),
                node("wan-2", None, "wan", "wan-2"),
                node("ppp-1", Some("wan-1"), "group", "source:ppp"),
                node("ppp-2", Some("wan-2"), "group", "source:ppp"),
            ],
            "sources": [], "plans": [], "observations": [], "collection": [], "conflicts": [],
            "subscribers": [
                sub("s1", "ppp-1", "100.64.0.1"),
                sub("s2", "ppp-1", "100.64.0.2"),
                sub("s3", "ppp-2", "100.64.0.3"),
            ],
        }))
        .unwrap()
    }

    fn sample(now: u64, extra: &[&str]) -> Telemetry {
        let mut hosts = BTreeMap::new();
        for (i, ip) in ["100.64.0.1", "100.64.0.2", "100.64.0.3"]
            .iter()
            .chain(extra)
            .enumerate()
        {
            hosts.insert(
                format!("circuit-{i}"),
                BTreeMap::from([(
                    ip.parse().unwrap(),
                    HostRate {
                        down_mbps: Some(10.0 * (i + 1) as f64),
                        up_mbps: Some(1.5),
                        rtt_ms: Some(2.0 * (i + 1) as f64),
                    },
                )]),
            );
        }
        Telemetry {
            version: 1,
            updated: now,
            stale: false,
            reason: None,
            hosts,
        }
    }

    fn node<'a>(t: &'a TopologyThroughput, id: &str) -> &'a NodeThroughput {
        t.nodes.iter().find(|n| n.node_id == id).unwrap()
    }

    #[test]
    fn rates_roll_up_to_each_node_and_follow_the_canvas_flows() {
        let t = compute(&view(false), Some(&sample(100, &[])), 105);
        assert!(!t.stale && t.complete, "{t:?}");
        assert_eq!(
            (node(&t, "ppp-1").down_bps, node(&t, "ppp-1").hosts),
            (30_000_000, 2)
        );
        assert_eq!(
            node(&t, "wan-1").down_bps,
            30_000_000,
            "a WAN sums its subtree"
        );
        assert_eq!(node(&t, "wan-1").up_bps, 3_000_000);
        assert_eq!(node(&t, "wan-2").down_bps, 30_000_000);
        assert_eq!(node(&t, "ppp-1").rtt_ms, Some(3.0), "median of 2 and 4 ms");
        let flows: Vec<_> = t
            .flows
            .iter()
            .map(|f| (f.from.as_str(), f.to.as_str(), f.down_bps))
            .collect();
        assert_eq!(
            flows,
            [
                ("wan-1", "source:ppp", 30_000_000),
                ("wan-2", "source:ppp", 30_000_000)
            ]
        );
    }

    #[test]
    fn a_missing_part_is_never_complete() {
        // An old sample is stale; its values stay, labeled.
        let old = compute(&view(false), Some(&sample(100, &[])), 200);
        assert!(old.stale && !old.complete && old.reason.is_some());
        // A host that no placed subscriber owns is counted, in no node.
        let extra = compute(&view(false), Some(&sample(100, &["192.0.2.9"])), 101);
        assert_eq!(extra.unmapped_hosts, 1);
        assert!(!extra.complete);
        assert_eq!(node(&extra, "wan-1").down_bps, 30_000_000);
        // A stale placement makes the join uncertain.
        assert!(!compute(&view(true), Some(&sample(100, &[])), 101).complete);
        // No sample at all.
        let none = compute(&view(false), None, 101);
        assert!(none.stale && !none.complete && none.nodes.is_empty());
        // A sample at its row limit can be cut, even when every host is placed.
        let ips: Vec<String> = (0..SAMPLE_LIMIT - 3)
            .map(|i| format!("10.0.{}.{}", i / 250, i % 250))
            .collect();
        let mut placed = view(false);
        for (i, ip) in ips.iter().enumerate() {
            placed.subscribers.push(serde_json::from_value(json!({"id": format!("x{i}"), "identities": [],
                "plan_id": null, "pin": null, "assignment": null, "active": {"node": "ppp-2", "ips": [ip],
                "min_mbps": [1.0, 1.0], "max_mbps": [1.0, 1.0], "fallback": false}})).unwrap());
        }
        let refs: Vec<&str> = ips.iter().map(String::as_str).collect();
        let full = compute(&placed, Some(&sample(100, &refs)), 101);
        assert_eq!(full.unmapped_hosts, 0);
        assert!(!full.complete, "512 sampled hosts can be a cut sample");
        let mut fewer = sample(100, &refs);
        fewer.hosts.pop_last();
        assert!(compute(&placed, Some(&fewer), 101).complete);
    }

    #[test]
    fn bad_values_count_as_zero_and_never_as_a_rate() {
        let mut t = sample(100, &[]);
        for ips in t.hosts.values_mut() {
            for r in ips.values_mut() {
                r.down_mbps = Some(f64::NAN);
                r.up_mbps = Some(-1.0);
                r.rtt_ms = Some(f64::INFINITY);
            }
        }
        let out = compute(&view(false), Some(&t), 101);
        assert_eq!(
            (node(&out, "wan-1").down_bps, node(&out, "wan-1").up_bps),
            (0, 0)
        );
        assert_eq!(node(&out, "wan-1").rtt_ms, None);
    }
}
