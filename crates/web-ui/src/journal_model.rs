//! The Journal screen: levels, paging, and search. Pure, so native tests cover it.
use web_contract::journal::JournalEntry;

/// The lines that the page keeps in memory; the oldest go first.
pub(crate) const KEEP: usize = 2000;

/// The text and tone of a syslog priority. The text carries the meaning.
pub(crate) fn level(priority: u8) -> (&'static str, &'static str) {
    match priority {
        0..=3 => ("Error", "bad"),
        4 => ("Warning", "warn"),
        5 => ("Notice", "info"),
        6 => ("Info", "neutral"),
        _ => ("Debug", "neutral"),
    }
}

/// The level filter choices: (highest priority number, label).
pub(crate) const LEVELS: [(u8, &str); 5] = [
    (3, "Errors only"),
    (4, "Warnings and errors"),
    (5, "Notices and above"),
    (6, "Information and above"),
    (7, "Everything, with debug"),
];

/// Add a newest page: the kept lines before its first line, then the page. No line repeats.
pub(crate) fn merge_newest(kept: &[JournalEntry], newest: &[JournalEntry]) -> Vec<JournalEntry> {
    let Some(first) = newest.first() else {
        return kept.to_vec();
    };
    let mut out: Vec<JournalEntry> = kept
        .iter()
        .filter(|e| e.at_usec < first.at_usec)
        .cloned()
        .collect();
    out.extend_from_slice(newest);
    if out.len() > KEEP {
        out.drain(..out.len() - KEEP);
    }
    out
}

/// Add an older page before the kept lines. No line repeats.
pub(crate) fn merge_older(older: &[JournalEntry], kept: &[JournalEntry]) -> Vec<JournalEntry> {
    let first = kept.first().map_or(u64::MAX, |e| e.at_usec);
    let mut out: Vec<JournalEntry> = older
        .iter()
        .filter(|e| e.at_usec < first)
        .cloned()
        .collect();
    out.extend_from_slice(kept);
    out.truncate(KEEP);
    out
}

pub(crate) fn matches(e: &JournalEntry, text: &str) -> bool {
    let t = text.trim().to_lowercase();
    t.is_empty() || e.message.to_lowercase().contains(&t)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn e(at: u64, m: &str) -> JournalEntry {
        JournalEntry {
            at_usec: at,
            priority: 6,
            message: m.into(),
        }
    }
    #[test]
    fn pages_join_without_repeats_and_search_ignores_case() {
        let kept = vec![e(1, "a"), e(2, "b"), e(3, "c")];
        let joined = merge_newest(&kept, &[e(3, "c"), e(4, "d")]);
        let at: Vec<u64> = joined.iter().map(|x| x.at_usec).collect();
        assert_eq!(at, [1, 2, 3, 4]);
        assert_eq!(merge_newest(&kept, &[]), kept);
        let older = merge_older(&[e(0, "z"), e(1, "a")], &kept);
        assert_eq!(
            older.iter().map(|x| x.at_usec).collect::<Vec<_>>(),
            [0, 1, 2, 3]
        );
        assert!(matches(&e(1, "Cycle DONE"), "cycle done") && !matches(&e(1, "x"), "y"));
        assert!(matches(&e(1, "x"), "  "));
        assert_eq!(level(2), ("Error", "bad"));
        assert_eq!(level(4).0, "Warning");
        assert_eq!(level(9).0, "Debug");
        // The level choices go from errors to debug, and the page default (6) is one of them.
        assert!(LEVELS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(LEVELS.iter().any(|(p, _)| *p == 6));
        let many: Vec<JournalEntry> = (0..KEEP as u64 + 10).map(|i| e(i, "x")).collect();
        assert_eq!(merge_newest(&many, &[e(KEEP as u64 + 10, "y")]).len(), KEEP);
    }
}
