//! Durable results of web mutations, found again by their request key.
//!
//! The registry rename and a receipt file cannot change in one step, so each mutation moves
//! through recoverable states, each written with an atomic rename before the next step:
//! `intent` (before the commit), `prepared` (after validation, just before the registry write,
//! with the new revision and a fingerprint of the new document), then `saved` or `failed`.
//! Recovery never repeats a mutation. It decides from the registry alone:
//! still at the base revision gives `not_committed`; the prepared revision with the same
//! fingerprint gives `saved`; anything else gives `outcome_unknown`.
use crate::topology_manager::model::RegistryDocument;
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use web_contract::topology::{WebCommand, WebReceipt};

/// Resolved receipts older than this are removed when the store opens.
pub const KEEP_SECS: u64 = 7 * 24 * 3600;

/// The key exists for another actor or another command. Use a new key for a new request.
#[derive(Debug)]
pub struct KeyReused;
impl std::fmt::Display for KeyReused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the request key belongs to another request")
    }
}
impl std::error::Error for KeyReused {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum State {
    Intent,
    Prepared {
        revision: u64,
        fingerprint: String,
    },
    Saved {
        revision: u64,
        subject_id: Option<String>,
    },
    NotCommitted,
    Failed {
        error: String,
    },
    OutcomeUnknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub key: String,
    pub actor: String,
    pub command: WebCommand,
    pub base_revision: u64,
    /// Unix seconds when the mutation started.
    pub at: u64,
    pub state: State,
}

impl Record {
    pub fn view(&self) -> WebReceipt {
        let (state, revision, subject_id, error) = match &self.state {
            State::Saved {
                revision,
                subject_id,
            } => (
                "saved",
                Some(revision.to_string()),
                subject_id.clone(),
                None,
            ),
            State::NotCommitted => ("not_committed", None, None, None),
            State::Failed { error } => ("failed", None, None, Some(error.clone())),
            // An unresolved record is unknown to the caller until recovery decides it.
            State::Intent | State::Prepared { .. } | State::OutcomeUnknown => {
                ("outcome_unknown", None, None, None)
            }
        };
        WebReceipt {
            key: self.key.clone(),
            state: state.into(),
            base_revision: self.base_revision.to_string(),
            revision,
            subject_id,
            error,
        }
    }
    fn unresolved(&self) -> bool {
        matches!(self.state, State::Intent | State::Prepared { .. })
    }
}

#[derive(Debug)]
pub enum Begin {
    /// Run the mutation; the intent is durable.
    New(Record),
    /// The key is known: answer with this record and do not run the mutation again.
    Existing(Record),
}

/// FNV-1a 64 of the document JSON. The registry is service-written, so this detects a different
/// document; it is not a defense against a chosen collision.
pub fn fingerprint(doc: &RegistryDocument) -> String {
    let bytes = serde_json::to_vec(doc).unwrap_or_default();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// A browser idempotency key: 16 to 64 letters, digits, or `-`. It is also the file name.
pub fn valid_key(key: &str) -> bool {
    (16..=64).contains(&key.len()) && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

pub struct Receipts {
    dir: PathBuf,
}

impl Receipts {
    /// Open `<owner>/topology-manager/web-receipts` (mode 0700). Refuse a directory that is not
    /// private: then no receipt can meet the recovery contract.
    pub fn open(owner: &Path) -> anyhow::Result<Self> {
        let dir = owner.join("topology-manager").join("web-receipts");
        match std::fs::DirBuilder::new().mode(0o700).create(&dir) {
            Ok(()) => std::fs::File::open(dir.parent().unwrap_or(owner))?.sync_all()?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e).context("cannot create the web receipt directory"),
        }
        super::store::check_private_directory(&dir)?;
        Ok(Self { dir })
    }

    fn path(&self, key: &str) -> Option<PathBuf> {
        valid_key(key).then(|| self.dir.join(format!("{key}.json")))
    }

    fn put(&self, record: &Record) -> anyhow::Result<()> {
        let path = self.path(&record.key).context("invalid request key")?;
        crate::files::write_secret(&path, &serde_json::to_vec(record)?)
    }

    pub fn get(&self, key: &str) -> anyhow::Result<Option<Record>> {
        let Some(path) = self.path(key) else {
            return Ok(None);
        };
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(
                serde_json::from_slice(&bytes).context("a web receipt is not valid")?,
            )),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).context("cannot read a web receipt"),
        }
    }

    /// Record the intent before the mutation, or return the record of a known key.
    pub fn begin(
        &self,
        key: &str,
        actor: &str,
        command: &WebCommand,
        base_revision: u64,
        now: u64,
    ) -> anyhow::Result<Begin> {
        ensure!(valid_key(key), "invalid request key");
        ensure!(!actor.is_empty() && actor.len() <= 64, "invalid actor");
        if let Some(existing) = self.get(key)? {
            if existing.actor != actor || existing.command != *command {
                return Err(anyhow::Error::new(KeyReused));
            }
            return Ok(Begin::Existing(existing));
        }
        let record = Record {
            key: key.into(),
            actor: actor.into(),
            command: command.clone(),
            base_revision,
            at: now,
            state: State::Intent,
        };
        self.put(&record)?;
        Ok(Begin::New(record))
    }

    /// The registry hook: make the new revision and fingerprint durable before the write.
    pub fn prepare(&self, record: &Record, doc: &RegistryDocument) -> anyhow::Result<()> {
        self.put(&Record {
            state: State::Prepared {
                revision: doc.revision,
                fingerprint: fingerprint(doc),
            },
            ..record.clone()
        })
    }

    pub fn finish_saved(
        &self,
        record: &Record,
        revision: u64,
        subject_id: Option<String>,
    ) -> anyhow::Result<Record> {
        let done = Record {
            state: State::Saved {
                revision,
                subject_id,
            },
            ..record.clone()
        };
        self.put(&done)?;
        Ok(done)
    }

    pub fn finish_failed(&self, record: &Record, error: &str) -> anyhow::Result<Record> {
        let done = Record {
            state: State::Failed {
                error: error.into(),
            },
            ..record.clone()
        };
        self.put(&done)?;
        Ok(done)
    }

    /// Decide an unresolved record from the registry alone. Never runs the mutation again.
    fn resolve(&self, record: Record, doc: &RegistryDocument) -> anyhow::Result<Record> {
        let state = match &record.state {
            _ if doc.revision == record.base_revision => State::NotCommitted,
            State::Prepared {
                revision,
                fingerprint: expected,
            } if doc.revision == *revision && fingerprint(doc) == *expected => State::Saved {
                revision: *revision,
                subject_id: None,
            },
            _ => State::OutcomeUnknown,
        };
        let done = Record { state, ..record };
        self.put(&done)?;
        Ok(done)
    }

    /// The record of `key` for `actor`, resolved first when a crash or a write error left it
    /// open. Another actor gets `None`.
    pub fn lookup(
        &self,
        key: &str,
        actor: &str,
        doc: &RegistryDocument,
    ) -> anyhow::Result<Option<Record>> {
        match self.get(key)? {
            Some(r) if r.actor != actor => Ok(None),
            Some(r) if r.unresolved() => self.resolve(r, doc).map(Some),
            other => Ok(other),
        }
    }

    /// Resolve every open record. Call it before any other registry commit after a restart.
    pub fn recover(&self, doc: &RegistryDocument) -> anyhow::Result<usize> {
        let mut resolved = 0;
        for record in self.records()? {
            if record.unresolved() {
                self.resolve(record, doc)?;
                resolved += 1;
            }
        }
        Ok(resolved)
    }

    /// Remove resolved records older than `KEEP_SECS`. Open records always stay.
    pub fn prune(&self, now: u64) -> anyhow::Result<usize> {
        let mut removed = 0;
        for record in self.records()? {
            if !record.unresolved()
                && now.saturating_sub(record.at) > KEEP_SECS
                && let Some(path) = self.path(&record.key)
            {
                std::fs::remove_file(path)?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    fn records(&self) -> anyhow::Result<Vec<Record>> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&self.dir)? {
            let name = entry?.file_name();
            let Some(key) = name.to_str().and_then(|n| n.strip_suffix(".json")) else {
                continue;
            };
            if let Some(record) = self.get(key)? {
                out.push(record);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
#[path = "web_receipts_tests.rs"]
mod tests;
