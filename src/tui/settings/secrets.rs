//! Typed secrets: they stay in memory until apply, then go only into their file (mode 0600, owner root).

use anyhow::Context;
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
    /// The path and its old bytes; `None` if the file did not exist.
    done: Vec<(PathBuf, Option<Vec<u8>>)>,
}

impl SecretUndo {
    /// Put each file back, in reverse order: remove a new file, write back a replaced one.
    pub fn rollback(self) {
        for (path, old) in self.done.into_iter().rev() {
            let r = match old {
                Some(bytes) => {
                    crate::files::write_secret(&path, bytes.strip_suffix(b"\n").unwrap_or(&bytes))
                }
                None => std::fs::remove_file(&path).map_err(Into::into),
            };
            if let Err(e) = r {
                eprintln!("error: cannot roll back {}: {e:#}", path.display());
            }
        }
    }
}

/// Write each secret file. On an error, the files written before are rolled back.
pub fn write_secrets(list: &[Pending]) -> anyhow::Result<SecretUndo> {
    let mut undo = SecretUndo::default();
    for p in list {
        let old = match std::fs::read(&p.path) {
            Ok(b) => Some(b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                undo.rollback();
                return Err(e).with_context(|| format!("cannot read {}", p.path.display()));
            }
        };
        if let Err(e) = crate::files::write_secret(&p.path, p.value.expose()) {
            undo.rollback();
            return Err(e);
        }
        undo.done.push((p.path.clone(), old));
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
