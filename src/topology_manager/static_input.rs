//! Current independent static policy. Generated output is never a source.
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub const MAX_STATIC_ROWS: usize = 100_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticBinding {
    pub path: PathBuf,
    pub revision: u64,
    pub digest: String,
}
impl StaticBinding {
    pub(crate) fn valid(&self) -> bool {
        self.path.is_absolute()
            && self
                .path
                .file_name()
                .is_some_and(|n| n == "static-input.json")
            && self.revision > 0
            && self.digest.len() == 64
            && self.digest.bytes().all(|b| b.is_ascii_hexdigit())
    }
    pub(super) fn check_current(&self) -> anyhow::Result<()> {
        ensure!(self.valid(), "invalid static source binding");
        let bytes = read(&self.path)?;
        ensure!(
            crate::write_receipt::sha256(&bytes) == self.digest,
            "authoritative static input changed"
        );
        Ok(())
    }
    pub(super) fn check_previous(&self, previous: Option<&Self>) -> anyhow::Result<()> {
        if let Some(previous) = previous {
            ensure!(
                self.path == previous.path && self.revision >= previous.revision,
                "static source path changed or revision regressed"
            );
            ensure!(
                self.revision != previous.revision || self.digest == previous.digest,
                "static source changed without a new revision"
            );
        }
        Ok(())
    }
    pub(super) fn check_state(&self, state: &Path) -> anyhow::Result<()> {
        ensure!(
            self.path == source_path(state)?,
            "static source belongs to a different runtime owner"
        );
        self.check_current()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema_version: u32,
    revision: u64,
    #[serde(deserialize_with = "read_rows")]
    rows: Vec<crate::model::CsvRow>,
    network: serde_json::Value,
}
fn read_rows<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<crate::model::CsvRow>, D::Error> {
    struct Rows;
    impl<'de> serde::de::Visitor<'de> for Rows {
        type Value = Vec<crate::model::CsvRow>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a bounded static row array")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            let mut rows = Vec::new();
            while let Some(row) = sequence.next_element()? {
                if rows.len() == MAX_STATIC_ROWS {
                    return Err(serde::de::Error::custom(
                        "static row count exceeds its budget",
                    ));
                }
                rows.push(row);
            }
            Ok(rows)
        }
    }
    deserializer.deserialize_seq(Rows)
}
pub(super) struct CurrentStatic {
    pub binding: StaticBinding,
    rows: Vec<crate::model::CsvRow>,
    network: serde_json::Value,
}
impl CurrentStatic {
    pub(super) fn load(state: &Path) -> anyhow::Result<Self> {
        let path = source_path(state)?;
        let bytes = read(&path)?;
        let document: Document = serde_json::from_slice(&bytes)
            .map_err(|_| anyhow::anyhow!("cannot decode authoritative static input"))?;
        ensure!(
            document.schema_version == 1 && document.revision > 0,
            "unsupported static input schema or revision"
        );
        Ok(Self {
            binding: StaticBinding {
                path,
                revision: document.revision,
                digest: crate::write_receipt::sha256(&bytes),
            },
            rows: document.rows,
            network: document.network,
        })
    }
    pub(super) fn input(&self) -> super::ownership::StaticInput<'_> {
        super::ownership::StaticInput {
            rows: &self.rows,
            network: &self.network,
        }
    }
}
pub(super) fn required(registry: &super::model::RegistryDocument) -> bool {
    registry
        .export_ownership
        .as_ref()
        .is_some_and(|p| !p.protected_rows.is_empty() || !p.protected_roots.is_empty())
}
fn source_path(state: &Path) -> anyhow::Result<PathBuf> {
    let parent = state.join("topology-manager");
    super::store::check_private_directory(&parent)?;
    Ok(std::fs::canonicalize(parent)?.join("static-input.json"))
}
pub(super) fn read(path: &Path) -> anyhow::Result<Vec<u8>> {
    super::store::check_private_directory(
        path.parent()
            .context("static source directory is missing")?,
    )?;
    let file = std::fs::File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .context("cannot open authoritative static input")?;
    let metadata = file.metadata()?;
    let limit = super::admission::MAX_PAIR_BYTES as u64;
    // SAFETY: geteuid has no preconditions.
    ensure!(
        metadata.is_file()
            && metadata.uid() == unsafe { libc::geteuid() }
            && metadata.mode() & 0o077 == 0
            && metadata.len() <= limit,
        "static input must be a private regular file within the byte budget"
    );
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= limit,
        "static input exceeds the byte budget"
    );
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;

    fn document(revision: u64) -> serde_json::Value {
        json!({"schema_version":1,"revision":revision,"rows":[],"network":{}})
    }
    fn write(state: &std::path::Path, value: &serde_json::Value) {
        let dir = state.join("topology-manager");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        crate::files::write_secret(
            &dir.join("static-input.json"),
            &serde_json::to_vec(value).unwrap(),
        )
        .unwrap();
    }
    #[test]
    fn source_binding_detects_changed_bytes_removed_files_and_revision_rollback() {
        let state = tempfile::tempdir().unwrap();
        write(state.path(), &document(2));
        let source = CurrentStatic::load(state.path()).unwrap();
        source.binding.check_current().unwrap();
        let previous = source.binding.clone();
        source.binding.check_previous(Some(&previous)).unwrap();
        write(state.path(), &document(3));
        assert!(source.binding.check_current().is_err());
        let next = CurrentStatic::load(state.path()).unwrap();
        next.binding.check_previous(Some(&previous)).unwrap();
        assert!(previous.check_previous(Some(&next.binding)).is_err());
        let mut changed = document(2);
        changed["network"] = json!({"changed":{}});
        write(state.path(), &changed);
        let changed = CurrentStatic::load(state.path()).unwrap();
        assert!(changed.binding.check_previous(Some(&previous)).is_err());
        std::fs::remove_file(state.path().join("topology-manager/static-input.json")).unwrap();
        assert!(changed.binding.check_current().is_err());
    }
    #[test]
    fn source_rejects_unsafe_files_invalid_versions_and_zero_revision() {
        let state = tempfile::tempdir().unwrap();
        write(state.path(), &document(0));
        assert!(CurrentStatic::load(state.path()).is_err());
        let mut invalid = document(1);
        invalid["schema_version"] = json!(2);
        write(state.path(), &invalid);
        assert!(CurrentStatic::load(state.path()).is_err());
        write(state.path(), &document(1));
        let path = state.path().join("topology-manager/static-input.json");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(CurrentStatic::load(state.path()).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(super::super::admission::MAX_PAIR_BYTES as u64 + 1)
            .unwrap();
        assert!(CurrentStatic::load(state.path()).is_err());
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink("/dev/null", &path).unwrap();
        assert!(CurrentStatic::load(state.path()).is_err());
    }
    #[test]
    fn compact_empty_rows_cannot_exceed_the_allocation_count_budget() {
        let bytes = format!(
            "{{\"schema_version\":1,\"revision\":1,\"network\":{{}},\"rows\":[{}]}}",
            vec!["{}"; MAX_STATIC_ROWS + 1].join(",")
        );
        assert!(bytes.len() < super::super::admission::MAX_PAIR_BYTES);
        assert!(serde_json::from_str::<Document>(&bytes).is_err());
    }
}
