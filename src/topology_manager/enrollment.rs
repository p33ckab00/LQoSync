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

pub(crate) fn prepare(
    cfg: &crate::config::Config,
    pair: &PreparedPair,
    manifest: &MigrationManifest,
    protected_roots: &[String],
    attachments: &[StaticAttachment],
) -> anyhow::Result<StaticAdoption> {
    pair.check_fresh()?;
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
