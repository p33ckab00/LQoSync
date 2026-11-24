//! Studio candidates in memory: actor-scoped, bounded, and lost at a process restart.
//! A candidate is a display result. The save path validates the current inputs again.
use super::evaluate::Evaluation;
use std::collections::VecDeque;
use web_contract::source_studio::{
    CANDIDATE_TTL_SECS, CandidatePage, CandidateRef, CandidateSummary, IssueCounts,
    MAX_CANDIDATES_PER_ACTOR, PAGE_MAX, ResolvedRecord, Severity, ValidationIssue,
};

const ACTOR_BYTES: usize = 64 * 1024 * 1024;
const TOTAL_BYTES: usize = 128 * 1024 * 1024;
/// A page ends early at this size, inside the 60 KiB reply budget.
const PAGE_BYTES: usize = 48 * 1024;

struct Entry {
    actor: String,
    summary: CandidateSummary,
    records: Vec<ResolvedRecord>,
    issues: Vec<ValidationIssue>,
    bytes: usize,
}

/// The answer to a candidate read.
#[derive(Debug, PartialEq)]
pub enum Lookup<T> {
    Found(T),
    /// No candidate, or the candidate of another actor: the same answer for both.
    NotFound,
    /// This actor's candidate expired, or a process restart dropped it.
    Expired,
    /// The cursor belongs to another candidate, actor, draft, or list.
    BadCursor,
}

pub struct Cache {
    /// Changes at each process start, so that an old ID reads as expired after a restart.
    boot: String,
    entries: VecDeque<Entry>,
    seq: u64,
}

impl Cache {
    pub fn new(boot: String) -> Self {
        Self {
            boot,
            entries: VecDeque::new(),
            seq: 0,
        }
    }

    /// Keep one evaluation for `actor`. The oldest candidates leave first when a limit binds.
    #[allow(clippy::too_many_arguments)]
    pub fn insert(
        &mut self,
        actor: &str,
        base_revision: u64,
        config_digest: &str,
        snapshot_id: &str,
        draft_digest: &str,
        ev: Evaluation,
        now: u64,
    ) -> CandidateRef {
        self.seq += 1;
        let reference = CandidateRef {
            id: format!("cand-{}-{}", self.boot, self.seq),
            base_revision: base_revision.to_string(),
            config_digest: config_digest.into(),
            snapshot_id: snapshot_id.into(),
            draft_digest: draft_digest.into(),
            expires_at: now + CANDIDATE_TTL_SECS,
        };
        let count = |s: Severity| ev.issues.iter().filter(|i| i.severity == s).count() as u64;
        // Counts cover every issue before any page cut.
        let issues = IssueCounts {
            blockers: count(Severity::Blocker),
            warnings: count(Severity::Warning),
            information: count(Severity::Information),
        };
        let publishable = ev.publishable();
        let summary = CandidateSummary {
            reference: reference.clone(),
            graph: ev.graph,
            diff: ev.diff,
            issues,
            publishable,
            complete: ev.complete,
        };
        let bytes = [
            serde_json::to_vec(&summary).map_or(0, |b| b.len()),
            serde_json::to_vec(&ev.records).map_or(0, |b| b.len()),
            serde_json::to_vec(&ev.issues).map_or(0, |b| b.len()),
        ]
        .iter()
        .sum();
        self.entries
            .retain(|e| e.summary.reference.expires_at > now);
        let mine = |e: &Entry| e.actor == actor;
        while self.entries.iter().filter(|e| mine(e)).count()
            >= usize::from(MAX_CANDIDATES_PER_ACTOR)
            || self
                .entries
                .iter()
                .filter(|e| mine(e))
                .map(|e| e.bytes)
                .sum::<usize>()
                + bytes
                > ACTOR_BYTES
        {
            let Some(i) = self.entries.iter().position(mine) else {
                break;
            };
            self.entries.remove(i);
        }
        while self.entries.iter().map(|e| e.bytes).sum::<usize>() + bytes > TOTAL_BYTES
            && self.entries.pop_front().is_some()
        {}
        self.entries.push_back(Entry {
            actor: actor.into(),
            summary,
            records: ev.records,
            issues: ev.issues,
            bytes,
        });
        reference
    }

    fn entry(&self, actor: &str, id: &str, now: u64) -> Lookup<&Entry> {
        let from_this_boot = id
            .strip_prefix("cand-")
            .and_then(|rest| rest.rsplit_once('-'))
            .is_some_and(|(boot, _)| boot == self.boot);
        if id.starts_with("cand-") && !from_this_boot {
            return Lookup::Expired;
        }
        match self.entries.iter().find(|e| e.summary.reference.id == id) {
            Some(e) if e.actor != actor => Lookup::NotFound,
            Some(e) if e.summary.reference.expires_at <= now => Lookup::Expired,
            Some(e) => Lookup::Found(e),
            None => Lookup::NotFound,
        }
    }

    pub fn summary(&self, actor: &str, id: &str, now: u64) -> Lookup<CandidateSummary> {
        match self.entry(actor, id, now) {
            Lookup::Found(e) => Lookup::Found(e.summary.clone()),
            Lookup::NotFound => Lookup::NotFound,
            Lookup::Expired => Lookup::Expired,
            Lookup::BadCursor => Lookup::BadCursor,
        }
    }

    pub fn records(
        &self,
        actor: &str,
        id: &str,
        cursor: Option<&str>,
        limit: u16,
        now: u64,
    ) -> Lookup<CandidatePage<ResolvedRecord>> {
        self.page(actor, id, "records", cursor, limit, now, |e| &e.records)
    }

    pub fn issues(
        &self,
        actor: &str,
        id: &str,
        cursor: Option<&str>,
        limit: u16,
        now: u64,
    ) -> Lookup<CandidatePage<ValidationIssue>> {
        self.page(actor, id, "issues", cursor, limit, now, |e| &e.issues)
    }

    /// The cursor is the next offset with a tag of the actor, candidate, draft, and list.
    fn tag(e: &Entry, list: &str) -> String {
        let r = &e.summary.reference;
        let text = format!("{}\n{}\n{}\n{list}", e.actor, r.id, r.draft_digest);
        format!("{:016x}", crate::topology::fnv1a64(text.as_bytes()))
    }

    #[allow(clippy::too_many_arguments)]
    fn page<T: Clone + serde::Serialize>(
        &self,
        actor: &str,
        id: &str,
        list: &str,
        cursor: Option<&str>,
        limit: u16,
        now: u64,
        items: impl Fn(&Entry) -> &Vec<T>,
    ) -> Lookup<CandidatePage<T>> {
        let e = match self.entry(actor, id, now) {
            Lookup::Found(e) => e,
            Lookup::NotFound => return Lookup::NotFound,
            Lookup::Expired => return Lookup::Expired,
            Lookup::BadCursor => return Lookup::BadCursor,
        };
        let all = items(e);
        let tag = Self::tag(e, list);
        let start = match cursor {
            None => 0,
            Some(c) => match c.split_once(':') {
                Some((offset, t)) if t == tag => match offset.parse::<usize>() {
                    Ok(n) if n > 0 && n < all.len() => n,
                    _ => return Lookup::BadCursor,
                },
                _ => return Lookup::BadCursor,
            },
        };
        let limit = usize::from(limit.clamp(1, PAGE_MAX));
        let mut out = Vec::new();
        let mut bytes = 0;
        for item in all.iter().skip(start).take(limit) {
            let size = serde_json::to_vec(item).map_or(0, |b| b.len());
            if !out.is_empty() && bytes + size > PAGE_BYTES {
                break;
            }
            bytes += size;
            out.push(item.clone());
        }
        let end = start + out.len();
        Lookup::Found(CandidatePage {
            candidate_id: id.into(),
            next_cursor: (end < all.len()).then(|| format!("{end}:{tag}")),
            total: all.len() as u64,
            items: out,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use web_contract::source_studio::{CandidateDiff, CandidateGraph};

    fn evaluation(issues: usize) -> Evaluation {
        Evaluation {
            graph: CandidateGraph {
                nodes: vec![],
                flows: vec![],
                nodes_truncated: false,
                flows_truncated: false,
            },
            diff: CandidateDiff {
                added: 0,
                removed: 0,
                changed: 0,
                unchanged: 0,
                markers: vec![],
                markers_truncated: false,
            },
            records: vec![],
            issues: (0..issues)
                .map(|i| ValidationIssue {
                    code: "x".into(),
                    severity: if i % 3 == 0 {
                        Severity::Blocker
                    } else {
                        Severity::Warning
                    },
                    entity_ref: format!("e{i}"),
                    field: None,
                    message: "m".into(),
                    remedy: None,
                })
                .collect(),
            complete: true,
        }
    }

    fn found<T>(l: Lookup<T>) -> T {
        match l {
            Lookup::Found(t) => t,
            _ => panic!("not found"),
        }
    }

    #[test]
    fn paged_issues_keep_complete_counts() {
        let mut cache = Cache::new("b1".into());
        let r = cache.insert("alice", 7, "d", "s", "draft", evaluation(250), 100);
        let summary = found(cache.summary("alice", &r.id, 100));
        assert_eq!(summary.issues.blockers + summary.issues.warnings, 250);
        assert_eq!(summary.issues.blockers, 84);
        assert!(!summary.publishable);
        let (mut seen, mut cursor) = (Vec::new(), None::<String>);
        loop {
            let page = found(cache.issues("alice", &r.id, cursor.as_deref(), 100, 100));
            assert_eq!(page.total, 250, "every page names the full total");
            assert!(page.items.len() <= 100);
            seen.extend(page.items.into_iter().map(|i| i.entity_ref));
            match page.next_cursor {
                Some(c) => cursor = Some(c),
                None => break,
            }
        }
        let want: Vec<String> = (0..250).map(|i| format!("e{i}")).collect();
        assert_eq!(seen, want, "pages continue from the last item");
        // A cursor of one list cannot read another list or another candidate.
        let first = found(cache.issues("alice", &r.id, None, 10, 100));
        let c = first.next_cursor.unwrap();
        assert_eq!(
            cache.records("alice", &r.id, Some(&c), 10, 100),
            Lookup::BadCursor
        );
        let other = cache.insert("alice", 7, "d", "s", "draft-2", evaluation(20), 100);
        assert_eq!(
            cache.issues("alice", &other.id, Some(&c), 10, 100),
            Lookup::BadCursor
        );
    }

    #[test]
    fn foreign_actor_cannot_read_candidate() {
        let mut cache = Cache::new("b1".into());
        let r = cache.insert("alice", 7, "d", "s", "draft", evaluation(3), 100);
        assert_eq!(cache.summary("mallory", &r.id, 100), Lookup::NotFound);
        assert_eq!(
            cache.summary("mallory", "cand-b1-999", 100),
            Lookup::NotFound
        );
        let page = found(cache.issues("alice", &r.id, None, 1, 100));
        let c = page.next_cursor.unwrap();
        assert_eq!(
            cache.issues("mallory", &r.id, Some(&c), 1, 100),
            Lookup::NotFound
        );
        // The owner reads an expired candidate as expired.
        assert_eq!(
            cache.summary("alice", &r.id, 100 + CANDIDATE_TTL_SECS),
            Lookup::Expired
        );
    }

    #[test]
    fn restart_expires_candidate() {
        let mut before = Cache::new("b1".into());
        let r = before.insert("alice", 7, "d", "s", "draft", evaluation(1), 100);
        let after = Cache::new("b2".into());
        assert_eq!(after.summary("alice", &r.id, 101), Lookup::Expired);
    }

    #[test]
    fn an_actor_keeps_at_most_four_candidates() {
        let mut cache = Cache::new("b1".into());
        let ids: Vec<_> = (0..5)
            .map(|_| {
                cache
                    .insert("alice", 7, "d", "s", "x", evaluation(1), 100)
                    .id
            })
            .collect();
        assert_eq!(cache.summary("alice", &ids[0], 100), Lookup::NotFound);
        assert!(matches!(
            cache.summary("alice", &ids[4], 100),
            Lookup::Found(_)
        ));
        let bob = cache.insert("bob", 7, "d", "s", "x", evaluation(1), 100);
        assert!(matches!(
            cache.summary("bob", &bob.id, 100),
            Lookup::Found(_)
        ));
        assert!(matches!(
            cache.summary("alice", &ids[1], 100),
            Lookup::Found(_)
        ));
    }
}
