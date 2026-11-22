//! Durable writer selection. A topology owner cannot fall back to the legacy cycle.
//! Only `topology activate --yes` claims the writer, and only `topology deactivate --yes` releases it.
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    schema_version: u32,
    writer: String,
}
pub(crate) fn topology_owns(state: &Path) -> anyhow::Result<bool> {
    super::staging::check_ready(state)?;
    let directory = state.join("topology-manager");
    match std::fs::symlink_metadata(&directory) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
        Ok(_) => super::store::check_private_directory(&directory)?,
    }
    let file = match std::fs::File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(directory.join("runtime-owner.json"))
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    let metadata = file.metadata()?;
    // SAFETY: geteuid has no preconditions.
    ensure!(
        metadata.is_file()
            && metadata.uid() == unsafe { libc::geteuid() }
            && metadata.mode() & 0o077 == 0
            && metadata.len() <= 1024,
        "invalid writer ownership file"
    );
    let mut bytes = Vec::new();
    file.take(1025).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 1024, "writer ownership file is too large");
    let owner: Owner = serde_json::from_slice(&bytes).context("cannot decode writer ownership")?;
    ensure!(
        owner.schema_version == 1 && owner.writer == "topology",
        "unsupported writer ownership"
    );
    Ok(true)
}
pub(crate) fn claim(state: &Path) -> anyhow::Result<()> {
    if topology_owns(state)? {
        return Ok(());
    }
    let directory = state.join("topology-manager");
    super::store::check_private_directory(&directory)?;
    crate::files::write_secret(
        &directory.join("runtime-owner.json"),
        &serde_json::to_vec(&Owner {
            schema_version: 1,
            writer: "topology".into(),
        })?,
    )?;
    ensure!(
        topology_owns(state)?,
        "writer ownership confirmation failed"
    );
    Ok(())
}

/// Remove the topology owner file. Only `topology deactivate` calls it. A missing file is no error.
#[allow(dead_code)] // shortcut: Task 4 of the activation plan (deactivate) is its caller; remove then.
pub(crate) fn release(state: &Path) -> anyhow::Result<()> {
    let directory = state.join("topology-manager");
    match std::fs::remove_file(directory.join("runtime-owner.json")) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        other => other.context("cannot remove the writer ownership file")?,
    }
    std::fs::File::open(&directory)?
        .sync_all()
        .context("cannot sync the topology manager directory")
}

#[cfg(test)]
mod tests {
    #[test]
    fn release_removes_the_owner_file() {
        let root = tempfile::tempdir().unwrap();
        drop(super::super::store::RegistryStore::open(root.path()).unwrap());
        super::claim(root.path()).unwrap();
        assert!(super::topology_owns(root.path()).unwrap());
        super::release(root.path()).unwrap();
        assert!(!super::topology_owns(root.path()).unwrap());
        super::release(root.path()).unwrap();
    }
}
