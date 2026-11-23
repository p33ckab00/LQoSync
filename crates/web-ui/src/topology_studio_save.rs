//! Separate access configuration, saved policy, and runtime evidence.
use crate::{router_form_model::Form, topology_studio_api as api, topology_studio_model::*};
use leptos::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use web_contract::{
    Principal,
    routers::RouterEditForm,
    source_studio::*,
    topology::{EditRequest, TopologySummary},
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
    message: RwSignal<String>,
    receipt: RwSignal<Option<(String, String)>>,
    receipt_inputs: RwSignal<Option<EditRequest>>,
    access_saved: RwSignal<Option<(String, String)>>,
    reauth: RwSignal<bool>,
    reauth_error: RwSignal<String>,
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
        message: RwSignal::new(String::new()),
        receipt: RwSignal::new(None),
        receipt_inputs: RwSignal::new(None),
        access_saved: RwSignal::new(None),
        reauth: RwSignal::new(false),
        reauth_error: RwSignal::new(String::new()),
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
    fn pending(self) -> Option<EditRequest> {
        let id = self
            .draft
            .with(|d| d.as_ref().map(|d| d.router_id.clone()))?;
        self.memory.with(|m| m.saves.get(&id).cloned())
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
    pub fn reason(self) -> Option<String> {
        if self.busy.get() {
            return Some("A save or status request is pending.".into());
        }
        let Some(d) = self.draft.get() else {
            return Some("Select a router and load its configuration.".into());
        };
        let Some(c) = self.caps.get() else {
            return Some("Studio save is unavailable on this backend.".into());
        };
        if d.config_digest == d.loaded_config_digest
            && self.config.with(|c| {
                c.as_ref()
                    .is_some_and(|c| c.config_digest != d.config_digest)
            })
        {
            return Some(
                "The configuration changed. Use current revisions before another explicit save."
                    .into(),
            );
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
                if self.pending().is_some() {
                    return Some("Recover the previous policy save before another save.".into());
                }
                if self
                    .data
                    .with(|s| s.as_ref().and_then(|s| s.registry_revision.clone()))
                    != d.policy_revision
                {
                    return Some(
                        "The registry changed. Use current revisions before a new save.".into(),
                    );
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
            self.message.set(e);
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
                self.memory.update(|m| {
                    m.saves.insert(d.router_id, r.clone());
                });
                self.policy(r, false);
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
                self.message.set(e);
                return;
            }
        };
        let selected = self.profile.get_untracked();
        let mut saved = d.profiles.clone();
        if secret.is_some() {
            saved[selected].credential_set = true;
        }
        drop(secret);
        self.reauth.set(false);
        self.credentials.update(|f| {
            f.password.clear();
            f.replace_password = false;
        });
        self.busy.set(true);
        self.message
            .set("Saving access configuration separately from policy…".into());
        let alive = self.alive.get_value();
        leptos::task::spawn_local(async move {
            let result = api::profiles(
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
            match result{
                Ok(reply) if reply.router_id==d.router_id=>{
                    self.access_saved.set(Some((d.router_id.clone(),reply.digest.clone())));
                    self.message.set(format!("Access configuration saved separately. Waiting for loaded digest {}. Policy is not saved.",reply.digest));
                    self.draft.update(|next|if let Some(next)=next&&next.router_id==d.router_id{
                        for p in &mut next.profiles{if saved.iter().any(|s|s.id==p.id&&s.credential_set){p.credential_set=true;}}
                        next.saved_profiles=saved;next.config_digest=reply.digest.clone();next.epoch=next.epoch.wrapping_add(1);
                    });
                    self.poll_loaded(d.router_id,reply.digest);
                },
                Ok(_)=>self.message.set("The access-save reply names another router. Reload cached configuration.".into()),
                Err(e)=>self.message.set(format!("Access configuration was not confirmed: {}. Reload cached configuration before another save; enter any new secret again.",e.message)),
            }
        });
    }
    fn poll_loaded(self, id: String, digest: String) {
        let alive = self.alive.get_value();
        leptos::task::spawn_local(async move {
            for _ in 0..60 {
                let result = api::get::<RouterSourceConfig>(
                    self.session,
                    &format!("/api/v1/routers/{}/source-config", api::encoded(&id)),
                )
                .await;
                if !alive.load(Ordering::Relaxed)
                    || self.draft.with_untracked(|d| {
                        d.as_ref()
                            .is_none_or(|d| d.router_id != id || d.config_digest != digest)
                    })
                {
                    return;
                }
                match result {
                    Ok(c)
                        if c.router_id == id
                            && c.config_digest == digest
                            && c.loaded_config_digest == digest =>
                    {
                        self.config.set(Some(c));
                        self.draft.update(|d| {
                            if let Some(d) = d {
                                d.loaded_config_digest = digest.clone();
                            }
                        });
                        self.message.set("Access configuration saved and loaded. Source policy is still unsaved.".into());
                        return;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        self.message.set(format!(
                            "Access configuration saved. Loaded state is unknown: {}",
                            e.message
                        ));
                        return;
                    }
                }
                gloo_timers::future::TimeoutFuture::new(1000).await;
            }
            self.message.set("Access configuration saved. The loaded digest is still pending. Refresh cached evidence explicitly.".into());
        });
    }
    fn policy(self, r: EditRequest, resume: bool) {
        if self.busy.get_untracked() {
            return;
        }
        let Some(d) = self.draft.get_untracked() else {
            return;
        };
        let allow = self.caps.with_untracked(|c| {
            c.as_ref()
                .and_then(|c| policy_edit(&d, c, r.key.clone()).ok())
                .is_some_and(|current| current == r)
        });
        self.busy.set(true);
        self.message
            .set("Waiting for the source-policy receipt…".into());
        let alive = self.alive.get_value();
        leptos::task::spawn_local(async move {
            let answer = api::edit(self.session, &r, resume, allow).await;
            if !alive.load(Ordering::Relaxed) {
                return;
            }
            self.busy.set(false);
            match answer {
                Ok(op) => {
                    if let Some(revision) = saved_revision(&op, &r) {
                        self.memory.update(|m| {
                            m.saves.remove(&d.router_id);
                        });
                        self.receipt
                            .set(Some((d.router_id.clone(), revision.clone())));
                        self.receipt_inputs.set(Some(r.clone()));
                        self.message.set(String::new());
                        self.draft.update(|next| {
                            if let Some(next) = next
                                && next.router_id == d.router_id
                                && next.epoch == d.epoch
                                && next.config_digest == d.config_digest
                                && policy_values_match(next, &r)
                            {
                                next.policy_revision = Some(revision);
                                next.epoch = next.epoch.wrapping_add(1);
                            }
                        });
                        self.refresh(false);
                    } else {
                        if matches!(op.state.as_str(), "failed" | "not_committed") {
                            self.memory.update(|m| {
                                m.saves.remove(&d.router_id);
                            });
                        }
                        self.message.set(format!(
                            "Policy save not confirmed: {}",
                            op.error
                                .or_else(|| op.receipt.and_then(|r| r.error))
                                .unwrap_or_else(|| format!(
                                    "{}; recover the original request",
                                    op.state
                                ))
                        ));
                        self.refresh(false);
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
                    self.message
                        .set(format!("Policy save not confirmed: {}", e.message));
                    if e.status == Some(409) {
                        self.refresh(false);
                    }
                }
            }
        });
    }
    fn refresh(self, rebase: bool) {
        if self.busy.get_untracked() {
            return;
        }
        let Some(d) = self.draft.get_untracked() else {
            return;
        };
        self.busy.set(true);
        let alive = self.alive.get_value();
        leptos::task::spawn_local(async move {
            let config = api::get::<RouterSourceConfig>(
                self.session,
                &format!(
                    "/api/v1/routers/{}/source-config",
                    api::encoded(&d.router_id)
                ),
            )
            .await;
            let topology = api::get::<TopologySummary>(self.session, "/api/v1/topology").await;
            if !alive.load(Ordering::Relaxed) {
                return;
            }
            self.busy.set(false);
            if self
                .draft
                .with_untracked(|d2| d2.as_ref().is_none_or(|d2| d2.router_id != d.router_id))
            {
                return;
            }
            match (config, topology) {
                (Ok(c), Ok(t)) if c.router_id == d.router_id => {
                    if rebase {
                        self.draft.update(|d| {
                            if let Some(d) = d {
                                d.config_digest = c.config_digest.clone();
                                d.loaded_config_digest = c.loaded_config_digest.clone();
                                d.policy_revision = t.registry_revision.clone();
                                d.epoch = d.epoch.wrapping_add(1);
                            }
                        });
                        self.message.set("Current revisions loaded. Draft values remain. Review them before another explicit save.".into());
                    }
                    self.config.set(Some(c));
                    self.data.set(Some(t));
                }
                (Err(e), _) | (_, Err(e)) => self
                    .message
                    .set(format!("Cached evidence could not load: {}", e.message)),
                _ => self
                    .message
                    .set("The cached source configuration names another router.".into()),
            }
        });
    }
    pub fn footer_status(self) -> String {
        let status = self.status();
        if status.chars().count() > 140 {
            format!("{}…", status.chars().take(137).collect::<String>())
        } else {
            status
        }
    }
    fn status(self) -> String {
        if !self.message.get().is_empty() {
            return self.message.get();
        }
        if let Some((id, revision)) = self.receipt.get()
            && self
                .draft
                .with(|d| d.as_ref().is_some_and(|d| d.router_id == id))
        {
            let unsaved = self.receipt_inputs.with(|r| {
                r.as_ref().is_some_and(|r| {
                    self.draft
                        .with(|d| d.as_ref().is_some_and(|d| !policy_values_match(d, r)))
                })
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
            let phase = self.data.with(|s| {
                s.as_ref()
                    .and_then(|s| s.publication.as_ref())
                    .map(|p| {
                        format!(
                            "revision {} · {}",
                            p.policy_revision.as_deref().unwrap_or("legacy/unknown"),
                            p.phase
                        )
                    })
                    .unwrap_or("unknown".into())
            });
            return format!(
                "Saved policy revision {revision}. Runtime acceptance is not confirmed. Publication: {phase}.{suffix}"
            );
        }
        "Access configuration, saved policy, and runtime acceptance are separate stages.".into()
    }
}
#[component]
pub(crate) fn SaveUi(state: Save) -> impl IntoView {
    view! {<section class="studio-save-evidence" aria-label="Source save status"><p role="status">{move||state.status()}</p><p class="field-help">{move||state.reason()}</p>
      {move||state.access_saved.get().filter(|(id,_)|state.draft.with(|d|d.as_ref().is_some_and(|d|d.router_id==*id))).map(|(_,digest)|view!{<p class="field-help">{format!("Access configuration saved separately · digest {digest}.")}</p>})}
      <Show when=move||state.data.with(|s|s.as_ref().is_some_and(|s|s.nodes_truncated))><p class="notice">"The cached topology is truncated. It does not show the complete policy projection."</p></Show>
      <Show when=move||state.draft.with(|d|d.as_ref().is_some_and(|d|d.policies.is_empty()))><p class="notice">"Saving this empty set explicitly selects no source policies for this router."</p></Show>
      <div class="studio-inline">
      <button type="button" class="canvas-tool" aria-label="Refresh saved evidence" title="Read cached configuration and runtime evidence" disabled=move||state.busy.get() on:click=move|_|state.refresh(false)>{crate::topology_canvas::icon("M20 11a8 8 0 1 0-2 6M20 4v7h-7")}</button>
      <button type="button" class="canvas-tool" aria-label="Use current revisions" title="Keep draft values and use current revisions for the next explicit save" disabled=move||state.busy.get() on:click=move|_|state.refresh(true)>{crate::topology_canvas::icon("M4 4h16v16H4zM8 8h8M8 12h8M8 16h5")}</button>
      <button type="button" class="canvas-tool" aria-label="Load saved topology" title="Read the saved canvas without a policy or layout write" on:click=move|_|{state.open.set(false);state.load_saved.run(());}>{crate::topology_canvas::icon("M3 5h18v14H3zM7 9h10M7 13h6")}</button>
      <Show when=move||state.pending().is_some()><button type="button" class="canvas-tool" aria-label="Recover policy save" title="Query the original operation before resubmission" disabled=move||state.busy.get() on:click=move|_|if let Some(r)=state.pending(){state.policy(r,true);}>{crate::topology_canvas::icon("M12 8v5l3 2M20 5v6h-6M20 11a8 8 0 1 0-2 6")}</button></Show>
      </div></section>
      <crate::router_form::Reauth open=state.reauth error=state.reauth_error studio=true on_confirm=Callback::new(move|password|state.profiles(Some(password)))/>
    }
}
