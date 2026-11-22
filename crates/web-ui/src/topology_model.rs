//! Pure helpers of the Topology, PPPoE, and IPoE screens, so native tests cover them.
use web_contract::topology::{
    BuilderEdit, Capacity, Fallback, Node, PageQuery, Plan, Publication, RuleInput, SourceFactor,
    SpeedDefaults,
};

/// Nodes in tree order with their depth. A node whose parent is absent is a root.
/// A parent cycle cannot loop: each node appears once.
pub(crate) fn tree_rows(nodes: &[Node]) -> Vec<(usize, &Node)> {
    let known: std::collections::HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let mut out = Vec::with_capacity(nodes.len());
    let mut seen = std::collections::HashSet::new();
    let mut children: std::collections::HashMap<&str, Vec<&Node>> =
        std::collections::HashMap::new();
    for node in nodes {
        if let Some(parent) = node.parent.as_deref() {
            children.entry(parent).or_default().push(node);
        }
    }
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
        if let Some(branch) = children.get(node.id.as_str()) {
            stack.extend(branch.iter().rev().map(|c| (depth + 1, *c)));
        }
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

/// One logical source may have several queue placements, each with one export parent.
pub(crate) fn shared_parent_names(nodes: &[Node], source: &str) -> Vec<String> {
    let mut names: Vec<_> = nodes
        .iter()
        .filter(|n| n.source_id.as_deref() == Some(source))
        .map(|n| {
            n.parent
                .as_ref()
                .map(|id| {
                    nodes
                        .iter()
                        .find(|p| p.id == *id)
                        .map(|p| p.name.clone())
                        .unwrap_or(id.clone())
                })
                .unwrap_or("Network".into())
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

pub(crate) fn additional_parents(nodes: &[Node], node: &Node) -> Vec<(String, String)> {
    let Some(source) = node.source_id.as_deref() else {
        return Vec::new();
    };
    let used: std::collections::HashSet<_> = nodes
        .iter()
        .filter(|n| n.source_id.as_deref() == Some(source))
        .filter_map(|n| n.parent.as_deref())
        .collect();
    nodes
        .iter()
        .filter(|n| {
            matches!(n.kind.as_str(), "wan" | "intermediate") && !used.contains(n.id.as_str())
        })
        .map(|n| (n.id.clone(), n.name.clone()))
        .collect()
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

/// Evidence strip texts for the latest write: (files/import, class, native, class, note).
/// Saved policy, files, import, and native verification never stand in for each other.
pub(crate) fn publication_labels(
    p: Option<&Publication>,
    saved: Option<&str>,
) -> (String, &'static str, String, &'static str, Option<String>) {
    let Some(p) = p else {
        return (
            "No write yet".into(),
            "neutral",
            "—".into(),
            "neutral",
            None,
        );
    };
    let which = p
        .policy_revision
        .as_ref()
        .map_or("Legacy write".to_string(), |r| format!("r{r}"));
    let (files, files_class) = match p.phase.as_str() {
        "intent" | "pending" | "candidate" => ("files pending", "warn"),
        "import_confirmed" | "applied" | "verify_timeout" => ("import confirmed", "good"),
        "write_failed" => ("write failed", "bad"),
        "apply_failed" => ("import failed", "bad"),
        "superseded" => ("superseded", "neutral"),
        "interrupted" => ("interrupted", "warn"),
        _ => ("unknown state", "warn"),
    };
    let (native, native_class) = match p.phase.as_str() {
        "applied" if p.accepted => ("applied", "good"),
        "import_confirmed" => ("waiting", "warn"),
        "verify_timeout" => ("verification timed out", "warn"),
        "write_failed" | "apply_failed" => ("not applied", "bad"),
        _ => ("not verified", "neutral"),
    };
    let note = match (&p.policy_revision, saved) {
        (Some(published), Some(saved)) if published != saved => Some(format!(
            "Revision {saved} is saved. The latest write published revision {published}; the next service cycle publishes the saved revision."
        )),
        (None, Some(_)) => Some(
            "The legacy writer made the latest write. It does not publish a topology revision."
                .into(),
        ),
        _ => None,
    };
    (
        format!("{which} · {files}"),
        files_class,
        format!("{which} · {native}"),
        native_class,
        note,
    )
}

/// One Builder action on the selected node, or a new node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Change {
    Initialize,
    Create,
    CreateChild(String),
    AttachSource(String, String, Option<String>),
    AddParent(Node),
    Edit(Node),
    Move(Node),
    Remove(Node),
    /// A WAN node on a discovered interface: router ID, router name, interface name.
    CreateWan(String, String, String),
    /// A discovered source: key and name.
    AddSource(String, String),
    CreatePlan,
    EditPlan(Plan),
    DeletePlan(Plan),
    /// One saved subscriber and its current plan. `Draft::parent` holds the chosen plan ID.
    SubscriberPlan(String, Option<String>),
    /// One saved subscriber and its current pin. `Draft::parent` holds the chosen WAN node ID.
    SubscriberPin(String, Option<String>),
    /// One source ID and its label, or all sources with `None`.
    Rebalance(Option<String>, String),
    SpeedDefaults(SpeedDefaults),
    GroupFactor(SourceFactor),
    /// The current fallback, if any.
    SetFallback(Option<Fallback>),
    ClearFallback,
}

/// The four speed rules in form order: download minimum, upload minimum, download maximum,
/// upload maximum.
pub(crate) const RULES: [&str; 4] = [
    "Download minimum",
    "Upload minimum",
    "Download maximum",
    "Upload maximum",
];

/// The form values as typed. The browser checks shape; the service decides every rule.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Draft {
    pub name: String,
    /// A node ID, or empty for the root.
    pub parent: String,
    /// `intermediate` or `group`.
    pub kind: String,
    /// `automatic`, `inherit`, `fixed`, or `wan_percent`.
    pub mode: String,
    pub down: String,
    pub up: String,
    /// Speed rules in `RULES` order: unit (`percent` or `mbps`) and value.
    pub rules: [(String, String); 4],
    /// Fallback node IDs.
    pub nodes: Vec<String>,
}

impl Draft {
    pub(crate) fn of(action: &Change) -> Self {
        let node = match action {
            Change::Create | Change::CreateChild(_) => {
                return Self {
                    kind: "intermediate".into(),
                    mode: "automatic".into(),
                    parent: match action {
                        Change::CreateChild(id) => id.clone(),
                        _ => String::new(),
                    },
                    ..Self::default()
                };
            }
            Change::AttachSource(_, _, parent) => {
                return Self {
                    parent: parent.clone().unwrap_or_default(),
                    ..Self::default()
                };
            }
            Change::AddParent(_) => return Self::default(),
            Change::CreateWan(_, router, interface) => {
                return Self {
                    name: format!("{router} {interface}"),
                    mode: "fixed".into(),
                    ..Self::default()
                };
            }
            Change::Initialize
            | Change::AddSource(..)
            | Change::CreatePlan
            | Change::Rebalance(..) => {
                return Self::default();
            }
            Change::EditPlan(p) | Change::DeletePlan(p) => {
                return Self {
                    name: p.name.clone(),
                    down: p.down_mbps.to_string(),
                    up: p.up_mbps.to_string(),
                    ..Self::default()
                };
            }
            Change::SpeedDefaults(sd) => {
                let r = |x: &web_contract::topology::SpeedRule| {
                    // A legacy rule is offered again as its percent; the review warns.
                    let unit = if x.unit == "mbps" { "mbps" } else { "percent" };
                    (unit.to_string(), x.value.clone())
                };
                return Self {
                    rules: [
                        r(&sd.down_min),
                        r(&sd.up_min),
                        r(&sd.down_max),
                        r(&sd.up_max),
                    ],
                    ..Self::default()
                };
            }
            Change::GroupFactor(f) => {
                return Self {
                    down: f.down.clone(),
                    up: f.up.clone(),
                    ..Self::default()
                };
            }
            Change::SetFallback(f) => {
                return Self {
                    nodes: f.as_ref().map(|f| f.node_ids.clone()).unwrap_or_default(),
                    down: f
                        .as_ref()
                        .map(|f| f.default_mbps[0].to_string())
                        .unwrap_or_default(),
                    up: f
                        .as_ref()
                        .map(|f| f.default_mbps[1].to_string())
                        .unwrap_or_default(),
                    ..Self::default()
                };
            }
            Change::ClearFallback => return Self::default(),
            Change::SubscriberPlan(_, current) | Change::SubscriberPin(_, current) => {
                return Self {
                    parent: current.clone().unwrap_or_default(),
                    ..Self::default()
                };
            }
            Change::Edit(n) | Change::Move(n) | Change::Remove(n) => n,
        };
        let (mode, down, up) = match &node.capacity {
            Capacity::Inherit => ("inherit", String::new(), String::new()),
            Capacity::Automatic => ("automatic", String::new(), String::new()),
            Capacity::Fixed {
                down_mbps, up_mbps, ..
            } => ("fixed", down_mbps.to_string(), up_mbps.to_string()),
            Capacity::WanPercent {
                down_percent,
                up_percent,
            } => (
                "wan_percent",
                down_percent.to_string(),
                up_percent.to_string(),
            ),
        };
        Self {
            name: node.name.clone(),
            parent: node.parent.clone().unwrap_or_default(),
            kind: node.kind.clone(),
            mode: mode.into(),
            down,
            up,
            ..Self::default()
        }
    }
}

fn capacity(d: &Draft, keep_factor: Option<[u64; 2]>) -> Result<Capacity, &'static str> {
    let number = |s: &str| s.trim().parse::<u64>().ok().filter(|n| *n > 0);
    Ok(match d.mode.as_str() {
        "inherit" => Capacity::Inherit,
        "automatic" => Capacity::Automatic,
        "fixed" => match (number(&d.down), number(&d.up)) {
            (Some(down_mbps), Some(up_mbps)) => Capacity::Fixed {
                down_mbps,
                up_mbps,
                factor_percent: keep_factor,
            },
            _ => return Err("Enter download and upload as whole Mbps above 0."),
        },
        "wan_percent" => match (number(&d.down), number(&d.up)) {
            (Some(down_percent), Some(up_percent)) if down_percent <= 100 && up_percent <= 100 => {
                Capacity::WanPercent {
                    down_percent,
                    up_percent,
                }
            }
            _ => return Err("Enter download and upload as whole percents from 1 to 100."),
        },
        _ => return Err("Select a capacity mode."),
    })
}

/// The service edit for one action. An error is a sentence for the form.
pub(crate) fn build_edit(action: &Change, d: &Draft) -> Result<BuilderEdit, &'static str> {
    let name = d.name.trim();
    let parent = (!d.parent.is_empty()).then(|| d.parent.clone());
    Ok(match action {
        Change::Initialize => BuilderEdit::Initialize,
        Change::Create | Change::CreateChild(_) => {
            if name.is_empty() {
                return Err("Enter a node name.");
            }
            BuilderEdit::NodeCreate {
                name: name.into(),
                parent,
                capacity: capacity(d, None)?,
                node_kind: Some(d.kind.clone()),
            }
        }
        Change::Edit(n) => {
            if name.is_empty() {
                return Err("Enter a node name.");
            }
            let factor = match n.capacity {
                Capacity::Fixed { factor_percent, .. } => factor_percent,
                _ => None,
            };
            let capacity = capacity(d, factor)?;
            BuilderEdit::NodeUpdate {
                node_id: n.id.clone(),
                name: (name != n.name).then(|| name.into()),
                capacity: (capacity != n.capacity).then_some(capacity),
            }
        }
        Change::Move(n) => {
            if parent.as_deref() == Some(n.id.as_str()) {
                return Err("A node cannot be its own parent.");
            }
            if parent == n.parent {
                return Err("Select another parent.");
            }
            BuilderEdit::NodeMove {
                node_id: n.id.clone(),
                parent,
            }
        }
        Change::Remove(n) => BuilderEdit::NodeRemove {
            node_id: n.id.clone(),
        },
        Change::CreateWan(router_id, _, interface) => {
            if name.is_empty() {
                return Err("Enter a WAN name.");
            }
            let Capacity::Fixed {
                down_mbps, up_mbps, ..
            } = capacity(
                &Draft {
                    mode: "fixed".into(),
                    ..d.clone()
                },
                None,
            )?
            else {
                return Err("Enter download and upload as whole Mbps above 0.");
            };
            BuilderEdit::WanCreate {
                router_id: router_id.clone(),
                endpoint_ref: interface.clone(),
                name: name.into(),
                down_mbps,
                up_mbps,
                factor_percent: None,
            }
        }
        Change::AddSource(key, _) => BuilderEdit::SourceAdd {
            key: key.clone(),
            parent,
        },
        Change::AttachSource(source_id, _, _) => BuilderEdit::InstanceAdd {
            source_id: source_id.clone(),
            parent,
            name: None,
        },
        Change::AddParent(node) => {
            let source_id = node
                .source_id
                .clone()
                .ok_or("This node has no shared subscriber source.")?;
            if parent.is_none() {
                return Err("Select an additional parent.");
            }
            BuilderEdit::InstanceAdd {
                source_id,
                parent,
                name: None,
            }
        }
        Change::CreatePlan | Change::EditPlan(_) => {
            if name.is_empty() {
                return Err("Enter a plan name.");
            }
            let Capacity::Fixed {
                down_mbps, up_mbps, ..
            } = capacity(
                &Draft {
                    mode: "fixed".into(),
                    ..d.clone()
                },
                None,
            )?
            else {
                return Err("Enter download and upload as whole Mbps above 0.");
            };
            match action {
                Change::EditPlan(p) => {
                    let changed = |new: u64, old: u64| (new != old).then_some(new);
                    let edit = BuilderEdit::PlanUpdate {
                        plan_id: p.id.clone(),
                        name: (name != p.name).then(|| name.into()),
                        down_mbps: changed(down_mbps, p.down_mbps),
                        up_mbps: changed(up_mbps, p.up_mbps),
                    };
                    if edit
                        == (BuilderEdit::PlanUpdate {
                            plan_id: p.id.clone(),
                            name: None,
                            down_mbps: None,
                            up_mbps: None,
                        })
                    {
                        return Err("Change the name or a rate first.");
                    }
                    edit
                }
                _ => BuilderEdit::PlanCreate {
                    name: name.into(),
                    down_mbps,
                    up_mbps,
                },
            }
        }
        Change::DeletePlan(p) => BuilderEdit::PlanDelete {
            plan_id: p.id.clone(),
        },
        Change::SubscriberPlan(id, current) | Change::SubscriberPin(id, current) => {
            if parent == *current {
                return Err("Select another value first.");
            }
            match action {
                Change::SubscriberPlan(..) => BuilderEdit::SubscriberPlan {
                    subscriber_id: id.clone(),
                    plan_id: parent,
                },
                _ => BuilderEdit::SubscriberPin {
                    subscriber_id: id.clone(),
                    wan_node_id: parent,
                },
            }
        }
        Change::Rebalance(source, _) => BuilderEdit::Rebalance {
            source_id: source.clone(),
        },
        Change::SpeedDefaults(_) => {
            let mut out = [RuleInput::Mbps(0); 4];
            for (i, (unit, value)) in d.rules.iter().enumerate() {
                let value = value.trim();
                out[i] = match unit.as_str() {
                    "mbps" => match value.parse::<u64>() {
                        Ok(n) if n > 0 => RuleInput::Mbps(n),
                        _ => return Err("Enter fixed rates as whole Mbps above 0."),
                    },
                    _ => match value.parse::<f64>() {
                        Ok(p) if p.is_finite() && p > 0.0 => RuleInput::Percent(p),
                        _ => {
                            return Err(
                                "Enter percents as numbers above 0, with one decimal at most.",
                            );
                        }
                    },
                };
            }
            BuilderEdit::SpeedDefaults {
                down_min: out[0],
                up_min: out[1],
                down_max: out[2],
                up_max: out[3],
            }
        }
        Change::GroupFactor(f) => {
            let changed = |new: &str, old: &str| {
                let new = new.trim();
                (new != old).then(|| new.to_string())
            };
            let (down, up) = (changed(&d.down, &f.down), changed(&d.up, &f.up));
            if down.is_none() && up.is_none() {
                return Err("Change a factor first.");
            }
            BuilderEdit::GroupFactor {
                source_id: f.source_id.clone(),
                down,
                up,
            }
        }
        Change::SetFallback(_) => {
            if d.nodes.is_empty() {
                return Err("Select at least one fallback node.");
            }
            let Capacity::Fixed {
                down_mbps, up_mbps, ..
            } = capacity(
                &Draft {
                    mode: "fixed".into(),
                    ..d.clone()
                },
                None,
            )?
            else {
                return Err("Enter download and upload as whole Mbps above 0.");
            };
            BuilderEdit::FallbackSet {
                node_ids: d.nodes.clone(),
                default_down_mbps: down_mbps,
                default_up_mbps: up_mbps,
            }
        }
        Change::ClearFallback => BuilderEdit::FallbackClear,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_labels_keep_each_fact_apart() {
        let write = |phase: &str, revision: Option<&str>, accepted: bool| Publication {
            write_id: "w".into(),
            phase: phase.into(),
            policy_revision: revision.map(str::to_owned),
            accepted,
            ..Publication::default()
        };
        assert_eq!(publication_labels(None, Some("7")).0, "No write yet");
        let (files, _, native, class, note) = publication_labels(
            Some(&write("import_confirmed", Some("7"), false)),
            Some("7"),
        );
        assert_eq!(
            (files.as_str(), native.as_str(), class),
            ("r7 · import confirmed", "r7 · waiting", "warn")
        );
        assert_eq!(note, None);
        // An import is not an application; an applied write needs native acceptance.
        let (_, _, native, _, _) =
            publication_labels(Some(&write("applied", Some("7"), false)), Some("7"));
        assert_eq!(native, "r7 · not verified");
        let (_, _, native, class, note) =
            publication_labels(Some(&write("applied", Some("7"), true)), Some("8"));
        assert_eq!((native.as_str(), class), ("r7 · applied", "good"));
        assert!(note.unwrap().starts_with("Revision 8 is saved."));
        let (files, class, _, _, _) =
            publication_labels(Some(&write("write_failed", Some("7"), false)), Some("7"));
        assert_eq!((files.as_str(), class), ("r7 · write failed", "bad"));
        let (files, _, _, _, note) =
            publication_labels(Some(&write("applied", None, true)), Some("7"));
        assert_eq!(files, "Legacy write · import confirmed");
        assert!(note.unwrap().contains("legacy writer"));
    }

    #[test]
    fn drafts_make_the_service_edit_or_a_form_error() {
        let wan = Node {
            capacity: Capacity::Fixed {
                down_mbps: 1000,
                up_mbps: 500,
                factor_percent: Some([50, 50]),
            },
            ..node("wan1", None)
        };
        let mut d = Draft::of(&Change::Create);
        assert_eq!(build_edit(&Change::Create, &d), Err("Enter a node name."));
        d.name = " Tower 1 ".into();
        d.parent = "wan1".into();
        assert_eq!(
            build_edit(&Change::Create, &d),
            Ok(BuilderEdit::NodeCreate {
                name: "Tower 1".into(),
                parent: Some("wan1".into()),
                capacity: Capacity::Automatic,
                node_kind: Some("intermediate".into()),
            })
        );
        d.mode = "wan_percent".into();
        d.down = "120".into();
        d.up = "50".into();
        assert!(build_edit(&Change::Create, &d).is_err());
        // An edit sends only what changed and keeps the factor override.
        let edit = Change::Edit(wan.clone());
        let mut d = Draft::of(&edit);
        assert_eq!((d.mode.as_str(), d.down.as_str()), ("fixed", "1000"));
        d.down = "800".into();
        assert_eq!(
            build_edit(&edit, &d),
            Ok(BuilderEdit::NodeUpdate {
                node_id: "wan1".into(),
                name: None,
                capacity: Some(Capacity::Fixed {
                    down_mbps: 800,
                    up_mbps: 500,
                    factor_percent: Some([50, 50]),
                }),
            })
        );
        d.down = "0".into();
        assert!(build_edit(&edit, &d).is_err());
        // A move needs another parent.
        let g = node("g1", Some("wan1"));
        let mv = Change::Move(g.clone());
        let mut d = Draft::of(&mv);
        assert_eq!(build_edit(&mv, &d), Err("Select another parent."));
        d.parent = String::new();
        assert_eq!(
            build_edit(&mv, &d),
            Ok(BuilderEdit::NodeMove {
                node_id: "g1".into(),
                parent: None
            })
        );
        d.parent = "g1".into();
        assert!(build_edit(&mv, &d).is_err());
        let wan = Change::CreateWan("lq3-c1".into(), "Core".into(), "pppoe-out7".into());
        let mut d = Draft::of(&wan);
        assert_eq!(d.name, "Core pppoe-out7");
        assert!(build_edit(&wan, &d).is_err(), "rates are required");
        d.down = "500".into();
        d.up = "100".into();
        assert_eq!(
            build_edit(&wan, &d),
            Ok(BuilderEdit::WanCreate {
                router_id: "lq3-c1".into(),
                endpoint_ref: "pppoe-out7".into(),
                name: "Core pppoe-out7".into(),
                down_mbps: 500,
                up_mbps: 100,
                factor_percent: None,
            })
        );
        let add = Change::AddSource("access-ppp".into(), "PPPoE Edge".into());
        let mut d = Draft::of(&add);
        d.parent = "wan1".into();
        assert_eq!(
            build_edit(&add, &d),
            Ok(BuilderEdit::SourceAdd {
                key: "access-ppp".into(),
                parent: Some("wan1".into())
            })
        );
        // Plans: a create needs rates; an edit sends only what changed.
        let mut d = Draft::of(&Change::CreatePlan);
        d.name = "Access 100".into();
        assert!(build_edit(&Change::CreatePlan, &d).is_err());
        d.down = "100".into();
        d.up = "50".into();
        assert_eq!(
            build_edit(&Change::CreatePlan, &d),
            Ok(BuilderEdit::PlanCreate {
                name: "Access 100".into(),
                down_mbps: 100,
                up_mbps: 50
            })
        );
        let plan = Plan {
            id: "plan-1".into(),
            name: "Access 100".into(),
            down_mbps: 100,
            up_mbps: 50,
            ppp_identities: 1,
            dhcp_identities: 0,
            subscribers: 1,
        };
        let edit = Change::EditPlan(plan.clone());
        let mut d = Draft::of(&edit);
        assert_eq!(
            build_edit(&edit, &d),
            Err("Change the name or a rate first.")
        );
        d.up = "60".into();
        assert_eq!(
            build_edit(&edit, &d),
            Ok(BuilderEdit::PlanUpdate {
                plan_id: "plan-1".into(),
                name: None,
                down_mbps: None,
                up_mbps: Some(60)
            })
        );
        assert_eq!(
            build_edit(&Change::DeletePlan(plan), &Draft::default()),
            Ok(BuilderEdit::PlanDelete {
                plan_id: "plan-1".into()
            })
        );
        // A subscriber plan and a pin address the stable ID; an empty choice clears.
        let choose = Change::SubscriberPlan("tm-sub-1".into(), Some("plan-1".into()));
        let mut d = Draft::of(&choose);
        assert_eq!(build_edit(&choose, &d), Err("Select another value first."));
        d.parent = String::new();
        assert_eq!(
            build_edit(&choose, &d),
            Ok(BuilderEdit::SubscriberPlan {
                subscriber_id: "tm-sub-1".into(),
                plan_id: None
            })
        );
        let pin = Change::SubscriberPin("tm-sub-1".into(), None);
        let mut d = Draft::of(&pin);
        d.parent = "wan1".into();
        assert_eq!(
            build_edit(&pin, &d),
            Ok(BuilderEdit::SubscriberPin {
                subscriber_id: "tm-sub-1".into(),
                wan_node_id: Some("wan1".into())
            })
        );
        assert_eq!(
            build_edit(
                &Change::Rebalance(None, "All sources".into()),
                &Draft::default()
            ),
            Ok(BuilderEdit::Rebalance { source_id: None })
        );
        // Speed rules keep each unit; a bad value stays in the form.
        let rule = |unit: &str, value: &str| web_contract::topology::SpeedRule {
            unit: unit.into(),
            value: value.into(),
        };
        let speed = Change::SpeedDefaults(SpeedDefaults {
            down_min: rule("percent", "20.5"),
            up_min: rule("mbps", "5"),
            down_max: rule("legacy", "100"),
            up_max: rule("percent", "100"),
            overrides: 2,
        });
        let mut d = Draft::of(&speed);
        // The form rows follow RULES order; row 2 is the download maximum.
        assert_eq!(RULES[2], "Download maximum");
        assert_eq!(d.rules[2], ("percent".to_string(), "100".to_string()));
        assert_eq!(
            build_edit(&speed, &d),
            Ok(BuilderEdit::SpeedDefaults {
                down_min: RuleInput::Percent(20.5),
                up_min: RuleInput::Mbps(5),
                down_max: RuleInput::Percent(100.0),
                up_max: RuleInput::Percent(100.0),
            })
        );
        d.rules[1].1 = "five".into();
        assert!(build_edit(&speed, &d).is_err());
        d.rules[0].1 = "NaN".into();
        assert!(build_edit(&speed, &d).is_err());
        // A factor edit sends only the changed direction.
        let factor = Change::GroupFactor(SourceFactor {
            source_id: "src".into(),
            name: "PPPoE".into(),
            down: "1".into(),
            up: "1".into(),
            allocation: "capacity".into(),
        });
        let mut d = Draft::of(&factor);
        assert_eq!(build_edit(&factor, &d), Err("Change a factor first."));
        d.down = "0.5".into();
        assert_eq!(
            build_edit(&factor, &d),
            Ok(BuilderEdit::GroupFactor {
                source_id: "src".into(),
                down: Some("0.5".into()),
                up: None
            })
        );
        // Fallback needs nodes and rates.
        let set = Change::SetFallback(None);
        let mut d = Draft::of(&set);
        d.down = "10".into();
        d.up = "5".into();
        assert!(build_edit(&set, &d).is_err());
        d.nodes = vec!["g1".into()];
        assert_eq!(
            build_edit(&set, &d),
            Ok(BuilderEdit::FallbackSet {
                node_ids: vec!["g1".into()],
                default_down_mbps: 10,
                default_up_mbps: 5
            })
        );
        assert_eq!(
            build_edit(&Change::ClearFallback, &Draft::default()),
            Ok(BuilderEdit::FallbackClear)
        );
        assert_eq!(
            build_edit(&Change::Remove(g), &Draft::default()),
            Ok(BuilderEdit::NodeRemove {
                node_id: "g1".into()
            })
        );
    }

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
            geo: None,
            position: None,
            router_id: None,
            auto_members: None,
            wan_health: None,
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
    fn contextual_node_and_instance_actions_keep_the_selected_parent() {
        let action = Change::CreateChild("wan".into());
        let mut draft = Draft::of(&action);
        draft.name = "Access".into();
        assert!(
            matches!(build_edit(&action, &draft).unwrap(), BuilderEdit::NodeCreate { parent: Some(p), .. } if p == "wan")
        );
        let action = Change::AttachSource("src".into(), "PPPoE".into(), Some("wan".into()));
        let draft = Draft::of(&action);
        assert_eq!(
            build_edit(&action, &draft).unwrap(),
            BuilderEdit::InstanceAdd {
                source_id: "src".into(),
                parent: Some("wan".into()),
                name: None
            }
        );
    }

    #[test]
    fn initialization_is_an_explicit_empty_edit() {
        assert_eq!(
            build_edit(&Change::Initialize, &Draft::of(&Change::Initialize)),
            Ok(BuilderEdit::Initialize)
        );
        assert_eq!(
            serde_json::to_value(BuilderEdit::Initialize).unwrap(),
            serde_json::json!({"kind":"topology.initialize"})
        );
    }

    #[test]
    fn shared_source_can_add_a_second_parent_without_moving_the_first() {
        let mut first = node("source-at-north", Some("north"));
        first.source_id = Some("shared-source".into());
        let mut north = node("north", None);
        north.kind = "wan".into();
        let mut south = node("south", None);
        south.kind = "wan".into();
        let all = vec![north, south, first.clone()];
        assert_eq!(
            additional_parents(&all, &first),
            vec![("south".into(), "south".into())]
        );
        let change = Change::AddParent(first.clone());
        let mut d = Draft::of(&change);
        assert!(build_edit(&change, &d).is_err());
        d.parent = "south".into();
        assert_eq!(
            build_edit(&change, &d).unwrap(),
            BuilderEdit::InstanceAdd {
                source_id: "shared-source".into(),
                parent: Some("south".into()),
                name: None
            }
        );
        assert_eq!(first.parent.as_deref(), Some("north"));
        assert_eq!(shared_parent_names(&all, "shared-source"), vec!["north"]);
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
