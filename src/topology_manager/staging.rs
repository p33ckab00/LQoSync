//! Recoverable first installation of a proven static source and registry policy.
use super::{legacy_static::StaticAdoption, model::RegistryDocument, store::RegistryStore};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

const JOURNAL: &str = "migration-pending.json";
const OWNER: &str = "runtime-owner.json";
// Earlier runtimes reject this writer. This marker never grants topology ownership.
const BLOCKER: &[u8] = b"{\"schema_version\":1,\"writer\":\"migration-pending\"}";
const LIMIT: usize = 3 * super::admission::MAX_PAIR_BYTES;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    owner_directory: PathBuf,
    base: RegistryDocument,
    target: RegistryDocument,
    source: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    digest: String,
    payload: Payload,
}
fn directory(store: &RegistryStore) -> PathBuf {
    store.owner_directory().join("topology-manager")
}
fn absent(path: &Path) -> anyhow::Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(e) => Err(e.into()),
    }
}
pub(crate) fn pending(state: &Path) -> anyhow::Result<bool> {
    Ok(!absent(&state.join("topology-manager").join(JOURNAL))?)
}
pub(super) fn check_ready(state: &Path) -> anyhow::Result<()> {
    ensure!(
        !pending(state)?,
        "migration is pending. Recover or cancel it before normal edits or publication"
    );
    Ok(())
}
fn read(path: &Path) -> anyhow::Result<Vec<u8>> {
    super::store::check_private_directory(
        path.parent().context("migration directory is missing")?,
    )?;
    let file = File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let metadata = file.metadata()?;
    // SAFETY: geteuid has no preconditions.
    ensure!(
        metadata.is_file()
            && metadata.uid() == unsafe { libc::geteuid() }
            && metadata.mode() & 0o077 == 0
            && metadata.len() <= LIMIT as u64,
        "migration file must be private, regular, and within its byte budget"
    );
    let mut bytes = Vec::new();
    file.take(LIMIT as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= LIMIT, "migration byte budget exceeded");
    Ok(bytes)
}
fn load(dir: &Path) -> anyhow::Result<Journal> {
    let journal: Journal = serde_json::from_slice(&read(&dir.join(JOURNAL))?)
        .context("cannot decode migration journal")?;
    ensure!(
        journal.schema_version == 1,
        "unsupported migration journal schema"
    );
    ensure!(
        journal.digest == crate::write_receipt::sha256(&serde_json::to_vec(&journal.payload)?),
        "migration journal integrity conflict"
    );
    let p = &journal.payload;
    ensure!(
        p.owner_directory.is_absolute() && p.owner_directory.join("topology-manager") == dir,
        "migration journal belongs to another runtime owner"
    );
    p.base.validate()?;
    p.target.validate()?;
    ensure!(
        p.base.revision.checked_add(1) == Some(p.target.revision),
        "migration target revision conflict"
    );
    ensure!(
        p.source.len() <= super::admission::MAX_PAIR_BYTES,
        "migration static source exceeds byte budget"
    );
    // Confirm durability after an earlier write may have failed after rename.
    File::open(dir)?.sync_all()?;
    Ok(journal)
}

/// Save a sealed proposal. This first-adoption path requires no source or active writer.
/// A successful stage blocks ordinary edits and publication until recovery or cancellation.
pub fn stage(store: &mut RegistryStore, proposal: &StaticAdoption) -> anyhow::Result<()> {
    proposal.check_seal()?;
    check_ready(store.owner_directory())?;
    let dir = directory(store);
    let base = store.document()?.clone();
    let source = proposal.source_document(1)?;
    let installed = dir.join("static-input.json");
    // An enrolled registry may enroll again before activation, if the static source is the same.
    let same_source = base.export_ownership.is_some()
        && !absent(&installed)?
        && super::static_input::read(&installed)? == source;
    ensure!(
        absent(&dir.join(OWNER))? && (absent(&installed)? || same_source),
        "first adoption requires an absent runtime owner and static source"
    );
    let target = store.preview(base.revision, proposal.registry.clone(), true)?;
    ensure!(
        target.revision != base.revision,
        "migration does not change registry policy"
    );
    let payload = Payload {
        owner_directory: store.owner_directory().to_owned(),
        base,
        target,
        source: String::from_utf8(source)?,
    };
    ensure!(
        payload.source.len() <= super::admission::MAX_PAIR_BYTES,
        "migration static source exceeds byte budget"
    );
    let journal = Journal {
        schema_version: 1,
        digest: crate::write_receipt::sha256(&serde_json::to_vec(&payload)?),
        payload,
    };
    let bytes = serde_json::to_vec(&journal)?;
    ensure!(
        bytes.len() <= LIMIT,
        "migration journal exceeds byte budget"
    );
    store.backup_current()?;
    proposal.check_seal()?;
    super::store::write_new(&dir.join(JOURNAL), &bytes)?;
    Ok(())
}

/// Finish a staged installation. Reopen the store after an uncertain registry write.
/// Return false when no transaction is pending. Never replace an unknown source or owner.
pub fn recover(store: &mut RegistryStore) -> anyhow::Result<bool> {
    recover_inner(store, |_| Ok(()))
}
fn recover_inner(
    store: &mut RegistryStore,
    checkpoint: impl Fn(u8) -> anyhow::Result<()>,
) -> anyhow::Result<bool> {
    let dir = directory(store);
    if absent(&dir.join(JOURNAL))? {
        return Ok(false);
    }
    let journal = load(&dir)?;
    let p = journal.payload;
    let current = store.document()?;
    ensure!(
        *current == p.base || *current == p.target,
        "registry differs from migration base and target"
    );
    // Also prove the actual disk state before touching the source or owner.
    store.backup_current()?;
    if *store.document()? == p.base {
        ensure!(
            store.preview(
                p.base.revision,
                {
                    let mut candidate = p.target.clone();
                    candidate.revision = p.base.revision;
                    candidate
                },
                true
            )? == p.target,
            "migration target fails current store invariants"
        );
    }
    checkpoint(0)?;
    if absent(&dir.join(OWNER))? {
        super::store::write_new(&dir.join(OWNER), BLOCKER)?;
    } else {
        ensure!(
            read(&dir.join(OWNER))? == BLOCKER,
            "migration runtime owner conflict"
        );
    }
    checkpoint(1)?;
    let source = dir.join("static-input.json");
    if absent(&source)? {
        ensure!(
            *store.document()? == p.base,
            "committed migration source is missing"
        );
        super::store::write_new(&source, p.source.as_bytes())?;
    } else {
        ensure!(
            super::static_input::read(&source)? == p.source.as_bytes(),
            "migration static source conflict"
        );
    }
    let installed = super::static_input::CurrentStatic::load(store.owner_directory())?;
    ensure!(
        installed.binding.revision == 1,
        "migration source revision conflict"
    );
    checkpoint(2)?;
    if *store.document()? == p.base {
        let mut candidate = p.target.clone();
        candidate.revision = p.base.revision;
        store.commit_staged(p.base.revision, candidate)?;
    }
    ensure!(
        *store.document()? == p.target,
        "migration registry confirmation failed"
    );
    installed.binding.check_current()?;
    checkpoint(3)?;
    remove_blocker(&dir)?;
    checkpoint(4)?;
    std::fs::remove_file(dir.join(JOURNAL))?;
    File::open(&dir)?.sync_all()?;
    Ok(true)
}
fn remove_blocker(dir: &Path) -> anyhow::Result<()> {
    if !absent(&dir.join(OWNER))? {
        ensure!(
            read(&dir.join(OWNER))? == BLOCKER,
            "migration runtime owner conflict"
        );
        std::fs::remove_file(dir.join(OWNER))?;
    }
    File::open(dir)?.sync_all()?;
    Ok(())
}

/// Cancel only before source installation. Installed migrations must recover forward.
pub fn cancel(store: &mut RegistryStore) -> anyhow::Result<()> {
    let dir = directory(store);
    let journal = load(&dir)?;
    ensure!(
        *store.document()? == journal.payload.base && absent(&dir.join("static-input.json"))?,
        "installed migration must recover forward"
    );
    store.backup_current()?;
    remove_blocker(&dir)?;
    std::fs::remove_file(dir.join(JOURNAL))?;
    File::open(dir)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
#[path = "staging_tests.rs"]
mod tests;
