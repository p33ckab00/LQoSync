//! Target selection (ADR-0002): the LibreQoS version from dpkg and the v3 topology source.

use crate::config::{Libreqos, Target};
use anyhow::{Context, bail};

/// The version of an installed package in a dpkg status file. A removed package has no version.
pub fn package_version(dpkg_status: &str, package: &str) -> Option<String> {
    dpkg_status.split("\n\n").find_map(|stanza| {
        let field = |k: &str| {
            stanza
                .lines()
                .find_map(|l| l.strip_prefix(k).map(|v| v.trim().to_string()))
        };
        (field("Package:").as_deref() == Some(package)
            && field("Status:").is_some_and(|s| s.ends_with(" installed")))
        .then(|| field("Version:"))
        .flatten()
    })
}

/// `[topology] source` of a v3 `lqos.conf`.
pub fn topology_source(lqos_conf: &str) -> anyhow::Result<Option<String>> {
    let t: toml::Table = toml::from_str(lqos_conf).context("TOML parse error")?;
    Ok(t.get("topology")
        .and_then(|v| v.get("source"))
        .and_then(|v| v.as_str())
        .map(str::to_string))
}

/// The selected target and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    pub target: Target,
    pub reason: String,
}

/// Select the target. A fixed target in TOML is not changed.
pub fn resolve(cfg: &Libreqos) -> anyhow::Result<Detected> {
    if cfg.target != Target::Auto {
        return Ok(Detected {
            target: cfg.target,
            reason: "fixed in libreqos.target".into(),
        });
    }
    let status = std::fs::read_to_string(&cfg.dpkg_status)
        .with_context(|| format!("cannot read {}", cfg.dpkg_status))?;
    let Some(version) = package_version(&status, "libreqos") else {
        bail!(
            "the package libreqos is not installed ({}); set libreqos.target",
            cfg.dpkg_status
        );
    };
    match version.split('.').next() {
        Some("2") => Ok(Detected {
            target: Target::V2Pair,
            reason: format!("LibreQoS {version}"),
        }),
        Some("3") => {
            let text = std::fs::read_to_string(&cfg.v3_config)
                .with_context(|| format!("LibreQoS {version}: cannot read {}", cfg.v3_config))?;
            let source = topology_source(&text).with_context(|| cfg.v3_config.clone())?;
            match source.as_deref() {
                Some("shaped_devices") => Ok(Detected {
                    target: Target::V3Pair,
                    reason: format!("LibreQoS {version}, [topology] source = \"shaped_devices\""),
                }),
                Some("libreqos_network") => Ok(Detected {
                    target: Target::V3Network,
                    reason: format!("LibreQoS {version}, [topology] source = \"libreqos_network\""),
                }),
                Some(other) => bail!(
                    "LibreQoS {version}: [topology] source = \"{other}\" in {}; a built-in integration owns the topology. Set the source to \"shaped_devices\" or \"libreqos_network\", or stop lqosync3",
                    cfg.v3_config
                ),
                None => bail!(
                    "LibreQoS {version}: {} has no [topology] source; set it to \"shaped_devices\" or \"libreqos_network\"",
                    cfg.v3_config
                ),
            }
        }
        _ => bail!("LibreQoS {version} is not supported; set libreqos.target"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Libreqos, Target};

    const V22: &str = "Package: libreqos-preflight\nStatus: install ok installed\nVersion: 1.0\n\n\
                       Package: libreqos\nStatus: hold ok installed\nPriority: optional\nVersion: 2.2.202608181454\n\n";

    fn v3(state: &str) -> String {
        format!("Package: libreqos\nStatus: {state}\nVersion: 3.0.7.202610310038\n")
    }

    struct Env {
        _d: tempfile::TempDir,
        cfg: Libreqos,
    }

    fn env(dpkg: &str, conf: Option<&str>) -> Env {
        let d = tempfile::tempdir().unwrap();
        let status = d.path().join("status");
        std::fs::write(&status, dpkg).unwrap();
        let conf_path = d.path().join("lqos.conf");
        if let Some(c) = conf {
            std::fs::write(&conf_path, c).unwrap();
        }
        let cfg = Libreqos {
            dpkg_status: status.display().to_string(),
            v3_config: conf_path.display().to_string(),
            ..Libreqos::default()
        };
        Env { _d: d, cfg }
    }

    #[test]
    fn package_version_reads_the_installed_stanza() {
        assert_eq!(
            package_version(V22, "libreqos").as_deref(),
            Some("2.2.202608181454")
        );
        assert_eq!(package_version(V22, "missing"), None);
        assert_eq!(
            package_version(&v3("deinstall ok config-files"), "libreqos"),
            None
        );
    }

    #[test]
    fn v2_gives_v2_pair() {
        let e = env(V22, None);
        let d = resolve(&e.cfg).unwrap();
        assert_eq!(d.target, Target::V2Pair);
        assert!(d.reason.contains("2.2.202608181454"), "{}", d.reason);
    }

    #[test]
    fn v3_source_selects_the_target() {
        for (src, t) in [
            ("shaped_devices", Target::V3Pair),
            ("libreqos_network", Target::V3Network),
        ] {
            let e = env(
                &v3("install ok installed"),
                Some(&format!(
                    "lqos_directory = \"/opt/libreqos/src\"\n[topology]\nsource = \"{src}\"\ncompile_mode = \"full\"\n"
                )),
            );
            let d = resolve(&e.cfg).unwrap();
            assert_eq!(d.target, t, "{src}");
            assert!(d.reason.contains(src), "{}", d.reason);
        }
    }

    #[test]
    fn v3_integration_source_or_no_source_is_an_error() {
        let e = env(
            &v3("install ok installed"),
            Some("[topology]\nsource = \"uisp\"\n"),
        );
        let m = format!("{:#}", resolve(&e.cfg).unwrap_err());
        assert!(m.contains("uisp"), "{m}");
        let e = env(
            &v3("install ok installed"),
            Some("lqos_directory = \"x\"\n"),
        );
        let m = format!("{:#}", resolve(&e.cfg).unwrap_err());
        assert!(m.contains("[topology] source"), "{m}");
        let e = env(&v3("install ok installed"), None);
        assert!(resolve(&e.cfg).is_err());
    }

    #[test]
    fn no_package_is_an_error_and_a_fixed_target_stays() {
        let e = env(
            "Package: other\nStatus: install ok installed\nVersion: 1\n",
            None,
        );
        let m = format!("{:#}", resolve(&e.cfg).unwrap_err());
        assert!(m.contains("libreqos"), "{m}");
        let fixed = Libreqos {
            target: Target::V3Pair,
            ..e.cfg.clone()
        };
        let d = resolve(&fixed).unwrap();
        assert_eq!(d.target, Target::V3Pair);
        assert!(d.reason.contains("fixed"), "{}", d.reason);
    }
}
