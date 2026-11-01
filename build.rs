//! Sets LQOSYNC_VERSION: `<major>.<minor>.<commits>+g<hash>`, with `.dirty` for changes that
//! are not committed. Without git, `<major>.<minor>.0+unknown`.

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn main() {
    // Each commit adds a line to logs/HEAD; a checkout changes HEAD; staging changes the index.
    // git-path also finds them in a worktree, where .git is a file.
    for p in ["HEAD", "logs/HEAD", "index"] {
        if let Some(path) = git(&["rev-parse", "--git-path", p]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    // A source change can make the build dirty.
    for p in ["src", "config", "Cargo.toml", "Cargo.lock", "build.rs"] {
        println!("cargo:rerun-if-changed={p}");
    }
    let major = std::env::var("CARGO_PKG_VERSION_MAJOR").unwrap();
    let minor = std::env::var("CARGO_PKG_VERSION_MINOR").unwrap();
    let version = match (
        git(&["rev-list", "--count", "HEAD"]),
        git(&["rev-parse", "--short=7", "HEAD"]),
    ) {
        (Some(count), Some(hash)) => {
            let dirty = git(&["status", "--porcelain", "--untracked-files=no"])
                .is_some_and(|s| !s.is_empty());
            format!(
                "{major}.{minor}.{count}+g{hash}{}",
                if dirty { ".dirty" } else { "" }
            )
        }
        _ => format!("{major}.{minor}.0+unknown"),
    };
    println!("cargo:rustc-env=LQOSYNC_VERSION={version}");
}
