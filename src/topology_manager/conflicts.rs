//! Preparation conflicts as data, so that a preview can name each conflict.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConflictView {
    /// `identity`, `placement`, or `aggregate`.
    pub kind: &'static str,
    pub subject: String,
    pub detail: String,
}

/// Critical conflicts that block preparation. Publication fails on this error as before.
#[derive(Debug)]
pub struct PreparationConflicts(pub Vec<ConflictView>);
impl std::fmt::Display for PreparationConflicts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.first() {
            Some(first) => write!(
                f,
                "{} conflict(s) block preparation; first: {} {}: {}",
                self.0.len(),
                first.kind,
                first.subject,
                first.detail
            ),
            None => f.write_str("conflicts block preparation"),
        }
    }
}
impl std::error::Error for PreparationConflicts {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_the_first_conflict_and_the_count() {
        let e = PreparationConflicts(vec![
            ConflictView {
                kind: "identity",
                subject: "dhcp-LAN/*l0".into(),
                detail: "DuplicateReference".into(),
            },
            ConflictView {
                kind: "identity",
                subject: "dhcp-LAN/*l1".into(),
                detail: "DuplicateReference".into(),
            },
        ]);
        let text = e.to_string();
        assert!(
            text.contains("dhcp-LAN/*l0") && text.contains("2 conflict"),
            "{text}"
        );
    }
}
