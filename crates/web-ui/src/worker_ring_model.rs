//! Geometry and data for the worker execution ring. Pure functions, so native tests cover them.
use web_contract::dpdk::{Phase, Worker};

/// The ring shows these phases clockwise from the top, as in the reviewed SVG reference.
/// "Empty RX" is idle polling, so the ring leaves it out. Each phase has an equal angle.
pub(crate) const RING: [(&str, &str, &str); 9] = [
    ("RX", "RX", "Receive processing"),
    ("Dissect", "Dissect", "Packet classification and dissection"),
    ("Select", "Select", "Queue selection"),
    ("Leaf", "Leaf", "Leaf queue processing"),
    ("TX", "TX", "Transmit processing"),
    ("Begin cycle", "Begin", "Loop begin accounting"),
    ("Drop overdue", "Drop overdue", "Overdue drop processing"),
    ("Charge", "Charge", "Charge stage"),
    ("Handoff", "Handoff", "Handoff stage"),
];
pub(crate) const CX: f64 = 260.0;
pub(crate) const CY: f64 = 196.0;
pub(crate) const RADIUS: f64 = 112.0;
const GAP_DEG: f64 = 4.0;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Sector {
    pub index: usize,
    pub label: &'static str,
    pub description: &'static str,
    /// `None` when the source has no value for this phase.
    pub cycles: Option<u64>,
    pub arc: String,
    /// Callout: anchor on the ring, elbow, label end, and text side.
    pub anchor: (f64, f64),
    pub elbow: (f64, f64),
    pub end: (f64, f64),
    pub left: bool,
}

fn point(deg: f64, r: f64) -> (f64, f64) {
    let rad = (deg - 90.0).to_radians();
    (CX + r * rad.cos(), CY + r * rad.sin())
}
fn round(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// The SVG arc of sector `i`: equal angles, a small gap, clockwise from the top.
pub(crate) fn arc(i: usize) -> String {
    let step = 360.0 / RING.len() as f64;
    let start = step * i as f64 + GAP_DEG / 2.0;
    let end = step * (i + 1) as f64 - GAP_DEG / 2.0;
    let (x1, y1) = point(start, RADIUS);
    let (x2, y2) = point(end, RADIUS);
    format!(
        "M{} {} A{RADIUS} {RADIUS} 0 0 1 {} {}",
        round(x1),
        round(y1),
        round(x2),
        round(y2)
    )
}

pub(crate) fn sectors(w: Option<&Worker>) -> Vec<Sector> {
    let find = |name: &str| -> Option<u64> {
        w.and_then(|w| w.phases.iter().find(|p: &&Phase| p.name == name))
            .map(|p| p.cycles)
    };
    let step = 360.0 / RING.len() as f64;
    RING.iter()
        .enumerate()
        .map(|(i, (source, label, description))| {
            let mid = step * (i as f64 + 0.5);
            let anchor = point(mid, RADIUS + 22.0);
            let elbow = point(mid, RADIUS + 44.0);
            let left = mid > 180.0;
            let end = (if left { 70.0 } else { 450.0 }, elbow.1);
            Sector {
                index: i,
                label,
                description,
                cycles: find(source),
                arc: arc(i),
                anchor: (round(anchor.0), round(anchor.1)),
                elbow: (round(elbow.0), round(elbow.1)),
                end: (end.0, round(end.1)),
                left,
            }
        })
        .collect()
}

/// Exact grouped count for the focused phase, for example `1,512,653,783,663`.
pub(crate) fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Short count for a callout, for example `1.51T`. The exact value stays in the focused view.
pub(crate) fn compact(n: u64) -> String {
    const UNITS: [(f64, &str); 5] = [(1e15, "P"), (1e12, "T"), (1e9, "G"), (1e6, "M"), (1e3, "K")];
    let v = n as f64;
    for (size, unit) in UNITS {
        if v >= size {
            let scaled = v / size;
            return if scaled >= 100.0 {
                format!("{scaled:.0}{unit}")
            } else {
                format!("{scaled:.2}{unit}")
            };
        }
    }
    n.to_string()
}

/// The busiest worker first, as the gateway ranks them. `selected` keeps a chosen ID while it exists.
pub(crate) fn pick(workers: &[Worker], selected: Option<u16>) -> Option<&Worker> {
    selected
        .and_then(|id| workers.iter().find(|w| w.id == id))
        .or_else(|| workers.first())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn worker(id: u16, phases: &[(&str, u64)]) -> Worker {
        Worker {
            id,
            phases: phases
                .iter()
                .map(|(name, cycles)| Phase {
                    name: (*name).into(),
                    cycles: *cycles,
                })
                .collect(),
            ..Default::default()
        }
    }
    #[test]
    fn nine_equal_sectors_follow_the_reference_order_and_skip_empty_rx() {
        let w = worker(
            0,
            &[
                ("RX", 1),
                ("Empty RX", 99),
                ("Begin cycle", 7),
                ("Charge", 0),
            ],
        );
        let s = sectors(Some(&w));
        assert_eq!(s.len(), 9);
        let labels: Vec<_> = s.iter().map(|s| s.label).collect();
        assert_eq!(
            labels,
            [
                "RX",
                "Dissect",
                "Select",
                "Leaf",
                "TX",
                "Begin",
                "Drop overdue",
                "Charge",
                "Handoff"
            ]
        );
        assert_eq!(s[0].cycles, Some(1));
        assert_eq!(s[5].cycles, Some(7));
        // A reported zero stays zero. A missing phase stays unknown.
        assert_eq!(s[7].cycles, Some(0));
        assert_eq!(s[1].cycles, None);
        assert!(sectors(None).iter().all(|s| s.cycles.is_none()));
    }
    #[test]
    fn arcs_are_clockwise_on_the_ring_with_a_gap() {
        // The first sector starts just right of the top and ends before 40 degrees.
        assert_eq!(arc(0), "M263.91 84.07 A112 112 0 0 1 328.95 107.74");
        for i in 0..9 {
            let s = arc(i);
            assert!(s.contains(" A112 112 0 0 1 "), "{s}");
        }
        // The callouts of the right half point right, the left half point left.
        let s = sectors(None);
        assert!(!s[0].left && s[8].left);
        assert!(s.iter().filter(|s| s.left).all(|s| s.end.0 < CX));
    }
    #[test]
    fn counts_are_exact_when_focused_and_short_in_callouts() {
        assert_eq!(grouped(1_512_653_783_663), "1,512,653,783,663");
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(compact(1_512_653_783_663), "1.51T");
        assert_eq!(compact(10_857_240_998_207), "10.86T");
        assert_eq!(compact(534_643_071_037), "535G");
        assert_eq!(compact(950), "950");
        assert_eq!(compact(u64::MAX), "18447P");
    }
    #[test]
    fn the_selected_worker_stays_while_it_exists() {
        let ws = [worker(3, &[]), worker(1, &[])];
        assert_eq!(pick(&ws, None).map(|w| w.id), Some(3));
        assert_eq!(pick(&ws, Some(1)).map(|w| w.id), Some(1));
        assert_eq!(pick(&ws, Some(9)).map(|w| w.id), Some(3));
        assert!(pick(&[], Some(1)).is_none());
    }
}
