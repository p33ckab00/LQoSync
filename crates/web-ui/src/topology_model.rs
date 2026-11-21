//! Pure helpers of the Topology, PPPoE, and IPoE screens, so native tests cover them.
use web_contract::topology::{Capacity, Node, PageQuery};

/// Nodes in tree order with their depth. A node whose parent is absent is a root.
/// A parent cycle cannot loop: each node appears once.
pub(crate) fn tree_rows(nodes: &[Node]) -> Vec<(usize, &Node)> {
    let known: std::collections::HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let mut out = Vec::with_capacity(nodes.len());
    let mut seen = std::collections::HashSet::new();
    let mut stack: Vec<(usize, &Node)> = nodes
        .iter()
        .rev()
        .filter(|n| n.parent.as_deref().is_none_or(|p| !known.contains(p)))
        .map(|n| (0, n))
        .collect();
    while let Some((depth, node)) = stack.pop() {
        if !seen.insert(node.id.as_str()) {
            continue;
        }
        out.push((depth, node));
        stack.extend(
            nodes
                .iter()
                .rev()
                .filter(|c| c.parent.as_deref() == Some(node.id.as_str()))
                .map(|c| (depth + 1, c)),
        );
    }
    // Nodes in a parent cycle have no root; show them last instead of hiding them.
    out.extend(
        nodes
            .iter()
            .filter(|n| !seen.contains(n.id.as_str()))
            .map(|n| (0, n)),
    );
    out
}

pub(crate) fn kind_label(kind: &str) -> &'static str {
    match kind {
        "wan" => "WAN",
        "intermediate" => "Intermediate",
        "group" => "Subscriber group",
        _ => "Node",
    }
}

pub(crate) fn capacity_label(c: &Capacity) -> String {
    match c {
        Capacity::Inherit => "Inherit from parent".into(),
        Capacity::Automatic => "Automatic · plan sum".into(),
        Capacity::Fixed {
            down_mbps,
            up_mbps,
            factor_percent,
        } => {
            let factor = factor_percent
                .map(|[d, u]| format!(" · factor {d}% / {u}%"))
                .unwrap_or_default();
            format!("Fixed · {down_mbps} / {up_mbps} Mbps{factor}")
        }
        Capacity::WanPercent {
            down_percent,
            up_percent,
        } => format!("WAN share · {down_percent}% / {up_percent}%"),
    }
}

pub(crate) fn mbps(v: Option<[f64; 2]>) -> String {
    v.map(|[d, u]| format!("{} / {} Mbps", trim(d), trim(u)))
        .unwrap_or("—".into())
}

fn trim(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    }
}

pub(crate) fn age(now: u64, at: u64) -> String {
    if at == 0 {
        return "never".into();
    }
    match now.saturating_sub(at) {
        s if s < 120 => format!("{s}s ago"),
        s if s < 7200 => format!("{}m ago", s / 60),
        s => format!("{}h ago", s / 3600),
    }
}

/// The cache is older than its own freshness limit.
pub(crate) fn stale(now: u64, updated: u64, fresh_for: u64) -> bool {
    now > updated.saturating_add(fresh_for)
}

pub(crate) fn state_label(state: &str) -> &'static str {
    match state {
        "online" => "Online",
        "offline" => "Offline",
        "bound" => "Bound",
        "unbound" => "Unbound",
        "disabled" => "Disabled",
        "stale" => "Stale",
        _ => "Unknown",
    }
}

pub(crate) fn link_label(link: &str) -> &'static str {
    match link {
        "verified" => "Verified link",
        "none" => "Observation only",
        _ => "Link not decided",
    }
}

fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// A GET URL with the non-empty filters of `q`.
pub(crate) fn page_url(base: &str, q: &PageQuery) -> String {
    let fields = [
        ("kind", &q.kind),
        ("router_id", &q.router_id),
        ("state", &q.state),
        ("link", &q.link),
        ("plan_id", &q.plan_id),
        ("q", &q.q),
        ("cursor", &q.cursor),
    ];
    let query: Vec<String> = fields
        .iter()
        .filter_map(|(k, v)| {
            v.as_deref()
                .filter(|v| !v.is_empty())
                .map(|v| format!("{k}={}", encode(v)))
        })
        .collect();
    if query.is_empty() {
        base.into()
    } else {
        format!("{base}?{}", query.join("&"))
    }
}

/// The account part of an identity reference, for display: `ppp-name/v1/alice` gives `alice`.
pub(crate) fn account(account_ref: &str) -> String {
    let tail = account_ref.rsplit('/').next().unwrap_or(account_ref);
    let bytes = tail.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, parent: Option<&str>) -> Node {
        Node {
            id: id.into(),
            name: id.into(),
            parent: parent.map(str::to_owned),
            kind: "group".into(),
            source_id: None,
            capacity: Capacity::Automatic,
            alias: false,
            calculated_mbps: None,
            exported_mbps: None,
            members: None,
        }
    }

    #[test]
    fn tree_rows_keep_catalog_order_and_survive_cycles() {
        let nodes = [
            node("wan1", None),
            node("wan2", None),
            node("g1", Some("wan1")),
            node("g2", Some("wan2")),
            node("lost", Some("gone")),
            node("a", Some("b")),
            node("b", Some("a")),
        ];
        let rows: Vec<(usize, &str)> = tree_rows(&nodes)
            .into_iter()
            .map(|(d, n)| (d, n.id.as_str()))
            .collect();
        assert_eq!(
            rows,
            [
                (0, "wan1"),
                (1, "g1"),
                (0, "wan2"),
                (1, "g2"),
                (0, "lost"),
                (0, "a"),
                (0, "b")
            ]
        );
    }

    #[test]
    fn labels_and_urls() {
        assert_eq!(
            capacity_label(&Capacity::Fixed {
                down_mbps: 400,
                up_mbps: 200,
                factor_percent: Some([50, 50])
            }),
            "Fixed · 400 / 200 Mbps · factor 50% / 50%"
        );
        assert_eq!(mbps(Some([400.0, 12.5])), "400 / 12.5 Mbps");
        assert_eq!(mbps(None), "—");
        assert_eq!(kind_label("group"), "Subscriber group");
        assert_eq!(state_label("bound"), "Bound");
        assert_eq!(state_label("x"), "Unknown");
        assert_eq!(link_label("verified"), "Verified link");
        assert_eq!(link_label("unknown"), "Link not decided");
        assert_eq!(age(100, 92), "8s ago");
        assert_eq!(age(10_000, 0), "never");
        assert!(stale(200, 100, 60) && !stale(150, 100, 60));
        let q = PageQuery {
            kind: Some("ppp".into()),
            q: Some("a b&c".into()),
            state: Some(String::new()),
            ..Default::default()
        };
        assert_eq!(
            page_url("/api/v1/topology/observations", &q),
            "/api/v1/topology/observations?kind=ppp&q=a%20b%26c"
        );
        assert_eq!(page_url("/x", &PageQuery::default()), "/x");
        assert_eq!(account("ppp-name/v1/al%20ice"), "al ice");
        assert_eq!(
            account("dhcp-mac/v1/02:00:00:00:00:01"),
            "02:00:00:00:00:01"
        );
    }
}
