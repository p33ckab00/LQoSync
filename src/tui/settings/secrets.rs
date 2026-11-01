//! Typed secrets: they stay in memory until apply, then go only into their file (mode 0600, owner root).

use std::path::PathBuf;

/// A typed password or token. `Debug` hides it, and there is no `Display`.
pub struct Secret(String);

impl Secret {
    pub fn new(s: String) -> Self {
        Secret(s)
    }

    pub fn len(&self) -> usize {
        self.0.chars().count()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn push(&mut self, c: char) {
        self.0.push(c);
    }

    pub fn pop(&mut self) {
        self.0.pop();
    }

    /// The bytes, only for the file write.
    fn expose(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(***)")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        // SAFETY: zeros keep the string valid UTF-8.
        unsafe { self.0.as_mut_vec().fill(0) };
    }
}

/// A secret file that the apply writes.
#[derive(Debug)]
pub struct Pending {
    pub path: PathBuf,
    pub value: Secret,
}

/// The old state of each written file, for a rollback.
#[derive(Debug, Default)]
pub struct SecretUndo {
    /// The path, and a hard link to its old file (`None` if the file did not exist).
    /// A rollback is then a rename: it needs no new disk space.
    done: Vec<(PathBuf, Option<PathBuf>)>,
}

fn old_link(path: &std::path::Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("secret");
    path.with_file_name(format!(".{name}.old-secret"))
}

impl SecretUndo {
    /// Put each file back, in reverse order: remove a new file, rename the old file back.
    /// Returns the errors; an empty list means all files are back.
    pub fn rollback(self) -> Vec<String> {
        let mut errors = Vec::new();
        for (path, link) in self.done.into_iter().rev() {
            let r = match &link {
                Some(l) => std::fs::rename(l, &path),
                None => match std::fs::remove_file(&path) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    r => r,
                },
            };
            if let Err(e) = r {
                errors.push(format!("cannot put back {}: {e}", path.display()));
            }
        }
        errors
    }

    /// The apply is done: remove the links to the old files.
    pub fn commit(self) {
        for l in self.done.into_iter().filter_map(|(_, l)| l) {
            let _ = std::fs::remove_file(l);
        }
    }
}

/// Write each secret file. On an error, the files written before are rolled back.
pub fn write_secrets(list: &[Pending]) -> anyhow::Result<SecretUndo> {
    let mut undo = SecretUndo::default();
    let fail = |undo: SecretUndo, e: anyhow::Error| {
        let errors = undo.rollback();
        if errors.is_empty() {
            e
        } else {
            e.context(format!("rollback: {}", errors.join("; ")))
        }
    };
    for p in list {
        let link = if p.path.exists() {
            let l = old_link(&p.path);
            let _ = std::fs::remove_file(&l);
            if let Err(e) = std::fs::hard_link(&p.path, &l) {
                let e = anyhow::Error::from(e)
                    .context(format!("cannot keep the old {}", p.path.display()));
                return Err(fail(undo, e));
            }
            Some(l)
        } else {
            None
        };
        // Record it first: a write that fails after its rename is also rolled back.
        undo.done.push((p.path.clone(), link));
        if let Err(e) = crate::files::write_secret(&p.path, p.value.expose()) {
            return Err(fail(undo, e));
        }
    }
    Ok(undo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    fn pending(p: &Path, v: &str) -> Pending {
        Pending {
            path: p.to_path_buf(),
            value: Secret::new(v.to_string()),
        }
    }

    #[test]
    fn secret_debug_hides_the_value() {
        let s = Secret::new("pw-123".into());
        assert!(!format!("{s:?}").contains("pw-123"));
        assert!(!format!("{:?}", pending(Path::new("/x"), "pw-123")).contains("pw-123"));
    }

    #[test]
    fn write_secrets_mode_0600_and_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let (new, old) = (
            dir.path().join("new.password"),
            dir.path().join("old.password"),
        );
        std::fs::write(&old, "old\n").unwrap();
        let undo = write_secrets(&[pending(&new, "n1"), pending(&old, "o2")]).unwrap();
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "n1\n");
        assert_eq!(std::fs::read_to_string(&old).unwrap(), "o2\n");
        for p in [&new, &old] {
            assert_eq!(
                std::fs::metadata(p).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        undo.rollback();
        assert!(!new.exists());
        assert_eq!(std::fs::read_to_string(&old).unwrap(), "old\n");
    }

    #[test]
    fn rollback_renames_the_old_file_back_and_commit_removes_the_copy() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("old.password");
        std::fs::write(&old, "old\n").unwrap();
        let undo = write_secrets(&[pending(&old, "new")]).unwrap();
        assert!(undo.rollback().is_empty());
        assert_eq!(std::fs::read_to_string(&old).unwrap(), "old\n");
        let undo = write_secrets(&[pending(&old, "new")]).unwrap();
        undo.commit();
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            1,
            "no copy stays"
        );
    }

    #[test]
    fn a_failed_rollback_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("old.password");
        std::fs::write(&old, "old\n").unwrap();
        let undo = write_secrets(&[pending(&old, "new")]).unwrap();
        for e in std::fs::read_dir(dir.path()).unwrap() {
            let p = e.unwrap().path();
            if p != old {
                std::fs::remove_file(p).unwrap();
            }
        }
        let errors = undo.rollback();
        assert!(
            errors.iter().any(|e| e.contains("old.password")),
            "{errors:?}"
        );
    }

    #[test]
    fn a_failed_secret_write_rolls_back_the_earlier_ones() {
        let dir = tempfile::tempdir().unwrap();
        let (a, old) = (
            dir.path().join("a.password"),
            dir.path().join("old.password"),
        );
        std::fs::write(&old, "old\n").unwrap();
        let bad = dir.path().join("no-folder/b.password");
        let e =
            write_secrets(&[pending(&a, "x"), pending(&old, "y"), pending(&bad, "z")]).unwrap_err();
        assert!(format!("{e:#}").contains("no-folder"), "{e:#}");
        assert!(!a.exists());
        assert_eq!(std::fs::read_to_string(&old).unwrap(), "old\n");
    }
}
