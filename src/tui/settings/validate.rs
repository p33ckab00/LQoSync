//! Validate a draft: the config rules, then the `lqosync3 check` rules.

use crate::cli::{Level, check_with};
use crate::config::Config;
use std::collections::HashSet;
use std::path::PathBuf;

/// The config rules only: no IO, about 1 ms. The screen runs it at each commit of a field.
pub fn parse(text: &str) -> Result<Config, String> {
    Config::from_toml_str(text).map_err(|e| format!("{e:#}"))
}

/// `parse`, then the `lqosync3 check` rules (secret files, folders, the dpkg status, a probe write in
/// `state_dir`). It has IO, so only the preview worker runs it. A file in `typed` counts as readable.
/// Note: an env variable in `bot_token_env` or `chat_id_env` is read from the TUI process.
pub fn validate(text: &str, typed: &HashSet<PathBuf>) -> Result<(Config, Vec<String>), String> {
    let cfg = parse(text)?;
    let lines = check_with(&cfg, 0, typed);
    let errors: Vec<&str> = lines
        .iter()
        .filter(|l| l.level == Level::Error)
        .map(|l| l.text.as_str())
        .collect();
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    let warnings = lines
        .into_iter()
        .filter(|l| l.level == Level::Warn)
        .map(|l| l.text)
        .collect();
    Ok((cfg, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Env {
        _dir: tempfile::TempDir,
        pw: PathBuf,
        base: String,
    }

    fn env() -> Env {
        let dir = tempfile::tempdir().unwrap();
        let lqos = dir.path().join("lqos");
        std::fs::create_dir_all(&lqos).unwrap();
        let pw = dir.path().join("R1.password");
        let base = format!(
            "[runtime]\nstate_dir = \"{}\"\n[libreqos]\ntarget = \"v2_pair\"\ndirectory = \"{}\"\n\
             [[routers]]\nname = \"R1\"\naddress = \"192.0.2.1\"\nusername = \"u\"\npassword_file = \"{}\"\n[routers.pppoe]\nenabled = true\n",
            dir.path().join("state").display(),
            lqos.display(),
            pw.display()
        );
        Env {
            _dir: dir,
            pw,
            base,
        }
    }

    #[test]
    fn validate_names_the_key() {
        let e = parse("[runtime]\nmode = \"x\"\n[[routers]]\nname=\"R1\"\naddress=\"a\"\nusername=\"u\"\npassword_file=\"/x\"\n").unwrap_err();
        assert!(e.contains("runtime.mode"), "{e}");
    }

    #[test]
    fn validate_accepts_a_typed_password_file() {
        let e = env();
        let err = validate(&e.base, &HashSet::new()).unwrap_err();
        assert!(err.contains("R1"), "{err}");
        let typed = HashSet::from([e.pw.clone()]);
        validate(&e.base, &typed).unwrap();
    }

    #[test]
    fn validate_returns_warnings() {
        let e = env();
        let text = e
            .base
            .replace("password_file", "tls = false\npassword_file");
        let (_, warn) = validate(&text, &HashSet::from([e.pw.clone()])).unwrap();
        assert!(warn.iter().any(|w| w.contains("plain API")), "{warn:?}");
    }
}
