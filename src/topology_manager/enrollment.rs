//! Confirm the current legacy files before controlled enrollment. No output writes occur here.
use super::{
    admission::MAX_PAIR_BYTES,
    compiler::read_regular,
    legacy_static::{self, StaticAdoption},
    migration::MigrationManifest,
    ownership::StaticAttachment,
    publication::PreparedPair,
};
use anyhow::Context;
use std::path::Path;

/// Read the live legacy pair as two bounded regular files. The CSV row order stays unchanged.
pub(crate) fn read_live_pair(
    cfg: &crate::config::Config,
) -> anyhow::Result<(Vec<crate::model::CsvRow>, serde_json::Value)> {
    let directory = Path::new(&cfg.libreqos.directory);
    let csv = read_regular(&directory.join("ShapedDevices.csv"), MAX_PAIR_BYTES, true)
        .context("cannot read the current legacy CSV")?;
    let network = read_regular(
        &directory.join("network.json"),
        MAX_PAIR_BYTES - csv.len(),
        true,
    )
    .context("cannot read the current legacy network")?;
    let rows =
        crate::state::read_csv(std::str::from_utf8(&csv).context("legacy CSV is not UTF-8")?)
            .map_err(|_| anyhow::anyhow!("cannot decode the current legacy CSV"))?;
    let network = serde_json::from_slice(&network)
        .map_err(|_| anyhow::anyhow!("cannot decode the current legacy network"))?;
    Ok((rows, network))
}

pub(crate) fn prepare(
    cfg: &crate::config::Config,
    pair: &PreparedPair,
    manifest: &MigrationManifest,
    protected_roots: &[String],
    attachments: &[StaticAttachment],
) -> anyhow::Result<StaticAdoption> {
    pair.check_fresh()?;
    let (rows, network) = read_live_pair(cfg)?;
    legacy_static::adopt_configured(
        cfg,
        pair,
        &rows,
        &network,
        manifest,
        protected_roots,
        attachments,
    )
}
