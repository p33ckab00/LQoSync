//! Command line: argument parse and the commands.

use crate::config::{Config, RuntimeMode, Target};
use crate::secrets::read_secret;
use anyhow::{Context, bail};
use std::path::{Path, PathBuf};

/// The default config file.
pub const DEFAULT_CONFIG: &str = "/etc/lqosync3/lqosync3.toml";

const USAGE: &str = "usage: lqosync3 <check | detect | once [--force] | run | diff | tui | defaults | version> [--config <path>]\n       lqosync3 router-edit [--config <path>]  (one JSON request on stdin, from the web broker)\n       lqosync3 sync-now [--config <path>]  (start a cycle now, from the web broker)\n       lqosync3 api status [--config <path>]\n       lqosync3 topology <policy propose --out <file> | check --policy <file> | policy install --policy <file> | propose --out <file> [--previous <file>] [--replace] | enroll --mapping <file> [--yes] [--replace] | activate [--yes] | live-test --report <file> [--import-timeout <seconds>] | live-test-restore --backup <dir>> [--config <path>]";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Api(ApiCommand),
    Topology(TopologyCommand),
    Check,
    Once {
        force: bool,
    },
    Run,
    Diff,
    Defaults,
    Detect,
    Tui,
    Version,
    /// One local router change from the web broker: JSON on stdin, JSON on stdout.
    RouterEdit,
    /// Start a cycle now (SIGUSR1 to the service): JSON on stdout.
    SyncNow,
}

/// Service API commands for operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiCommand {
    Status,
}

/// Topology Manager policy, enrollment, and explicit activation commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyCommand {
    PolicyPropose {
        out: PathBuf,
    },
    Check {
        policy: PathBuf,
    },
    PolicyInstall {
        policy: PathBuf,
    },
    Propose {
        out: PathBuf,
        previous: Option<PathBuf>,
        replace: bool,
    },
    Enroll {
        mapping: PathBuf,
        yes: bool,
        replace: bool,
    },
    /// Compile current enrolled sources. Only `--yes` claims the writer and publishes.
    Activate {
        yes: bool,
    },
    /// One live acceptance test on production. The service must be stopped.
    LiveTest {
        report: PathBuf,
        import_timeout: u64,
    },
    /// Put the backup pair back, request an import, and wait for it.
    LiveTestRestore {
        backup: PathBuf,
    },
}

/// Default and maximum wait, in seconds, for one LibreQoS import in the live test.
pub const DEFAULT_IMPORT_TIMEOUT: u64 = 120;
const MAX_IMPORT_TIMEOUT: u64 = 3600;

fn parse_topology(
    words: &[String],
    flags: &std::collections::BTreeMap<String, String>,
    yes: bool,
    replace: bool,
) -> anyhow::Result<TopologyCommand> {
    let file = |name: &str| -> anyhow::Result<PathBuf> {
        flags
            .get(name)
            .map(PathBuf::from)
            .with_context(|| format!("{name} needs a path\n{USAGE}"))
    };
    let words: Vec<_> = words.iter().map(String::as_str).collect();
    let allowed: &[&str] = match words.as_slice() {
        ["policy", "propose"] | ["propose"] => &["--out", "--previous"],
        ["check"] | ["policy", "install"] => &["--policy"],
        ["enroll"] => &["--mapping"],
        ["activate"] => &[],
        ["live-test"] => &["--report", "--import-timeout"],
        ["live-test-restore"] => &["--backup"],
        _ => bail!("unknown topology command\n{USAGE}"),
    };
    if let Some(extra) = flags.keys().find(|k| !allowed.contains(&k.as_str())) {
        bail!("unknown argument \"{extra}\"\n{USAGE}");
    }
    if yes && words != ["enroll"] && words != ["activate"] {
        bail!("--yes is only for topology enroll and activate\n{USAGE}");
    }
    if replace && words != ["enroll"] && words != ["propose"] {
        bail!("--replace is only for topology propose and enroll\n{USAGE}");
    }
    Ok(match words.as_slice() {
        ["policy", "propose"] if !flags.contains_key("--previous") => {
            TopologyCommand::PolicyPropose {
                out: file("--out")?,
            }
        }
        ["propose"] => TopologyCommand::Propose {
            out: file("--out")?,
            previous: flags.get("--previous").map(PathBuf::from),
            replace,
        },
        ["check"] => TopologyCommand::Check {
            policy: file("--policy")?,
        },
        ["policy", "install"] => TopologyCommand::PolicyInstall {
            policy: file("--policy")?,
        },
        ["enroll"] => TopologyCommand::Enroll {
            mapping: file("--mapping")?,
            yes,
            replace,
        },
        ["activate"] => TopologyCommand::Activate { yes },
        ["live-test"] => TopologyCommand::LiveTest {
            report: file("--report")?,
            import_timeout: match flags.get("--import-timeout") {
                None => DEFAULT_IMPORT_TIMEOUT,
                Some(v) => match v.parse::<u64>() {
                    Ok(n) if (1..=MAX_IMPORT_TIMEOUT).contains(&n) => n,
                    _ => bail!(
                        "--import-timeout must be a whole number from 1 to {MAX_IMPORT_TIMEOUT} seconds\n{USAGE}"
                    ),
                },
            },
        },
        ["live-test-restore"] => TopologyCommand::LiveTestRestore {
            backup: file("--backup")?,
        },
        _ => bail!("unknown argument \"--previous\"\n{USAGE}"),
    })
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
    let mut api = false;
    let (mut topology, mut words, mut flags, mut yes, mut replace) = (
        false,
        vec![],
        std::collections::BTreeMap::new(),
        false,
        false,
    );
    while let Some(a) = it.next() {
        match a.as_str() {
            "topology" if command.is_none() && !topology && !api => topology = true,
            "api" if command.is_none() && !topology && !api => api = true,
            "--config" => match it.next() {
                Some(p) => config = PathBuf::from(p),
                None => bail!("--config needs a path\n{USAGE}"),
            },
            "--yes" if topology => yes = true,
            "--replace" if topology => replace = true,
            "--out" | "--previous" | "--policy" | "--mapping" | "--report" | "--backup"
            | "--import-timeout"
                if topology =>
            {
                match it.next() {
                    Some(p) if flags.insert(a.clone(), p.clone()).is_none() => {}
                    Some(_) => bail!("{a} is given twice\n{USAGE}"),
                    None => bail!("{a} needs a path\n{USAGE}"),
                }
            }
            w if (topology || api) && !w.starts_with('-') => words.push(a.clone()),
            "check" | "detect" | "once" | "run" | "diff" | "tui" | "defaults" | "version"
            | "router-edit" | "sync-now"
                if command.is_none() =>
            {
                command = Some(a.clone())
            }
            "--version" if command.is_none() => command = Some("version".into()),
            "--force" if !topology => force = true,
            other => bail!("unknown argument \"{other}\"\n{USAGE}"),
        }
    }
    if api {
        return match words
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice()
        {
            ["status"] => Ok(Args {
                command: Command::Api(ApiCommand::Status),
                config,
            }),
            _ => bail!("unknown api command\n{USAGE}"),
        };
    }
    if topology {
        return Ok(Args {
            command: Command::Topology(parse_topology(&words, &flags, yes, replace)?),
            config,
        });
    }
    let command = match command.as_deref() {
        Some("check") => Command::Check,
        Some("router-edit") => Command::RouterEdit,
        Some("sync-now") => Command::SyncNow,
        Some("once") => Command::Once { force },
        Some("run") => Command::Run,
        Some("diff") => Command::Diff,
        Some("defaults") => Command::Defaults,
        Some("detect") => Command::Detect,
        Some("tui") => Command::Tui,
        Some("version") => Command::Version,
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

/// The TOML name of a target.
pub fn target_name(t: Target) -> &'static str {
    match t {
        Target::Auto => "auto",
        Target::V2Pair => "v2_pair",
        Target::V3Pair => "v3_pair",
        Target::V3Network => "v3_network",
    }
}

/// The digest of a config file text: 16 hex digits (FNV-1a 64).
pub fn config_digest(bytes: &[u8]) -> String {
    format!("{:016x}", crate::topology::fnv1a64(bytes))
}

/// Read and check the config file. Also gives the digest of the same bytes.
pub fn read_config(path: &Path) -> anyhow::Result<(Config, String)> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let cfg = Config::from_toml_str(&text).with_context(|| format!("{}", path.display()))?;
    Ok((cfg, config_digest(text.as_bytes())))
}

/// Read the config again. On an error, `cfg` and `digest` stay the same.
pub fn reload(cfg: &mut Config, digest: &mut String, path: &Path) -> anyhow::Result<()> {
    let (next, next_digest) = read_config(path)?;
    anyhow::ensure!(
        next.runtime.state_dir == cfg.runtime.state_dir,
        "changing runtime.state_dir requires a service restart; the active state lock stays in use"
    );
    (*cfg, *digest) = (next, next_digest);
    Ok(())
}

/// A warning if `status.json` in `state_dir` shows a loaded config that is not `digest` (the file on disk).
pub fn digest_line(state_dir: &Path, digest: &str) -> Option<CheckLine> {
    let text = std::fs::read_to_string(state_dir.join("status.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let loaded = v["config_digest"].as_str().filter(|d| !d.is_empty())?;
    (loaded != digest).then(|| {
        line(
            Level::Warn,
            format!("the config file on disk is not the loaded config (status.json has {loaded}, the file has {digest}); send SIGHUP to load it"),
        )
    })
}

/// The checks of `lqosync3 check` (runtime design section 8). `owner_uid` is the required secret owner.
pub fn check(cfg: &Config, owner_uid: u32) -> Vec<CheckLine> {
    check_with(cfg, owner_uid, &std::collections::HashSet::new())
}

/// `check`, but a password or token file in `typed` counts as readable: the TUI writes it at apply.
pub fn check_with(
    cfg: &Config,
    owner_uid: u32,
    typed: &std::collections::HashSet<PathBuf>,
) -> Vec<CheckLine> {
    let mut out = Vec::new();
    let target = match crate::detect::resolve(&cfg.libreqos) {
        Ok(d) => {
            out.push(line(
                Level::Ok,
                format!("target {} ({})", target_name(d.target), d.reason),
            ));
            Some(d.target)
        }
        Err(e) => {
            out.push(line(Level::Error, format!("libreqos.target: {e:#}")));
            None
        }
    };
    let state = Path::new(&cfg.runtime.state_dir);
    if let Some(p) = crate::probe::cached(&cfg.libreqos, state) {
        let native = target == Some(crate::config::Target::V3Network);
        out.push(match p {
            Some(p) if p.ready() => line(Level::Ok, "native schema 2: ready"),
            Some(p) => line(
                if native { Level::Error } else { Level::Ok },
                format!(
                    "native schema 2: {}; keep [topology] source = \"shaped_devices\"",
                    p.summary()
                ),
            ),
            None => line(
                Level::Ok,
                "native schema 2: not probed for this LibreQoS yet; the service probes it in its next cycle",
            ),
        });
    }
    for r in &cfg.routers {
        let at = format!("router {}", r.name);
        if typed.contains(Path::new(&r.password_file)) {
            out.push(line(
                Level::Ok,
                format!("{at}: a new password will be written"),
            ));
        } else {
            match read_secret(Path::new(&r.password_file), owner_uid) {
                Ok(_) => out.push(line(Level::Ok, format!("{at}: password file is readable"))),
                Err(e) => out.push(line(Level::Error, format!("{at}: {e:#}"))),
            }
        }
        if !r.writer_password_file.is_empty() && !typed.contains(Path::new(&r.writer_password_file))
        {
            match read_secret(Path::new(&r.writer_password_file), owner_uid) {
                Ok(_) => out.push(line(
                    Level::Ok,
                    format!("{at}: writer password file is readable"),
                )),
                Err(e) => out.push(line(Level::Error, format!("{at}: writer password: {e:#}"))),
            }
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
        } else if !r.tls_ca_file.is_empty()
            // A CA file that the same apply writes counts as readable, as a typed secret does.
            && !typed.contains(Path::new(&r.tls_ca_file))
            && std::fs::metadata(&r.tls_ca_file).is_err()
        {
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
    let lqos_state = Path::new(&cfg.libreqos.state_directory);
    if !lqos_state.is_dir() {
        out.push(line(
            Level::Warn,
            format!(
                "libreqos.state_directory {} does not exist; on LibreQoS 2.x each reload is then a full run",
                lqos_state.display()
            ),
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
    if tg.enabled && typed.contains(Path::new(&tg.bot_token_file)) {
        out.push(line(Level::Ok, "telegram: a new bot token will be written"));
    } else if tg.enabled {
        match crate::notify::token(tg, owner_uid) {
            Ok(_) => out.push(line(Level::Ok, "telegram: the bot token is readable")),
            Err(e) => out.push(line(Level::Error, format!("telegram: {e}"))),
        }
    }
    if tg.enabled && crate::notify::chat_id(tg).is_empty() {
        out.push(line(Level::Error, "telegram: the chat ID is empty"));
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
    fn api_status_parses() {
        assert_eq!(
            parse(&s(&["l", "api", "status"])).unwrap().command,
            Command::Api(ApiCommand::Status)
        );
        assert!(parse(&s(&["l", "api"])).is_err());
        assert!(parse(&s(&["l", "api", "stop"])).is_err());
    }

    #[test]
    fn router_edit_parses_with_its_config() {
        let a = parse(&s(&["l", "router-edit", "--config", "/etc/x.toml"])).unwrap();
        assert_eq!(a.command, Command::RouterEdit);
        assert_eq!(a.config, PathBuf::from("/etc/x.toml"));
        assert!(parse(&s(&["l", "router-edit", "extra"])).is_err());
        assert_eq!(
            parse(&s(&["l", "sync-now"])).unwrap().command,
            Command::SyncNow
        );
    }

    #[test]
    fn topology_commands_parse() {
        use TopologyCommand as T;
        let cmd = |v: &[&str]| parse(&s(v)).unwrap().command;
        assert_eq!(
            cmd(&["l", "topology", "policy", "propose", "--out", "p.json"]),
            Command::Topology(T::PolicyPropose {
                out: "p.json".into()
            })
        );
        assert_eq!(
            cmd(&["l", "topology", "check", "--policy", "p.json"]),
            Command::Topology(T::Check {
                policy: "p.json".into()
            })
        );
        assert_eq!(
            cmd(&["l", "topology", "policy", "install", "--policy", "p.json"]),
            Command::Topology(T::PolicyInstall {
                policy: "p.json".into()
            })
        );
        assert_eq!(
            cmd(&[
                "l",
                "topology",
                "propose",
                "--out",
                "m.json",
                "--previous",
                "old.json"
            ]),
            Command::Topology(T::Propose {
                out: "m.json".into(),
                previous: Some("old.json".into()),
                replace: false
            })
        );
        assert_eq!(
            cmd(&["l", "topology", "enroll", "--mapping", "m.json", "--yes"]),
            Command::Topology(T::Enroll {
                mapping: "m.json".into(),
                yes: true,
                replace: false
            })
        );
        assert_eq!(
            cmd(&[
                "l",
                "topology",
                "enroll",
                "--mapping",
                "m.json",
                "--replace"
            ]),
            Command::Topology(T::Enroll {
                mapping: "m.json".into(),
                yes: false,
                replace: true
            })
        );
        assert!(
            parse(&s(&[
                "l",
                "topology",
                "check",
                "--policy",
                "p",
                "--replace"
            ]))
            .is_err()
        );
        let a = parse(&s(&[
            "l",
            "--config",
            "c.toml",
            "topology",
            "enroll",
            "--mapping",
            "m.json",
        ]))
        .unwrap();
        assert_eq!(a.config, PathBuf::from("c.toml"));
        assert_eq!(
            a.command,
            Command::Topology(T::Enroll {
                mapping: "m.json".into(),
                yes: false,
                replace: false
            })
        );
    }

    #[test]
    fn topology_activation_is_an_explicit_dry_run_or_confirmation() {
        for yes in [false, true] {
            let mut args = s(&["l", "topology", "activate"]);
            if yes {
                args.push("--yes".into());
            }
            assert_eq!(
                parse(&args).unwrap().command,
                Command::Topology(TopologyCommand::Activate { yes })
            );
        }
        assert!(parse(&s(&["l", "topology", "activate", "--replace"])).is_err());
        assert!(parse(&s(&["l", "topology", "activate", "--mapping", "m"])).is_err());
    }

    #[test]
    fn topology_commands_need_their_files() {
        for v in [
            &["l", "topology"][..],
            &["l", "topology", "check"],
            &["l", "topology", "policy", "propose"],
            &["l", "topology", "policy", "remove", "--policy", "p"],
            &["l", "topology", "propose"],
            &["l", "topology", "enroll", "--yes"],
            &["l", "topology", "enroll", "--mapping", "m", "--force"],
        ] {
            assert!(parse(&s(v)).is_err(), "{v:?}");
        }
    }

    #[test]
    fn live_test_flags_parse_with_the_default_timeout() {
        use TopologyCommand as T;
        let cmd = |v: &[&str]| parse(&s(v)).unwrap().command;
        assert_eq!(
            cmd(&["l", "topology", "live-test", "--report", "r.json"]),
            Command::Topology(T::LiveTest {
                report: "r.json".into(),
                import_timeout: 120
            })
        );
        assert_eq!(
            cmd(&[
                "l",
                "topology",
                "live-test",
                "--report",
                "r.json",
                "--import-timeout",
                "3600"
            ]),
            Command::Topology(T::LiveTest {
                report: "r.json".into(),
                import_timeout: 3600
            })
        );
        for bad in ["0", "3601", "-1", "x", "18446744073709551616"] {
            assert!(
                parse(&s(&[
                    "l",
                    "topology",
                    "live-test",
                    "--report",
                    "r",
                    "--import-timeout",
                    bad
                ]))
                .is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn live_test_restore_needs_a_backup() {
        use TopologyCommand as T;
        assert!(parse(&s(&["l", "topology", "live-test-restore"])).is_err());
        assert!(parse(&s(&["l", "topology", "live-test"])).is_err());
        assert_eq!(
            parse(&s(&["l", "topology", "live-test-restore", "--backup", "b"]))
                .unwrap()
                .command,
            Command::Topology(T::LiveTestRestore { backup: "b".into() })
        );
    }

    #[test]
    fn unknown_live_test_flags_are_refused() {
        for v in [
            &["l", "topology", "live-test", "--report", "r", "--yes"][..],
            &[
                "l",
                "topology",
                "live-test",
                "--report",
                "r",
                "--backup",
                "b",
            ],
            &["l", "topology", "live-test", "--report", "r", "--force"],
            &[
                "l",
                "topology",
                "live-test-restore",
                "--backup",
                "b",
                "--yes",
            ],
            &[
                "l",
                "topology",
                "live-test-restore",
                "--backup",
                "b",
                "--report",
                "r",
            ],
            &[
                "l",
                "topology",
                "live-test-restore",
                "--backup",
                "b",
                "--import-timeout",
                "5",
            ],
            &[
                "l",
                "topology",
                "enroll",
                "--mapping",
                "m",
                "--import-timeout",
                "5",
            ],
        ] {
            assert!(parse(&s(v)).is_err(), "{v:?}");
        }
    }

    #[test]
    fn version_is_3_0_commit_count_and_hash() {
        // 3.0.<commits of the branch>+g<short hash>, ".dirty" with changes that are not committed.
        let v = crate::VERSION;
        let (core, build) = v.split_once('+').expect(v);
        let parts: Vec<&str> = core.split('.').collect();
        assert_eq!(&parts[..2], ["3", "0"], "{v}");
        assert!(parts[2].parse::<u64>().is_ok(), "{v}");
        let hash = build.strip_suffix(".dirty").unwrap_or(build);
        assert!(
            build == "unknown"
                || (hash.len() >= 8
                    && hash.starts_with('g')
                    && hash[1..].chars().all(|c| c.is_ascii_hexdigit())),
            "{v}"
        );
    }

    #[test]
    fn parse_version() {
        for a in ["version", "--version"] {
            assert_eq!(
                parse(&s(&["lqosync3", a])).unwrap().command,
                Command::Version
            );
        }
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
        std::fs::create_dir_all(lqos.join("state")).unwrap();
        // The checks must not depend on the host: a LibreQoS 2.2 package of its own.
        let dpkg = dir.path().join("dpkg-status");
        std::fs::write(
            &dpkg,
            "Package: libreqos\nStatus: install ok installed\nVersion: 2.2.1\n",
        )
        .unwrap();
        let toml = format!(
            "[runtime]\nstate_dir=\"{}\"\n[libreqos]\ndirectory=\"{}\"\nstate_directory=\"{}\"\ndpkg_status=\"{}\"\n{extra}\n\
             [[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"{}\"\n{extra_router}\n",
            dir.path().join("state").display(),
            lqos.display(),
            lqos.join("state").display(),
            dpkg.display(),
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
    fn check_reports_a_missing_writer_password_file() {
        let e = env(
            "writer_username=\"w\"\nwriter_password_file=\"/nonexistent/w.password\"\n",
            "",
        );
        let lines = check(&Config::from_toml_str(&e.toml).unwrap(), e.uid);
        let errors = texts(&lines, Level::Error).join("\n");
        assert!(errors.contains("writer password"), "{errors}");
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
    fn check_warns_when_the_libreqos_state_directory_is_missing() {
        let e = env("[routers.pppoe]\nenabled=true\n", "");
        let mut c = Config::from_toml_str(&e.toml).unwrap();
        c.libreqos.state_directory = "/nonexistent/lqos-state".into();
        let warn = texts(&check(&c, e.uid), Level::Warn).join("\n");
        assert!(warn.contains("/nonexistent/lqos-state"), "{warn}");
        assert!(warn.contains("full run"), "{warn}");
        c.libreqos.state_directory = std::env::temp_dir().display().to_string();
        let warn = texts(&check(&c, e.uid), Level::Warn).join("\n");
        assert!(!warn.contains("state_directory"), "{warn}");
    }

    #[test]
    fn check_shows_the_native_probe_of_libreqos_v3() {
        let e = env("[routers.pppoe]\nenabled=true\n", "");
        let mut c = Config::from_toml_str(&e.toml).unwrap();
        assert!(
            !format!("{:?}", check(&c, e.uid)).contains("native"),
            "no line on v2"
        );
        let lqos = Path::new(&c.libreqos.directory).to_path_buf();
        std::fs::write(
            &c.libreqos.dpkg_status,
            "Package: libreqos\nStatus: install ok installed\nVersion: 3.0.7.1\n",
        )
        .unwrap();
        std::fs::write(lqos.join(crate::probe::BINDING), "x").unwrap();
        c.libreqos.v3_config = lqos.join("lqos.conf").display().to_string();
        std::fs::write(
            &c.libreqos.v3_config,
            "[topology]\nsource = \"shaped_devices\"\n",
        )
        .unwrap();
        let all = |c: &Config| format!("{:?}", check(c, e.uid));
        assert!(
            all(&c).contains("native schema 2: not probed"),
            "{}",
            all(&c)
        );

        let state = Path::new(&c.runtime.state_dir);
        std::fs::create_dir_all(state).unwrap();
        let p = crate::probe::Probe {
            key: crate::probe::key(&c.libreqos).unwrap(),
            state: "blocked".into(),
            detail: "node PROBE-SITE compiled to 1/1 Mbps".into(),
            at: 1,
        };
        std::fs::write(
            state.join(crate::probe::CACHE),
            serde_json::to_string(&p).unwrap(),
        )
        .unwrap();
        let ok = texts(&check(&c, e.uid), Level::Ok).join("\n");
        assert!(
            ok.contains("native schema 2: blocked: node PROBE-SITE"),
            "{ok}"
        );
        // With the native source, a blocked probe is an error.
        std::fs::write(
            &c.libreqos.v3_config,
            "[topology]\nsource = \"libreqos_network\"\n",
        )
        .unwrap();
        c.libreqos.target = crate::config::Target::Auto;
        let err = texts(&check(&c, e.uid), Level::Error).join("\n");
        assert!(err.contains("native schema 2: blocked"), "{err}");
    }

    #[test]
    fn check_shows_the_target_or_the_detection_error() {
        let e = env("[routers.pppoe]\nenabled=true\n", "");
        let mut c = Config::from_toml_str(&e.toml).unwrap();
        c.libreqos.target = crate::config::Target::V3Pair;
        let lines = check(&c, e.uid);
        assert!(
            texts(&lines, Level::Ok)
                .join("\n")
                .contains("target v3_pair"),
            "{lines:?}"
        );
        assert!(!format!("{lines:?}").contains("Plan 3"), "{lines:?}");
        c.libreqos.target = crate::config::Target::Auto;
        c.libreqos.dpkg_status = "/nonexistent/status".into();
        assert!(
            texts(&check(&c, e.uid), Level::Error)
                .join("\n")
                .contains("/nonexistent/status")
        );
    }

    #[test]
    fn tui_command_parses() {
        let a = parse(&s(&["lqosync3", "tui", "--config", "/x.toml"])).unwrap();
        assert_eq!(
            (a.command, a.config),
            (Command::Tui, PathBuf::from("/x.toml"))
        );
    }

    #[test]
    fn parse_detect() {
        assert_eq!(
            parse(&s(&["lqosync3", "detect"])).unwrap().command,
            Command::Detect
        );
    }

    #[test]
    fn reload_keeps_old_config_on_error() {
        let e = env("[routers.pppoe]\nenabled=true\n", "");
        let (mut cfg, mut digest) = read_config(&e.path).unwrap();
        assert_eq!(digest, config_digest(e.toml.as_bytes()));
        let first = digest.clone();
        std::fs::write(&e.path, "[runtime]\nscan_intervall=5\n").unwrap();
        assert!(reload(&mut cfg, &mut digest, &e.path).is_err());
        assert_eq!((cfg.routers[0].name.as_str(), &digest), ("R1", &first));
        std::fs::write(&e.path, e.toml.replace("\"R1\"", "\"R2\"")).unwrap();
        reload(&mut cfg, &mut digest, &e.path).unwrap();
        assert_eq!(cfg.routers[0].name, "R2");
        assert_ne!(digest, first);
    }

    #[test]
    fn reload_requires_restart_for_a_different_state_directory() {
        let e = env("[routers.pppoe]\nenabled=true\n", "");
        let (mut cfg, mut digest) = read_config(&e.path).unwrap();
        let original = cfg.runtime.state_dir.clone();
        let old_digest = digest.clone();
        std::fs::write(
            &e.path,
            e.toml.replace(&original, "/tmp/lqosync-different-state"),
        )
        .unwrap();
        assert!(reload(&mut cfg, &mut digest, &e.path).is_err());
        assert_eq!(cfg.runtime.state_dir, original);
        assert_eq!(digest, old_digest);
    }

    #[test]
    fn config_digest_is_stable_and_16_hex() {
        let d = config_digest(b"a");
        assert_eq!(d, config_digest(b"a"));
        assert_ne!(d, config_digest(b"b"));
        assert_eq!(d.len(), 16);
        assert!(d.chars().all(|c| c.is_ascii_hexdigit()), "{d}");
    }

    #[test]
    fn check_warns_when_the_file_is_not_loaded() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("status.json"), r#"{"config_digest":"aaa"}"#).unwrap();
        let l = digest_line(dir.path(), "bbb").unwrap();
        assert_eq!(l.level, Level::Warn);
        assert!(l.text.contains("not the loaded config"), "{}", l.text);
        assert_eq!(digest_line(dir.path(), "aaa"), None);
    }

    #[test]
    fn check_has_no_warning_without_status_json() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(digest_line(dir.path(), "bbb"), None);
        std::fs::write(dir.path().join("status.json"), r#"{"version":2}"#).unwrap();
        assert_eq!(digest_line(dir.path(), "bbb"), None);
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
