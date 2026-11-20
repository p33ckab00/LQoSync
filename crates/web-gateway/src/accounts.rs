//! Root-owned account storage. No HTTP route can mutate this store.
use anyhow::{Context, Result, bail, ensure};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand::{RngCore, rngs::OsRng};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
#[derive(Clone)]
pub struct Accounts {
    path: PathBuf,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: i64,
    pub username: String,
    pub epoch: i64,
    pub disabled: bool,
}
fn name_ok(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-.".contains(&c))
        && !name.starts_with('.')
}
fn hash(password: &str) -> Result<String> {
    ensure!(
        (12..=256).contains(&password.len()),
        "password must contain 12 to 256 bytes"
    );
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let salt =
        SaltString::encode_b64(&salt).map_err(|_| anyhow::anyhow!("salt encoding failed"))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| anyhow::anyhow!("password hashing failed"))
}
impl Accounts {
    pub fn new(path: PathBuf) -> Result<Self> {
        let this = Self { path };
        let db = this.open()?;
        db.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS accounts(id INTEGER PRIMARY KEY AUTOINCREMENT, username TEXT UNIQUE NOT NULL, password TEXT NOT NULL, epoch INTEGER NOT NULL DEFAULT 1, disabled INTEGER NOT NULL DEFAULT 0); CREATE TABLE IF NOT EXISTS audit(id INTEGER PRIMARY KEY, at INTEGER NOT NULL DEFAULT(unixepoch()), uid INTEGER NOT NULL, account TEXT NOT NULL, action TEXT NOT NULL);")?;
        Ok(this)
    }
    fn open(&self) -> Result<Connection> {
        // A private directory also protects SQLite sidecars and backup files.
        let parent = self.path.parent().context("account directory missing")?;
        ensure!(
            parent.is_dir(),
            "create the private account directory first"
        );
        ensure!(
            std::fs::metadata(parent)?.permissions().mode() & 0o077 == 0,
            "account directory must have mode 0700"
        );
        if !self.path.exists() {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&self.path)?;
        }
        ensure!(
            !std::fs::symlink_metadata(&self.path)?
                .file_type()
                .is_symlink(),
            "account database must be a regular file"
        );
        let db = Connection::open(&self.path)?;
        db.busy_timeout(std::time::Duration::from_secs(3))?;
        Ok(db)
    }
    pub fn create(&self, username: &str, password: &str) -> Result<()> {
        ensure!(name_ok(username), "invalid username");
        let password = hash(password)?;
        let mut db = self.open()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO accounts(username,password) VALUES(?1,?2)",
            params![username, password],
        )?;
        Self::audit(&tx, username, "create")?;
        tx.commit()?;
        Ok(())
    }
    fn audit(db: &Connection, name: &str, action: &str) -> Result<()> {
        db.execute(
            "INSERT INTO audit(uid,account,action) VALUES(?1,?2,?3)",
            params![unsafe { libc::geteuid() }, name, action],
        )?;
        Ok(())
    }
    pub fn list(&self) -> Result<Vec<Account>> {
        let db = self.open()?;
        let mut q =
            db.prepare("SELECT id,username,epoch,disabled FROM accounts ORDER BY username")?;
        Ok(q.query_map([], |r| {
            Ok(Account {
                id: r.get(0)?,
                username: r.get(1)?,
                epoch: r.get(2)?,
                disabled: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn verify(&self, username: &str, password: &str) -> Result<Account> {
        ensure!(
            username.len() <= 64 && password.len() <= 256,
            "invalid credentials"
        );
        let db = self.open()?;
        let found: Option<(i64, String, i64, bool)> = db
            .query_row(
                "SELECT id,password,epoch,disabled FROM accounts WHERE username=?1",
                [username],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        // Run the same expensive hash check for an absent account to reduce enumeration signals.
        let dummy = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHR2YWx1ZQ$LTB7rX6yjiAOXNsdgkuufzULZJm33JueE7dJAT1WkzU";
        let stored = found.as_ref().map(|x| x.1.as_str()).unwrap_or(dummy);
        let valid = PasswordHash::new(stored).ok().is_some_and(|h| {
            Argon2::default()
                .verify_password(password.as_bytes(), &h)
                .is_ok()
        });
        match found {
            Some((id, _, epoch, false)) if valid => Ok(Account {
                id,
                username: username.into(),
                epoch,
                disabled: false,
            }),
            _ => bail!("invalid credentials"),
        }
    }
    pub fn active(&self, id: i64, epoch: i64) -> Result<bool> {
        Ok(self.open()?.query_row(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1 AND epoch=?2 AND disabled=0)",
            params![id, epoch],
            |r| r.get(0),
        )?)
    }
    fn edit(
        &self,
        username: &str,
        action: &str,
        change: impl FnOnce(&Connection) -> Result<usize>,
    ) -> Result<()> {
        let mut db = self.open()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure!(change(&tx)? == 1, "account not found");
        let count: i64 =
            tx.query_row("SELECT count(*) FROM accounts WHERE disabled=0", [], |r| {
                r.get(0)
            })?;
        ensure!(
            count > 0,
            "cannot remove or disable the final active superadmin"
        );
        Self::audit(&tx, username, action)?;
        tx.commit()?;
        Ok(())
    }
    pub fn password(&self, name: &str, password: &str) -> Result<()> {
        let password = hash(password)?;
        self.edit(name, "password", |db| {
            Ok(db.execute(
                "UPDATE accounts SET password=?1,epoch=epoch+1 WHERE username=?2",
                params![password, name],
            )?)
        })
    }
    pub fn rename(&self, name: &str, new: &str) -> Result<()> {
        ensure!(name_ok(new), "invalid username");
        self.edit(name, "rename", |db| {
            Ok(db.execute(
                "UPDATE accounts SET username=?1,epoch=epoch+1 WHERE username=?2",
                params![new, name],
            )?)
        })
    }
    pub fn disable(&self, name: &str, disabled: bool) -> Result<()> {
        self.edit(name, if disabled { "disable" } else { "enable" }, |db| {
            Ok(db.execute(
                "UPDATE accounts SET disabled=?1,epoch=epoch+1 WHERE username=?2",
                params![disabled, name],
            )?)
        })
    }
    pub fn delete(&self, name: &str) -> Result<()> {
        self.edit(name, "delete", |db| {
            Ok(db.execute("DELETE FROM accounts WHERE username=?1", [name])?)
        })
    }
}
