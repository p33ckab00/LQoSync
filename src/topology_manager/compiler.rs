//! Controlled compilation of one fresh pair. The child cannot apply a native world.
use super::{admission, publication::PreparedPair};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

const LIBRARY_LIMIT: usize = 64 * 1024 * 1024;
const CONFIG_LIMIT: usize = 1024 * 1024;
const CHILD: &str = r#"
import os, sys
sys.path.insert(0, os.environ['COMPILER_LIBRARY'])
import liblqos_python as compiler
if not compiler.write_compiled_topology_from_legacy_artifacts('shaped_devices', os.environ['COMPILER_MODE']):
    sys.exit(1)
"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerBinding {
    pub library: PathBuf,
    pub config: PathBuf,
    pub digest: String,
    pub effective_config_digest: String,
}
impl CompilerBinding {
    pub(crate) fn valid(&self) -> bool {
        self.library.is_absolute()
            && self.config.is_absolute()
            && self
                .library
                .file_name()
                .is_some_and(|n| n == "liblqos_python.so")
            && self.effective_config_digest.len() == 64
            && self
                .effective_config_digest
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
            && self.digest.len() == 64
            && self.digest.bytes().all(|b| b.is_ascii_hexdigit())
    }
    pub(crate) fn check_config(&self, cfg: &crate::config::Libreqos) -> anyhow::Result<()> {
        ensure!(
            self.library == absolute(&Path::new(&cfg.directory).join("liblqos_python.so"))?
                && self.config == absolute(Path::new(&cfg.v3_config))?,
            "compiler belongs to a different runtime configuration"
        );
        self.check_current()?;
        let text = read_regular(&self.config, CONFIG_LIMIT, false)?;
        let config = parse_config(&text)?;
        check_directories(&config, cfg)
    }
    pub(crate) fn check_current(&self) -> anyhow::Result<()> {
        ensure!(self.valid(), "invalid compiler binding");
        let library = read_regular(&self.library, LIBRARY_LIMIT, false)?;
        let config = read_regular(&self.config, CONFIG_LIMIT, false)?;
        ensure!(
            fingerprint(&library, &config) == self.digest,
            "compiler library or configuration changed"
        );
        Ok(())
    }
}
fn fingerprint(library: &[u8], config: &[u8]) -> String {
    crate::write_receipt::sha256(
        &serde_json::to_vec(&(
            crate::write_receipt::sha256(library),
            crate::write_receipt::sha256(config),
        ))
        .expect("digest tuple serializes"),
    )
}
struct Workspace(PathBuf);
impl Workspace {
    fn create(parent: &Path) -> anyhow::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
        super::store::check_private_directory(parent)?;
        for _ in 0..32 {
            let path = parent.join(format!(
                "compile-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Self(std::fs::canonicalize(path)?)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
        anyhow::bail!("cannot reserve a private compiler workspace")
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub struct ControlledCompiler {
    binding: CompilerBinding,
    native_config: toml::Table,
    cache: Workspace,
}
pub(crate) struct CompiledPair {
    admitted: admission::AdmittedPair,
    binding: CompilerBinding,
}
impl CompiledPair {
    pub(crate) fn generation(&self) -> u64 {
        self.admitted.generation()
    }
    pub(crate) fn expected_revision(&self) -> u64 {
        self.admitted.expected_revision()
    }
    pub(crate) fn check(&self) -> anyhow::Result<()> {
        self.admitted.check_fresh()?;
        self.binding.check_current()
    }
    pub(crate) fn binding(&self) -> &CompilerBinding {
        &self.binding
    }
    pub(crate) fn publish(
        self,
        store: &mut super::store::RegistryStore,
        tracker: &mut crate::write_receipt::Tracker,
        request: super::publication::PublishRequest<'_>,
    ) -> anyhow::Result<crate::write_receipt::WriteResult> {
        self.binding.check_current()?;
        self.admitted.publish(store, tracker, request)
    }
}
impl ControlledCompiler {
    pub fn open(cfg: &crate::config::Libreqos, scratch: &Path) -> anyhow::Result<Self> {
        let library = absolute(&Path::new(&cfg.directory).join("liblqos_python.so"))?;
        let config = absolute(Path::new(&cfg.v3_config))?;
        let library_bytes = read_regular(&library, LIBRARY_LIMIT, false)?;
        let config_bytes = read_regular(&config, CONFIG_LIMIT, false)?;
        let native_config: toml::Table = toml::from_str(
            std::str::from_utf8(&config_bytes)
                .map_err(|_| anyhow::anyhow!("native compiler configuration is not UTF-8"))?,
        )
        .map_err(|_| anyhow::anyhow!("cannot parse native compiler configuration"))?;
        ensure!(
            native_config
                .get("topology")
                .and_then(|t| t.get("source"))
                .and_then(toml::Value::as_str)
                == Some("shaped_devices"),
            "manager compiler requires shaped_devices source"
        );
        check_directories(&native_config, cfg)?;
        let cache = Workspace::create(scratch)?;
        write_exact(&cache.0.join("liblqos_python.so"), &library_bytes)?;
        Ok(Self {
            binding: CompilerBinding {
                library,
                config,
                digest: fingerprint(&library_bytes, &config_bytes),
                effective_config_digest: crate::write_receipt::sha256(
                    toml::to_string(&native_config)?.as_bytes(),
                ),
            },
            native_config,
            cache,
        })
    }
    pub(crate) fn check_config(&self, cfg: &crate::config::Libreqos) -> anyhow::Result<()> {
        self.binding.check_config(cfg)
    }
    pub(crate) async fn compile(&mut self, pair: PreparedPair) -> anyhow::Result<CompiledPair> {
        self.compile_code(pair, CHILD).await
    }
    async fn compile_code(
        &mut self,
        pair: PreparedPair,
        code: &str,
    ) -> anyhow::Result<CompiledPair> {
        pair.check_fresh()?;
        self.binding.check_current()?;
        let frozen = read_regular(&self.cache.0.join("liblqos_python.so"), LIBRARY_LIMIT, true)?;
        let source_config = read_regular(&self.binding.config, CONFIG_LIMIT, false)?;
        ensure!(
            fingerprint(&frozen, &source_config) == self.binding.digest,
            "compiler snapshot changed"
        );
        drop(frozen);
        drop(source_config);
        let workspace = Workspace::create(&self.cache.0)?;
        let src = workspace.0.join("src");
        let state = workspace.0.join("state");
        std::fs::DirBuilder::new().mode(0o700).create(&src)?;
        std::fs::DirBuilder::new().mode(0o700).create(&state)?;
        write_exact(&src.join("ShapedDevices.csv"), pair.csv().as_bytes())?;
        write_exact(&src.join("network.json"), pair.network_text().as_bytes())?;
        let mut native_config = self.native_config.clone();
        native_config.insert(
            "lqos_directory".into(),
            src.to_string_lossy().into_owned().into(),
        );
        native_config.insert(
            "state_directory".into(),
            state.to_string_lossy().into_owned().into(),
        );
        let mode = native_config
            .get("topology")
            .and_then(|t| t.get("compile_mode"))
            .and_then(toml::Value::as_str)
            .context("native compile mode is missing")?
            .to_owned();
        let config = workspace.0.join("lqos.conf");
        crate::files::write_secret(&config, toml::to_string(&native_config)?.as_bytes())?;
        let mut command = tokio::process::Command::new("/usr/bin/python3");
        command
            .args(["-I", "-S", "-B", "-c", code])
            .current_dir(&workspace.0)
            .env_clear()
            .env("LQOS_CONFIG", &config)
            .env("COMPILER_LIBRARY", &self.cache.0)
            .env("COMPILER_MODE", mode)
            .env("COMPILER_STATE", &state)
            .env("TMPDIR", &state)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        super::compiler_sandbox::command(&mut command, &workspace.0)?;
        let mut child = command
            .spawn()
            .context("cannot start the isolated compiler")?;
        let deadline = pair
            .deadline()
            .min(tokio::time::Instant::now() + Duration::from_secs(60));
        let result = tokio::time::timeout_at(deadline, child.wait()).await;
        let status = match result {
            Ok(status) => status?,
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                anyhow::bail!("compiler deadline expired")
            }
        };
        ensure!(status.success(), "isolated compiler failed");
        pair.check_fresh()?;
        self.binding.check_current()?;
        let canonical = read_regular(
            &state.join("topology/topology_canonical_state.json"),
            admission::MAX_ARTIFACT_BYTES,
            true,
        )?;
        let shaping = read_regular(
            &state.join("shaping/topology_compiled_shaping.json"),
            admission::MAX_ARTIFACT_BYTES.saturating_sub(canonical.len()),
            true,
        )?;
        ensure!(
            read_regular(&src.join("network.json"), admission::MAX_PAIR_BYTES, true)?
                == pair.network_text().as_bytes()
                && read_regular(
                    &src.join("ShapedDevices.csv"),
                    admission::MAX_PAIR_BYTES,
                    true
                )? == pair.csv().as_bytes(),
            "compiler changed its requested input pair"
        );
        let mut effective = parse_config(&read_regular(&config, CONFIG_LIMIT, true)?)?;
        ensure!(
            effective
                .get("lqos_directory")
                .and_then(toml::Value::as_str)
                == src.to_str()
                && effective
                    .get("state_directory")
                    .and_then(toml::Value::as_str)
                    == state.to_str()
                && effective.get("topology") == self.native_config.get("topology"),
            "compiler changed its source, mode, or workspace configuration"
        );
        effective.insert(
            "lqos_directory".into(),
            self.native_config["lqos_directory"].clone(),
        );
        effective.insert(
            "state_directory".into(),
            self.native_config["state_directory"].clone(),
        );
        let mut binding = self.binding.clone();
        binding.effective_config_digest =
            crate::write_receipt::sha256(toml::to_string(&effective)?.as_bytes());
        let mut admitted = admission::check_compiled(
            pair,
            &serde_json::from_slice(&canonical)?,
            &serde_json::from_slice(&shaping)?,
        )?;
        admitted.bind_compiler(binding.clone());
        Ok(CompiledPair { admitted, binding })
    }
}
fn absolute(path: &Path) -> anyhow::Result<PathBuf> {
    Ok(if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    })
}
fn read_regular(path: &Path, limit: usize, no_follow: bool) -> anyhow::Result<Vec<u8>> {
    let file = std::fs::File::options()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | if no_follow { libc::O_NOFOLLOW } else { 0 })
        .open(path)?;
    ensure!(
        file.metadata()?.is_file() && file.metadata()?.len() <= limit as u64,
        "compiler input or artifact exceeds its regular-file bound"
    );
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= limit,
        "compiler input or artifact exceeds its byte budget"
    );
    Ok(bytes)
}

fn write_exact(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let mut file = std::fs::File::options()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn parse_config(bytes: &[u8]) -> anyhow::Result<toml::Table> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| anyhow::anyhow!("native compiler configuration is not UTF-8"))?;
    toml::from_str(text).map_err(|_| anyhow::anyhow!("cannot parse native compiler configuration"))
}
fn check_directories(config: &toml::Table, cfg: &crate::config::Libreqos) -> anyhow::Result<()> {
    let source = config
        .get("lqos_directory")
        .and_then(toml::Value::as_str)
        .context("native source directory is missing")?;
    let state = config
        .get("state_directory")
        .and_then(toml::Value::as_str)
        .context("native state directory is missing")?;
    ensure!(
        std::fs::canonicalize(source)? == std::fs::canonicalize(&cfg.directory)?,
        "native source directory differs from the writer directory"
    );
    ensure!(
        std::fs::canonicalize(state)? == std::fs::canonicalize(&cfg.state_directory)?,
        "native state directory differs from the verifier directory"
    );
    Ok(())
}

#[cfg(test)]
#[path = "compiler_tests.rs"]
pub(crate) mod tests;
