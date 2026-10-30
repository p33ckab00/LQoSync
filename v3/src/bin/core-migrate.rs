//! Explicit local maintenance entry point. Never run by the console or scheduler.
use anyhow::{Context, Result, ensure};
use lqosync_v3::{
    core_adapter::{self, Prepared},
    core_migration::RowKey,
    store,
};
use std::{
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
fn private_read(path: &Path) -> Result<String> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    let m = file.metadata()?;
    // SAFETY: geteuid takes no arguments and does not retain pointers.
    ensure!(
        m.is_file() && m.mode() & 0o077 == 0 && m.uid() == unsafe { libc::geteuid() },
        "Plan and scope files must be private regular files owned by the operator"
    );
    ensure!(
        m.len() <= 64 * 1024 * 1024,
        "Maintenance input exceeds size limit"
    );
    let mut text = String::new();
    file.take(64 * 1024 * 1024 + 1).read_to_string(&mut text)?;
    ensure!(
        text.len() <= 64 * 1024 * 1024,
        "Maintenance input exceeds size limit"
    );
    Ok(text)
}
#[tokio::main]
async fn main() -> Result<()> {
    let a = std::env::args().collect::<Vec<_>>();
    ensure!(
        a.len() >= 2,
        "Usage: core-migrate prepare ROOT SCOPE_JSON PLAN_JSON | apply/rollback ROOT PLAN_JSON REVISION EXCLUSIVE_ROUTER_ACCESS:REVISION"
    );
    let root = Path::new(a.get(2).context("ROOT required")?).canonicalize()?;
    match a[1].as_str() {
        "prepare" => {
            ensure!(a.len() == 5, "prepare requires ROOT SCOPE_JSON PLAN_JSON");
            let scope: Vec<RowKey> = serde_json::from_str(&private_read(Path::new(&a[3]))?)?;
            ensure!(
                !Path::new(&a[4]).exists(),
                "Plan output already exists; use a new private filename"
            );
            let prepared = core_adapter::prepare(&root, scope).await?;
            store::atomic_write(Path::new(&a[4]), &serde_json::to_string_pretty(&prepared)?)?;
            println!(
                "Prepared review-only plan revision: {}",
                prepared.revision()?
            );
        }
        "apply" | "rollback" => {
            ensure!(
                a.len() == 6,
                "apply/rollback requires ROOT PLAN_JSON REVISION EXCLUSIVE_ROUTER_ACCESS:REVISION"
            );
            let plan: Prepared = serde_json::from_str(&private_read(Path::new(&a[3]))?)?;
            core_adapter::execute(&root, &plan, &a[4], &a[5], a[1] == "rollback").await?;
            println!("Maintenance operation verified. No LibreQoS apply was requested.");
        }
        _ => anyhow::bail!("Unknown maintenance command"),
    }
    Ok(())
}
