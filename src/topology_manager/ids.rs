//! Catalog IDs share the durable monotonic counter. Labels and endpoints do not contribute to IDs.

use super::model::RegistryDocument;
use anyhow::{Context, ensure};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdKind {
    Router,
    Source,
    Profile,
    Node,
}

impl IdKind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Router => "tm-router-",
            Self::Source => "tm-source-",
            Self::Profile => "tm-profile-",
            Self::Node => "tm-node-",
        }
    }
}

pub(super) fn validate_id(id: &str, kind: IdKind, next: u64) -> anyhow::Result<()> {
    let reserved = [
        IdKind::Router,
        IdKind::Source,
        IdKind::Profile,
        IdKind::Node,
    ]
    .into_iter()
    .find(|k| id.starts_with(k.prefix()));
    let Some(actual) = reserved else {
        return Ok(());
    };
    ensure!(actual == kind, "generated catalog ID has the wrong kind");
    let sequence = &id[actual.prefix().len()..];
    ensure!(
        sequence.len() == 16
            && sequence
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "invalid generated catalog ID"
    );
    let number = u64::from_str_radix(sequence, 16).context("invalid generated catalog ID")?;
    ensure!(
        number > 0 && number < next,
        "catalog ID counter must exceed allocated IDs"
    );
    Ok(())
}

/// Reserve IDs in a draft only. The caller persists the completed edit through RegistryStore.
pub fn reserve_ids(
    document: &mut RegistryDocument,
    expected_revision: u64,
    kinds: &[IdKind],
) -> anyhow::Result<Vec<String>> {
    ensure!(
        document.revision == expected_revision,
        "catalog ID allocation revision conflict"
    );
    document.validate()?;
    let count = u64::try_from(kinds.len()).context("catalog ID batch exceeds counter range")?;
    let end = document
        .next_id
        .checked_add(count)
        .context("catalog ID counter is exhausted")?;
    let ids = kinds
        .iter()
        .zip(document.next_id..end)
        .map(|(kind, sequence)| format!("{}{:016x}", kind.prefix(), sequence))
        .collect();
    document.next_id = end;
    Ok(ids)
}

#[cfg(test)]
#[path = "ids_tests.rs"]
mod tests;
