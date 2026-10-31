//! The lqosync3 program.

use lqosync3::cli::{self, Command};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let parsed = match cli::parse(&args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e:#}");
            std::process::exit(2);
        }
    };
    match parsed.command {
        Command::Defaults => print!("{}", lqosync3::config::DEFAULTS_TOML),
        other => {
            eprintln!("{other:?}: not ready yet");
            std::process::exit(2);
        }
    }
}
