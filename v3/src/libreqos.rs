use anyhow::{Context, Result, bail, ensure};
use std::{process::Stdio, time::Duration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Adapter {
    Legacy,
    V3,
}

pub fn select(version: &str) -> Result<Adapter> {
    let release = if let Some((epoch, release)) = version.split_once(':') {
        ensure!(
            !epoch.is_empty()
                && epoch.bytes().all(|b| b.is_ascii_digit())
                && !release.contains(':'),
            "Invalid version epoch"
        );
        release
    } else {
        version
    };
    let numeric = release
        .split(['-', '~', '+'])
        .next()
        .context("Missing release")?;
    ensure!(
        numeric
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())),
        "Invalid numeric release"
    );
    let mut parts = numeric.split('.');
    let major = parts
        .next()
        .context("Missing major version")?
        .parse::<u32>()?;
    let minor = parts
        .next()
        .context("Missing minor version")?
        .parse::<u32>()?;
    match (major, minor) {
        (2, 0..=2) => Ok(Adapter::Legacy),
        (3, _) => Ok(Adapter::V3),
        _ => bail!(
            "Unsupported LibreQoS version {version}; supported adapter families are 2.0–2.2 and 3.x"
        ),
    }
}
pub fn installed_version(output: &str) -> Result<String> {
    let candidates: Vec<_> = output
        .lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            (fields.len() == 3
                && fields[0] == "libreqos"
                && fields[2] == "installed"
                && !fields[1].is_empty())
            .then(|| fields[1].to_string())
        })
        .collect();
    ensure!(
        candidates.len() == 1,
        "No unique, fully installed main libreqos package was detected"
    );
    Ok(candidates[0].clone())
}

impl Adapter {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Legacy => "Legacy v2 adapter",
            Self::V3 => "V3 scheduler adapter",
        }
    }
    pub fn request_description(&self) -> &'static str {
        match self {
            Self::Legacy => "/opt/libreqos/src/LibreQoS.py --updateonly",
            Self::V3 => {
                "ReloadLibreQoS via authenticated v3 control interface; then verify the applied generation"
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Detection {
    pub version: Option<String>,
    pub adapter: Option<Adapter>,
    pub detail: String,
}

pub async fn detect() -> Detection {
    let result = async {
        let output = tokio::process::Command::new("dpkg-query")
            .args([
                "-W",
                "-f=${Package}\t${Version}\t${db:Status-Status}\n",
                "libreqos",
            ])
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output()
            .await?;
        ensure!(
            output.status.success(),
            "Main LibreQoS package could not be queried"
        );
        installed_version(&String::from_utf8(output.stdout)?)
    };
    match tokio::time::timeout(Duration::from_secs(5), result).await {
        Ok(Ok(version)) => match select(&version) {
            Ok(adapter) => Detection {
                version: Some(version),
                adapter: Some(adapter),
                detail: "Adapter selected. Production execution remains disabled.".into(),
            },
            Err(e) => Detection {
                version: Some(version),
                adapter: None,
                detail: e.to_string(),
            },
        },
        Ok(Err(e)) => Detection {
            version: None,
            adapter: None,
            detail: format!("Detection unavailable: {e}"),
        },
        Err(_) => Detection {
            version: None,
            adapter: None,
            detail: "Package detection timed out".into(),
        },
    }
}
