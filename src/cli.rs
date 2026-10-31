//! Command line: argument parse and the commands.

use anyhow::bail;
use std::path::PathBuf;

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
}
