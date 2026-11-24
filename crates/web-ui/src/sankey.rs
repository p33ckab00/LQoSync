//! Sankey geometry for the Topology canvas. Pure Rust: no Leptos, no browser.
//! Columns come from the graph (longest path), card height follows the card value, and each WAN
//! is a separate stream that stacks on the card edges.
// Task 5 of the Sankey plan uses this module and removes this line.
#![cfg_attr(not(test), allow(dead_code))]

use web_contract::topology::{TopologySummary, TopologyThroughput};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    LiveMbps,
    Subscribers,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub from: String,
    pub to: String,
    /// The WAN node ID of the stream. `None`: "WAN unknown".
    pub wan: Option<String>,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Input {
    pub cards: Vec<String>,
    pub segments: Vec<Segment>,
    /// The WAN node IDs in stream order.
    pub wans: Vec<String>,
    pub metric: Metric,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardBox {
    pub id: String,
    pub column: usize,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ribbon {
    pub from: String,
    pub to: String,
    pub wan: Option<String>,
    pub value: f64,
    pub d: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sankey {
    pub cards: Vec<CardBox>,
    pub ribbons: Vec<Ribbon>,
    pub columns: usize,
    pub cycle: bool,
    pub bounds: [f64; 4],
}

pub const CARD_W: f64 = 232.0;
pub const COL_GAP: f64 = 168.0;
pub const GAP: f64 = 14.0;
pub const MIN_H: f64 = 72.0;
/// The band height of the column with the largest total value.
pub const MAX_BAND: f64 = 720.0;

/// The card of a node: the instances of one source, or the copies of one router, share one card.
fn card_of(n: &web_contract::topology::Node) -> &str {
    if n.card_id.is_empty() {
        &n.id
    } else {
        &n.card_id
    }
}

/// Live flows when `t` is fresh, else subscriber flows with `wan: None`. Adds the service tier
/// cards `tier:ppp`, `tier:dhcp`, `tier:hotspot` (from `Source.access` of each group node).
pub fn input(summary: &TopologySummary, t: Option<&TopologyThroughput>) -> Input {
    let mut cards: Vec<String> = Vec::new();
    for n in &summary.nodes {
        if !cards.iter().any(|c| c == card_of(n)) {
            cards.push(card_of(n).to_string());
        }
    }
    let wans: Vec<String> = summary
        .nodes
        .iter()
        .filter(|n| n.kind == "wan")
        .map(|n| n.id.clone())
        .collect();
    let (metric, mut segments) = match t.filter(|t| !t.stale) {
        Some(t) => (
            Metric::LiveMbps,
            t.flows
                .iter()
                .map(|f| Segment {
                    from: f.from.clone(),
                    to: f.to.clone(),
                    wan: f.wan.clone(),
                    value: f.down_bps as f64 / 1_000_000.0,
                })
                .collect::<Vec<_>>(),
        ),
        None => (
            Metric::Subscribers,
            summary
                .flows
                .iter()
                .map(|f| Segment {
                    from: f.from.clone(),
                    to: f.to.clone(),
                    wan: None,
                    value: f.subscribers as f64,
                })
                .collect(),
        ),
    };
    // Each group card feeds the tier of its access type with everything that enters it.
    let mut tiers: Vec<(String, String)> = Vec::new();
    for n in summary.nodes.iter().filter(|n| n.kind == "group") {
        let access = n
            .source_id
            .as_deref()
            .and_then(|id| summary.sources.iter().find(|s| s.id == id))
            .and_then(|s| s.access.as_deref());
        if let Some(access) = access.filter(|a| matches!(*a, "ppp" | "dhcp" | "hotspot")) {
            let pair = (card_of(n).to_string(), format!("tier:{access}"));
            if !tiers.contains(&pair) {
                tiers.push(pair);
            }
        }
    }
    let mut extra: Vec<Segment> = Vec::new();
    for (group, tier) in &tiers {
        for s in segments.iter().filter(|s| &s.to == group) {
            match extra
                .iter_mut()
                .find(|e| &e.from == group && e.wan == s.wan)
            {
                Some(e) => e.value += s.value,
                None => extra.push(Segment {
                    from: group.clone(),
                    to: tier.clone(),
                    wan: s.wan.clone(),
                    value: s.value,
                }),
            }
        }
    }
    for tier in ["tier:ppp", "tier:dhcp", "tier:hotspot"] {
        if extra.iter().any(|e| e.to == tier) {
            cards.push(tier.to_string());
        }
    }
    segments.extend(extra);
    Input {
        cards,
        segments,
        wans,
        metric,
    }
}

/// `order`: the card order of the last layout, for stable columns.
pub fn layout(input: &Input, order: &[String]) -> Sankey {
    use std::collections::{BTreeMap, HashMap};
    let n = input.cards.len();
    let index: HashMap<&str, usize> = input
        .cards
        .iter()
        .enumerate()
        .map(|(i, c)| (c.as_str(), i))
        .collect();
    let edge = |s: &Segment| Some((*index.get(s.from.as_str())?, *index.get(s.to.as_str())?));
    let mut succ: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (a, b) in input.segments.iter().filter_map(edge) {
        if a != b && !succ[a].contains(&b) {
            succ[a].push(b);
        }
    }
    // Remove back edges (depth-first, in card order). 0 = new, 1 = on the stack, 2 = done.
    let mut state = vec![0u8; n];
    let mut back: Vec<(usize, usize)> = Vec::new();
    for root in 0..n {
        if state[root] != 0 {
            continue;
        }
        let mut stack = vec![(root, 0usize)];
        state[root] = 1;
        while let Some((v, i)) = stack.pop() {
            if let Some(&w) = succ[v].get(i) {
                stack.push((v, i + 1));
                match state[w] {
                    0 => {
                        state[w] = 1;
                        stack.push((w, 0));
                    }
                    1 => back.push((v, w)),
                    _ => {}
                }
            } else {
                state[v] = 2;
            }
        }
    }
    let cycle = !back.is_empty();
    let forward = |a: usize, b: usize| a != b && !back.contains(&(a, b));
    // Longest path: relax the acyclic edges at most n times.
    let mut column = vec![0usize; n];
    for _ in 0..n {
        let mut changed = false;
        for a in 0..n {
            for &b in &succ[a] {
                if forward(a, b) && column[b] < column[a] + 1 {
                    column[b] = column[a] + 1;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let segments: Vec<(usize, usize, &Segment)> = input
        .segments
        .iter()
        .filter_map(|s| edge(s).map(|(a, b)| (a, b, s)))
        .filter(|(a, b, _)| forward(*a, *b))
        .collect();
    let (mut inflow, mut outflow) = (vec![0.0f64; n], vec![0.0f64; n]);
    for (a, b, s) in &segments {
        outflow[*a] += s.value.max(0.0);
        inflow[*b] += s.value.max(0.0);
    }
    let value: Vec<f64> = (0..n).map(|i| inflow[i].max(outflow[i])).collect();
    let columns = column.iter().max().map_or(0, |m| m + 1);
    let mut totals = vec![0.0f64; columns];
    for i in 0..n {
        totals[column[i]] += value[i];
    }
    let top = totals.iter().copied().fold(0.0, f64::max);
    let scale = if top > 0.0 { MAX_BAND / top } else { 0.0 };
    let height: Vec<f64> = value.iter().map(|v| (v * scale).max(MIN_H)).collect();
    // Order and stack each column, left to right.
    let rank: HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(i, c)| (c.as_str(), i))
        .collect();
    let mut y = vec![0.0f64; n];
    for c in 0..columns {
        let mut members: Vec<(usize, f64)> = (0..n)
            .filter(|i| column[*i] == c)
            .map(|i| {
                let (mut sum, mut weight) = (0.0, 0.0);
                for (a, b, s) in &segments {
                    if *b == i && column[*a] < c {
                        let w = s.value.max(0.0) + 1e-9;
                        sum += (y[*a] + height[*a] / 2.0) * w;
                        weight += w;
                    }
                }
                (i, if weight > 0.0 { sum / weight } else { 0.0 })
            })
            .collect();
        members.sort_by(|(a, ya), (b, yb)| {
            let ra = rank
                .get(input.cards[*a].as_str())
                .copied()
                .unwrap_or(usize::MAX);
            let rb = rank
                .get(input.cards[*b].as_str())
                .copied()
                .unwrap_or(usize::MAX);
            ra.cmp(&rb)
                .then(ya.total_cmp(yb))
                .then(input.cards[*a].cmp(&input.cards[*b]))
        });
        let mut at = 0.0;
        for (i, _) in members {
            y[i] = at;
            at += height[i] + GAP;
        }
    }
    let x = |i: usize| column[i] as f64 * (CARD_W + COL_GAP);
    // Stack the stream ends on each card edge: WAN order (None last), then the far card height.
    let wan_rank = |w: &Option<String>| match w {
        Some(w) => input
            .wans
            .iter()
            .position(|x| x == w)
            .unwrap_or(input.wans.len()),
        None => usize::MAX,
    };
    let mut out_order: Vec<usize> = (0..segments.len()).collect();
    out_order.sort_by(|p, q| {
        let (a, b, s) = segments[*p];
        let (a2, b2, s2) = segments[*q];
        a.cmp(&a2)
            .then(wan_rank(&s.wan).cmp(&wan_rank(&s2.wan)))
            .then(y[b].total_cmp(&y[b2]))
    });
    let mut in_order: Vec<usize> = (0..segments.len()).collect();
    in_order.sort_by(|p, q| {
        let (a, b, s) = segments[*p];
        let (a2, b2, s2) = segments[*q];
        b.cmp(&b2)
            .then(wan_rank(&s.wan).cmp(&wan_rank(&s2.wan)))
            .then(y[a].total_cmp(&y[a2]))
    });
    let band = |i: usize| segments[i].2.value.max(0.0) * scale;
    let mut start = vec![0.0f64; segments.len()];
    let mut end = vec![0.0f64; segments.len()];
    let mut cursor: BTreeMap<usize, f64> = BTreeMap::new();
    for &i in &out_order {
        let a = segments[i].0;
        let c = cursor
            .entry(a)
            .or_insert(y[a] + (height[a] - outflow[a] * scale) / 2.0);
        start[i] = *c;
        *c += band(i);
    }
    cursor.clear();
    for &i in &in_order {
        let b = segments[i].1;
        let c = cursor
            .entry(b)
            .or_insert(y[b] + (height[b] - inflow[b] * scale) / 2.0);
        end[i] = *c;
        *c += band(i);
    }
    let mut ribbons: Vec<Ribbon> = segments
        .iter()
        .enumerate()
        .map(|(i, (a, b, s))| {
            let (x0, x1) = (x(*a) + CARD_W, x(*b));
            let mid = (x0 + x1) / 2.0;
            let (y0a, y1a, h) = (start[i], end[i], band(i));
            let (y0b, y1b) = (y0a + h, y1a + h);
            Ribbon {
                from: s.from.clone(),
                to: s.to.clone(),
                wan: s.wan.clone(),
                value: s.value,
                d: format!(
                    "M{x0},{y0a} C{mid},{y0a} {mid},{y1a} {x1},{y1a} L{x1},{y1b} C{mid},{y1b} {mid},{y0b} {x0},{y0b} Z"
                ),
            }
        })
        .collect();
    ribbons.sort_by_key(|r| wan_rank(&r.wan));
    let cards: Vec<CardBox> = (0..n)
        .map(|i| CardBox {
            id: input.cards[i].clone(),
            column: column[i],
            x: x(i),
            y: y[i],
            w: CARD_W,
            h: height[i],
            value: value[i],
        })
        .collect();
    let bounds = cards.iter().fold([0.0f64; 4], |b, c| {
        [b[0], b[1], b[2].max(c.x + c.w), b[3].max(c.y + c.h)]
    });
    Sankey {
        cards,
        ribbons,
        columns,
        cycle,
        bounds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use web_contract::topology::{Flow, FlowThroughput, Node, Source};

    fn seg(from: &str, to: &str, wan: Option<&str>, value: f64) -> Segment {
        Segment {
            from: from.into(),
            to: to.into(),
            wan: wan.map(str::to_string),
            value,
        }
    }

    fn live(cards: &[&str], segments: Vec<Segment>, wans: &[&str]) -> Input {
        Input {
            cards: cards.iter().map(|c| c.to_string()).collect(),
            segments,
            wans: wans.iter().map(|w| w.to_string()).collect(),
            metric: Metric::LiveMbps,
        }
    }

    fn card<'a>(s: &'a Sankey, id: &str) -> &'a CardBox {
        s.cards.iter().find(|c| c.id == id).unwrap()
    }

    /// The card IDs of one column, top to bottom.
    fn column(s: &Sankey, n: usize) -> Vec<String> {
        let mut c: Vec<&CardBox> = s.cards.iter().filter(|c| c.column == n).collect();
        c.sort_by(|a, b| a.y.total_cmp(&b.y));
        c.into_iter().map(|c| c.id.clone()).collect()
    }

    /// The `(top, bottom)` of each ribbon end on one card edge: `right` for outgoing ribbons.
    fn ends(s: &Sankey, id: &str, right: bool) -> Vec<(f64, f64, Option<String>)> {
        let numbers = |d: &str| -> Vec<f64> {
            d.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
                .filter_map(|t| t.parse().ok())
                .collect()
        };
        let mut out: Vec<(f64, f64, Option<String>)> = s
            .ribbons
            .iter()
            .filter(|r| if right { r.from == id } else { r.to == id })
            .map(|r| {
                // M x0 y0a C .. .. .. .. x1 y1a L x1 y1b C .. .. .. .. x0 y0b Z
                let n = numbers(&r.d);
                if right {
                    (n[1], n[15], r.wan.clone())
                } else {
                    (n[7], n[9], r.wan.clone())
                }
            })
            .collect();
        out.sort_by(|a, b| a.0.total_cmp(&b.0));
        out
    }

    fn chain() -> Input {
        live(
            &["core", "wan-1", "lqos", "distro", "grp", "grp-b"],
            vec![
                seg("core", "wan-1", Some("wan-1"), 30.0),
                seg("wan-1", "lqos", Some("wan-1"), 30.0),
                seg("lqos", "distro", Some("wan-1"), 30.0),
                seg("distro", "grp", Some("wan-1"), 20.0),
                seg("distro", "grp-b", Some("wan-1"), 0.0),
                seg("wan-1", "grp-b", Some("wan-1"), 0.0),
            ],
            &["wan-1"],
        )
    }

    #[test]
    fn columns_follow_the_longest_path() {
        let s = layout(&chain(), &[]);
        let cols: Vec<usize> = ["core", "wan-1", "lqos", "distro", "grp", "grp-b"]
            .iter()
            .map(|id| card(&s, id).column)
            .collect();
        assert_eq!(cols, [0, 1, 2, 3, 4, 4], "grp-b takes its longest path");
        assert_eq!(s.columns, 5);
        assert!(!s.cycle);
        assert_eq!(card(&s, "lqos").x, 2.0 * (CARD_W + COL_GAP));
    }

    #[test]
    fn bands_fill_card_edges_without_overlap() {
        let input = live(
            &["wan-1", "wan-2", "distro", "a", "b"],
            vec![
                seg("wan-1", "distro", Some("wan-1"), 40.0),
                seg("wan-2", "distro", Some("wan-2"), 20.0),
                seg("distro", "a", Some("wan-1"), 30.0),
                seg("distro", "a", Some("wan-2"), 15.0),
                seg("distro", "b", Some("wan-1"), 10.0),
                seg("distro", "b", Some("wan-2"), 5.0),
            ],
            &["wan-1", "wan-2"],
        );
        let s = layout(&input, &[]);
        let d = card(&s, "distro");
        assert_eq!(d.value, 60.0);
        let scale = MAX_BAND / 60.0;
        for right in [false, true] {
            let e = ends(&s, "distro", right);
            let total: f64 = e.iter().map(|(t, b, _)| b - t).sum();
            assert!((total - 60.0 * scale).abs() < 1e-6, "{right}: {e:?}");
            for w in e.windows(2) {
                assert!(w[0].1 <= w[1].0 + 1e-9, "overlap {w:?}");
            }
            assert!(e[0].0 >= d.y - 1e-9 && e.last().unwrap().1 <= d.y + d.h + 1e-9);
        }
    }

    #[test]
    fn streams_stack_in_wan_order_with_none_last() {
        let input = live(
            &["src", "dst"],
            vec![
                seg("src", "dst", None, 5.0),
                seg("src", "dst", Some("wan-2"), 5.0),
                seg("src", "dst", Some("wan-1"), 5.0),
            ],
            &["wan-1", "wan-2"],
        );
        let s = layout(&input, &[]);
        let wans: Vec<Option<String>> = ends(&s, "src", true).into_iter().map(|e| e.2).collect();
        assert_eq!(wans, [Some("wan-1".into()), Some("wan-2".into()), None]);
    }

    #[test]
    fn a_cycle_stops_and_sets_the_flag() {
        let input = live(
            &["a", "b"],
            vec![seg("a", "b", None, 1.0), seg("b", "a", None, 1.0)],
            &[],
        );
        let s = layout(&input, &[]);
        assert!(s.cycle);
        assert_eq!(s.cards.len(), 2);
        assert_eq!((card(&s, "a").column, card(&s, "b").column), (0, 1));
    }

    #[test]
    fn order_is_stable_between_refreshes() {
        // Without an order, barycenters put y (child of the top WAN) above x.
        let input = live(
            &["w1", "w2", "x", "y"],
            vec![seg("w1", "y", None, 10.0), seg("w2", "x", None, 10.0)],
            &[],
        );
        assert_eq!(column(&layout(&input, &[]), 1), ["y", "x"]);
        // The order of the last layout wins, so a refresh does not move cards.
        let order: Vec<String> = ["w1", "w2", "x", "y"].map(String::from).to_vec();
        assert_eq!(column(&layout(&input, &order), 1), ["x", "y"]);
        // A new card goes after the known ones.
        let mut more = input.clone();
        more.cards.push("z".into());
        more.segments.push(seg("w1", "z", None, 1.0));
        assert_eq!(column(&layout(&more, &order), 1), ["x", "y", "z"]);
    }

    #[test]
    fn a_zero_value_card_keeps_the_minimum_height() {
        let s = layout(&chain(), &[]);
        assert_eq!(card(&s, "grp-b").h, MIN_H);
        let lonely = layout(&live(&["alone"], vec![], &[]), &[]);
        assert_eq!(card(&lonely, "alone").h, MIN_H);
    }

    fn summary() -> TopologySummary {
        let node = |id: &str,
                    parent: Option<&str>,
                    kind: &str,
                    source: Option<&str>,
                    card: &str|
         -> Node {
            serde_json::from_value(json!({"id": id, "name": id, "parent": parent, "kind": kind,
                "source_id": source, "capacity": {"mode": "automatic"}, "alias": false,
                "calculated_mbps": null, "exported_mbps": null, "members": null, "card_id": card}))
            .unwrap()
        };
        let source = |id: &str, access: &str| -> Source {
            serde_json::from_value(
                json!({"id": id, "name": id, "kind": "subscribers", "access": access}),
            )
            .unwrap()
        };
        TopologySummary {
            nodes: vec![
                node("wan-1", None, "wan", None, ""),
                node(
                    "ppp-1",
                    Some("wan-1"),
                    "group",
                    Some("src-ppp"),
                    "source:src-ppp",
                ),
                node(
                    "dhcp-1",
                    Some("wan-1"),
                    "group",
                    Some("src-dhcp"),
                    "source:src-dhcp",
                ),
            ],
            flows: vec![
                Flow {
                    from: "wan-1".into(),
                    to: "source:src-ppp".into(),
                    subscribers: 7,
                },
                Flow {
                    from: "wan-1".into(),
                    to: "source:src-dhcp".into(),
                    subscribers: 3,
                },
            ],
            sources: vec![source("src-ppp", "ppp"), source("src-dhcp", "dhcp")],
            ..Default::default()
        }
    }

    fn throughput(stale: bool) -> TopologyThroughput {
        let f = |from: &str, to: &str, mbps: u64| FlowThroughput {
            from: from.into(),
            to: to.into(),
            wan: Some("wan-1".into()),
            down_bps: mbps * 1_000_000,
            up_bps: 0,
        };
        TopologyThroughput {
            updated: 1,
            stale,
            complete: !stale,
            flows: vec![
                f("wan-1", "source:src-ppp", 40),
                f("wan-1", "source:src-dhcp", 12),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn a_stale_sample_gives_subscriber_segments() {
        let i = input(&summary(), Some(&throughput(true)));
        assert_eq!(i.metric, Metric::Subscribers);
        assert!(i.segments.iter().all(|s| s.wan.is_none()));
        assert!(
            i.segments
                .contains(&seg("wan-1", "source:src-ppp", None, 7.0))
        );
        let none = input(&summary(), None);
        assert_eq!(none.metric, Metric::Subscribers);
        let fresh = input(&summary(), Some(&throughput(false)));
        assert_eq!(fresh.metric, Metric::LiveMbps);
        assert!(
            fresh
                .segments
                .contains(&seg("wan-1", "source:src-ppp", Some("wan-1"), 40.0))
        );
        assert_eq!(fresh.wans, ["wan-1"]);
    }

    #[test]
    fn groups_feed_their_service_tier_card() {
        let i = input(&summary(), Some(&throughput(false)));
        assert!(
            i.segments
                .contains(&seg("source:src-ppp", "tier:ppp", Some("wan-1"), 40.0))
        );
        assert!(
            i.segments
                .contains(&seg("source:src-dhcp", "tier:dhcp", Some("wan-1"), 12.0))
        );
        assert!(i.cards.contains(&"tier:ppp".to_string()));
        assert!(
            !i.cards.contains(&"tier:hotspot".to_string()),
            "only used tiers"
        );
    }
}
