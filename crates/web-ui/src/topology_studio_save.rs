//! Separate access configuration, saved policy, and runtime evidence.
use crate::{router_form_model::Form, topology_studio_api as api, topology_studio_model::*};
use leptos::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use web_contract::{
    Principal, routers::RouterEditForm, source_studio::*, topology::TopologySummary,
};
pub(crate) struct Inputs {
    pub draft: RwSignal<Option<StudioDraft>>,
    pub caps: RwSignal<Option<StudioCapabilities>>,
    pub config: RwSignal<Option<RouterSourceConfig>>,
    pub data: RwSignal<Option<TopologySummary>>,
    pub session: RwSignal<Option<Principal>>,
    pub section: RwSignal<StudioSection>,
    pub profile: RwSignal<usize>,
    pub credentials: RwSignal<Form>,
    pub open: RwSignal<bool>,
    pub load_saved: Callback<()>,
    pub clock: RwSignal<u64>,
}
type MergeVersion = (String, String, Option<String>, u64);
#[derive(Clone, Copy)]
pub(crate) struct Save {
    draft: RwSignal<Option<StudioDraft>>,
    caps: RwSignal<Option<StudioCapabilities>>,
    config: RwSignal<Option<RouterSourceConfig>>,
    data: RwSignal<Option<TopologySummary>>,
    session: RwSignal<Option<Principal>>,
    section: RwSignal<StudioSection>,
    profile: RwSignal<usize>,
    credentials: RwSignal<Form>,
    open: RwSignal<bool>,
    load_saved: Callback<()>,
    clock: RwSignal<u64>,
    memory: RwSignal<DraftVault>,
    pub busy: RwSignal<bool>,
    busy_router: RwSignal<String>,
    reauth: RwSignal<bool>,
    reauth_error: RwSignal<String>,
    conflicts: RwSignal<Vec<MergeConflict>>,
    conflict_router: RwSignal<String>,
    choices: RwSignal<std::collections::HashMap<String, bool>>,
    merge_version: StoredValue<Option<MergeVersion>>,
    alive: StoredValue<Arc<AtomicBool>>,
}
pub(crate) fn controller(i: Inputs) -> Save {
    let s = Save {
        draft: i.draft,
        caps: i.caps,
        config: i.config,
        data: i.data,
        session: i.session,
        section: i.section,
        profile: i.profile,
        credentials: i.credentials,
        open: i.open,
        load_saved: i.load_saved,
        clock: i.clock,
        memory: expect_context::<crate::topology_studio::StudioMemory>().0,
        busy: RwSignal::new(false),
        busy_router: RwSignal::new(String::new()),
        reauth: RwSignal::new(false),
        reauth_error: RwSignal::new(String::new()),
        conflicts: RwSignal::new(Vec::new()),
        conflict_router: RwSignal::new(String::new()),
        choices: RwSignal::new(Default::default()),
        merge_version: StoredValue::new(None),
        alive: StoredValue::new(Arc::new(AtomicBool::new(true))),
    };
    let alive = s.alive.get_value();
    on_cleanup(move || alive.store(false, Ordering::Relaxed));
    Effect::new(move |_| {
        if !s.open.get() || s.section.get() != StudioSection::Access {
            s.reauth.set(false);
        }
    });
    s
}
impl Save {
    fn id(self) -> String {
        self.draft
            .with(|d| d.as_ref().map(|d| d.router_id.clone()).unwrap_or_default())
    }
    fn pending(self) -> Option<PendingPolicy> {
        let id = self.id();
        self.memory.with(|m| m.saves.get(&id).cloned())
    }
    fn access(self) -> Option<AccessState> {
        let id = self.id();
        self.memory.with(|m| m.access.get(&id).cloned())
    }
    fn message(self, id: &str, text: impl Into<String>) {
        let text = text.into();
        self.memory.update(|m| {
            m.messages.insert(id.into(), text);
        });
    }
    fn start(self, id: &str, text: &str) {
        self.busy.set(true);
        self.busy_router.set(id.into());
        self.message(id, text);
    }
    fn router_draft(self, id: &str) -> Option<StudioDraft> {
        self.draft
            .get_untracked()
            .filter(|d| d.router_id == id)
            .or_else(|| self.memory.with_untracked(|m| m.drafts.get(id).cloned()))
    }
    fn put_draft(self, d: StudioDraft) {
        let id = d.router_id.clone();
        self.memory.update(|m| {
            m.drafts.insert(id.clone(), d.clone());
        });
        if self
            .draft
            .with_untracked(|now| now.as_ref().is_some_and(|now| now.router_id == id))
        {
            self.draft.set(Some(d));
        }
    }
    fn secret(self) -> Option<String> {
        self.credentials.with(|f| {
            (!f.password.is_empty()
                && (f.replace_password
                    || self.draft.with(|d| {
                        d.as_ref()
                            .and_then(|d| d.profiles.get(self.profile.get()))
                            .is_some_and(|p| !p.credential_set)
                    })))
            .then(|| f.password.clone())
        })
    }
    fn clear_secret(self) {
        self.credentials.update(|f| {
            f.password.clear();
            f.replace_password = false;
        });
    }
    pub fn reason(self) -> Option<String> {
        if self.busy.get() {
            return Some(format!(
                "Wait for the save/status request for router {}.",
                self.busy_router.get()
            ));
        }
        if !self.conflicts.get().is_empty() && self.conflict_router.get() == self.id() {
            return Some("Resolve the saved/draft conflicts and merge current revisions before another save.".into());
        }
        let Some(d) = self.draft.get() else {
            return Some("Select a router and load its configuration.".into());
        };
        let Some(c) = self.caps.get() else {
            return Some("Studio save is unavailable on this backend.".into());
        };
        if let Some(a) = self.access() {
            match a.phase.as_str(){
                "unknown"=>return Some("The access-save outcome is unknown. Reconcile access save before another write. No secret was retained.".into()),
                "retry_secret"=>{
                    if self.section.get()!=StudioSection::Access||self.secret().is_none()||d.profiles.get(self.profile.get()).is_none_or(|p|Some(&p.id)!=a.secret_profile.as_ref()) {return Some("Reenter a replacement credential for the unresolved access profile, then save it with confirmation.".into());}
                },
                "retry_metadata" if self.section.get()!=StudioSection::Access=>return Some("Confirm the access configuration with an explicit access-profile save first.".into()),
                "reload_unconfirmed" if self.section.get()!=StudioSection::Access=>return Some("Access configuration saved, but service reload was not confirmed. Resolve the reload before policy save.".into()),
                _=>{},
            }
        }
        if d.config_digest == d.loaded_config_digest
            && self.config.with(|c| {
                c.as_ref()
                    .is_some_and(|c| c.config_digest != d.config_digest)
            })
        {
            return Some("The configuration changed. Use current revisions to merge saved changes before another write.".into());
        }
        match self.section.get() {
            StudioSection::Connection => {
                Some("Use the connection editor to save connection changes.".into())
            }
            StudioSection::Access => {
                if self
                    .credentials
                    .with(|f| f.replace_password && f.password.is_empty())
                {
                    return Some(
                        "Enter the replacement credential or clear Replace the credential.".into(),
                    );
                }
                if d.config_digest != d.loaded_config_digest {
                    return Some("Wait for the saved access configuration to load.".into());
                }
                access_edit(&d, &c, self.profile.get(), self.secret().as_deref()).err()
            }
            StudioSection::Sources => {
                if let Some(p) = self.pending() {
                    return Some(if let Some(r) = p.saved_revision {
                        format!(
                            "Policy revision {r} saved. Refresh its assigned policy IDs before another save."
                        )
                    } else {
                        "Recover the previous policy save before another save.".into()
                    });
                }
                if self
                    .data
                    .with(|s| s.as_ref().and_then(|s| s.registry_revision.clone()))
                    != d.policy_revision
                {
                    return Some("The registry changed. Use current revisions to merge saved changes before another save.".into());
                }
                policy_edit(
                    &d,
                    &c,
                    "validation-key-maximum-size-000000000000000000000000000000000000".into(),
                )
                .err()
            }
        }
    }
    pub fn save(self) {
        if let Some(e) = self.reason() {
            self.message(&self.id(), e);
            return;
        }
        if self.section.get_untracked() == StudioSection::Access {
            if self.secret().is_some() {
                self.reauth_error.set(String::new());
                self.reauth.set(true);
            } else {
                self.profiles(None);
            }
        } else {
            let (Some(d), Some(c)) = (self.draft.get_untracked(), self.caps.get_untracked()) else {
                return;
            };
            if let Ok(r) = policy_edit(&d, &c, crate::topology_forms::new_key()) {
                let p = PendingPolicy::new(&d, r);
                self.memory.update(|m| {
                    m.saves.insert(d.router_id, p.clone());
                });
                self.policy(p, false);
            }
        }
    }
    fn profiles(self, password: Option<String>) {
        if self.busy.get_untracked() {
            return;
        }
        let (Some(d), Some(c)) = (self.draft.get_untracked(), self.caps.get_untracked()) else {
            return;
        };
        let secret = self.secret();
        let edit = match access_edit(&d, &c, self.profile.get_untracked(), secret.as_deref()) {
            Ok(e) => e,
            Err(e) => {
                self.message(&d.router_id, e);
                return;
            }
        };
        let mut sent = d.profiles.clone();
        let secret_profile = secret
            .as_ref()
            .map(|_| sent[self.profile.get_untracked()].id.clone());
        if secret.is_some() {
            sent[self.profile.get_untracked()].credential_set = true;
        }
        drop(secret);
        self.reauth.set(false);
        self.clear_secret();
        self.start(
            &d.router_id,
            "Saving access configuration separately from policy…",
        );
        let prior = self
            .memory
            .with_untracked(|m| m.access.get(&d.router_id).cloned());
        let pending = AccessState {
            phase: "unknown".into(),
            before_digest: d.config_digest.clone(),
            secret_profile: secret_profile.clone(),
            ..Default::default()
        };
        self.memory.update(|m| {
            m.access.insert(d.router_id.clone(), pending.clone());
        });
        let alive = self.alive.get_value();
        leptos::task::spawn_local(async move {
            let answer = api::profiles(
                self.session,
                &d.router_id,
                &RouterEditForm {
                    edit,
                    current_password: password,
                },
            )
            .await;
            if !alive.load(Ordering::Relaxed) {
                return;
            }
            self.busy.set(false);
            match answer{
                Ok(reply) if reply.router_id==d.router_id&&!reply.digest.is_empty()=>{
                    let state=AccessState{phase:if reply.reload=="sent"{"saved"}else{"reload_unconfirmed"}.into(),digest:reply.digest.clone(),before_digest:d.config_digest.clone(),reload:reply.reload.clone(),warnings:reply.warnings.clone(),secret_profile:secret_profile.or_else(||prior.as_ref().and_then(|a|a.secret_profile.clone()))};
                    self.memory.update(|m|{m.access.insert(d.router_id.clone(),state);});
                    if let Some(mut target)=self.router_draft(&d.router_id){
                        for p in &mut target.profiles{if sent.iter().any(|s|s.id==p.id&&s.credential_set){p.credential_set=true;}}
                        target.saved_profiles=sent;target.config_digest=reply.digest.clone();target.epoch=target.epoch.wrapping_add(1);self.put_draft(target);
                    }
                    if reply.reload=="sent"{self.message(&d.router_id,format!("Access configuration saved separately. Waiting for loaded digest {}. Policy is not saved.",reply.digest));self.poll_loaded(d.router_id,reply.digest);}
                    else{self.message(&d.router_id,format!("Access configuration saved separately. Service reload is unconfirmed: {}. {}",reply.reload,reply.warnings.join(" ")));}
                },
                Ok(_)=>self.message(&d.router_id,"Access-save reply is inconsistent. Outcome unknown. Reconcile access save before another write."),
                Err(e)=>{
                    if !e.uncertain{
                        self.memory.update(|m|{if let Some(a)=prior{m.access.insert(d.router_id.clone(),a);}else{m.access.remove(&d.router_id);}});
                    }
                    self.message(&d.router_id,format!("Access configuration was not confirmed: {}. Draft metadata stays; enter any new secret again.",e.message));
                },
            }
        });
    }
    fn loaded(self, id: &str, c: &RouterSourceConfig) {
        if c.router_id != id {
            return;
        }
        if let Some(mut d) = self.router_draft(id)
            && d.config_digest == c.config_digest
        {
            d.loaded_config_digest = c.loaded_config_digest.clone();
            self.put_draft(d);
        }
        let state = self.memory.with_untracked(|m| m.access.get(id).cloned());
        if let Some(mut a) = state
            && (a.phase == "saved"
                || (a.phase == "reload_unconfirmed" && a.before_digest != a.digest))
            && a.digest == c.config_digest
            && a.digest == c.loaded_config_digest
            && (a.reload == "sent" || a.before_digest != a.digest)
        {
            if a.secret_profile.is_some() && a.before_digest == a.digest {
                a.phase = "digests_match".into();
                self.message(id,"Access credential saved separately. Configuration digests match; they do not prove service pickup of the rotated credential.");
            } else {
                a.phase = "loaded".into();
                self.message(
                    id,
                    "Access configuration saved and loaded. Source policy is still unsaved.",
                );
            }
            self.memory.update(|m| {
                m.access.insert(id.into(), a);
            });
        }
    }
    fn poll_loaded(self, id: String, digest: String) {
        let alive = self.alive.get_value();
        leptos::task::spawn_local(async move {
            for _ in 0..60 {
                let c = api::get::<RouterSourceConfig>(
                    self.session,
                    &format!("/api/v1/routers/{}/source-config", api::encoded(&id)),
                )
                .await;
                if !alive.load(Ordering::Relaxed)
                    || self
                        .router_draft(&id)
                        .is_none_or(|d| d.config_digest != digest)
                {
                    return;
                }
                match c {
                    Ok(c)
                        if c.router_id == id
                            && c.config_digest == digest
                            && c.loaded_config_digest == digest =>
                    {
                        self.loaded(&id, &c);
                        if self.id() == id {
                            self.config.set(Some(c));
                        }
                        return;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        self.message(
                            &id,
                            format!(
                                "Access configuration saved. Loaded evidence is unavailable: {}",
                                e.message
                            ),
                        );
                        return;
                    }
                }
                gloo_timers::future::TimeoutFuture::new(1000).await;
            }
            self.message(&id,"Access configuration saved. The loaded digest remains pending. Refresh saved evidence explicitly.");
        });
    }
    fn policy(self, p: PendingPolicy, resume: bool) {
        if self.busy.get_untracked() {
            return;
        }
        if p.saved_revision.is_some() {
            self.refresh(false, false);
            return;
        }
        let Some(d) = self.draft.get_untracked() else {
            return;
        };
        self.start(&d.router_id, "Waiting for the source-policy receipt…");
        let alive = self.alive.get_value();
        let live = alive.clone();
        let original = p.request.clone();
        let epoch = d.epoch;
        let allow = move || {
            if !live.load(Ordering::Relaxed) {
                return false;
            }
            let Some(now) = self.draft.try_get_untracked().flatten() else {
                return false;
            };
            now.epoch == epoch
                && self.config.try_get_untracked().flatten().is_some_and(|c| {
                    c.version == VERSION
                        && c.router_id == now.router_id
                        && c.config_digest == now.config_digest
                        && c.loaded_config_digest == now.loaded_config_digest
                })
                && self
                    .data
                    .try_get_untracked()
                    .flatten()
                    .is_some_and(|t| t.registry_revision == now.policy_revision)
                && self.memory.try_get_untracked().is_some_and(|m| {
                    !m.access.get(&now.router_id).is_some_and(|a| {
                        matches!(
                            a.phase.as_str(),
                            "unknown" | "retry_secret" | "retry_metadata" | "reload_unconfirmed"
                        )
                    })
                })
                && self
                    .caps
                    .try_get_untracked()
                    .flatten()
                    .and_then(|c| policy_edit(&now, &c, original.key.clone()).ok())
                    .is_some_and(|r| r == original)
        };
        leptos::task::spawn_local(async move {
            let answer = api::edit(self.session, &p.request, resume, allow).await;
            if !alive.load(Ordering::Relaxed) {
                return;
            }
            self.busy.set(false);
            match answer {
                Ok(op) => {
                    if let Some(revision) = saved_revision(&op, &p.request) {
                        let mut saved = p.clone();
                        saved.saved_revision = Some(revision.clone());
                        self.memory.update(|m| {
                            m.last_policies.insert(d.router_id.clone(), saved.clone());
                            if saved.needs_ids() {
                                m.saves.insert(d.router_id.clone(), saved.clone());
                            } else {
                                m.saves.remove(&d.router_id);
                            }
                        });
                        if let Some(mut target) = self.router_draft(&d.router_id) {
                            target.saved_policies = saved.rows.clone();
                            if target.epoch == d.epoch && policy_values_match(&target, &p.request) {
                                target.policy_revision = Some(revision);
                            }
                            target.epoch = target.epoch.wrapping_add(1);
                            self.put_draft(target);
                        }
                        self.message(&d.router_id, "");
                        self.refresh_router(d.router_id, false, false);
                    } else {
                        if matches!(op.state.as_str(), "failed" | "not_committed") {
                            self.memory.update(|m| {
                                m.saves.remove(&d.router_id);
                            });
                        }
                        self.message(
                            &d.router_id,
                            format!(
                                "Policy save not confirmed: {}",
                                op.error
                                    .or_else(|| op.receipt.and_then(|r| r.error))
                                    .unwrap_or_else(|| format!(
                                        "{}; recover the original request",
                                        op.state
                                    ))
                            ),
                        );
                        self.refresh_router(d.router_id, false, false);
                    }
                }
                Err(e) => {
                    if e.code == "absent_changed"
                        || matches!(e.status, Some(400 | 403 | 409 | 410 | 413))
                    {
                        self.memory.update(|m| {
                            m.saves.remove(&d.router_id);
                        });
                    }
                    self.message(
                        &d.router_id,
                        format!("Policy save not confirmed: {}", e.message),
                    );
                    if e.status == Some(409) {
                        self.refresh_router(d.router_id, false, false);
                    }
                }
            }
        });
    }
    fn refresh(self, rebase: bool, reconcile: bool) {
        self.refresh_router(self.id(), rebase, reconcile);
    }
    fn refresh_router(self, id: String, rebase: bool, reconcile: bool) {
        if self.busy.get_untracked() || id.is_empty() {
            return;
        }
        self.busy.set(true);
        self.busy_router.set(id.clone());
        let alive = self.alive.get_value();
        leptos::task::spawn_local(async move {
            let c = api::get::<RouterSourceConfig>(
                self.session,
                &format!("/api/v1/routers/{}/source-config", api::encoded(&id)),
            )
            .await;
            let t = api::get::<TopologySummary>(self.session, "/api/v1/topology").await;
            let inventory = if rebase {
                api::get::<web_contract::routers::RouterInventory>(self.session, "/api/v1/routers")
                    .await
                    .ok()
            } else {
                None
            };
            if !alive.load(Ordering::Relaxed) {
                return;
            }
            self.busy.set(false);
            let (c, t) = match (c, t) {
                (Ok(c), Ok(t)) => (c, t),
                (Err(e), _) | (_, Err(e)) => {
                    self.message(
                        &id,
                        format!("Cached evidence could not load: {}", e.message),
                    );
                    return;
                }
            };
            if rebase && inventory.is_none() {
                self.message(
                    &id,
                    "Current router metadata could not load. No new revision was accepted.",
                );
                return;
            }
            if c.version != VERSION {
                self.message(&id,"This source configuration version is unsupported. No new revision was accepted.");
                return;
            }
            if c.router_id != id {
                self.message(&id, "The saved configuration names another router.");
                return;
            }
            let Some(mut d) = self.router_draft(&id) else {
                return;
            };
            if let Some(p) = self.memory.with_untracked(|m| {
                m.saves
                    .get(&id)
                    .filter(|p| p.saved_revision.is_some())
                    .cloned()
            }) {
                if adopt_saved_ids(&mut d, &p, &c) {
                    self.memory.update(|m| {
                        m.saves.remove(&id);
                    });
                    self.put_draft(d.clone());
                } else {
                    self.message(&id,"Policy saved. Assigned IDs are not confirmed by current metadata. Draft values stay; refresh saved evidence before another save.");
                    if self.id() == id {
                        self.config.set(Some(c));
                        self.data.set(Some(t));
                    }
                    return;
                }
            }
            self.loaded(&id, &c);
            if rebase {
                if c.policy_revision != t.registry_revision
                    || self
                        .data
                        .with_untracked(|s| {
                            s.as_ref()
                                .and_then(|s| s.registry_revision.as_ref())
                                .and_then(|r| r.parse::<u64>().ok())
                        })
                        .zip(
                            t.registry_revision
                                .as_ref()
                                .and_then(|r| r.parse::<u64>().ok()),
                        )
                        .is_some_and(|(current, read)| current > read)
                {
                    if self.id() == id
                        && self.data.with_untracked(|s| {
                            s.as_ref()
                                .and_then(|s| s.registry_revision.as_ref())
                                .and_then(|r| r.parse::<u64>().ok())
                                .unwrap_or(0)
                        }) <= t
                            .registry_revision
                            .as_ref()
                            .and_then(|r| r.parse::<u64>().ok())
                            .unwrap_or(0)
                    {
                        self.data.set(Some(t));
                    }
                    self.message(&id,"Cached policy revisions differ. Retry cached reads; no new revision was accepted.");
                    return;
                }
                let Some(mut current) = self.router_draft(&id) else {
                    return;
                };
                let version = (
                    id.clone(),
                    c.config_digest.clone(),
                    c.policy_revision.clone(),
                    current.epoch,
                );
                if self.merge_version.get_value().as_ref() != Some(&version) {
                    self.choices.set(Default::default());
                    self.merge_version.set_value(Some(version));
                }
                let username = inventory.as_ref().and_then(|i| {
                    i.routers
                        .iter()
                        .find(|r| r.id == id)
                        .map(|r| r.username.as_str())
                });
                if c.access_profiles.iter().any(|p| p.id == "default") && username.is_none() {
                    self.message(
                        &id,
                        "Current router metadata is incomplete. No new revision was accepted.",
                    );
                    return;
                }
                match rebase_draft(&mut current, &c, username, &self.choices.get_untracked()) {
                    Ok(()) => {
                        self.put_draft(current);
                        self.conflicts.set(Vec::new());
                        self.choices.set(Default::default());
                        self.message(&id,"Current revisions loaded. Saved changes were merged; draft values remain. Review before another explicit save.");
                    }
                    Err(conflicts) => {
                        self.conflict_router.set(id.clone());
                        self.conflicts.set(conflicts);
                        self.message(&id,"Saved and draft entries overlap. Review each conflict and choose its values, then use current revisions again. No new revision was accepted.");
                        return;
                    }
                }
                if reconcile {
                    self.memory.update(|m| {
                        if let Some(a) = m.access.get_mut(&id) {
                            a.phase = if a.secret_profile.is_some() {
                                "retry_secret"
                            } else {
                                "retry_metadata"
                            }
                            .into();
                        }
                    });
                    self.message(&id,"Access outcome remains unconfirmed. Cached metadata cannot verify a secret. Reenter any replacement credential and save the access configuration explicitly.");
                }
            }
            if self.id() == id {
                self.config.set(Some(c));
                self.data.set(Some(t));
            }
        });
    }
    fn status(self) -> String {
        let id = self.id();
        let message = self
            .memory
            .with(|m| m.messages.get(&id).cloned())
            .unwrap_or_default();
        if !self.busy.get()
            && self.access().is_some_and(|a| a.phase == "unknown")
            && !message.starts_with("Access configuration was not confirmed")
        {
            return "Access configuration was not confirmed: the outcome is unknown. Reconcile access save before another write. No secret was retained.".into();
        }
        if !message.is_empty() {
            return message;
        }
        if let Some(p) = self.memory.with(|m| m.last_policies.get(&id).cloned())
            && let Some(revision) = p.saved_revision
        {
            let unsaved = self.draft.with(|d| {
                d.as_ref()
                    .is_some_and(|d| !policy_values_match(d, &p.request))
            });
            let suffix = if unsaved {
                " Current draft changes are unsaved."
            } else {
                ""
            };
            if self.data.with(|s| {
                s.as_ref()
                    .is_some_and(|s| runtime_applied(s, &revision, self.clock.get()))
            }) {
                return format!(
                    "Applied policy revision {revision}: matching runtime acceptance confirmed.{suffix}"
                );
            }
            return format!(
                "Saved policy revision {revision}. Runtime acceptance is not confirmed.{suffix}"
            );
        }
        "Access configuration, saved policy, and runtime acceptance are separate stages.".into()
    }
    pub fn footer_status(self) -> String {
        let status = self.status();
        if status.chars().count() > 140 {
            format!("{}…", status.chars().take(137).collect::<String>())
        } else {
            status
        }
    }
}
#[component]
pub(crate) fn SaveUi(state: Save) -> impl IntoView {
    view! {<section class="studio-save-evidence" aria-label="Source save status"><p role="status">{move||state.status()}</p><p class="field-help">{move||state.reason()}</p>
      {move||state.access().filter(|a|!a.digest.is_empty()).map(|a|view!{<div><p class="field-help">{format!("Access configuration saved separately · digest {} · initial reload result {}.",a.digest,a.reload)}</p>{a.warnings.into_iter().map(|w|view!{<p class="notice">{w}</p>}).collect_view()}</div>})}
      <Show when=move||state.data.with(|s|s.as_ref().is_some_and(|s|s.nodes_truncated))><p class="notice">"The cached topology is truncated. It does not show the complete policy projection."</p></Show>
      <Show when=move||state.draft.with(|d|d.as_ref().is_some_and(|d|d.policies.is_empty()))><p class="notice">"Saving this empty set explicitly selects no source policies for this router."</p></Show>
      {move||state.conflicts.get().into_iter().filter(move|_|state.id()==state.conflict_router.get()).map(move|c|{let local=c.key.clone();let saved=c.key.clone();let ls=c.key.clone();let rs=c.key.clone();view!{<div class="notice"><p>{c.label}</p><details><summary>"Draft and saved values"</summary><strong>"Draft"</strong><pre style="white-space:pre-wrap;overflow-wrap:anywhere;max-block-size:160px;overflow:auto">{c.local}</pre><strong>"Saved"</strong><pre style="white-space:pre-wrap;overflow-wrap:anywhere;max-block-size:160px;overflow:auto">{c.saved}</pre></details><button type="button" class="canvas-tool" aria-label=format!("Keep draft {}",c.key) title="Keep the draft version of this entry" aria-pressed=move||state.choices.with(|v|v.get(&ls)==Some(&true)).to_string() on:click=move|_|state.choices.update(|v|{v.insert(local.clone(),true);})>{crate::topology_canvas::icon("M16 3l5 5-12 12-6 1 1-6z")}</button><button type="button" class="canvas-tool" aria-label=format!("Use saved {}",c.key) title="Use the saved version of this entry" aria-pressed=move||state.choices.with(|v|v.get(&rs)==Some(&false)).to_string() on:click=move|_|state.choices.update(|v|{v.insert(saved.clone(),false);})>{crate::topology_canvas::icon("M3 5h18v14H3zM7 9h10M7 13h6")}</button></div>}}).collect_view()}
      <div class="studio-inline">
      <button type="button" class="canvas-tool" aria-label="Refresh saved evidence" title="Read cached configuration and runtime evidence" disabled=move||state.busy.get() on:click=move|_|state.refresh(false,false)>{crate::topology_canvas::icon("M20 11a8 8 0 1 0-2 6M20 4v7h-7")}</button>
      <button type="button" class="canvas-tool" aria-label="Use current revisions" title="Merge saved and draft entries before accepting a new revision" disabled=move||state.busy.get() on:click=move|_|state.refresh(true,false)>{crate::topology_canvas::icon("M4 4h16v16H4zM8 8h8M8 12h8M8 16h5")}</button>
      <Show when=move||state.access().is_some_and(|a|matches!(a.phase.as_str(),"unknown"|"retry_secret"|"retry_metadata"))><button type="button" class="canvas-tool" aria-label="Reconcile access save" title="Read current metadata, then explicitly confirm the access save; no secret was retained" disabled=move||state.busy.get() on:click=move|_|state.refresh(true,true)>{crate::topology_canvas::icon("M12 8v5l3 2M20 5v6h-6M20 11a8 8 0 1 0-2 6")}</button></Show>
      <button type="button" class="canvas-tool" aria-label="Load saved topology" title="Read the saved canvas without a policy or layout write" on:click=move|_|{state.open.set(false);state.load_saved.run(());}>{crate::topology_canvas::icon("M3 5h18v14H3zM7 9h10M7 13h6")}</button>
      <Show when=move||state.pending().is_some()><button type="button" class="canvas-tool" aria-label="Recover policy save" title="Read the original operation or its saved policy IDs before any resubmission" disabled=move||state.busy.get() on:click=move|_|if let Some(p)=state.pending(){state.policy(p,true);}>{crate::topology_canvas::icon("M12 8v5l3 2M20 5v6h-6M20 11a8 8 0 1 0-2 6")}</button></Show>
      </div></section>
      <crate::router_form::Reauth open=state.reauth error=state.reauth_error studio=true on_confirm=Callback::new(move|password|state.profiles(Some(password))) on_cancel=Callback::new(move|()|state.clear_secret())/>
    }
}
