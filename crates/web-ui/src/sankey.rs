//! Sankey geometry for the Topology canvas. Pure Rust: no Leptos, no browser.
//! Columns come from the graph (longest path), card height follows the card value, and each WAN
//! is a separate stream that stacks on the card edges.

use web_contract::topology::{TopologySummary, TopologyThroughput};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    /// Ribbon width: the configured (calculated) download capacity in Mbps.
    Capacity,
    /// No capacity data: ribbon width is the subscriber count.
    Subscribers,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub from: String,
    pub to: String,
    /// The WAN node ID of the stream. `None`: "WAN unknown".
    pub wan: Option<String>,
    pub value: f64,
    /// The live download in Mbps on this stream, from a fresh host sample.
    pub live: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Input {
    pub cards: Vec<String>,
    pub segments: Vec<Segment>,
    /// The structure: parent → child card pairs. They set the columns and the order, so a card
    /// keeps its column when its hosts are not in the live sample. They draw no ribbon.
    pub edges: Vec<(String, String)>,
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
    pub live: Option<f64>,
    pub d: String,
    /// The live band inside the ribbon: its height (live ÷ capacity, at most the full band)
    /// and its path, top-aligned in the ribbon.
    pub live_h: f64,
    pub d_live: Option<String>,
    /// The lanes of a ribbon that skips columns: column, top, and height. A lane holds space
    /// in the column stack, so the ribbon never passes under a card.
    pub lanes: Vec<(usize, f64, f64)>,
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
pub const MIN_H: f64 = 112.0;
/// The band height of the column with the largest total value.
pub const MAX_BAND: f64 = 280.0;

/// The card of a node: the instances of one source, or the copies of one router, share one card.
fn card_of(n: &web_contract::topology::Node) -> &str {
    if n.card_id.is_empty() {
        &n.id
    } else {
        &n.card_id
    }
}

/// The capacity that a node instance passes on: its calculated rate, or, for a display-only node
/// or a node without a rate, the sum of what its children get. A parent cycle counts as zero.
fn rolled(
    id: &str,
    nodes: &[web_contract::topology::Node],
    capacity: &dyn Fn(&web_contract::topology::Node) -> Option<f64>,
    memo: &mut std::collections::HashMap<String, f64>,
    depth: usize,
) -> f64 {
    if let Some(v) = memo.get(id) {
        return *v;
    }
    let Some(n) = nodes
        .iter()
        .find(|x| x.id == id)
        .filter(|_| depth <= nodes.len())
    else {
        return 0.0;
    };
    let value = match capacity(n) {
        Some(c) if !n.tree_excluded => c,
        _ => nodes
            .iter()
            .filter(|c| c.parent.as_deref() == Some(id))
            .map(|c| rolled(&c.id, nodes, capacity, memo, depth + 1))
            .sum(),
    };
    memo.insert(id.to_string(), value);
    value
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
    let capacity = |n: &web_contract::topology::Node| {
        n.calculated_mbps
            .or(n.exported_mbps)
            .map(|m| m[0])
            .filter(|v| v.is_finite())
            .map(|v| v.max(0.0))
    };
    let live_flows: &[web_contract::topology::FlowThroughput] =
        t.filter(|t| !t.stale).map_or(&[], |t| &t.flows[..]);
    let (metric, mut segments) = if summary.nodes.iter().any(|n| capacity(n).is_some()) {
        let find = |id: &str| summary.nodes.iter().find(|x| x.id == id);
        // The WAN of a node: the node itself or its nearest WAN ancestor.
        let tree_wan = |n: &web_contract::topology::Node| {
            let mut at = Some(n);
            for _ in 0..=summary.nodes.len() {
                let x = at?;
                if x.kind == "wan" {
                    return Some(x.id.clone());
                }
                at = x.parent.as_deref().and_then(find);
            }
            None
        };
        let mut segments: Vec<Segment> = Vec::new();
        let mut memo = std::collections::HashMap::new();
        for n in &summary.nodes {
            let Some(p) = n.parent.as_deref().and_then(find) else {
                continue;
            };
            let (from, to, wan) = (card_of(p), card_of(n), tree_wan(n));
            if from == to {
                continue;
            }
            let value = rolled(&n.id, &summary.nodes, &capacity, &mut memo, 0);
            match segments
                .iter_mut()
                .find(|s| s.from == from && s.to == to && s.wan == wan)
            {
                Some(s) => s.value += value,
                None => segments.push(Segment {
                    from: from.to_string(),
                    to: to.to_string(),
                    wan,
                    value,
                    live: None,
                }),
            }
        }
        for s in &mut segments {
            s.live = live_flows
                .iter()
                .find(|f| f.from == s.from && f.to == s.to && f.wan == s.wan)
                .map(|f| f.down_bps as f64 / 1_000_000.0);
        }
        (Metric::Capacity, segments)
    } else {
        (
            Metric::Subscribers,
            summary
                .flows
                .iter()
                .map(|f| Segment {
                    from: f.from.clone(),
                    to: f.to.clone(),
                    wan: None,
                    value: f.subscribers as f64,
                    live: None,
                })
                .collect(),
        )
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
                Some(e) => {
                    e.value += s.value;
                    e.live = match (e.live, s.live) {
                        (None, None) => None,
                        (a, b) => Some(a.unwrap_or(0.0) + b.unwrap_or(0.0)),
                    };
                }
                None => extra.push(Segment {
                    from: group.clone(),
                    to: tier.clone(),
                    wan: s.wan.clone(),
                    value: s.value,
                    live: s.live,
                }),
            }
        }
    }
    for tier in ["tier:ppp", "tier:dhcp", "tier:hotspot"] {
        if tiers.iter().any(|(_, t)| t == tier) {
            cards.push(tier.to_string());
        }
    }
    segments.extend(extra);
    let mut edges: Vec<(String, String)> = Vec::new();
    let mut add = |a: &str, b: &str| {
        let pair = (a.to_string(), b.to_string());
        if a != b && !edges.contains(&pair) {
            edges.push(pair);
        }
    };
    for n in &summary.nodes {
        match n
            .parent
            .as_deref()
            .and_then(|p| summary.nodes.iter().find(|x| x.id == p))
        {
            Some(p) => add(card_of(p), card_of(n)),
            // The routing WANs feed a top card of a tree without WAN nodes.
            None if n.kind != "wan" => {
                for w in summary.nodes.iter().filter(|w| w.kind == "wan") {
                    add(card_of(w), card_of(n));
                }
            }
            None => {}
        }
    }
    for (group, tier) in &tiers {
        add(group, tier);
    }
    Input {
        cards,
        segments,
        edges,
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
    let structure = input
        .edges
        .iter()
        .filter_map(|(a, b)| Some((*index.get(a.as_str())?, *index.get(b.as_str())?)));
    for (a, b) in input.segments.iter().filter_map(edge).chain(structure) {
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
    let band = |v: f64| v.max(0.0) * scale;
    // A lane for each column that a ribbon skips: (segment, column) → top.
    let mut lane_y: HashMap<(usize, usize), f64> = HashMap::new();
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
        // Lanes go below the cards of the column, in the order of their source height.
        let mut lanes: Vec<(usize, f64)> = segments
            .iter()
            .enumerate()
            .filter(|(_, (a, b, _))| column[*a] < c && c < column[*b])
            .map(|(k, (a, _, _))| (k, y[*a]))
            .collect();
        lanes.sort_by(|p, q| p.1.total_cmp(&q.1).then(p.0.cmp(&q.0)));
        for (k, _) in lanes {
            lane_y.insert((k, c), at);
            at += band(segments[k].2.value) + GAP;
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
    let band = |i: usize| band(segments[i].2.value);
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
            let (y0a, y1a, h) = (start[i], end[i], band(i));
            let lanes: Vec<(usize, f64, f64)> = (column[*a] + 1..column[*b])
                .map(|c| (c, lane_y[&(i, c)], h))
                .collect();
            let lane_x = |c: usize| c as f64 * (CARD_W + COL_GAP);
            // A band of height `h`, top-aligned in the ribbon: top edge forward through each
            // lane, bottom edge back.
            let shape = |h: f64| {
                let (mut d, mut cx, mut cy) = (format!("M{x0},{y0a}"), x0, y0a);
                for &(c, ly, _) in &lanes {
                    let (lx, mid) = (lane_x(c), (cx + lane_x(c)) / 2.0);
                    d += &format!(" C{mid},{cy} {mid},{ly} {lx},{ly} L{},{ly}", lx + CARD_W);
                    (cx, cy) = (lx + CARD_W, ly);
                }
                let mid = (cx + x1) / 2.0;
                d += &format!(" C{mid},{cy} {mid},{y1a} {x1},{y1a} L{x1},{}", y1a + h);
                (cx, cy) = (x1, y1a + h);
                for &(c, ly, _) in lanes.iter().rev() {
                    let (rx, mid) = (lane_x(c) + CARD_W, (cx + lane_x(c) + CARD_W) / 2.0);
                    d += &format!(
                        " C{mid},{cy} {mid},{} {rx},{} L{},{}",
                        ly + h,
                        ly + h,
                        rx - CARD_W,
                        ly + h
                    );
                    (cx, cy) = (rx - CARD_W, ly + h);
                }
                let mid = (cx + x0) / 2.0;
                d += &format!(" C{mid},{cy} {mid},{} {x0},{} Z", y0a + h, y0a + h);
                d
            };
            let live_h = match s.live {
                Some(l) if s.value > 0.0 && l.is_finite() => h * (l.max(0.0) / s.value).min(1.0),
                _ => 0.0,
            };
            Ribbon {
                from: s.from.clone(),
                to: s.to.clone(),
                wan: s.wan.clone(),
                value: s.value,
                live: s.live,
                d: shape(h),
                live_h,
                d_live: (live_h > 0.0).then(|| shape(live_h)),
                lanes,
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
    let lanes_bottom = lane_y
        .iter()
        .map(|(&(k, _), &top)| top + band(k))
        .fold(0.0, f64::max);
    let bounds = cards.iter().fold([0.0, 0.0, 0.0, lanes_bottom], |b, c| {
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
            live: None,
        }
    }

    fn live(cards: &[&str], segments: Vec<Segment>, wans: &[&str]) -> Input {
        Input {
            cards: cards.iter().map(|c| c.to_string()).collect(),
            segments,
            edges: Vec::new(),
            wans: wans.iter().map(|w| w.to_string()).collect(),
            metric: Metric::Capacity,
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
    fn without_capacity_the_ribbons_count_subscribers() {
        // `summary()` has no calculated or exported rates.
        for t in [None, Some(throughput(false)), Some(throughput(true))] {
            let i = input(&summary(), t.as_ref());
            assert_eq!(i.metric, Metric::Subscribers);
            assert!(
                i.segments
                    .iter()
                    .all(|s| s.wan.is_none() && s.live.is_none())
            );
            assert!(
                i.segments
                    .contains(&seg("wan-1", "source:src-ppp", None, 7.0))
            );
            assert!(
                i.segments
                    .contains(&seg("source:src-ppp", "tier:ppp", None, 7.0))
            );
        }
    }

    /// WAN 1 (1000 Mbps) and WAN 2 (500 Mbps) above two copies of router R (400 and 100 Mbps),
    /// each above one instance of the PPP group G (300 and 100 Mbps).
    fn capacity_summary() -> TopologySummary {
        let node = |id: &str,
                    parent: Option<&str>,
                    kind: &str,
                    source: Option<&str>,
                    card: &str,
                    mbps: f64|
         -> Node {
            serde_json::from_value(json!({"id": id, "name": id, "parent": parent, "kind": kind,
                "source_id": source, "capacity": {"mode": "automatic"}, "alias": false,
                "calculated_mbps": [mbps, mbps / 2.0], "exported_mbps": null, "members": null, "card_id": card}))
            .unwrap()
        };
        let source: Source = serde_json::from_value(
            json!({"id": "src-ppp", "name": "src-ppp", "kind": "subscribers", "access": "ppp"}),
        )
        .unwrap();
        TopologySummary {
            nodes: vec![
                node("wan-1", None, "wan", None, "", 1000.0),
                node("wan-2", None, "wan", None, "", 500.0),
                node("r1", Some("wan-1"), "intermediate", None, "router:r", 400.0),
                node("r2", Some("wan-2"), "intermediate", None, "router:r", 100.0),
                node(
                    "g1",
                    Some("r1"),
                    "group",
                    Some("src-ppp"),
                    "source:g",
                    300.0,
                ),
                node(
                    "g2",
                    Some("r2"),
                    "group",
                    Some("src-ppp"),
                    "source:g",
                    100.0,
                ),
            ],
            sources: vec![source],
            ..Default::default()
        }
    }

    /// From, to, WAN, capacity, live.
    type Stream<'a> = (&'a str, &'a str, Option<&'a str>, f64, Option<f64>);

    fn flows(i: &Input) -> Vec<Stream<'_>> {
        i.segments
            .iter()
            .map(|s| {
                (
                    s.from.as_str(),
                    s.to.as_str(),
                    s.wan.as_deref(),
                    s.value,
                    s.live,
                )
            })
            .collect()
    }

    #[test]
    fn ribbons_carry_the_configured_capacity_of_each_wan() {
        let i = input(&capacity_summary(), None);
        assert_eq!(i.metric, Metric::Capacity);
        let got = flows(&i);
        for want in [
            ("wan-1", "router:r", Some("wan-1"), 400.0, None),
            ("wan-2", "router:r", Some("wan-2"), 100.0, None),
            ("router:r", "source:g", Some("wan-1"), 300.0, None),
            ("router:r", "source:g", Some("wan-2"), 100.0, None),
            ("source:g", "tier:ppp", Some("wan-1"), 300.0, None),
            ("source:g", "tier:ppp", Some("wan-2"), 100.0, None),
        ] {
            assert!(got.contains(&want), "{want:?} in {got:?}");
        }
        assert_eq!(got.len(), 6);
    }

    #[test]
    fn live_traffic_attaches_to_its_capacity_ribbon() {
        let f = |from: &str, to: &str, mbps: u64| FlowThroughput {
            from: from.into(),
            to: to.into(),
            wan: Some("wan-1".into()),
            down_bps: mbps * 1_000_000,
            up_bps: 0,
        };
        let mut t = TopologyThroughput {
            updated: 1,
            complete: true,
            flows: vec![f("wan-1", "router:r", 120), f("router:r", "source:g", 90)],
            ..Default::default()
        };
        let with_live = input(&capacity_summary(), Some(&t));
        let got = flows(&with_live);
        assert!(got.contains(&("wan-1", "router:r", Some("wan-1"), 400.0, Some(120.0))));
        assert!(got.contains(&("source:g", "tier:ppp", Some("wan-1"), 300.0, Some(90.0))));
        assert!(got.contains(&("wan-2", "router:r", Some("wan-2"), 100.0, None)));
        t.stale = true;
        let i = input(&capacity_summary(), Some(&t));
        assert_eq!(
            i.metric,
            Metric::Capacity,
            "a stale sample keeps the capacity"
        );
        assert!(i.segments.iter().all(|s| s.live.is_none()));
    }

    #[test]
    fn the_live_band_fills_its_share_of_the_ribbon() {
        let mut input = live(
            &["a", "b", "c"],
            vec![seg("a", "b", None, 400.0), seg("a", "c", None, 50.0)],
            &[],
        );
        input.metric = Metric::Capacity;
        input.segments[0].live = Some(100.0);
        input.segments[1].live = Some(80.0);
        let s = layout(&input, &[]);
        let scale = MAX_BAND / 450.0;
        let r = |to: &str| s.ribbons.iter().find(|r| r.to == to).unwrap();
        assert!(
            (r("b").live_h - 400.0 * scale / 4.0).abs() < 1e-9,
            "a quarter of the ribbon"
        );
        assert!(
            (r("c").live_h - 50.0 * scale).abs() < 1e-9,
            "more live than capacity fills it once"
        );
        assert!(r("b").d_live.as_deref().is_some_and(|d| d.ends_with('Z')));
        let none = layout(
            &live(&["a", "b"], vec![seg("a", "b", None, 10.0)], &[]),
            &[],
        );
        assert_eq!(
            (none.ribbons[0].live_h, none.ribbons[0].d_live.is_none()),
            (0.0, true)
        );
    }

    /// WAN → distro → PPP group, and a DHCP group right below the WAN. The live sample has only
    /// the PPP hosts.
    fn mixed() -> (TopologySummary, TopologyThroughput) {
        let node = |id: &str, parent: Option<&str>, kind: &str, source: Option<&str>| -> Node {
            serde_json::from_value(json!({"id": id, "name": id, "parent": parent, "kind": kind,
                "source_id": source, "capacity": {"mode": "automatic"}, "alias": false,
                "calculated_mbps": null, "exported_mbps": null, "members": null, "card_id": ""}))
            .unwrap()
        };
        let source = |id: &str, access: &str| -> Source {
            serde_json::from_value(
                json!({"id": id, "name": id, "kind": "subscribers", "access": access}),
            )
            .unwrap()
        };
        let summary = TopologySummary {
            nodes: vec![
                node("wan-1", None, "wan", None),
                node("distro", Some("wan-1"), "intermediate", None),
                node("ppp-1", Some("distro"), "group", Some("src-ppp")),
                node("dhcp-1", Some("wan-1"), "group", Some("src-dhcp")),
            ],
            flows: vec![
                Flow {
                    from: "wan-1".into(),
                    to: "distro".into(),
                    subscribers: 4,
                },
                Flow {
                    from: "distro".into(),
                    to: "ppp-1".into(),
                    subscribers: 4,
                },
                Flow {
                    from: "wan-1".into(),
                    to: "dhcp-1".into(),
                    subscribers: 2,
                },
            ],
            sources: vec![source("src-ppp", "ppp"), source("src-dhcp", "dhcp")],
            ..Default::default()
        };
        let f = |from: &str, to: &str| FlowThroughput {
            from: from.into(),
            to: to.into(),
            wan: Some("wan-1".into()),
            down_bps: 30_000_000,
            up_bps: 0,
        };
        let live = TopologyThroughput {
            updated: 1,
            complete: true,
            flows: vec![f("wan-1", "distro"), f("distro", "ppp-1")],
            ..Default::default()
        };
        (summary, live)
    }

    fn columns(s: &Sankey) -> Vec<(String, usize)> {
        let mut c: Vec<(String, usize)> =
            s.cards.iter().map(|c| (c.id.clone(), c.column)).collect();
        c.sort();
        c
    }

    #[test]
    fn columns_do_not_depend_on_the_live_sample() {
        let (summary, live) = mixed();
        let with_hosts = layout(&input(&summary, Some(&live)), &[]);
        let stale = TopologyThroughput {
            stale: true,
            ..live.clone()
        };
        let without = layout(&input(&summary, Some(&stale)), &[]);
        assert_eq!(columns(&with_hosts), columns(&without));
        assert_eq!(
            card(&with_hosts, "dhcp-1").column,
            1,
            "an idle group keeps its column"
        );
        assert!(
            with_hosts.cards.iter().any(|c| c.id == "tier:dhcp"),
            "an idle tier keeps its card"
        );
    }

    #[test]
    fn ribbons_that_skip_a_column_pass_beside_its_cards() {
        let input = live(
            &["w", "r", "g", "t"],
            vec![
                seg("w", "r", None, 10.0),
                seg("r", "g", None, 10.0),
                seg("g", "t", None, 10.0),
                seg("w", "t", None, 5.0),
            ],
            &[],
        );
        let s = layout(&input, &[]);
        let long = s
            .ribbons
            .iter()
            .find(|r| r.from == "w" && r.to == "t")
            .unwrap();
        assert_eq!(long.lanes.iter().map(|l| l.0).collect::<Vec<_>>(), [1, 2]);
        for (column, top, height) in &long.lanes {
            for c in s.cards.iter().filter(|c| c.column == *column) {
                assert!(
                    top + height <= c.y + 1e-9 || c.y + c.h <= *top + 1e-9,
                    "lane {column} crosses {}",
                    c.id
                );
            }
        }
        assert!(s.bounds[3] >= long.lanes.iter().map(|l| l.1 + l.2).fold(0.0, f64::max));
    }

    /// Production shape (2026-11-24): each WAN has one copy of a display-only router (fixed
    /// 100000 Mbps, no calculated rate), and groups below it with calculated rates.
    #[test]
    fn a_display_only_router_carries_what_its_children_get() {
        let node = |id: &str,
                    parent: Option<&str>,
                    kind: &str,
                    card: &str,
                    calc: Option<f64>,
                    excluded: bool|
         -> Node {
            serde_json::from_value(json!({"id": id, "name": id, "parent": parent, "kind": kind,
                "source_id": null, "capacity": {"mode": "fixed", "down_mbps": 100000, "up_mbps": 100000, "factor_percent": null},
                "alias": false, "calculated_mbps": calc.map(|c| [c, c]), "exported_mbps": null,
                "members": null, "card_id": card, "tree_excluded": excluded}))
            .unwrap()
        };
        let summary = TopologySummary {
            nodes: vec![
                node("wan-1", None, "wan", "wan-1", Some(320.0), false),
                node("wan-2", None, "wan", "wan-2", Some(320.0), false),
                node("r1", Some("wan-1"), "intermediate", "router:d", None, true),
                node("r2", Some("wan-2"), "intermediate", "router:d", None, true),
                node("g1", Some("r1"), "group", "source:a", Some(100.0), false),
                node("g2", Some("r1"), "group", "source:b", None, false),
                node("g3", Some("r1"), "group", "source:c", Some(107.5), false),
                node("g4", Some("r2"), "group", "source:a", Some(35.0), false),
            ],
            ..Default::default()
        };
        let i = input(&summary, None);
        let got = flows(&i);
        assert!(
            got.contains(&("wan-1", "router:d", Some("wan-1"), 207.5, None)),
            "{got:?}"
        );
        assert!(
            got.contains(&("wan-2", "router:d", Some("wan-2"), 35.0, None)),
            "{got:?}"
        );
        assert!(got.contains(&("router:d", "source:a", Some("wan-1"), 100.0, None)));
        assert!(got.contains(&("router:d", "source:a", Some("wan-2"), 35.0, None)));
        // A shaped node keeps its own calculated rate, even when its children get less.
        let mut shaped = summary.clone();
        shaped.nodes[2] = node(
            "r1",
            Some("wan-1"),
            "intermediate",
            "router:d",
            Some(250.0),
            false,
        );
        let shaped_input = input(&shaped, None);
        let got = flows(&shaped_input)
            .into_iter()
            .map(|f| (f.0, f.1, f.2, f.3))
            .collect::<Vec<_>>();
        assert!(
            got.contains(&("wan-1", "router:d", Some("wan-1"), 250.0)),
            "{got:?}"
        );
    }
}
