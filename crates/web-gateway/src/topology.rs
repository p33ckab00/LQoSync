//! Topology reads from the service cache `topology.json`. Reading it starts no router I/O.
//! Filters run before the page cut; a cursor binds the snapshot, the filter, and the offset.
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::path::Path;
use web_contract::topology::*;

/// The topology part of a broker reply stays under this size. The reply limit is 64 KiB.
pub const BUDGET: usize = 60 * 1024;
/// The service cache grows with the subscriber count: about 400 bytes per subscriber.
const MAX_CACHE: u64 = 64 * 1024 * 1024;
/// Web changes in the summary, newest first.
const HISTORY: usize = 20;
/// The summary keeps this much room for the first conflicts.
const CONFLICT_BUDGET: usize = 8 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Read {
    Summary,
    Assignments(PageQuery),
    Observations(PageQuery),
    Subscriber(String),
}

impl Read {
    /// Bounded browser input. Longer values are not IDs that the service writes.
    pub fn valid(&self) -> bool {
        let short = |v: &Option<String>| v.as_ref().is_none_or(|s| s.len() <= 128);
        match self {
            Read::Summary => true,
            Read::Subscriber(id) => !id.is_empty() && id.len() <= 128,
            Read::Assignments(q) | Read::Observations(q) => [
                &q.kind,
                &q.router_id,
                &q.state,
                &q.link,
                &q.plan_id,
                &q.q,
                &q.cursor,
            ]
            .into_iter()
            .all(short),
        }
    }
}

/// The cache, or `None` when it is absent, too large, or another version.
// shortcut: parses the whole file on each read; cache it by mtime if 50k-subscriber reads are slow.
pub fn load(root: &Path) -> Option<TopologyView> {
    let view: TopologyView = crate::overview::read_within(root, CACHE, MAX_CACHE)?;
    (view.version == VERSION && view.updated > 0).then_some(view)
}

pub fn answer(view: &TopologyView, read: &Read) -> TopologyReply {
    match read {
        Read::Summary => TopologyReply::Summary(Box::new(summary(view))),
        Read::Assignments(q) => {
            let rows = view.subscribers.iter().filter(|s| subscriber_matches(s, q));
            match page(view, q, rows) {
                Some(p) => TopologyReply::Assignments(p),
                None => TopologyReply::Expired,
            }
        }
        Read::Observations(q) => {
            let rows = view
                .observations
                .iter()
                .filter(|o| observation_matches(o, q));
            match page(view, q, rows) {
                Some(p) => TopologyReply::Observations(p),
                None => TopologyReply::Expired,
            }
        }
        Read::Subscriber(id) => match view.subscribers.iter().find(|s| s.id == *id) {
            Some(s) => TopologyReply::Subscriber(Box::new(SubscriberDetail {
                subscriber: s.clone(),
                observations: view
                    .observations
                    .iter()
                    .filter(|o| o.subscriber_id.as_ref() == Some(id))
                    .take(MAX_PAGE)
                    .cloned()
                    .collect(),
            })),
            None => TopologyReply::NotFound,
        },
    }
}

fn size<T: Serialize>(value: &T) -> usize {
    serde_json::to_vec(value).map_or(usize::MAX, |b| b.len())
}

fn summary(v: &TopologyView) -> TopologySummary {
    let count = |kind: &str| v.observations.iter().filter(|o| o.kind == kind).count() as u64;
    let mut s = TopologySummary {
        updated: v.updated,
        fresh_for: v.fresh_for,
        snapshot_id: v.snapshot_id.clone(),
        registry_revision: v.registry_revision.clone(),
        mode: v.mode.clone(),
        writer: v.writer.clone(),
        enrolled: v.enrolled,
        observed: v.observed.clone(),
        nodes: Vec::new(),
        nodes_truncated: false,
        sources: v.sources.clone(),
        fallback: v.fallback.clone(),
        plans: v.plans.clone(),
        collection: v.collection.clone(),
        conflicts: Vec::new(),
        conflicts_total: v.conflicts.len() as u64,
        subscribers_total: v.subscribers.len() as u64,
        subscribers_active: v.subscribers.iter().filter(|s| s.active.is_some()).count() as u64,
        ppp_total: count("ppp"),
        dhcp_total: count("dhcp"),
        speed: v.speed.clone(),
        factors: v.factors.clone(),
        publication: v.publication.clone(),
        history: v.history.iter().take(HISTORY).cloned().collect(),
    };
    // Envelope: `{"summary":{...}}`.
    let mut used = size(&s) + 16;
    let mut room = CONFLICT_BUDGET;
    for c in &v.conflicts {
        let n = size(c) + 1;
        if n > room {
            break;
        }
        room -= n;
        used += n;
        s.conflicts.push(c.clone());
    }
    for node in &v.nodes {
        let n = size(node) + 1;
        if used + n > BUDGET {
            s.nodes_truncated = true;
            break;
        }
        used += n;
        s.nodes.push(node.clone());
    }
    s
}

fn has(field: &Option<String>, value: &str) -> bool {
    field.as_ref().is_none_or(|f| f == value)
}

fn text(q: &PageQuery, values: &[&str]) -> bool {
    q.q.as_ref().is_none_or(|t| {
        let t = t.to_lowercase();
        values.iter().any(|v| v.to_lowercase().contains(&t))
    })
}

fn observation_matches(o: &Observation, q: &PageQuery) -> bool {
    has(&q.kind, &o.kind)
        && has(&q.router_id, &o.router_id)
        && has(&q.state, &o.state)
        && has(&q.link, &o.link)
        && text(
            q,
            &[&o.name, o.subscriber_id.as_deref().unwrap_or_default()],
        )
}

fn subscriber_matches(s: &Subscriber, q: &PageQuery) -> bool {
    let state = if s.active.is_some() {
        "active"
    } else {
        "offline"
    };
    let refs: Vec<&str> = s
        .identities
        .iter()
        .map(|i| i.account_ref.as_str())
        .collect();
    has(&q.state, state)
        && q.plan_id
            .as_ref()
            .is_none_or(|p| s.plan_id.as_ref() == Some(p))
        && q.router_id
            .as_ref()
            .is_none_or(|r| s.identities.iter().any(|i| i.router_id == *r))
        && q.kind
            .as_ref()
            .is_none_or(|k| s.identities.iter().any(|i| i.kind == *k))
        && text(q, &[&[s.id.as_str()][..], &refs].concat())
}

/// The filter without its cursor, as a short stable digest.
fn filter_digest(q: &PageQuery) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    PageQuery {
        cursor: None,
        ..q.clone()
    }
    .hash(&mut h);
    h.finish()
}

/// One page in service order. `None`: the cursor belongs to another snapshot or filter.
fn page<'a, T: Serialize + Clone + 'a>(
    view: &TopologyView,
    q: &PageQuery,
    rows: impl Iterator<Item = &'a T>,
) -> Option<Page<T>> {
    let digest = filter_digest(q);
    let start = match &q.cursor {
        None => 0,
        Some(c) => {
            let mut parts = c.rsplitn(3, '~');
            let offset: usize = parts.next()?.parse().ok()?;
            let filter = u64::from_str_radix(parts.next()?, 16).ok()?;
            (parts.next()? == view.snapshot_id && filter == digest).then_some(offset)?
        }
    };
    let rows: Vec<&T> = rows.collect();
    let mut out = Page {
        snapshot_id: view.snapshot_id.clone(),
        total: rows.len() as u64,
        items: Vec::new(),
        next_cursor: None,
    };
    // Envelope, snapshot ID, and the longest cursor.
    let mut used = size(&out) + 2 * view.snapshot_id.len() + 64;
    for (i, row) in rows.iter().enumerate().skip(start) {
        let n = size(row) + 1;
        if out.items.len() == MAX_PAGE || used + n > BUDGET {
            out.next_cursor = Some(format!("{}~{digest:016x}~{i}", view.snapshot_id));
            break;
        }
        used += n;
        out.items.push((*row).clone());
    }
    Some(out)
}
