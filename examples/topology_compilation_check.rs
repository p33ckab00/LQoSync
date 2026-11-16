//! Check scratch compiler output. This command does not authorize publication or access native sockets.
use anyhow::{Context, ensure};
use std::{io::Read, path::Path};
fn read(path: &str, remaining: &mut usize) -> anyhow::Result<String> {
    let limit = *remaining;
    let mut bytes = Vec::new();
    std::fs::File::open(Path::new(path))?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= limit,
        "compilation input exceeds byte budget"
    );
    *remaining -= bytes.len();
    Ok(String::from_utf8(bytes)?)
}
fn main() -> anyhow::Result<()> {
    use lqosync3::topology_manager::admission::{
        MAX_ARTIFACT_BYTES, MAX_PAIR_BYTES, check_artifacts,
    };
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 4,
        "expected network, CSV, canonical, and compiled shaping paths"
    );
    let mut pair_budget = MAX_PAIR_BYTES;
    let mut artifact_budget = MAX_ARTIFACT_BYTES;
    let network = read(&args[0], &mut pair_budget)?;
    let csv = read(&args[1], &mut pair_budget)?;
    let canonical = serde_json::from_str(&read(&args[2], &mut artifact_budget)?)?;
    let shaping = serde_json::from_str(&read(&args[3], &mut artifact_budget)?)?;
    check_artifacts(&network, &csv, &canonical, &shaping)
        .context("compiled candidate admission failed")?;
    println!(
        "compiled candidate preserves requested policy and endpoints; no live apply performed"
    );
    Ok(())
}
