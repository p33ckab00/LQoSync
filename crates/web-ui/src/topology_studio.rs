//! Router Studio shell. Cached reads and non-secret drafts stay separate from connection forms.
use crate::router_form::{CredentialFields, Ctx, EditableSection, NewRouter, SavedNotice};
use crate::router_form_model::{Form, Section};
use crate::topology_studio_model::*;
use leptos::prelude::*;
use std::collections::HashMap;
use web_contract::{
    Principal, routers::RouterInventory, source_studio::*, topology::TopologySummary,
};

#[component]
pub(crate) fn Studio(
    open: RwSignal<bool>,
    router_id: RwSignal<Option<String>>,
    session: RwSignal<Option<Principal>>,
    data: RwSignal<Option<TopologySummary>>,
) -> impl IntoView {
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let help = NodeRef::<leptos::html::Dialog>::new();
    let help_open = RwSignal::new(false);
    let inventory = RwSignal::new(None::<RouterInventory>);
    let refresh = RwSignal::new(0u32);
    let saved = RwSignal::new(None);
    provide_context(Ctx {
        inventory,
        refresh,
        saved,
        session,
    });
    let caps = RwSignal::new(None::<StudioCapabilities>);
    let config = RwSignal::new(None::<RouterSourceConfig>);
    let draft = RwSignal::new(None::<StudioDraft>);
    let drafts = StoredValue::new(HashMap::<String, StudioDraft>::new());
    let section = RwSignal::new(StudioSection::Connection);
    let adding = RwSignal::new(false);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let profile = RwSignal::new(0usize);
    let credentials = RwSignal::new(Form::default());
    let credential_errors = RwSignal::new(vec![]);
    let generation = StoredValue::new(0u64);
    let actor = StoredValue::new(session.get_untracked().map(|p| p.username));
    let alive = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let end = alive.clone();
    on_cleanup(move || end.store(false, std::sync::atomic::Ordering::Relaxed));
    Effect::new(move |_| {
        let next = session.get().map(|p| p.username);
        if actor.get_value() != next {
            actor.set_value(next);
            drafts.update_value(|d| d.clear());
            draft.set(None);
            credentials.update(|f| f.password.clear());
        }
    });
    Effect::new(move |_| {
        if let Some(d) = dialog.get() {
            if open.get() {
                if !d.open() {
                    let _ = d.show_modal();
                }
            } else if d.open() {
                d.close();
            }
        }
    });
    Effect::new(move |_| {
        let active = open.get();
        let requested = router_id.get();
        refresh.track();
        generation.update_value(|n| *n = n.wrapping_add(1));
        let seq = generation.get_value();
        if !active {
            return;
        }
        if let Some(d) = draft.get_untracked() {
            drafts.update_value(|all| {
                all.insert(d.router_id.clone(), d);
            });
        }
        draft.set(None);
        config.set(None);
        credentials.update(|f| f.password.clear());
        loading.set(true);
        error.set(String::new());
        let expected_actor = actor.get_value();
        let alive = alive.clone();
        leptos::task::spawn_local(async move {
            let routers = crate::topology::get::<RouterInventory>(session, "/api/v1/routers").await;
            let capabilities = crate::topology::get::<StudioCapabilities>(
                session,
                "/api/v1/topology/source-capabilities",
            )
            .await;
            if !alive.load(std::sync::atomic::Ordering::Relaxed)
                || seq != generation.get_value()
                || expected_actor != session.get_untracked().map(|p| p.username)
            {
                return;
            }
            if let Ok(c) = capabilities {
                if c.version == VERSION {
                    caps.set(Some(c));
                } else {
                    caps.set(None);
                }
            } else {
                caps.set(None);
            }
            match routers {
                Ok(list) => {
                    let chosen = requested.or_else(|| list.routers.first().map(|r| r.id.clone()));
                    inventory.set(Some(list));
                    if let Some(id) = chosen {
                        // Set the initial selection only once; the next effect owns its request.
                        if router_id.get_untracked().as_ref() != Some(&id) {
                            router_id.set(Some(id));
                            return;
                        }
                        let source = crate::topology::get::<RouterSourceConfig>(
                            session,
                            &format!("/api/v1/routers/{id}/source-config"),
                        )
                        .await;
                        if !alive.load(std::sync::atomic::Ordering::Relaxed)
                            || seq != generation.get_value()
                            || expected_actor != session.get_untracked().map(|p| p.username)
                        {
                            return;
                        }
                        if let Ok(c) = source {
                            if c.version == VERSION && c.router_id == id {
                                let local = drafts
                                    .with_value(|all| all.get(&id).cloned())
                                    .unwrap_or_else(|| {
                                        let mut d = StudioDraft::from(c.clone());
                                        if let Some(p) =
                                            d.profiles.iter_mut().find(|p| p.id == "default")
                                        {
                                            p.username = inventory
                                                .with_untracked(|i| {
                                                    i.as_ref().and_then(|i| {
                                                        i.routers
                                                            .iter()
                                                            .find(|r| r.id == id)
                                                            .map(|r| r.username.clone())
                                                    })
                                                })
                                                .unwrap_or_default();
                                        }
                                        d
                                    });
                                draft.set(Some(local));
                                config.set(Some(c));
                            } else {
                                error.set("Source configuration version or router identity does not match.".into());
                            }
                        } else {
                            error.set("Source configuration is unavailable. Connection editing remains available.".into());
                        }
                    }
                }
                Err(_) => {
                    error.set("The router inventory is unavailable. Retry the cached view.".into())
                }
            }
            let has_router = inventory.with_untracked(|i| {
                i.as_ref().is_some_and(|i| {
                    i.routers
                        .iter()
                        .any(|r| Some(&r.id) == router_id.get_untracked().as_ref())
                })
            });
            if section_reason(
                section.get_untracked(),
                has_router,
                caps.get_untracked().as_ref(),
                config.get_untracked().as_ref(),
            )
            .is_some()
            {
                section.set(StudioSection::Connection);
            }
            loading.set(false);
        });
    });
    let router = move || {
        inventory.with(|i| {
            i.as_ref().and_then(|i| {
                i.routers
                    .iter()
                    .find(|r| Some(&r.id) == router_id.get().as_ref())
                    .cloned()
            })
        })
    };
    let reason = move |part| {
        section_reason(
            part,
            router().is_some(),
            caps.get().as_ref(),
            config.get().as_ref(),
        )
    };
    let selected_profile = move || {
        draft.with(|d| {
            d.as_ref()
                .and_then(|d| d.profiles.get(profile.get()).cloned())
        })
    };
    // Profile metadata is durable draft state. Credentials use the shared, separate form.
    Effect::new(move |_| {
        let username = credentials.with(|f| f.username.clone());
        draft.update_untracked(|d| {
            if let Some(d) = d
                && let Some(p) = d.profiles.get_mut(profile.get_untracked())
                && p.username != username
            {
                p.username = username;
                d.epoch = d.epoch.wrapping_add(1);
            }
        });
    });
    let credential_new = Memo::new(move |_| selected_profile().is_some_and(|p| !p.credential_set));
    let source_ready = Memo::new(move |_| draft.with(Option::is_some));
    let select_profile = move |index: usize| {
        profile.set(index);
        let username = draft
            .with_untracked(|d| {
                d.as_ref()
                    .and_then(|d| d.profiles.get(index).map(|p| p.username.clone()))
            })
            .unwrap_or_default();
        credentials.set(Form {
            username,
            ..Form::default()
        });
    };
    let close = move || {
        open.set(false);
        credentials.update(|f| f.password.clear());
    };
    view! {
      <dialog node_ref=dialog class="topology-dialog canvas-dialog studio-router-dialog" data-panel="studio" aria-labelledby="studio-router-title"
        on:close=move|_|{if dialog.get_untracked().is_some_and(|d|d.open()){return;}close();}>
        <header class="add-router-heading"><div><h2 id="studio-router-title">"Router source configuration"</h2><p class="field-help">{move||router().map(|r|r.name).unwrap_or("Select a router".into())}</p></div>
          <button type="button" class="canvas-tool" aria-label="Configuration information" title="Configuration information" on:click=move|_|{help_open.set(true);if let Some(d)=help.get_untracked(){let _=d.show_modal();}}>{crate::topology_canvas::icon("M12 11v6M12 7v1M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0")}</button>
          <button type="button" class="canvas-tool" aria-label="Close router configuration" title="Close router configuration" on:click=move|_|close()>{crate::topology_canvas::icon("M6 6l12 12M18 6 6 18")}</button>
        </header>
        <Show when=move||open.get()&&!adding.get()>
        <div class="studio-router-body">
          <aside class="studio-router-list" aria-label="Registered routers">
            <div class="studio-inline"><button type="button" class="canvas-tool" aria-label="Add router" title="Add router" disabled=move||inventory.with(|i|i.as_ref().is_none_or(|i|i.config_digest.is_empty())) on:click=move|_|adding.set(true)>{crate::topology_canvas::icon("M12 5v14M5 12h14")}</button>
              <button type="button" class="canvas-tool" aria-label="Reload router configuration" title="Reload router configuration" on:click=move|_|refresh.update(|n|*n=n.wrapping_add(1))>{crate::topology_canvas::icon("M20 11a8 8 0 1 0-2 6M20 4v7h-7")}</button></div>
            {move||inventory.with(|i|i.as_ref().map(|i|i.routers.iter().map(|r|{let id=r.id.clone();let selected=id.clone();view!{<button type="button" class="quiet studio-router-choice" aria-label=format!("Configure {}",r.name) aria-pressed=move||(router_id.get().as_ref()==Some(&selected)).to_string()
              on:click=move|_|{credentials.set(Form::default());section.set(StudioSection::Connection);profile.set(0);router_id.set(Some(id.clone()));}><strong>{r.name.clone()}</strong><small>{if r.enabled{"Enabled"}else{"Disabled · no connections"}}</small></button>}}).collect_view()))}
            <Show when=move||inventory.with(|i|i.as_ref().is_some_and(|i|i.truncated))><p class="notice">"The inventory is incomplete. Use Routers to inspect the full list."</p></Show>
          </aside>
          <div class="studio-router-content">
            <nav class="studio-section-tabs" aria-label="Router configuration sections">
              {[(StudioSection::Connection,"Connection","M5 3v5h14V3M3 8h18v8H3M5 16v5M19 16v5"),(StudioSection::Access,"Access profiles","M14 7a4 4 0 1 1 0 6l-7 7H3v-4l7-7"),(StudioSection::Sources,"Source policies","M4 5h16M4 12h16M4 19h16M8 3v4M16 10v4M10 17v4")].into_iter().map(move|(part,label,path)|view!{
                <button type="button" class="canvas-tool" aria-label=label title=move||reason(part).unwrap_or(label) aria-pressed=move||(section.get()==part).to_string() disabled=move||loading.get()||reason(part).is_some()
                  on:click=move|_|{credentials.update(|f|f.password.clear());if part==StudioSection::Access{select_profile(profile.get_untracked());}section.set(part);}>{crate::topology_canvas::icon(path)}</button>
              }).collect_view()}
            </nav>
            <Show when=move||loading.get()><p role="status">"Loading cached router configuration…"</p></Show>
            <Show when=move||!error.get().is_empty()><p class="notice" role="status">{move||error.get()}</p></Show>
            <Show when=move||caps.get().is_none()&&!loading.get()><p class="notice">"This backend does not support Studio version 1. The connection editor remains available."</p></Show>
            {move||router().map(|r|{let id=r.id.clone();view!{<SavedNotice router_id=id icons=true/>}})}
            <Show when=move||draft.with(|d|d.as_ref().is_some_and(|d|d.config_digest!=d.loaded_config_digest))><p class="notice">"The service has not loaded the saved access configuration. Policy save remains unavailable."</p></Show>
            {move||match section.get(){
              StudioSection::Connection=>view!{<h3>"Connection"</h3>{move||router().map(|r|view!{<EditableSection router=r section=Section::Connection icons=true/>})}}.into_any(),
              StudioSection::Sources=>view!{<p class="studio-draft-note" role="note">"Local draft. Preview and source-policy save are unavailable in this build. Your draft stays in this tab."</p>
                {move||caps.get().filter(|_|source_ready.get()).map(|caps|view!{<crate::topology_source_editor::SourceEditor draft caps data/>})}
              }.into_any(),
              StudioSection::Access=>view!{
                <h3>"Access profiles"</h3><p class="field-help">"One profile can serve reads and explicit supported writes. Stored passwords never return to this page."</p>
                <label>"Access profile"<select aria-label="Access profile" prop:value=move||profile.get().to_string() on:change={move|ev|if let Ok(i)=event_target_value(&ev).parse::<usize>(){select_profile(i);}}>
                  {move||draft.with(|d|d.as_ref().map(|d|d.profiles.iter().enumerate().map(|(i,p)|view!{<option value=i.to_string()>{p.label.clone()}</option>}).collect_view()))}
                </select></label>
                <button type="button" class="canvas-tool" aria-label="Add access profile" title="Add access profile" disabled=move||draft.with(|d|d.as_ref().is_none_or(|d|d.profiles.len()>=caps.with(|c|c.as_ref().map(|c|c.limits.access_profiles.min(MAX_ACCESS_PROFILES as u64) as usize).unwrap_or(0))))
                  on:click=move|_|{draft.update(|d|if let Some(d)=d{let i=d.profiles.len();d.profiles.push(ProfileDraft{id:format!("profile-{}",i+1),label:"New access profile".into(),username:String::new(),credential_set:false,supported_writes:vec![]});d.epoch=d.epoch.wrapping_add(1);profile.set(i);});credentials.set(Form::default());}>{crate::topology_canvas::icon("M12 5v14M5 12h14")}</button>
                <label>"Profile ID"<input aria-label="Profile ID" prop:value=move||selected_profile().map(|p|p.id).unwrap_or_default() disabled=move||selected_profile().is_some_and(|p|p.credential_set)
                  on:input=move|ev|{let v=event_target_value(&ev);draft.update(|d|if let Some(d)=d&&let Some(p)=d.profiles.get_mut(profile.get_untracked()){p.id=v;d.epoch=d.epoch.wrapping_add(1);});}/></label>
                <label>"Profile label"<input aria-label="Profile label" prop:value=move||selected_profile().map(|p|p.label).unwrap_or_default()
                  on:input=move|ev|{let v=event_target_value(&ev);draft.update(|d|if let Some(d)=d&&let Some(p)=d.profiles.get_mut(profile.get_untracked()){p.label=v;d.epoch=d.epoch.wrapping_add(1);});}/></label>
                <p class="field-help">{move||if selected_profile().is_some_and(|p|p.credential_set){"Credential stored. Enter the profile user before an edit; leave replacement off to keep its secret."}else{"Unsaved profile. Save is unavailable in this build. Leaving this panel clears secret input."}}</p>
                <Show when=move||credential_new.get() fallback=move||view!{<CredentialFields form=credentials errors=credential_errors new=false icons=true/>}><CredentialFields form=credentials errors=credential_errors new=true icons=true/></Show>
                <label class="check"><input type="checkbox" aria-label="Allow WAN address-list writes" prop:checked=move||selected_profile().is_some_and(|p|p.supported_writes.contains(&WriteScope::WanAddressLists))
                  on:change=move|ev|{let on=event_target_checked(&ev);draft.update(|d|if let Some(d)=d&&let Some(p)=d.profiles.get_mut(profile.get_untracked()){p.supported_writes=if on{vec![WriteScope::WanAddressLists]}else{vec![]};d.epoch=d.epoch.wrapping_add(1);});}/>"Allow WAN address-list writes"</label>
              }.into_any(),
            }}
          </div>
        </div>
        <footer class="canvas-dialog-footer"><p role="status">{move||if section.get()==StudioSection::Connection{"Connection edits use the current service."}else{"Local source configuration draft"}}</p>
          <button type="button" class="canvas-tool" aria-label="Preview draft" title="Preview is unavailable in this build" disabled>{crate::topology_canvas::icon("M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0")}</button>
          <button type="button" class="canvas-tool" aria-label="Save configuration" title="Source configuration save is unavailable in this build" disabled>{crate::topology_canvas::icon("M4 3h13l4 4v14H4zM8 3v6h8V3M8 21v-8h9v8")}</button>
        </footer>
        </Show>
        <Show when=move||adding.get()>
          <NewRouter studio=true draft_test=caps.with(|c|c.as_ref().is_some_and(|c|c.features.contains(&StudioFeature::DraftTest)))
            on_closed=Callback::new(move|()|adding.set(false)) on_added=Callback::new(move|id:String|{section.set(StudioSection::Connection);profile.set(0);credentials.set(Form::default());adding.set(false);router_id.set(Some(id));refresh.update(|n|*n=n.wrapping_add(1));})/>
        </Show>
      </dialog>
      <dialog node_ref=help class="topology-dialog canvas-dialog" data-panel="help" aria-labelledby="studio-router-help-title" on:close=move|_|help_open.set(false)>
        <header class="add-router-heading"><h2 id="studio-router-help-title">"Configuration workflow"</h2><button type="button" class="canvas-tool" aria-label="Close" title="Close" on:click=move|_|if let Some(d)=help.get_untracked(){d.close();}>{crate::topology_canvas::icon("M6 6l12 12M18 6 6 18")}</button></header>
        <div class="canvas-dialog-body"><Show when=move||help_open.get()><crate::topology_studio_help::StudioHelp selected=None/></Show></div>
      </dialog>
    }
}
