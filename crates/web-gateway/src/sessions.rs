//! Opaque persistent sessions; account state is checked through the broker on every request.
use crate::{accounts::Account, broker::now};
use anyhow::{Result, ensure};
use rand::{RngCore, rngs::OsRng};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
#[derive(Clone)]
pub struct Sessions {
    path: PathBuf,
}
pub struct Session {
    pub id: i64,
    pub epoch: i64,
    pub username: String,
    pub csrf: String,
}
pub fn random() -> String {
    let mut b = [0; 32];
    OsRng.fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}
fn digest(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
impl Sessions {
    pub fn new(path: PathBuf) -> Result<Self> {
        use std::os::unix::fs::OpenOptionsExt;
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("missing session directory"))?;
        ensure!(
            std::fs::metadata(parent)?.permissions().mode() & 0o077 == 0,
            "session directory must have mode 0700"
        );
        if !path.exists() {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)?;
        }
        ensure!(
            std::fs::symlink_metadata(&path)?.is_file(),
            "invalid session database"
        );
        let this = Self { path };
        this.open()?.execute_batch("PRAGMA journal_mode=WAL;CREATE TABLE IF NOT EXISTS sessions(token TEXT PRIMARY KEY,account INTEGER NOT NULL,epoch INTEGER NOT NULL,username TEXT NOT NULL,csrf TEXT NOT NULL,created INTEGER NOT NULL,seen INTEGER NOT NULL);")?;
        Ok(this)
    }
    fn open(&self) -> Result<Connection> {
        let c = Connection::open(&self.path)?;
        c.busy_timeout(std::time::Duration::from_secs(2))?;
        Ok(c)
    }
    pub fn create(&self, a: &Account) -> Result<(String, String)> {
        let mut db = self.open()?;
        let c = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let time = now();
        c.execute(
            "DELETE FROM sessions WHERE created<?1 OR seen<?2",
            params![time.saturating_sub(28800), time.saturating_sub(1800)],
        )?;
        let count: i64 = c.query_row("SELECT count(*) FROM sessions", [], |r| r.get(0))?;
        ensure!(count < 1000, "session capacity reached");
        let token = random();
        let csrf = random();
        c.execute(
            "INSERT INTO sessions VALUES(?1,?2,?3,?4,?5,?6,?6)",
            params![digest(&token), a.id, a.epoch, a.username, csrf, time],
        )?;
        c.commit()?;
        Ok((token, csrf))
    }
    pub fn get(&self, token: &str) -> Result<Option<Session>> {
        if token.len() != 64 {
            return Ok(None);
        }
        let c = self.open()?;
        let time = now();
        let s=c.query_row("SELECT account,epoch,username,csrf FROM sessions WHERE token=?1 AND created>=?2 AND seen>=?3",params![digest(token),time.saturating_sub(28800),time.saturating_sub(1800)],|r|Ok(Session{id:r.get(0)?,epoch:r.get(1)?,username:r.get(2)?,csrf:r.get(3)?})).optional()?;
        if s.is_some() {
            c.execute(
                "UPDATE sessions SET seen=?1 WHERE token=?2 AND seen<?3",
                params![time, digest(token), time.saturating_sub(60)],
            )?;
        }
        Ok(s)
    }
    pub fn remove(&self, token: &str) -> Result<()> {
        self.open()?
            .execute("DELETE FROM sessions WHERE token=?1", [digest(token)])?;
        Ok(())
    }
}
