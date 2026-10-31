//! Command line: argument parse and the commands.

use crate::config::{Config, RuntimeMode, Target};
use crate::secrets::read_secret;
use anyhow::{Context, bail};
use std::path::{Path, PathBuf};

/// The default config file.
pub const DEFAULT_CONFIG: &str = "/etc/lqosync3/lqosync3.toml";

const USAGE: &str =
    "usage: lqosync3 <check | once [--force] | run | diff | defaults> [--config <path>]";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Check,
    Once { force: bool },
    Run,
    Diff,
    Defaults,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub command: Command,
    pub config: PathBuf,
}

/// Parse the command line. `args[0]` is the program name.
pub fn parse(args: &[String]) -> anyhow::Result<Args> {
    let mut command = None;
    let mut force = false;
    let mut config = PathBuf::from(DEFAULT_CONFIG);
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "check" | "once" | "run" | "diff" | "defaults" if command.is_none() => {
                command = Some(a.clone())
            }
            "--force" => force = true,
            "--config" => match it.next() {
                Some(p) => config = PathBuf::from(p),
                None => bail!("--config needs a path\n{USAGE}"),
            },
            other => bail!("unknown argument \"{other}\"\n{USAGE}"),
        }
    }
    let command = match command.as_deref() {
        Some("check") => Command::Check,
        Some("once") => Command::Once { force },
        Some("run") => Command::Run,
        Some("diff") => Command::Diff,
        Some("defaults") => Command::Defaults,
        _ => bail!("no command\n{USAGE}"),
    };
    Ok(Args { command, config })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Ok,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckLine {
    pub level: Level,
    pub text: String,
}

fn line(level: Level, text: impl Into<String>) -> CheckLine {
    CheckLine {
        level,
        text: text.into(),
    }
}

/// Read and check the config file.
pub fn load_config(path: &Path) -> anyhow::Result<Config> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    Config::from_toml_str(&text).with_context(|| format!("{}", path.display()))
}

/// Read the config again. On an error, `cfg` stays the same.
pub fn reload(cfg: &mut Config, path: &Path) -> anyhow::Result<()> {
    *cfg = load_config(path)?;
    Ok(())
}

/// The checks of `lqosync3 check` (runtime design section 8). `owner_uid` is the required secret owner.
pub fn check(cfg: &Config, owner_uid: u32) -> Vec<CheckLine> {
    let mut out = Vec::new();
    if matches!(cfg.libreqos.target, Target::V3Pair | Target::V3Network) {
        out.push(line(
            Level::Error,
            "libreqos.target: LibreQoS v3 targets come in Plan 3; use \"v2_pair\" or \"auto\"",
        ));
    }
    for r in &cfg.routers {
        let at = format!("router {}", r.name);
        match read_secret(Path::new(&r.password_file), owner_uid) {
            Ok(_) => out.push(line(Level::Ok, format!("{at}: password file is readable"))),
            Err(e) => out.push(line(Level::Error, format!("{at}: {e:#}"))),
        }
        if !(r.pppoe.enabled || r.hotspot.enabled || r.dhcp.enabled) {
            out.push(line(Level::Warn, format!("{at}: no source is enabled (pppoe, hotspot, dhcp); its subscribers will be removed")));
        }
        if !r.tls {
            out.push(line(Level::Warn, format!("{at}: plain API (tls = false); the password goes over the network in clear text")));
        } else if !r.tls_verify {
            out.push(line(
                Level::Warn,
                format!("{at}: tls_verify = false; the router certificate is not checked"),
            ));
        } else if !r.tls_ca_file.is_empty() && std::fs::metadata(&r.tls_ca_file).is_err() {
            out.push(line(
                Level::Error,
                format!("{at}: tls_ca_file {} cannot be read", r.tls_ca_file),
            ));
        }
    }
    let state = Path::new(&cfg.runtime.state_dir);
    let probe = state.join(".write-check");
    match std::fs::create_dir_all(state).and_then(|_| std::fs::write(&probe, b"x")) {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            out.push(line(
                Level::Ok,
                format!("state_dir {} is writable", state.display()),
            ));
        }
        Err(e) => out.push(line(
            Level::Error,
            format!("state_dir {}: {e}", state.display()),
        )),
    }
    let lqos = Path::new(&cfg.libreqos.directory);
    if lqos.is_dir() {
        out.push(line(
            Level::Ok,
            format!("libreqos.directory {} exists", lqos.display()),
        ));
    } else {
        out.push(line(
            Level::Error,
            format!("libreqos.directory {} does not exist", lqos.display()),
        ));
    }
    if cfg.runtime.mode == RuntimeMode::Apply && !Path::new(&cfg.libreqos.v2_command).exists() {
        out.push(line(
            Level::Error,
            format!(
                "libreqos.v2_command {} does not exist",
                cfg.libreqos.v2_command
            ),
        ));
    }
    let tg = &cfg.telegram;
    if tg.enabled {
        match crate::notify::token(tg, owner_uid) {
            Ok(_) => out.push(line(Level::Ok, "telegram: the bot token is readable")),
            Err(e) => out.push(line(Level::Error, format!("telegram: {e}"))),
        }
        if crate::notify::chat_id(tg).is_empty() {
            out.push(line(Level::Error, "telegram: the chat ID is empty"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn parse_commands_and_config_path() {
        let a = parse(&s(&[
            "lqosync3",
            "once",
            "--force",
            "--config",
            "/tmp/x.toml",
        ]))
        .unwrap();
        assert_eq!(
            (a.command, a.config),
            (Command::Once { force: true }, PathBuf::from("/tmp/x.toml"))
        );
        assert_eq!(
            parse(&s(&["lqosync3", "run"])).unwrap().config,
            PathBuf::from("/etc/lqosync3/lqosync3.toml")
        );
        assert_eq!(
            parse(&s(&["lqosync3", "defaults"])).unwrap().command,
            Command::Defaults
        );
        assert!(format!("{:#}", parse(&s(&["lqosync3", "wipe"])).unwrap_err()).contains("usage"));
        assert!(parse(&s(&["lqosync3"])).is_err());
        assert!(parse(&s(&["lqosync3", "run", "--config"])).is_err());
    }

    use crate::config::Config;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    struct Env {
        _dir: tempfile::TempDir,
        toml: String,
        uid: u32,
        path: PathBuf,
    }

    fn env(extra_router: &str, extra: &str) -> Env {
        let dir = tempfile::tempdir().unwrap();
        let pw = dir.path().join("r.password");
        std::fs::write(&pw, "pw\n").unwrap();
        std::fs::set_permissions(&pw, std::fs::Permissions::from_mode(0o600)).unwrap();
        let uid = std::fs::metadata(&pw).unwrap().uid();
        let lqos = dir.path().join("lqos");
        std::fs::create_dir_all(&lqos).unwrap();
        let toml = format!(
            "[runtime]\nstate_dir=\"{}\"\n[libreqos]\ndirectory=\"{}\"\n{extra}\n\
             [[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"{}\"\n{extra_router}\n",
            dir.path().join("state").display(),
            lqos.display(),
            pw.display()
        );
        let path = dir.path().join("lqosync3.toml");
        std::fs::write(&path, &toml).unwrap();
        Env {
            _dir: dir,
            toml,
            uid,
            path,
        }
    }

    fn texts(lines: &[CheckLine], level: Level) -> Vec<String> {
        lines
            .iter()
            .filter(|l| l.level == level)
            .map(|l| l.text.clone())
            .collect()
    }

    #[test]
    fn check_finds_plain_api_and_no_source_router() {
        let e = env("tls=false\n", "");
        let lines = check(&Config::from_toml_str(&e.toml).unwrap(), e.uid);
        let warn = texts(&lines, Level::Warn).join("\n");
        assert!(warn.contains("plain API"), "{warn}");
        assert!(warn.contains("no source"), "{warn}");
        assert!(texts(&lines, Level::Error).is_empty(), "{lines:?}");
    }

    #[test]
    fn check_reports_a_bad_secret_and_a_missing_lqos_dir() {
        let e = env("[routers.pppoe]\nenabled=true\n", "");
        let lines = check(&Config::from_toml_str(&e.toml).unwrap(), e.uid + 1);
        assert!(
            texts(&lines, Level::Error).join("\n").contains("owner"),
            "{lines:?}"
        );
        let mut c = Config::from_toml_str(&e.toml).unwrap();
        c.libreqos.directory = "/nonexistent/lqos".into();
        assert!(
            texts(&check(&c, e.uid), Level::Error)
                .join("\n")
                .contains("/nonexistent/lqos")
        );
    }

    #[test]
    fn check_rejects_v3_target_for_now() {
        let e = env("[routers.pppoe]\nenabled=true\n", "");
        let mut c = Config::from_toml_str(&e.toml).unwrap();
        c.libreqos.target = crate::config::Target::V3Pair;
        assert!(
            texts(&check(&c, e.uid), Level::Error)
                .join("\n")
                .contains("Plan 3")
        );
    }

    #[test]
    fn reload_keeps_old_config_on_error() {
        let e = env("[routers.pppoe]\nenabled=true\n", "");
        let mut cfg = load_config(&e.path).unwrap();
        std::fs::write(&e.path, "[runtime]\nscan_intervall=5\n").unwrap();
        assert!(reload(&mut cfg, &e.path).is_err());
        assert_eq!(cfg.routers[0].name, "R1");
        std::fs::write(&e.path, e.toml.replace("\"R1\"", "\"R2\"")).unwrap();
        reload(&mut cfg, &e.path).unwrap();
        assert_eq!(cfg.routers[0].name, "R2");
    }

    #[test]
    fn check_reads_the_telegram_token_when_enabled() {
        let e = env(
            "[routers.pppoe]\nenabled=true\n",
            "[telegram]\nenabled=true\nchat_id=\"1\"\nbot_token_file=\"/nonexistent/telegram.token\"\n",
        );
        let lines = check(&Config::from_toml_str(&e.toml).unwrap(), e.uid);
        let err = texts(&lines, Level::Error).join("\n");
        assert!(err.contains("telegram"), "{lines:?}");
        assert!(
            !texts(&lines, Level::Warn).join("\n").contains("Plan 2b"),
            "{lines:?}"
        );
    }
}
