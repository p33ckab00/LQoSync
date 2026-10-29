use crate::model::Router;
use anyhow::{Result, ensure};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Credentials {
    routers: BTreeMap<String, Secret>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Secret {
    password: String,
}
pub async fn password(root: &Path, router: &Router) -> Result<String> {
    let path = root.join("credentials.toml");
    match tokio::fs::metadata(&path).await {
        Ok(metadata) => {
            ensure!(
                metadata.len() <= 1024 * 1024,
                "Local credential TOML exceeds size limit"
            );
            let text = tokio::fs::read_to_string(path)
                .await
                .map_err(|_| anyhow::anyhow!("Cannot read local credential TOML"))?;
            // TOML parser errors can quote the secret line. Never forward them.
            let mut credentials: Credentials = toml::from_str(&text)
                .map_err(|_| anyhow::anyhow!("Invalid local credential TOML format"))?;
            if let Some(secret) = credentials.routers.remove(&router.id) {
                ensure!(
                    !secret.password.is_empty(),
                    "Local router password is empty; complete credentials.toml first"
                );
                return Ok(secret.password);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => anyhow::bail!("Cannot access local credential TOML"),
    }
    std::env::var(&router.password_env).map_err(|_| {
        anyhow::anyhow!(
            "No local TOML credential or available password environment reference for router"
        )
    })
}
