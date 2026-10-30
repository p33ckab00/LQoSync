//! Durable cancellation across cooperating processes. This module never contacts a router.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::Path,
};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct State {
    schema: u32,
    epoch: String,
    revision: String,
    enabled: bool,
    inhibited: bool,
}
const NAME: &str = "state/core-control-state.json";
pub(crate) fn read(root: &Path) -> Result<Option<State>> {
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(root.join(NAME))
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    ensure!(
        file.metadata()?.is_file() && file.metadata()?.len() <= 4096,
        "Unsafe control state"
    );
    let mut raw = String::new();
    file.take(4097).read_to_string(&mut raw)?;
    ensure!(raw.len() <= 4096, "Control state exceeds limit");
    let state: State = serde_json::from_str(&raw)?;
    ensure!(
        state.schema == 1
            && !(state.enabled && state.inhibited)
            && uuid::Uuid::parse_str(&state.epoch).is_ok_and(|v| v.to_string() == state.epoch)
            && !state.revision.is_empty()
            && state.revision.len() <= 128,
        "Invalid control state"
    );
    Ok(Some(state))
}
struct ControlLock {
    _file: File,
    fresh: bool,
}
fn lock(root: &Path) -> Result<ControlLock> {
    crate::store::durable_directory(&root.join("state"))?;
    let open = |new| {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(new)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(root.join("state/core-control-state.lock"))
    };
    let (file, fresh) = match open(true) {
        Ok(file) => (file, true),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (open(false)?, false),
        Err(e) => return Err(e.into()),
    };
    ensure!(file.metadata()?.is_file(), "Unsafe control lock");
    // SAFETY: file owns a live descriptor. The lock covers only local settings IO.
    ensure!(
        unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
        "Another process is saving control settings; retry"
    );
    if fresh {
        file.sync_all()?;
        File::open(root.join("state"))?.sync_all()?;
    }
    Ok(ControlLock { _file: file, fresh })
}
pub(crate) fn publish(root: &Path, revision: &str, enabled: bool, inhibited: bool) -> Result<()> {
    let state = State {
        schema: 1,
        epoch: uuid::Uuid::new_v4().to_string(),
        revision: revision.into(),
        enabled,
        inhibited,
    };
    crate::store::atomic_write(&root.join(NAME), &serde_json::to_string(&state)?)
}
pub(crate) fn initialize(root: &Path, settings: &crate::model::Settings) -> Result<()> {
    let held = lock(root)?;
    if read(root)?.is_none() {
        publish(
            root,
            &crate::core_console::revision(settings)?,
            crate::core_console::enabled(settings) && held.fresh,
            !held.fresh,
        )?;
    }
    Ok(())
}
pub(crate) fn save(
    root: &Path,
    settings: &crate::model::Settings,
    resume_from: Option<&crate::model::Settings>,
    write: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let held = lock(root)?;
    let previous = read(root)?;
    let lost_or_inhibited = previous.as_ref().map_or(!held.fresh, |s| s.inhibited);
    let paused_mode = settings
        .core_control
        .as_ref()
        .is_some_and(|c| c.mode == crate::core_console::Mode::Paused);
    let inhibited = paused_mode || (lost_or_inhibited && resume_from.is_none());
    if let Some(expected) = resume_from {
        let actual: crate::model::Settings =
            toml::from_str(&std::fs::read_to_string(root.join("settings.toml"))?)?;
        ensure!(
            &actual == expected,
            "Persisted settings changed; reload before resuming previews"
        );
    }
    let revision = crate::core_console::revision(settings)?;
    // Revoke before settings IO. A failed save or process exit leaves a durable pause.
    publish(root, &revision, false, true)?;
    write()?;
    publish(
        root,
        &revision,
        crate::core_console::enabled(settings) && !inhibited,
        inhibited,
    )
}
pub(crate) fn pause(root: &Path, revision: &str) -> Result<()> {
    let _lock = lock(root)?;
    publish(root, revision, false, true)
}
pub(crate) fn capture(root: &Path, revision: &str) -> Result<Option<State>> {
    let state = read(root)?;
    if state.is_none() {
        ensure!(
            std::fs::symlink_metadata(root.join("state/core-control-state.lock"))
                .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
            "Initialized shared control state is missing"
        );
    }
    if let Some(state) = &state {
        ensure!(
            state.enabled && state.revision == revision,
            "Shared control is paused or settings changed"
        );
    }
    Ok(state)
}
pub(crate) fn matches(root: &Path, expected: &Option<State>, revision: &str) -> bool {
    capture(root, revision).is_ok_and(|current| &current == expected)
}
pub(crate) fn require_initialized(state: &Option<State>) -> Result<()> {
    state
        .as_ref()
        .context("Shared writer controls are not initialized")?;
    Ok(())
}

pub(crate) fn status(root: &Path) -> Result<&'static str> {
    Ok(match read(root)? {
        Some(s) if s.enabled => "Shared control enabled",
        Some(_) => "Shared control paused",
        None => "Shared control not initialized",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn settings() -> crate::model::Settings {
        crate::model::Settings {
            core_control: Some(Default::default()),
            wan_health: Some(crate::wan_health::Config {
                monitors: vec![crate::wan_health::Monitor {
                    wan: "wan".into(),
                    probes: [
                        crate::wan_health::Probe {
                            id: "*1".into(),
                            target: "192.0.2.1".parse().unwrap(),
                        },
                        crate::wan_health::Probe {
                            id: "*2".into(),
                            target: "192.0.2.2".parse().unwrap(),
                        },
                    ],
                }],
                ..Default::default()
            }),
            ..Default::default()
        }
    }
    #[test]
    fn failed_settings_io_remains_paused_after_restart_and_ordinary_save() {
        let d = tempfile::tempdir().unwrap();
        let s = settings();
        initialize(d.path(), &s).unwrap();
        assert!(read(d.path()).unwrap().unwrap().enabled);
        assert!(
            save(d.path(), &s, None, || anyhow::bail!(
                "synthetic interruption"
            ))
            .is_err()
        );
        initialize(d.path(), &s).unwrap();
        save(d.path(), &s, None, || Ok(())).unwrap();
        assert!(!read(d.path()).unwrap().unwrap().enabled);
        assert!(read(d.path()).unwrap().unwrap().inhibited);
    }
    #[test]
    fn ordinary_save_cannot_clear_pause_and_stale_resume_rejects() {
        let d = tempfile::tempdir().unwrap();
        let s = settings();
        std::fs::write(d.path().join("settings.toml"), toml::to_string(&s).unwrap()).unwrap();
        let revision = crate::core_console::revision(&s).unwrap();
        initialize(d.path(), &s).unwrap();
        pause(d.path(), &revision).unwrap();
        save(d.path(), &s, None, || Ok(())).unwrap();
        assert!(!read(d.path()).unwrap().unwrap().enabled);
        let mut stale = s.clone();
        stale.interval_seconds += 1;
        assert!(save(d.path(), &s, Some(&stale), || panic!("stale save called")).is_err());
        save(d.path(), &s, Some(&s), || Ok(())).unwrap();
        assert!(read(d.path()).unwrap().unwrap().enabled);
    }
    #[test]
    fn competing_save_never_runs_settings_io() {
        let d = tempfile::tempdir().unwrap();
        let _held = lock(d.path()).unwrap();
        assert!(
            save(d.path(), &settings(), None, || panic!(
                "competing save called"
            ))
            .is_err()
        );
        assert!(read(d.path()).unwrap().is_none());
    }
    #[test]
    fn missing_or_corrupt_control_permanently_revokes_an_observed_grant() {
        for corrupt in [false, true] {
            let d = tempfile::tempdir().unwrap();
            let s = settings();
            let revision = crate::core_console::revision(&s).unwrap();
            initialize(d.path(), &s).unwrap();
            let c = crate::core_coordination::Coordinator::for_root(d.path()).unwrap();
            c.configure(&revision, true);
            let lease = c
                .begin_writer(d.path(), &revision, &uuid::Uuid::new_v4().to_string())
                .unwrap();
            let guard = lease.guard();
            let bytes = std::fs::read(d.path().join(NAME)).unwrap();
            assert!(guard.active());
            if corrupt {
                std::fs::write(d.path().join(NAME), "invalid").unwrap();
            } else {
                std::fs::remove_file(d.path().join(NAME)).unwrap();
            }
            assert!(!guard.active());
            std::fs::write(d.path().join(NAME), bytes).unwrap();
            assert!(!guard.active());
        }
    }
    #[test]
    fn paused_mode_and_lost_state_cannot_be_cleared_by_ordinary_saves() {
        for change in ["mode", "missing_restart", "missing_save"] {
            let d = tempfile::tempdir().unwrap();
            let enabled = settings();
            initialize(d.path(), &enabled).unwrap();
            if change == "mode" {
                let mut paused = enabled.clone();
                paused.core_control.as_mut().unwrap().mode = crate::core_console::Mode::Paused;
                save(d.path(), &paused, None, || Ok(())).unwrap();
            } else {
                pause(d.path(), &crate::core_console::revision(&enabled).unwrap()).unwrap();
                std::fs::remove_file(d.path().join(NAME)).unwrap();
                if change == "missing_restart" {
                    initialize(d.path(), &enabled).unwrap();
                }
            }
            save(d.path(), &enabled, None, || Ok(())).unwrap();
            let state = read(d.path()).unwrap().unwrap();
            assert!(!state.enabled && state.inhibited, "{change}");
        }
    }
}
