//! Scoped production I/O without a live enable path. The integration must supply a fresh verifier.
use crate::{
    core_migration::{Member, RequestNotSent},
    core_writer::{Evidence, Operation, Transport},
    model::{self, Settings},
    routeros::{self, Row},
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    future::Future,
    path::{Path, PathBuf},
};
/// Must freshly validate settings, sources, session identities, WAN health, and explicit writer enable authorization.
/// Return complete actual address-list rows for every configured binding core, never simulated desired rows.
pub trait Verifier {
    fn verify(&mut self) -> impl Future<Output = Result<Evidence>>;
}
pub struct RouterTransport<V> {
    root: PathBuf,
    settings: Settings,
    verifier: V,
    clients: BTreeMap<String, routeros::Client>,
}
impl<V: Verifier> RouterTransport<V> {
    pub fn new(root: &Path, settings: Settings, verifier: V) -> Result<Self> {
        model::validate(&settings)?;
        let root = root.canonicalize()?;
        ensure!(
            !root.starts_with("/opt/libreqos") && !root.starts_with("/etc/libreqos"),
            "Use separate application data"
        );
        Ok(Self {
            root,
            settings,
            verifier,
            clients: BTreeMap::new(),
        })
    }
    async fn client(&mut self, router: &str) -> Result<&routeros::Client> {
        if !self.clients.contains_key(router) {
            let r = self
                .settings
                .routers
                .iter()
                .find(|r| r.id == router)
                .context("Unknown writer router")?;
            ensure!(
                matches!(r.role.as_str(), "core" | "both")
                    && self
                        .settings
                        .core_lists
                        .bindings
                        .iter()
                        .any(|b| b.router == router),
                "Router is outside core ownership scope"
            );
            let password = crate::credentials::password(&self.root, r).await?;
            let client = routeros::Client::connect_with_password(r, &password).await?;
            drop(password);
            self.clients.insert(router.into(), client);
        }
        Ok(&self.clients[router])
    }
    fn scope(&self, operation: &Operation) -> Result<()> {
        let validate = |m: &Member, added: bool| -> Result<()> {
            ensure!(
                if added {
                    m.id.is_empty()
                } else {
                    m.id.strip_prefix('*').is_some_and(|s| {
                        !s.is_empty() && s.len() <= 32 && s.bytes().all(|c| c.is_ascii_hexdigit())
                    })
                },
                "Invalid owned host ID"
            );
            ensure!(
                !m.disabled
                    && !m.dynamic
                    && m.timeout.as_ref().is_none_or(|s| s.is_empty() || s == "0s"),
                "Owned host flags changed"
            );
            crate::core_plan::host(&m.address)?;
            let (_, wan) = crate::core_plan::owned(&m.comment, &self.settings.core_lists.owner)?
                .context("Host is not owned")?;
            ensure!(
                self.settings
                    .core_lists
                    .bindings
                    .iter()
                    .any(|b| b.router == operation.router() && b.list == m.list && b.wan == wan),
                "Host is outside configured scope"
            );
            Ok(())
        };
        match operation {
            Operation::Add { member, .. } => validate(member, true),
            Operation::Remove { before, .. } => validate(before, false),
            Operation::Move { before, after, .. } => {
                validate(before, false)?;
                validate(after, false)?;
                ensure!(
                    before.id == after.id
                        && before.address == after.address
                        && before.disabled == after.disabled
                        && before.dynamic == after.dynamic
                        && before.timeout == after.timeout
                        && before.comment.split(':').nth(2) == after.comment.split(':').nth(2)
                        && before.list != after.list,
                    "Invalid same-core move"
                );
                Ok(())
            }
        }
    }
    async fn preflight(&mut self, op: &Operation, guard: &Evidence) -> Result<()> {
        self.scope(op)?;
        self.client(op.router()).await?;
        let mut current = self.verifier.verify().await?;
        let mut expected = guard.rows.clone();
        canonical(&mut current.rows);
        canonical(&mut expected);
        ensure!(
            current.context == guard.context && current.rows == expected,
            "Write verifier context or complete core rows changed"
        );
        let actual = members(
            &self
                .client(op.router())
                .await?
                .read("/ip/firewall/address-list/print")
                .await?,
        )?;
        ensure!(
            expected.get(op.router()) == Some(&actual),
            "Core rows changed during final target read"
        );
        match op {
            Operation::Remove { before, .. } | Operation::Move { before, .. } => ensure!(
                actual.iter().any(|m| m == before),
                "Exact owned host precondition failed"
            ),
            Operation::Add { member, .. } => {
                let ip = crate::core_plan::host(&member.address)?;
                for row in &actual {
                    if !row.disabled
                        && self
                            .settings
                            .core_lists
                            .bindings
                            .iter()
                            .any(|b| b.router == op.router() && b.list == row.list)
                    {
                        let (a, z) = crate::core_plan::span(&row.address)?;
                        ensure!(ip < a || ip > z, "New host overlaps existing membership");
                    }
                }
            }
        }
        // The target read awaited after the first verifier. Revalidate identity and authorization now.
        let mut final_evidence = self.verifier.verify().await?;
        canonical(&mut final_evidence.rows);
        ensure!(
            final_evidence.context == guard.context && final_evidence.rows == expected,
            "Verification changed during final target read"
        );
        let now = crate::orchestrator::now();
        for e in [&current, &final_evidence, guard] {
            ensure!(
                e.captured_at <= now && now - e.captured_at <= 30 && now <= e.valid_until,
                "Verification expired before mutation send"
            );
        }
        Ok(())
    }
}
fn canonical(rows: &mut crate::core_writer::Rows) {
    for r in rows.values_mut() {
        r.sort_by(|a, b| a.id.cmp(&b.id));
    }
}
pub fn members(rows: &[Row]) -> Result<Vec<Member>> {
    let mut out = vec![];
    let mut ids = std::collections::BTreeSet::new();
    for r in rows {
        let flag = |key: &str| match r.get(key).map(String::as_str) {
            None | Some("false") => Ok(false),
            Some("true") => Ok(true),
            _ => Err(anyhow::anyhow!("Unknown address-list flag")),
        };
        let id = r.get(".id").context("Missing address-list ID")?.clone();
        ensure!(ids.insert(id.clone()), "Duplicate address-list ID");
        out.push(Member {
            id,
            list: r.get("list").context("Missing address-list name")?.clone(),
            address: r.get("address").context("Missing list address")?.clone(),
            comment: r.get("comment").cloned().unwrap_or_default(),
            dynamic: flag("dynamic")?,
            disabled: flag("disabled")?,
            timeout: r.get("timeout").cloned(),
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}
impl<V: Verifier> Transport for RouterTransport<V> {
    async fn observe(&mut self) -> Result<Evidence> {
        self.verifier.verify().await
    }
    async fn execute(&mut self, op: &Operation, guard: &Evidence) -> Result<Option<String>> {
        self.preflight(op, guard).await.context(RequestNotSent)?;
        let client = &self.clients[op.router()];
        match op {
            Operation::Add { member, .. } => {
                client
                    .migration_command(
                        "add",
                        &[
                            ("list", &member.list),
                            ("address", &member.address),
                            ("comment", &member.comment),
                            ("disabled", "no"),
                        ],
                    )
                    .await
            }
            Operation::Remove { before, .. } => {
                client
                    .migration_command("remove", &[(".id", &before.id)])
                    .await
            }
            Operation::Move { before, after, .. } => {
                client
                    .move_owned_host(&before.id, &after.list, &after.comment)
                    .await?;
                Ok(None)
            }
        }
    }
}
