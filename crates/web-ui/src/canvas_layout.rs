//! Ported from p33ckab00/traffic-intelligence src/ui/canvas.rs (same owner).
//! Framework-independent canvas geometry: card positions, fit, and zoom.

use std::collections::BTreeMap;

/// The row of a card. WAN cards are at the top, then router cards, then group cards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Wan,
    Router,
    Group,
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardBox {
    pub id: String,
    pub tier: Tier,
    pub parent: Option<String>,
    /// Width and height in canvas pixels.
    pub size: [f64; 2],
}

/// The free space around each card.
const GAP: f64 = 16.0;
const MARGIN: f64 = 48.0;
/// The vertical space between a parent and the row of its children.
const ROW_GAP: f64 = 56.0;

/// Positions (top-left corners) of all cards. A manual position stays. An automatic card goes
/// in its tier row, below its parent when the parent has a position, and never overlaps a card.
pub fn layout(
    cards: &[CardBox],
    manual: &BTreeMap<String, [f64; 2]>,
) -> BTreeMap<String, [f64; 2]> {
    let size: BTreeMap<&str, [f64; 2]> = cards.iter().map(|c| (c.id.as_str(), c.size)).collect();
    // Manual cards reserve space first.
    let mut out: BTreeMap<String, [f64; 2]> = cards
        .iter()
        .filter_map(|c| manual.get(&c.id).map(|p| (c.id.clone(), *p)))
        .collect();
    let row = ROW_GAP + cards.iter().map(|c| c.size[1]).fold(0.0, f64::max);
    let mut order: Vec<&CardBox> = cards.iter().filter(|c| !out.contains_key(&c.id)).collect();
    order.sort_by_key(|c| c.tier);
    for card in order {
        let [w, h] = card.size;
        let mut p = match card
            .parent
            .as_deref()
            .and_then(|id| Some((out.get(id)?, size[id])))
        {
            Some((parent, [_, ph])) => [parent[0], parent[1] + ph + ROW_GAP],
            None => [MARGIN, MARGIN + card.tier as u8 as f64 * row],
        };
        // Move right past each card in the way. `x` only grows, so the search ends.
        while let Some((id, q)) = out.iter().find(|(id, q)| {
            let [ow, oh] = size[id.as_str()];
            p[0] < q[0] + ow + GAP
                && q[0] < p[0] + w + GAP
                && p[1] < q[1] + oh + GAP
                && q[1] < p[1] + h + GAP
        }) {
            p[0] = q[0] + size[id.as_str()][0] + GAP;
        }
        out.insert(card.id.clone(), p);
    }
    out
}

/// Positions in a canvas direction. The layout runs in an abstract space (cross axis, depth)
/// with the card sizes swapped for a horizontal direction, then turns into canvas coordinates:
/// mirrored for bottom-up and right-to-left, transposed for the horizontal directions. Manual
/// positions stay; an automatic card that hits one moves along the cross axis.
pub fn layout_in(
    cards: &[CardBox],
    manual: &BTreeMap<String, [f64; 2]>,
    direction: web_contract::topology::Direction,
) -> BTreeMap<String, [f64; 2]> {
    use web_contract::topology::Direction;
    if direction == Direction::TopDown {
        return layout(cards, manual);
    }
    let horizontal = matches!(direction, Direction::LeftRight | Direction::RightLeft);
    let abstract_cards: Vec<CardBox> = cards
        .iter()
        .map(|c| CardBox {
            size: if horizontal {
                [c.size[1], c.size[0]]
            } else {
                c.size
            },
            ..c.clone()
        })
        .collect();
    let auto = layout(&abstract_cards, &BTreeMap::new());
    let depth_end = abstract_cards
        .iter()
        .map(|c| auto[&c.id][1] + c.size[1])
        .fold(0.0, f64::max);
    let size: BTreeMap<&str, [f64; 2]> = cards.iter().map(|c| (c.id.as_str(), c.size)).collect();
    let mut out: BTreeMap<String, [f64; 2]> = cards
        .iter()
        .filter_map(|c| manual.get(&c.id).map(|p| (c.id.clone(), *p)))
        .collect();
    let mut order: Vec<&CardBox> = cards.iter().filter(|c| !out.contains_key(&c.id)).collect();
    order.sort_by_key(|c| c.tier);
    for card in order {
        let [a, b] = auto[&card.id];
        let [w, h] = card.size;
        let mut p = match direction {
            Direction::BottomUp => [a, depth_end + MARGIN - b - h],
            Direction::LeftRight => [b, a],
            _ => [depth_end + MARGIN - b - w, a],
        };
        // The cross axis: x for vertical directions, y for horizontal ones. It only grows.
        while let Some((id, q)) = out.iter().find(|(id, q)| {
            let [ow, oh] = size[id.as_str()];
            p[0] < q[0] + ow + GAP
                && q[0] < p[0] + w + GAP
                && p[1] < q[1] + oh + GAP
                && q[1] < p[1] + h + GAP
        }) {
            let [ow, oh] = size[id.as_str()];
            if horizontal {
                p[1] = q[1] + oh + GAP;
            } else {
                p[0] = q[0] + ow + GAP;
            }
        }
        out.insert(card.id.clone(), p);
    }
    out
}

/// Zoom and pan that fit `bounds` (`[min x, min y, max x, max y]`) in `viewport`, with a 4 %
/// border. Returns `(zoom, pan x, pan y)`.
pub fn fit(bounds: [f64; 4], viewport: [f64; 2]) -> (f64, f64, f64) {
    let (w, h) = (bounds[2] - bounds[0], bounds[3] - bounds[1]);
    if viewport[0] <= 0.0 || viewport[1] <= 0.0 || w <= 0.0 || h <= 0.0 {
        return (1.0, 0.0, 0.0);
    }
    let zoom = clamp_zoom((viewport[0] / w).min(viewport[1] / h) * 0.96);
    (
        zoom,
        (viewport[0] - w * zoom) / 2.0 - bounds[0] * zoom,
        (viewport[1] - h * zoom) / 2.0 - bounds[1] * zoom,
    )
}

pub fn clamp_zoom(z: f64) -> f64 {
    z.clamp(0.25, 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str, tier: Tier, parent: Option<&str>) -> CardBox {
        CardBox {
            id: id.into(),
            tier,
            parent: parent.map(Into::into),
            size: [200.0, 100.0],
        }
    }

    fn overlaps(cards: &[CardBox], at: &BTreeMap<String, [f64; 2]>) -> Option<(String, String)> {
        for (i, a) in cards.iter().enumerate() {
            for b in &cards[i + 1..] {
                let (p, q) = (at[&a.id], at[&b.id]);
                if p[0] < q[0] + b.size[0] + GAP
                    && q[0] < p[0] + a.size[0] + GAP
                    && p[1] < q[1] + b.size[1] + GAP
                    && q[1] < p[1] + a.size[1] + GAP
                {
                    return Some((a.id.clone(), b.id.clone()));
                }
            }
        }
        None
    }

    /// A small fixed-seed generator: the test needs no random crate.
    fn next(seed: &mut u64) -> u64 {
        *seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *seed >> 33
    }

    fn random_cards(n: usize, seed: &mut u64) -> Vec<CardBox> {
        let mut cards: Vec<CardBox> = Vec::with_capacity(n);
        for i in 0..n {
            let tier =
                [Tier::Wan, Tier::Router, Tier::Group, Tier::Other][(next(seed) % 4) as usize];
            let parent = (i > 0 && next(seed).is_multiple_of(2))
                .then(|| cards[(next(seed) as usize) % i].id.clone());
            cards.push(CardBox {
                id: format!("c{i}"),
                tier,
                parent,
                size: [
                    160.0 + (next(seed) % 101) as f64,
                    70.0 + (next(seed) % 71) as f64,
                ],
            });
        }
        cards
    }

    #[test]
    fn no_automatic_card_overlaps_for_1_to_200_cards() {
        let mut seed = 7;
        for n in 1..=200 {
            let cards = random_cards(n, &mut seed);
            let at = layout(&cards, &BTreeMap::new());
            assert_eq!(at.len(), n);
            assert_eq!(overlaps(&cards, &at), None, "{n} cards");
        }
    }

    #[test]
    fn a_manual_position_is_kept_and_avoided() {
        let cards = [
            card("wan1", Tier::Wan, None),
            card("r1", Tier::Router, Some("wan1")),
            card("g1", Tier::Group, Some("r1")),
        ];
        let auto = layout(&cards, &BTreeMap::new());
        // The operator drops the router card on the place of the group card.
        let manual = BTreeMap::from([("r1".to_string(), auto["g1"])]);
        let at = layout(&cards, &manual);
        assert_eq!(at["r1"], auto["g1"]);
        assert_eq!(overlaps(&cards, &at), None);
    }

    #[test]
    fn an_empty_manual_map_is_the_reset_layout() {
        let cards = random_cards(30, &mut 3);
        let auto = layout(&cards, &BTreeMap::new());
        let manual = BTreeMap::from([("c0".to_string(), [900.0, 900.0])]);
        assert_ne!(layout(&cards, &manual), auto);
        assert_eq!(layout(&cards, &BTreeMap::new()), auto);
    }

    #[test]
    fn a_group_is_below_its_parent() {
        let cards = [
            card("wan1", Tier::Wan, None),
            card("r1", Tier::Router, Some("wan1")),
            card("g1", Tier::Group, Some("r1")),
            card("g2", Tier::Group, Some("r1")),
        ];
        let at = layout(&cards, &BTreeMap::new());
        assert!(at["r1"][1] > at["wan1"][1] + 100.0);
        for g in ["g1", "g2"] {
            assert!(at[g][1] > at["r1"][1] + 100.0, "{g}");
        }
        assert_eq!(at["g1"][0], at["r1"][0]);
    }

    #[test]
    fn each_direction_puts_wans_on_its_start_side() {
        use web_contract::topology::Direction;
        let cards = [
            card("wan1", Tier::Wan, None),
            card("r1", Tier::Router, Some("wan1")),
            card("g1", Tier::Group, Some("r1")),
        ];
        let at = |d| layout_in(&cards, &BTreeMap::new(), d);
        let td = at(Direction::TopDown);
        assert!(td["wan1"][1] < td["r1"][1] && td["r1"][1] < td["g1"][1]);
        let bu = at(Direction::BottomUp);
        assert!(bu["wan1"][1] > bu["r1"][1] && bu["r1"][1] > bu["g1"][1]);
        let lr = at(Direction::LeftRight);
        assert!(lr["wan1"][0] < lr["r1"][0] && lr["r1"][0] < lr["g1"][0]);
        let rl = at(Direction::RightLeft);
        assert!(rl["wan1"][0] > rl["r1"][0] && rl["r1"][0] > rl["g1"][0]);
        // Top-down is the original layout.
        assert_eq!(td, layout(&cards, &BTreeMap::new()));
    }

    #[test]
    fn no_overlap_in_any_direction_for_1_to_200_cards() {
        use web_contract::topology::Direction;
        for d in [
            Direction::BottomUp,
            Direction::LeftRight,
            Direction::RightLeft,
        ] {
            let mut seed = 5;
            for n in 1..=200 {
                let cards = random_cards(n, &mut seed);
                let manual = BTreeMap::from([("c0".to_string(), [300.0, 300.0])]);
                let at = layout_in(&cards, &manual, d);
                assert_eq!(at["c0"], [300.0, 300.0]);
                assert_eq!(overlaps(&cards, &at), None, "{d:?} {n} cards");
            }
        }
    }

    #[test]
    fn fit_contains_every_card() {
        let cards = random_cards(10, &mut 11);
        let at = layout(&cards, &BTreeMap::new());
        let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        for c in &cards {
            let p = at[&c.id];
            b = [
                b[0].min(p[0]),
                b[1].min(p[1]),
                b[2].max(p[0] + c.size[0]),
                b[3].max(p[1] + c.size[1]),
            ];
        }
        let viewport = [1280.0, 720.0];
        let (zoom, px, py) = fit(b, viewport);
        let screen = |x: f64, y: f64| (x * zoom + px, y * zoom + py);
        let (x0, y0) = screen(b[0], b[1]);
        let (x1, y1) = screen(b[2], b[3]);
        assert!(
            zoom > clamp_zoom(0.0),
            "the test needs a zoom above the minimum"
        );
        assert!(x0 >= 0.0 && y0 >= 0.0, "{x0} {y0}");
        assert!(x1 <= viewport[0] && y1 <= viewport[1], "{x1} {y1}");
        assert_eq!(fit([0.0; 4], [0.0, 0.0]), (1.0, 0.0, 0.0));
        assert_eq!((clamp_zoom(0.01), clamp_zoom(9.0)), (0.25, 2.0));
    }
}
