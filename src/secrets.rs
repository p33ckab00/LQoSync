//! Secret files: passwords and tokens. A secret never goes into a log or an error.

use anyhow::{Context, ensure};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// Read a secret file. The file must have no group or other permission bits and belong to
/// `owner_uid` (`0` = root in production). One final line end is removed.
pub fn read_secret(path: &Path, owner_uid: u32) -> anyhow::Result<String> {
    let meta = std::fs::metadata(path)
        .with_context(|| format!("secret file {}: cannot read it", path.display()))?;
    ensure!(
        meta.mode() & 0o077 == 0,
        "secret file {}: mode must be 0600 (no group or other access)",
        path.display()
    );
    ensure!(
        meta.uid() == owner_uid,
        "secret file {}: owner must be uid {owner_uid}",
        path.display()
    );
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("secret file {}: cannot read it", path.display()))?;
    let text = text.strip_suffix('\n').unwrap_or(&text);
    Ok(text.strip_suffix('\r').unwrap_or(text).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    fn file(content: &str, mode: u32) -> (tempfile::TempDir, std::path::PathBuf, u32) {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("router.password");
        std::fs::write(&p, content).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(mode)).unwrap();
        let uid = std::fs::metadata(&p).unwrap().uid();
        (d, p, uid)
    }

    #[test]
    fn reads_and_trims_a_good_secret() {
        let (_d, p, uid) = file("pa ss\n", 0o600);
        assert_eq!(read_secret(&p, uid).unwrap(), "pa ss");
        let (_d, p, uid) = file("pw\r\n", 0o400);
        assert_eq!(read_secret(&p, uid).unwrap(), "pw");
    }

    #[test]
    fn rejects_group_or_other_bits() {
        let (_d, p, uid) = file("pw", 0o640);
        assert!(format!("{:#}", read_secret(&p, uid).unwrap_err()).contains("0600"));
    }

    #[test]
    fn rejects_wrong_owner() {
        let (_d, p, uid) = file("pw", 0o600);
        assert!(format!("{:#}", read_secret(&p, uid + 1).unwrap_err()).contains("owner"));
    }

    #[test]
    fn missing_file_names_the_path() {
        let e = read_secret(std::path::Path::new("/nonexistent/lq3.pw"), 0).unwrap_err();
        assert!(format!("{e:#}").contains("/nonexistent/lq3.pw"));
    }

    #[test]
    fn error_never_contains_the_secret() {
        let (_d, p, uid) = file("s3cret", 0o644);
        assert!(!format!("{:#}", read_secret(&p, uid).unwrap_err()).contains("s3cret"));
    }
}
