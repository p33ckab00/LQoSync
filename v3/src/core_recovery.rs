//! Bounded, read-only inspection. No executor, credentials, router client, or marker mutation.
use crate::core_writer::{self, JournalReview};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    ffi::CString,
    fs::{self, File, OpenOptions},
    io::{self, Read},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::OpenOptionsExt,
    },
    path::Path,
};
const BUDGET: usize = 64 * 1024 * 1024;
const MAX_JOURNALS: usize = 16;
#[derive(Default)]
pub struct Report {
    pub marker: String,
    pub active_revision: Option<String>,
    pub journals: Vec<JournalReview>,
    pub issues: Vec<String>,
    pub limited: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    journal: String,
    revision: String,
}
fn child(parent: &File, name: &str, directory: bool) -> Result<Option<File>> {
    let name = CString::new(name)?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: parent owns a live directory descriptor and name is a terminated string.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::NotFound {
            return Ok(None);
        }
        return Err(error.into());
    }
    // SAFETY: openat returned a new descriptor; this File becomes its sole owner.
    Ok(Some(unsafe { File::from_raw_fd(fd) }))
}
fn text(parent: &File, name: &str, budget: &mut usize) -> Result<Option<String>> {
    let Some(file) = child(parent, name, false)? else {
        return Ok(None);
    };
    let limit = *budget;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= limit as u64,
        "Inspection file is not regular or exceeds limit"
    );
    // Reserve the remaining budget before IO. Failed reads retain the full charge.
    *budget = 0;
    let mut raw = String::new();
    file.take(limit as u64 + 1).read_to_string(&mut raw)?;
    ensure!(raw.len() <= limit, "Inspection file grew past limit");
    *budget = limit - raw.len();
    Ok(Some(raw))
}
fn revision(name: &str) -> bool {
    uuid::Uuid::parse_str(name).is_ok_and(|v| v.to_string() == name)
}
pub fn inspect(root: &Path) -> Result<Report> {
    let root = root.canonicalize()?;
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(&root)?;
    let mut report = Report {
        marker: "No recovery marker".into(),
        ..Default::default()
    };
    let Some(state) = child(&directory, "state", true)? else {
        return Ok(report);
    };
    let before = text(&state, "core-migration-active.json", &mut 65536)?;
    if let Some(raw) = &before {
        report.marker = "Recovery marker present; execution blocked".into();
        match serde_json::from_str::<Marker>(raw) {
            Ok(marker) if revision(&marker.revision) && marker.journal==root.join("state/core-writer").join(format!("{}.json",marker.revision)).to_string_lossy()=>{
                report.active_revision=Some(marker.revision);
            },
            Ok(_)=>report.issues.push("Marker refers to another recovery engine or an unsupported path. Do not follow its path manually.".into()),
            Err(_)=>report.issues.push("Recovery marker is invalid; its state is unknown.".into()),
        }
    }
    if let Some(journals) = child(&state, "core-writer", true)? {
        let mut names = Vec::new();
        if let Some(active) = &report.active_revision {
            names.push(format!("{active}.json"));
        }
        // The descriptor pins the directory even if an ancestor is renamed during inspection.
        let mut entries = fs::read_dir(format!("/proc/self/fd/{}", journals.as_raw_fd()))?;
        let mut found = Vec::new();
        for _ in 0..128 {
            let Some(entry) = entries.next() else { break };
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if let Some(stem) = name.strip_suffix(".json") {
                if revision(stem) {
                    found.push(name.to_string());
                } else {
                    report.limited = true;
                }
            }
        }
        if entries.next().is_some() {
            report.limited = true;
        }
        found.sort();
        names.extend(found);
        let mut seen = std::collections::BTreeSet::new();
        names.retain(|n| seen.insert(n.clone()));
        if names.len() > MAX_JOURNALS {
            report.limited = true;
            names.truncate(MAX_JOURNALS);
        }
        let mut remaining = BUDGET;
        for name in names {
            let Some(stem) = name.strip_suffix(".json") else {
                continue;
            };
            if remaining == 0 {
                report.limited = true;
                break;
            }
            match text(&journals, &name, &mut remaining) {
                Ok(Some(raw)) => match core_writer::inspect_journal(&raw, stem) {
                    Ok(summary) => report.journals.push(summary),
                    Err(_) => report.issues.push(format!(
                        "Saved journal {stem} failed validation. No action is authorized."
                    )),
                },
                _ => report.issues.push(format!(
                    "Saved journal {stem} is unavailable, unsafe, or exceeds the inspection budget."
                )),
            }
        }
    }
    if report
        .active_revision
        .as_ref()
        .is_some_and(|active| !report.journals.iter().any(|j| &j.revision == active))
    {
        report.issues.push(
            "The active writer journal is missing or unavailable; recovery remains blocked.".into(),
        );
    }
    if before.is_none()
        && report
            .journals
            .iter()
            .any(|j| j.pending.is_some() || !j.saved_verified)
    {
        report.issues.push(
            "Unresolved saved journal without a recovery marker. Do not execute or clear records."
                .into(),
        );
    }
    ensure!(
        text(&state, "core-migration-active.json", &mut 65536)? == before,
        "Recovery marker changed during inspection; reload"
    );
    Ok(report)
}
static INSPECTIONS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
pub async fn inspect_async(root: std::path::PathBuf) -> Result<Report> {
    let permit = INSPECTIONS
        .try_acquire()
        .context("Another recovery inspection is active")?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        inspect(&root)
    })
    .await
    .context("Recovery inspection worker failed")?
}
