//! Router registration and local edits. A save changes the configuration only: it makes no router I/O.
//! The page shows two stages: saved by the core, then loaded by the service (the inventory digest).
use crate::dashboard::local_time;
use crate::route::go_to;
use crate::route_model::{Tab, router_href};
use crate::router_form_model::{Form, Section, changes, check, create, removes_a_source, update};
use crate::routers_model::{impact, role_label, tracks, transport};
use gloo_net::http::Request;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_contract::Principal;
use web_contract::routers::{
    RouterActivity, RouterEditForm, RouterEditOutcome, RouterEditReply, RouterInventory,
    RouterSummary,
};

/// Shared state of the Routers screen.
#[derive(Clone, Copy)]
pub(crate) struct Ctx {
    pub inventory: RwSignal<Option<RouterInventory>>,
    /// A change loads the inventory again.
    pub refresh: RwSignal<u32>,
    pub saved: RwSignal<Option<Saved>>,
    pub session: RwSignal<Option<Principal>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Saved {
    pub router_id: String,
    pub digest: String,
    pub reload: String,
    pub warnings: Vec<String>,
    pub what: &'static str,
    /// A disable shows two result tracks: connections and the shaping removal.
    pub disable: bool,
}

pub(crate) fn ctx() -> Ctx {
    expect_context::<Ctx>()
}

/// The digest of the loaded configuration. Empty: the service is too old for web edits.
pub(crate) fn loaded_digest(c: Ctx) -> String {
    c.inventory
        .with_untracked(|i| i.as_ref().map(|i| i.config_digest.clone()))
        .unwrap_or_default()
}

pub(crate) struct Failure {
    pub code: String,
    pub message: String,
}

/// POST a new router or PATCH one router. The core can need some seconds, so the limit is 35 s.
pub(crate) async fn send(
    c: Ctx,
    router_id: Option<String>,
    form: RouterEditForm,
) -> Result<RouterEditReply, Failure> {
    let fail = |code: &str, message: &str| Failure {
        code: code.into(),
        message: message.into(),
    };
    let csrf = c
        .session
        .get_untracked()
        .map(|p| p.csrf)
        .unwrap_or_default();
    let controller = web_sys::AbortController::new()
        .map_err(|_| fail("internal", "The browser cannot send the change."))?;
    let signal = controller.signal();
    let _timer = gloo_timers::callback::Timeout::new(35_000, move || controller.abort());
    let builder = match &router_id {
        None => Request::post("/api/v1/routers"),
        Some(id) => Request::patch(&format!("/api/v1/routers/{id}")),
    };
    let request = builder
        .header("x-csrf-token", &csrf)
        .abort_signal(Some(&signal))
        .json(&form)
        .map_err(|_| fail("internal", "The browser cannot send the change."))?;
    let response = request.send().await.map_err(|_| {
        fail(
            "network",
            "The change did not reach the server. Check the connection, then try again.",
        )
    })?;
    if response.status() == 401 {
        c.session.set(None);
        return Err(fail("session", "Your session ended. Sign in again."));
    }
    match response.json::<RouterEditOutcome>().await {
        Ok(RouterEditOutcome {
            code,
            reply: Some(reply),
            ..
        }) if code == "ok" => Ok(reply),
        Ok(o) => Err(Failure {
            code: o.code,
            message: o.message,
        }),
        Err(_) if response.status() == 403 => Err(fail(
            "forbidden",
            "The server refused the change. Load the page again, then try again.",
        )),
        Err(_) => Err(fail(
            "internal",
            "The server could not save the change. Try again; if it fails again, use the TUI.",
        )),
    }
}

/// After a save: load the inventory again every 2 s until it shows the saved digest, for at most 60 s.
pub(crate) fn follow(c: Ctx, saved: Saved) {
    let digest = saved.digest.clone();
    c.saved.set(Some(saved));
    leptos::task::spawn_local(async move {
        for _ in 0..30 {
            c.refresh.update(|n| *n = n.wrapping_add(1));
            gloo_timers::future::TimeoutFuture::new(2000).await;
            if c.inventory
                .with_untracked(|i| i.as_ref().is_some_and(|i| i.config_digest == digest))
            {
                break;
            }
        }
    });
}

/// The two save stages for one router, in place, with the reload result and warnings.
#[component]
pub(crate) fn SavedNotice(
    router_id: String,
    #[prop(default = false)] icons: bool,
) -> impl IntoView {
    let c = ctx();
    let id = router_id.clone();
    let mine = Memo::new(move |_| c.saved.get().filter(|s| s.router_id == id));
    let loaded = Memo::new(move |_| {
        mine.with(|s| {
            s.as_ref().is_some_and(|s| {
                c.inventory
                    .with(|i| i.as_ref().is_some_and(|i| i.config_digest == s.digest))
            })
        })
    });
    view! {
      {move||mine.get().map(|s|view!{
        <div class=move||format!("saved-notice {}",if loaded.get(){"done"}else{"waiting"}) role="status">
          <p><b>{s.what}</b>{move||if loaded.get(){" The service loaded the change."}else{" Waiting for the service to load the change…"}}</p>
          {s.disable.then(||{let id=s.router_id.clone();view!{
            <dl class="tracks">{move||c.inventory.with(|i|i.as_ref().and_then(|i|i.routers.iter().find(|r|r.id==id).map(|r|{
              let (io,shaping)=tracks(r,&i.shaping,loaded.get());
              view!{<div><dt>"Router connections"</dt><dd>{io}</dd></div><div><dt>"Shaping removal"</dt><dd>{shaping}</dd></div>}
            })))}</dl>
          }})}
          {(s.reload!="sent").then(||view!{<p class="router-warn">{format!("The reload signal did not reach the service: {}. The change applies when the service starts again.",s.reload)}</p>})}
          {s.warnings.iter().map(|w|view!{<p class="router-warn">{w.clone()}</p>}).collect_view()}
          <button type="button" class=if icons{"canvas-tool"}else{"quiet"} aria-label="Dismiss" title="Dismiss" on:click=move|_|c.saved.set(None)>{if icons{crate::topology_canvas::icon("M6 6l12 12M18 6 6 18").into_any()}else{"Dismiss".into_any()}}</button>
        </div>
      })}
    }
}

fn set_text(form: RwSignal<Form>, f: fn(&mut Form, String)) -> impl Fn(leptos::ev::Event) + Copy {
    move |ev| {
        let v = event_target_value(&ev);
        form.update(|x| f(x, v));
    }
}
fn set_check(form: RwSignal<Form>, f: fn(&mut Form, bool)) -> impl Fn(leptos::ev::Event) + Copy {
    move |ev| {
        let v = event_target_checked(&ev);
        form.update(|x| f(x, v));
    }
}

/// A labelled text field with its help text and its error below the field.
#[component]
fn Field(
    id: &'static str,
    label: &'static str,
    #[prop(optional)] help: &'static str,
    errors: RwSignal<Vec<(&'static str, String)>>,
    children: Children,
) -> impl IntoView {
    let error = move || errors.with(|e| e.iter().find(|(k, _)| *k == id).map(|(_, m)| m.clone()));
    view! {
      <div class="field" class:invalid=move||error().is_some()>
        <label for=id>{label}</label>
        {children()}
        {(!help.is_empty()).then(||view!{<p class="field-help" id=format!("{id}-help")>{help}</p>})}
        <p class="field-error" id=format!("{id}-error") aria-live="polite">{error}</p>
      </div>
    }
}

/// While the operator types, an error goes away when its field passes. No new error appears before
/// the next Next or Save.
fn clear_fixed(
    form: RwSignal<Form>,
    errors: RwSignal<Vec<(&'static str, String)>>,
    new: bool,
    section: impl Fn() -> Section + 'static,
) {
    Effect::new(move |_| {
        let still = form.with(|f| check(f, new, section()));
        if errors.with_untracked(|e| e.iter().any(|(k, _)| !still.iter().any(|(n, _)| n == k))) {
            errors.update(|e| e.retain(|(k, _)| still.iter().any(|(n, _)| n == k)));
        }
    });
}

/// Move focus to the first invalid field, after a failed step or save.
fn focus_first(errors: &[(&'static str, String)]) {
    let Some((id, _)) = errors.first() else {
        return;
    };
    let id = if *id == "location" { "latitude" } else { id };
    if let Some(e) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = e.focus();
    }
}

/// The connection fields: shared by Add router and the Connection editor.
#[component]
fn ConnectionFields(
    form: RwSignal<Form>,
    errors: RwSignal<Vec<(&'static str, String)>>,
    new: bool,
    #[prop(default = false)] icons: bool,
) -> impl IntoView {
    let invalid =
        move |k: &'static str| errors.with(|e| e.iter().any(|(x, _)| *x == k)).to_string();
    let ca_error = RwSignal::new(String::new());
    view! {
      {new.then(||view!{
        <Field id="name" label="Router name" help="The router identity. It cannot change later." errors>
          <input id="name" type="text" autocomplete="off" spellcheck="false" maxlength="64" required
            aria-invalid=move||invalid("name") aria-describedby="name-help name-error"
            prop:value=move||form.with(|f|f.name.clone()) on:input=set_text(form,|f,v|f.name=v)/>
        </Field>
      })}
      <div class="field-row">
        <Field id="address" label="Address" help="IP address or host name of the RouterOS API." errors>
          <input id="address" type="text" autocomplete="off" spellcheck="false" maxlength="253" required
            aria-invalid=move||invalid("address") aria-describedby="address-help address-error"
            prop:value=move||form.with(|f|f.address.clone()) on:input=set_text(form,|f,v|f.address=v)/>
        </Field>
        <Field id="port" label="API port" errors>
          <input id="port" type="text" inputmode="numeric" autocomplete="off" maxlength="5" required class="short"
            aria-invalid=move||invalid("port") aria-describedby="port-error"
            prop:value=move||form.with(|f|f.port.clone()) on:input=set_text(form,|f,v|f.port=v)/>
        </Field>
      </div>
      <div class="check">
        <input id="tls" type="checkbox" prop:checked=move||form.with(|f|f.tls)
          on:change=move|ev|{let on=event_target_checked(&ev);form.update(|f|{f.tls=on;f.port=if on{"8729".into()}else{"8728".into()};});}/>
        <label for="tls">"Use TLS (API-SSL)"<small>"The default port changes: 8729 with TLS, 8728 without."</small></label>
      </div>
      <Show when=move||form.with(|f|f.tls)>
        <div class="check">
          <input id="tls_verify" type="checkbox" prop:checked=move||form.with(|f|f.tls_verify) on:change=set_check(form,|f,v|f.tls_verify=v)/>
          <label for="tls_verify">"Check the router certificate"<small>"Turn this off only for a test: the connection then accepts any certificate."</small></label>
        </div>
        <Field id="ca" label="CA certificate" help="A PEM file of the CA that signed the router certificate. Without it, the public CAs apply." errors>
          <input id="ca" type="file" accept=".pem,.crt,.cer" aria-describedby="ca-help ca-error"
            on:change=move|ev|{
              let input:web_sys::HtmlInputElement=event_target(&ev);
              let Some(file)=input.files().and_then(|l|l.get(0)) else {return};
              if file.size()>16384.0 {ca_error.set("The file is larger than 16 KiB.".into());return;}
              ca_error.set(String::new());
              let name=file.name();
              leptos::task::spawn_local(async move{
                if let Ok(text)=wasm_bindgen_futures::JsFuture::from(file.text()).await
                  && let Some(text)=text.as_string()
                {
                  form.update(|f|{f.ca_pem=Some(text);f.ca_name=name;f.clear_ca=false;});
                }
              });
            }/>
          <p class="field-error" aria-live="polite">{move||ca_error.get()}</p>
          <Show when=move||form.with(|f|f.ca_pem.is_some())>
            <p class="field-help">{move||format!("Selected: {}",form.with(|f|f.ca_name.clone()))}
              " "<button type="button" class=if icons{"canvas-tool"}else{"link"} aria-label="Remove the selection" title="Remove the selection" on:click=move|_|form.update(|f|{f.ca_pem=None;f.ca_name.clear();})>{if icons{crate::topology_canvas::icon("M6 6l12 12M18 6 6 18").into_any()}else{"Remove the selection".into_any()}}</button></p>
          </Show>
        </Field>
      </Show>
      <CredentialFields form errors new icons/>
    }
}

/// Shared credential fields. Studio keeps this form outside its policy draft.
#[component]
pub(crate) fn CredentialFields(
    form: RwSignal<Form>,
    errors: RwSignal<Vec<(&'static str, String)>>,
    new: bool,
    #[prop(default = false)] icons: bool,
) -> impl IntoView {
    let show = RwSignal::new(false);
    let invalid =
        move |k: &'static str| errors.with(|e| e.iter().any(|(x, _)| *x == k)).to_string();
    view! {
      <Field id="username" label="RouterOS user" help="Use the RouterOS user for this connection. Read-only access is sufficient for discovery." errors>
        <input id="username" type="text" autocomplete="off" spellcheck="false" maxlength="64" required
          aria-invalid=move||invalid("username") aria-describedby="username-help username-error"
          prop:value=move||form.with(|f|f.username.clone()) on:input=set_text(form,|f,v|f.username=v)/>
      </Field>
      {(!new).then(||view!{
        <div class="check">
          <input id="replace_password" type="checkbox" prop:checked=move||form.with(|f|f.replace_password) on:change=set_check(form,|f,v|{f.replace_password=v;if !v{f.password.clear();}})/>
          <label for="replace_password">"Replace the credential"<small>"Without this, the current credential stays. You confirm with your own password."</small></label>
        </div>
      })}
      <Show when=move||new||form.with(|f|f.replace_password)>
        <Field id="password" label="Credential" help="The RouterOS password. Responses never return a stored credential." errors>
          <div class="password-field">
            <input id="password" type=move||if show.get(){"text"}else{"password"} autocomplete="new-password" maxlength="256" required
              aria-invalid=move||invalid("password") aria-describedby="password-help password-error"
              prop:value=move||form.with(|f|f.password.clone()) on:input=set_text(form,|f,v|f.password=v)/>
            <button type="button" aria-label=move||if show.get(){"Hide credential"}else{"Show credential"} class:canvas-tool=icons title=move||if show.get(){"Hide credential"}else{"Show credential"} on:click=move|_|show.update(|v| *v = !*v)>{if icons{crate::topology_canvas::icon("M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0").into_any()}else{view!{<span>{move||if show.get(){"Hide"}else{"Show"}}</span>}.into_any()}}</button>
          </div>
        </Field>
      </Show>
    }
}

/// The source fields: shared by Add router and the Sources editor.
#[component]
fn SourceFields(
    form: RwSignal<Form>,
    errors: RwSignal<Vec<(&'static str, String)>>,
) -> impl IntoView {
    let check_box = move |id: &'static str,
                          label: &'static str,
                          help: &'static str,
                          get: fn(&Form) -> bool,
                          put: fn(&mut Form, bool)| {
        view! {
          <div class="check">
            <input id=id type="checkbox" prop:checked=move||form.with(get) on:change=set_check(form,put)/>
            <label for=id>{label}<small>{help}</small></label>
          </div>
        }
    };
    view! {
      <fieldset class="sub"><legend>"Subscriber sources"</legend>
        {check_box("pppoe","PPPoE","Read PPP secrets and active sessions.",|f|f.pppoe,|f,v|f.pppoe=v)}
        {check_box("dhcp","DHCP","Read DHCP leases.",|f|f.dhcp,|f,v|f.dhcp=v)}
        {check_box("hotspot","Hotspot","Read Hotspot users and active hosts.",|f|f.hotspot,|f,v|f.hotspot=v)}
        <p class="field-help">"A router without a source is a WAN-only core: the cycle does not read it."</p>
      </fieldset>
      <fieldset class="sub"><legend>"Source options"</legend>
        {check_box("ppp_per_plan_node","PPPoE node per plan","Group PPPoE circuits by plan.",|f|f.ppp_per_plan_node,|f,v|f.ppp_per_plan_node=v)}
        {check_box("hotspot_include_mac","Hotspot MAC in circuit","Add the MAC address to each Hotspot circuit.",|f|f.hotspot_include_mac,|f,v|f.hotspot_include_mac=v)}
        {check_box("dhcp_require_bound","DHCP bound leases only","Ignore leases that are not bound.",|f|f.dhcp_require_bound,|f,v|f.dhcp_require_bound=v)}
        {check_box("dhcp_discover","DHCP server discovery","Read all DHCP servers of the router.",|f|f.dhcp_discover,|f,v|f.dhcp_discover=v)}
        {check_box("dhcp_shape_unassigned","Shape unassigned leases","Shape leases without a server in the group \"unassigned\".",|f|f.dhcp_shape_unassigned,|f,v|f.dhcp_shape_unassigned=v)}
      </fieldset>
      <fieldset class="sub"><legend>"Site Map location"</legend>
        <div class="field-row">
          <Field id="latitude" label="Latitude" errors>
            <input id="latitude" type="text" inputmode="decimal" autocomplete="off" class="short" aria-describedby="latitude-error location-error"
              prop:value=move||form.with(|f|f.latitude.clone()) on:input=set_text(form,|f,v|f.latitude=v)/>
          </Field>
          <Field id="longitude" label="Longitude" errors>
            <input id="longitude" type="text" inputmode="decimal" autocomplete="off" class="short" aria-describedby="location-error"
              prop:value=move||form.with(|f|f.longitude.clone()) on:input=set_text(form,|f,v|f.longitude=v)/>
          </Field>
        </div>
        <p class="field-error" id="location-error" aria-live="polite">{move||errors.with(|e|e.iter().find(|(k,_)|*k=="location").map(|(_,m)|m.clone()))}</p>
        <p class="field-help">"Leave both empty for no location."</p>
      </fieldset>
    }
}

const STEPS: [&str; 3] = ["Connection", "Sources", "Review"];

/// Add router: three steps. The router is saved disabled; it gets no connection until an enable.
#[component]
pub(crate) fn NewRouter(
    #[prop(optional)] on_added: Option<Callback<String>>,
    #[prop(optional)] on_closed: Option<Callback<()>>,
    #[prop(default = false)] studio: bool,
    #[prop(default = false)] draft_test: bool,
) -> impl IntoView {
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    Effect::new(move |_| {
        if let Some(d) = dialog.get() {
            let _ = d.show_modal();
        }
    });
    let c = ctx();
    let nav = crate::route::page();
    let form = RwSignal::new(Form::new_router());
    let step = RwSignal::new(0usize);
    let errors = RwSignal::new(Vec::new());
    let busy = RwSignal::new(false);
    let failure = RwSignal::new(String::new());
    let digest = Memo::new(move |_| {
        c.inventory
            .with(|i| i.as_ref().map(|i| i.config_digest.clone()))
            .unwrap_or_default()
    });
    clear_fixed(form, errors, true, move || {
        if step.get_untracked() == 0 {
            Section::Connection
        } else {
            Section::Sources
        }
    });
    let next = move || {
        let section = if step.get_untracked() == 0 {
            Section::Connection
        } else {
            Section::Sources
        };
        let found = form.with_untracked(|f| check(f, true, section));
        let ok = found.is_empty();
        errors.set(found);
        if ok {
            step.update(|s| *s += 1);
        } else {
            request_animation_frame(move || errors.with_untracked(|e| focus_first(e)));
        }
    };
    let submit = move || {
        if busy.get_untracked() {
            return;
        }
        let edit = form.with_untracked(|f| create(f, &digest.get_untracked()));
        busy.set(true);
        failure.set(String::new());
        leptos::task::spawn_local(async move {
            match send(
                c,
                None,
                RouterEditForm {
                    edit,
                    current_password: None,
                },
            )
            .await
            {
                Ok(reply) => {
                    let href = router_href(&reply.router_id, Tab::Overview);
                    let id = reply.router_id.clone();
                    form.update(|f| f.password.clear());
                    follow(
                        c,
                        Saved {
                            router_id: reply.router_id,
                            digest: reply.digest,
                            reload: reply.reload,
                            warnings: reply.warnings,
                            what: "Router added and disabled.",
                            disable: false,
                        },
                    );
                    if let Some(done) = on_added {
                        done.run(id);
                    } else {
                        go_to(nav, &href);
                    }
                }
                Err(f) => failure.set(f.message),
            }
            busy.try_set(false);
        });
    };
    view! {
      <dialog node_ref=dialog class=if studio{"topology-dialog add-router-dialog"}else{"add-router-dialog"} data-panel="studio-add" aria-labelledby="add-router-title" aria-describedby="add-router-help"
        on:cancel=move|ev: web_sys::Event|{if busy.get_untracked(){ev.prevent_default();}}
        on:close=move|_|{
          form.update(|f|f.password.clear());
          if let Some(done)=on_closed{done.run(());}
          else if nav.get_untracked()==crate::route_model::Page::NewRouter {
            go_to(nav,"/routers");
            request_animation_frame(|| {
              if let Some(button)=web_sys::window().and_then(|w|w.document()).and_then(|d|d.get_element_by_id("add-router")) {
                use wasm_bindgen::JsCast;
                if let Ok(button)=button.dyn_into::<web_sys::HtmlElement>() {let _=button.focus();}
              }
            });
          }
        }>
      <header class="add-router-heading">
        <div><h2 id="add-router-title" tabindex="-1" autofocus>"Add router"</h2>
          <p id="add-router-help">"The router is saved disabled. LQoSync makes no connection to it until you enable it."</p></div>
        <button type="button" class="router-icon quiet" aria-label="Close Add router" title="Close" disabled=move||busy.get()
          on:click=move|_|{if let Some(d)=dialog.get(){d.close();}}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18"/></svg>
        </button>
      </header>
      <Show when=move||digest.with(String::is_empty)>
        <div class="notice" role="status">"Router editing needs the updated LQoSync service. Install the new lqosync3 binary, then load this page again."</div>
      </Show>
      <ol class="steps" aria-label="Steps">
        {STEPS.iter().enumerate().map(|(i,label)|view!{
          <li class:done=move||{step.get()>i} aria-current=move||(step.get()==i).then_some("step")><span>{i+1}</span>{*label}</li>
        }).collect_view()}
      </ol>
      <form class="panel router-form" novalidate on:submit=move|ev|{ev.prevent_default(); if step.get_untracked()<2 {next()} else {submit()}}>
        {move||match step.get() {
          0=>view!{<fieldset><legend>"Connection"</legend><ConnectionFields form errors new=true icons=studio/>
            {studio.then(||view!{<DraftConnectionTest form supported=draft_test busy/>})}
          </fieldset>}.into_any(),
          1=>view!{<fieldset><legend>"Sources"</legend><SourceFields form errors/></fieldset>}.into_any(),
          _=>form.with(|f|{
            let roles:Vec<&str>=[(f.pppoe,"PPPoE"),(f.dhcp,"DHCP"),(f.hotspot,"Hotspot")].into_iter().filter(|(on,_)|*on).map(|(_,l)|l).collect();
            view!{<fieldset><legend>"Review"</legend>
              <dl class="router-facts">
                <div><dt>"Router name"</dt><dd>{f.name.clone()}</dd></div>
                <div><dt>"Address"</dt><dd>{format!("{}:{}",f.address,f.port)}</dd></div>
                <div><dt>"Transport"</dt><dd>{if !f.tls{"Plain API".to_string()}else if !f.tls_verify{"TLS, certificate not checked".into()}else if f.ca_pem.is_some(){format!("TLS, checked with {}",f.ca_name)}else{"TLS, checked with public CAs".into()}}</dd></div>
                <div><dt>"RouterOS user"</dt><dd>{f.username.clone()}</dd></div>
                <div><dt>"Credential"</dt><dd>"Entered (not shown)"</dd></div>
                <div><dt>"Sources"</dt><dd>{if roles.is_empty(){"None: a WAN-only core".to_string()}else{roles.join(", ")}}</dd></div>
                <div><dt>"Administrative state"</dt><dd>"Disabled after the save"</dd></div>
              </dl>
              <p class="field-help">"Add router saves the configuration only. It does not sign in to the router, test it, or read from it."</p>
            </fieldset>}.into_any()
          }),
        }}
        <Show when=move||!failure.get().is_empty()><p class="form-failure" role="alert">{move||failure.get()}</p></Show>
        <div class="form-actions">
          <Show when=move||{step.get()>0}><button type="button" class="quiet" class:canvas-tool=studio on:click=move|_|{errors.set(Vec::new());step.update(|s|*s-=1);} aria-label="Back" title="Back">{if studio{crate::topology_canvas::icon("M15 5l-7 7 7 7").into_any()}else{"Back".into_any()}}</button></Show>
          <button type="button" class="quiet" class:canvas-tool=studio disabled=move||busy.get() on:click=move|_|{if let Some(d)=dialog.get(){d.close();}} aria-label="Cancel" title="Cancel">{if studio{crate::topology_canvas::icon("M6 6l12 12M18 6 6 18").into_any()}else{"Cancel".into_any()}}</button>
          <button type="submit" class="primary" class:canvas-tool=studio disabled=move||busy.get()||digest.with(String::is_empty) aria-label=move||if step.get()==2{"Add router"}else{"Next"} title=move||if step.get()==2{"Add router"}else{"Next"}>
            {if studio{view!{<span>{move||crate::topology_canvas::icon(if step.get()==2{"M5 12l4 4L19 6"}else{"M9 5l7 7-7 7"})}</span>}.into_any()}else{view!{<span>{move||match (step.get(),busy.get()) {(2,true)=>"Adding…",(2,false)=>"Add router",_=>"Next"}}</span>}.into_any()}}
          </button>
        </div>
      </form>
      </dialog>
    }
}

/// The password check before a credential replacement (owner decision D2). A native modal dialog
/// keeps focus inside, closes with Escape, and returns focus to the trigger.
#[component]
fn Reauth(
    open: RwSignal<bool>,
    error: RwSignal<String>,
    on_confirm: Callback<String>,
    #[prop(default = false)] studio: bool,
) -> impl IntoView {
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let password = RwSignal::new(String::new());
    Effect::new(move |_| {
        let Some(d) = dialog.get() else { return };
        if open.get() {
            if !d.open() {
                let _ = d.show_modal();
            }
        } else if d.open() {
            d.close();
        }
    });
    view! {
      <dialog node_ref=dialog class=if studio{"reauth topology-dialog"}else{"reauth"} data-panel="settings" aria-labelledby="reauth-title" on:close=move|_|{open.set(false);password.set(String::new());}>
        <form method="dialog" on:submit=move|ev|{ev.prevent_default();on_confirm.run(password.get_untracked());}>
          <h2 id="reauth-title">"Confirm with your password"</h2>
          <p>"Replacing a router credential needs your current LQoSync password."</p>
          <label for="current-password">"Your password"</label>
          <input id="current-password" type="password" autocomplete="current-password" maxlength="256" required
            prop:value=move||password.get() on:input=move|ev|password.set(event_target_value(&ev))/>
          <p class="field-error" role="alert">{move||error.get()}</p>
          <div class="form-actions">
            <button type="button" class="quiet" class:canvas-tool=studio on:click=move|_|open.set(false) aria-label="Cancel" title="Cancel">{if studio{crate::topology_canvas::icon("M6 6l12 12M18 6 6 18").into_any()}else{"Cancel".into_any()}}</button>
            <button type="submit" class="primary" class:canvas-tool=studio aria-label="Confirm and save" title="Confirm and save">{if studio{crate::topology_canvas::icon("M5 12l4 4L19 6").into_any()}else{"Confirm and save".into_any()}}</button>
          </div>
        </form>
      </dialog>
    }
}

/// A visible hint repeats the icon's accessible name. Escape dismisses it without moving focus.
#[component]
fn ActionHint(label: &'static str, children: Children) -> impl IntoView {
    let dismissed = RwSignal::new(false);
    let escape = window_event_listener(leptos::ev::keydown, move |ev| {
        if ev.key() == "Escape" {
            dismissed.set(true);
        }
    });
    on_cleanup(move || escape.remove());
    view! {
      <span class="action-hint" class:hint-dismissed=move||dismissed.get()
        on:mouseenter=move|_|dismissed.set(false)
        on:focusin=move|_|dismissed.set(false)>
        {children()}
        <span class="action-tooltip" aria-hidden="true">{label}</span>
      </span>
    }
}

/// A Connection or Sources section: the facts, and an editor after Edit.
#[component]
pub(crate) fn EditableSection(
    router: RouterSummary,
    section: Section,
    #[prop(default = false)] icons: bool,
) -> impl IntoView {
    let c = ctx();
    let editing = RwSignal::new(false);
    let form = RwSignal::new(Form::from(&router));
    let errors = RwSignal::new(Vec::new());
    let busy = RwSignal::new(false);
    let failure = RwSignal::new(String::new());
    let digest = RwSignal::new(String::new());
    let reauth = RwSignal::new(false);
    let reauth_error = RwSignal::new(String::new());
    let r = StoredValue::new(router.clone());
    clear_fixed(form, errors, false, move || section);
    let save = move |current_password: Option<String>| {
        let edit =
            form.with_untracked(|f| update(f, &r.get_value(), section, &digest.get_untracked()));
        busy.set(true);
        failure.set(String::new());
        let id = r.with_value(|r| r.id.clone());
        leptos::task::spawn_local(async move {
            match send(
                c,
                Some(id.clone()),
                RouterEditForm {
                    edit,
                    current_password,
                },
            )
            .await
            {
                Ok(reply) => {
                    reauth.set(false);
                    form.update(|f| {
                        f.password.clear();
                        f.replace_password = false;
                    });
                    editing.set(false);
                    follow(
                        c,
                        Saved {
                            router_id: id,
                            digest: reply.digest,
                            reload: reply.reload,
                            warnings: reply.warnings,
                            what: "Changes saved.",
                            disable: false,
                        },
                    );
                }
                Err(f) if f.code == "reauth" => {
                    reauth_error.set("The password is not correct, or there were too many tries. Wait one minute, then try again.".into());
                    reauth.set(true);
                }
                Err(f) => {
                    reauth.set(false);
                    failure.set(f.message);
                }
            }
            busy.try_set(false);
        });
    };
    let submit = move || {
        if busy.get_untracked() {
            return;
        }
        let found = form.with_untracked(|f| check(f, false, section));
        let ok = found.is_empty();
        errors.set(found);
        if !ok {
            request_animation_frame(move || errors.with_untracked(|e| focus_first(e)));
            return;
        }
        let edit =
            form.with_untracked(|f| update(f, &r.get_value(), section, &digest.get_untracked()));
        if !changes(&edit) {
            failure.set("There is no change to save.".into());
            return;
        }
        if edit.password.is_some() {
            reauth_error.set(String::new());
            reauth.set(true);
        } else {
            save(None);
        }
    };
    let label = match section {
        Section::Connection => "Edit connection",
        Section::Sources => "Edit sources",
    };
    let edit_button = move || {
        view! {
          <button type="button" class=if section==Section::Connection{"router-icon secondary"}else{"secondary"}
            aria-label=label disabled=move||c.inventory.with(|i|i.as_ref().is_none_or(|i|i.config_digest.is_empty()))
            on:click=move|_|{form.set(Form::from(&r.get_value()));errors.set(Vec::new());failure.set(String::new());digest.set(loaded_digest(c));editing.set(true);}>
            {if section==Section::Connection {
              view!{<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m16 3 5 5-12 12-6 1 1-6ZM14 5l5 5"/></svg>}.into_any()
            }else{label.into_any()}}
          </button>
        }
    };
    view! {
      <Show when=move||editing.get() fallback=move||view!{
        {if section==Section::Connection {
          view!{
            <TestConnection router=r.get_value()><ActionHint label="Edit connection">{edit_button()}</ActionHint></TestConnection>
            {connection_facts(&r.get_value()).into_any()}
          }.into_any()
        }else{
          view!{
            {sources_facts(&r.get_value()).into_any()}
            <div class="form-actions start">{edit_button()}</div>
          }.into_any()
        }}
      }>
        <form class="router-form inline" novalidate on:submit=move|ev|{ev.prevent_default();submit();}>
          {match section {
            Section::Connection=>view!{<ConnectionFields form errors new=false icons/>}.into_any(),
            Section::Sources=>view!{<SourceFields form errors/>}.into_any(),
          }}
          <Show when=move||section==Section::Sources && form.with(|f|removes_a_source(f,&r.get_value()))>
            <p class="router-warn" role="note">"This router is enabled. A source that you turn off loses its circuits after the grace time, through the removal guard."</p>
          </Show>
          <Show when=move||!failure.get().is_empty()><p class="form-failure" role="alert">{move||failure.get()}</p></Show>
          <div class="form-actions start">
            <button type="submit" class="primary" class:canvas-tool=icons aria-label="Save changes" title="Save changes" disabled=move||busy.get()>{if icons{crate::topology_canvas::icon("M5 12l4 4L19 6").into_any()}else{view!{<span>{move||if busy.get(){"Saving…"}else{"Save changes"}}</span>}.into_any()}}</button>
            <button type="button" class="quiet" class:canvas-tool=icons aria-label="Cancel" title="Cancel" on:click=move|_|{form.update(|f|{f.password.clear();f.replace_password=false;});editing.set(false);}>{if icons{crate::topology_canvas::icon("M6 6l12 12M18 6 6 18").into_any()}else{"Cancel".into_any()}}</button>
          </div>
        </form>
        <Reauth open=reauth error=reauth_error on_confirm=Callback::new(move|pw:String|save(Some(pw))) studio=icons/>
      </Show>
    }
}

pub(crate) fn connection_facts(r: &RouterSummary) -> impl IntoView {
    let (link, tone) = transport(r);
    view! {
      <dl class="router-facts">
        <div><dt>"Address"</dt><dd>{r.address.clone()}</dd></div>
        <div><dt>"API port"</dt><dd class="num">{r.port}</dd></div>
        <div><dt>"Transport"</dt><dd><span class=format!("router-transport {tone}")>{link}</span></dd></div>
        {r.tls.then(||view!{<div><dt>"CA certificate"</dt><dd>{if r.tls_ca{"Configured"}else{"Not configured"}}</dd></div>})}
        <div><dt>"Username"</dt><dd>{r.username.clone()}</dd></div>
        <div><dt>"Credential"</dt><dd>{if r.credential{"Configured".into_any()}else{view!{<span class="router-warn">"Missing: the service cannot sign in"</span>}.into_any()}}</dd></div>
      </dl>
    }
}

fn yes(b: bool) -> &'static str {
    if b { "Yes" } else { "No" }
}
fn count(n: Option<u64>) -> String {
    n.map(|n| n.to_string()).unwrap_or("—".into())
}

pub(crate) fn sources_facts(r: &RouterSummary) -> impl IntoView {
    let o = &r.options;
    view! {
      <div class="table-scroll" role="region" aria-label="Subscriber sources" tabindex="0">
        <table class="router-table compact">
          <thead><tr><th scope="col">"Source"</th><th scope="col">"Enabled"</th><th scope="col" class="num">"Records"</th><th scope="col" class="num">"Online"</th><th scope="col" class="num">"Offline"</th></tr></thead>
          <tbody>{r.sources.iter().map(|s|view!{<tr>
            <th scope="row">{role_label(&s.kind)}</th><td>{yes(s.enabled)}</td>
            <td class="num">{count(s.records)}</td><td class="num">{count(s.online)}</td><td class="num">{count(s.offline)}</td>
          </tr>}).collect_view()}</tbody>
        </table>
      </div>
      <dl class="router-facts">
        <div><dt>"PPPoE node per plan"</dt><dd>{yes(o.ppp_per_plan_node)}</dd></div>
        <div><dt>"Hotspot MAC in circuit"</dt><dd>{yes(o.hotspot_include_mac)}</dd></div>
        <div><dt>"DHCP bound leases only"</dt><dd>{yes(o.dhcp_require_bound)}</dd></div>
        <div><dt>"DHCP server discovery"</dt><dd>{yes(o.dhcp_discover)}</dd></div>
        <div><dt>"DHCP server settings"</dt><dd class="num">{o.dhcp_servers}</dd></div>
        <div><dt>"DHCP client overrides"</dt><dd class="num">{o.dhcp_clients}</dd></div>
        <div><dt>"Shape unassigned leases"</dt><dd>{yes(o.dhcp_shape_unassigned)}</dd></div>
        <div><dt>"Router exclusions"</dt><dd class="num">{o.exclusions}</dd></div>
        <div><dt>"Location"</dt><dd>{r.location.map(|[a,b]|format!("{a:.5}, {b:.5}")).unwrap_or("Not set".into())}</dd></div>
      </dl>
    }
}

fn action_label(a: &str) -> &'static str {
    match a {
        "create" => "Added",
        "update" => "Changed",
        "disable" => "Disabled",
        "enable" => "Enabled",
        _ => "Action",
    }
}

/// The audit events of one router, newest first. It loads again after a save.
#[component]
pub(crate) fn ActivityTab(router_id: String) -> impl IntoView {
    let c = ctx();
    let events = RwSignal::new(None::<Result<RouterActivity, String>>);
    let id = router_id.clone();
    Effect::new(move |_| {
        c.saved.track();
        let id = id.clone();
        leptos::task::spawn_local(async move {
            let (signal, _timer) = crate::app::deadline();
            let r = Request::get(&format!("/api/v1/routers/{id}/activity"))
                .abort_signal(Some(&signal))
                .send()
                .await;
            let out = match r {
                Ok(r) if r.ok() => r
                    .json::<RouterActivity>()
                    .await
                    .map_err(|_| "The activity response is not valid.".to_string()),
                Ok(r) if r.status() == 401 => {
                    c.session.set(None);
                    return;
                }
                _ => Err("The activity could not load. Try again later.".to_string()),
            };
            events.try_set(Some(out));
        });
    });
    view! {
      {move||match events.get() {
        None=>view!{<p class="router-empty" role="status">"Loading activity…"</p>}.into_any(),
        Some(Err(e))=>view!{<p class="notice" role="status">{e}</p>}.into_any(),
        Some(Ok(a)) if a.events.is_empty()=>view!{<p class="router-empty">"No web changes to this router yet. Changes from the TUI are not in this list."</p>}.into_any(),
        Some(Ok(a))=>view!{
          <ol class="activity">
            {a.events.into_iter().map(|e|view!{<li>
              <span class="activity-time">{local_time(e.at)}</span>
              <span><b>{action_label(&e.action)}</b>" by "{e.actor}</span>
              <span class="activity-changes">{e.changes.iter().map(|c|c.replace('_'," ")).collect::<Vec<_>>().join(", ")}</span>
            </li>}).collect_view()}
          </ol>
          <p class="field-help">"Web changes only, newest first. Credential changes show as \"credential\"; no value is kept."</p>
        }.into_any(),
      }}
    }
}

/// Disable or enable one router, after a review in a modal dialog. A disable shows its shaping impact
/// first and needs the operator password (owner decision D2). Neither action contacts the router.
#[component]
pub(crate) fn AdminAction(router: RouterSummary) -> impl IntoView {
    let c = ctx();
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let ack = RwSignal::new(false);
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let digest = RwSignal::new(String::new());
    let r = StoredValue::new(router);
    let disable = r.with_value(|r| r.enabled);
    let preview = Memo::new(move |_| {
        c.inventory
            .with(|i| i.as_ref().map(|i| impact(&r.get_value(), &i.shaping)))
    });
    let needs_ack = move || {
        preview.with(|p| {
            p.as_ref()
                .is_some_and(|p| !p.removal_off && p.circuits.unwrap_or(0) > 0)
        })
    };
    let open = move |_| {
        ack.set(false);
        password.set(String::new());
        error.set(String::new());
        digest.set(loaded_digest(c));
        if let Some(d) = dialog.get() {
            let _ = d.show_modal();
        }
    };
    let close = move || {
        if let Some(d) = dialog.get() {
            d.close();
        }
    };
    let confirm = move || {
        if busy.get_untracked() || (disable && needs_ack() && !ack.get_untracked()) {
            return;
        }
        busy.set(true);
        error.set(String::new());
        let id = r.with_value(|r| r.id.clone());
        let form = RouterEditForm {
            edit: web_contract::routers::RouterEdit {
                expected_digest: digest.get_untracked(),
                enabled: Some(!disable),
                ..Default::default()
            },
            current_password: disable.then(|| password.get_untracked()),
        };
        leptos::task::spawn_local(async move {
            match send(c, Some(id.clone()), form).await {
                Ok(reply) => {
                    close();
                    follow(
                        c,
                        Saved {
                            router_id: id,
                            digest: reply.digest,
                            reload: reply.reload,
                            warnings: reply.warnings,
                            what: if disable { "Router disabled." } else { "Router enabled." },
                            disable,
                        },
                    );
                }
                Err(f) if f.code == "reauth" => error.set(
                    "The password is not correct, or there were too many tries. Wait one minute, then try again."
                        .into(),
                ),
                Err(f) => error.set(f.message),
            }
            busy.try_set(false);
        });
    };
    let name = r.with_value(|r| r.name.clone());
    let title = if disable {
        format!("Disable {name}?")
    } else {
        format!("Enable {name}?")
    };
    let credential = r.with_value(|r| r.credential);
    view! {
      <button type="button" class=if disable {"danger"} else {"secondary"} on:click=open
        disabled=move||c.inventory.with(|i|i.as_ref().is_none_or(|i|i.config_digest.is_empty()))>
        {if disable {"Disable router"} else {"Enable router"}}
      </button>
      <dialog node_ref=dialog class="reauth admin" aria-labelledby="admin-title">
        <form method="dialog" on:submit=move|ev|{ev.prevent_default();confirm();}>
          <h2 id="admin-title">{title}</h2>
          {if disable {
            view!{
              <ul class="impact">
                <li>"LQoSync stops every connection to this router: no reads, discovery, tests, or writes."</li>
                <li>{move||preview.get().map(|p| if p.removal_off {
                    "Removal of inactive entries is off (runtime.remove_inactive_entries), so the circuits of this router stay in the shaping.".to_string()
                  } else {
                    match p.circuits {
                      Some(0)=>"This router has no circuits in the LQoSync shaping list.".to_string(),
                      Some(n)=>format!("{n} circuits of this router leave the shaping after the grace time ({} min).",p.grace_minutes),
                      None=>"The circuit count of this router is not known yet. Its circuits leave after the grace time.".to_string(),
                    }
                  }).unwrap_or_default()}</li>
                <Show when=move||preview.with(|p|p.as_ref().is_some_and(|p|p.guard_trips))>
                  <li class="router-warn">"This removal is larger than a removal guard limit, so the guard holds it for one cycle first."</li>
                </Show>
                <li>"The router entry, its credential, and its history stay. You can enable the router again."</li>
              </ul>
              <Show when=needs_ack>
                <div class="check">
                  <input id="admin-ack" type="checkbox" prop:checked=move||ack.get() on:change=move|ev|ack.set(event_target_checked(&ev))/>
                  <label for="admin-ack">{move||format!("I understand that {} circuits leave the shaping.",preview.with(|p|p.as_ref().and_then(|p|p.circuits).unwrap_or(0)))}</label>
                </div>
              </Show>
              <label for="admin-password">"Your password"</label>
              <input id="admin-password" type="password" autocomplete="current-password" maxlength="256" required
                prop:value=move||password.get() on:input=move|ev|password.set(event_target_value(&ev))/>
            }.into_any()
          } else {
            view!{
              <ul class="impact">
                <li>"LQoSync reads the router again from the next cycle."</li>
                <li>"Old circuits do not come back from a cache. The next successful read adds them again."</li>
                {(!credential).then(||view!{<li class="router-warn">"The credential is missing: the reads fail until you add one in Connection."</li>})}
              </ul>
            }.into_any()
          }}
          <p class="field-error" role="alert">{move||error.get()}</p>
          <div class="form-actions">
            <button type="button" class="quiet" on:click=move|_|close()>"Cancel"</button>
            <button type="submit" class=if disable {"danger"} else {"primary"}
              disabled=move||busy.get()||(disable&&needs_ack()&&!ack.get())>
              {move||match (busy.get(),disable) {(true,true)=>"Disabling…",(true,false)=>"Enabling…",(false,true)=>"Disable router",(false,false)=>"Enable router"}}
            </button>
          </div>
        </form>
      </dialog>
    }
}

/// A POST without a body, with the CSRF token and a time limit.
async fn post(c: Ctx, url: &str, limit_ms: u32) -> Result<gloo_net::http::Response, ()> {
    let csrf = c
        .session
        .get_untracked()
        .map(|p| p.csrf)
        .unwrap_or_default();
    let controller = web_sys::AbortController::new().map_err(|_| ())?;
    let signal = controller.signal();
    let _timer = gloo_timers::callback::Timeout::new(limit_ms, move || controller.abort());
    let r = Request::post(url)
        .header("x-csrf-token", &csrf)
        .abort_signal(Some(&signal))
        .send()
        .await
        .map_err(|_| ())?;
    if r.status() == 401 {
        c.session.set(None);
        return Err(());
    }
    Ok(r)
}

/// Test connection: the service connects, signs in, and reads the identity of this router only.
/// A disabled router has no test: LQoSync makes no connection to it.
#[component]
fn TestConnection(router: RouterSummary, children: Children) -> impl IntoView {
    let c = ctx();
    let result = RwSignal::new(None::<(String, &'static str, String)>);
    let busy = RwSignal::new(false);
    let r = StoredValue::new(router);
    let enabled = r.with_value(|r| r.enabled);
    let run = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        let id = r.with_value(|r| r.id.clone());
        leptos::task::spawn_local(async move {
            let out = match post(c, &format!("/api/v1/routers/{id}/test"), 75_000).await {
                Ok(resp) if resp.ok() => {
                    match resp.json::<web_contract::routers::RouterTestResult>().await {
                        Ok(t) => {
                            let (text, tone) =
                                crate::routers_model::test_message(&t, &r.get_value());
                            (
                                text,
                                tone,
                                format!("Tested at {} in {} ms.", local_time(t.at), t.elapsed_ms),
                            )
                        }
                        Err(_) => ("The test answer is not valid.".into(), "bad", String::new()),
                    }
                }
                _ => (
                    "The test request did not complete. Try again.".into(),
                    "bad",
                    String::new(),
                ),
            };
            result.try_set(Some(out));
            busy.try_set(false);
        });
    };
    view! {
      <div class="test-connection">
        <div class="connection-actions" role="group" aria-label="Connection actions">
          <ActionHint label="Test connection">
            <button type="button" class="router-icon secondary" aria-label="Test connection" aria-describedby="connection-test-help"
              aria-busy=move||busy.get().to_string() disabled=move||busy.get()||!enabled on:click=run>
              <svg class:connection-testing=move||busy.get() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path d="M9 3v5M15 3v5M7 8h10v3a5 5 0 0 1-10 0ZM12 16v5"/>
              </svg>
            </button>
          </ActionHint>
          {children()}
        </div>
        <p class="field-help" id="connection-test-help">{move||if busy.get(){"Testing connection…"}else if enabled {"Connects, signs in, and reads the router identity. It changes nothing."} else {"Enable the router to test it: LQoSync makes no connection to a disabled router."}}</p>
        <div aria-live="polite">
          {move||result.get().map(|(text,tone,when)|view!{
            <div class=format!("test-result {tone}")><p><b>{text}</b></p><p class="field-help">{when}</p></div>
          })}
        </div>
      </div>
    }
}

/// Sync now: a cycle starts now for all enabled routers. The table updates after the cycle.
#[component]
pub(crate) fn SyncNow(message: RwSignal<Option<(&'static str, &'static str)>>) -> impl IntoView {
    let c = ctx();
    let busy = RwSignal::new(false);
    let run = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        leptos::task::spawn_local(async move {
            let ok = matches!(post(c, "/api/v1/routers/sync", 30_000).await, Ok(r) if r.ok());
            message.try_set(Some(if ok {
                (
                    "Sync started for all enabled routers. The table updates after the cycle.",
                    "good",
                )
            } else {
                (
                    "Sync now did not reach the service. Check that it runs, then try again.",
                    "bad",
                )
            }));
            busy.try_set(false);
        });
    };
    view! {
      <button type="button" class="router-icon secondary" aria-label=move||if busy.get(){"Starting sync"}else{"Sync now"}
        title="Sync now" aria-busy=move||busy.get().to_string() disabled=move||busy.get() on:click=run>
        <svg class:sync-pending=move||busy.get() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M20 7v5h-5M4 17v-5h5M6.1 7a7 7 0 0 1 11.5-1L20 9M4 15l2.4 3A7 7 0 0 0 17.9 17"/>
        </svg>
      </button>
    }
}

/// A one-shot test reuses the connection fields, their checks, and the operation transport.
#[component]
fn DraftConnectionTest(
    form: RwSignal<Form>,
    supported: bool,
    busy: RwSignal<bool>,
) -> impl IntoView {
    use web_contract::source_studio::*;
    let c = ctx();
    let message = RwSignal::new(String::new());
    let report = RwSignal::new(None::<(String, DraftTestReport)>);
    let epoch = RwSignal::new(0u64);
    let alive = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let end = alive.clone();
    on_cleanup(move || end.store(false, std::sync::atomic::Ordering::Relaxed));
    let signature = move || {
        form.with(|f| {
            format!(
                "{:?}",
                (
                    &f.address,
                    &f.port,
                    f.tls,
                    f.tls_verify,
                    &f.username,
                    &f.ca_pem
                )
            )
        })
    };
    let previous = StoredValue::new(signature());
    Effect::new(move |_| {
        form.track();
        let next = signature();
        let changed = previous.get_value() != next;
        previous.set_value(next);
        epoch.update(|n| *n = n.wrapping_add(1));
        if changed
            || form.with(|f| !f.password.is_empty())
            || report.with_untracked(|r| r.as_ref().is_some_and(|(s, _)| *s != signature()))
        {
            report.set(None);
            message.set(String::new());
        }
    });
    let test = move || {
        if busy.get_untracked() || !supported {
            return;
        }
        let mut values = form.get_untracked();
        values.name = "DraftTest".into();
        let errors = check(&values, true, Section::Connection);
        if !errors.is_empty() {
            message.set(
                errors
                    .iter()
                    .map(|(_, m)| m.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            return;
        }
        let sources = [
            (values.pppoe, SourceKind::Ppp),
            (values.dhcp, SourceKind::Dhcp),
            (values.hotspot, SourceKind::Hotspot),
        ]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, s)| s)
        .collect();
        let connection = DraftConnection {
            address: values.address,
            port: values.port.parse().unwrap(),
            tls: values.tls,
            tls_verify: values.tls_verify,
            username: values.username,
            password: SecretInput(values.password),
            ca_pem: values.ca_pem,
        };
        let at = epoch.get_untracked();
        let sig = signature();
        let actor = c.session.get_untracked().map(|p| p.username);
        busy.set(true);
        message.set(String::new());
        report.set(None);
        let alive = alive.clone();
        leptos::task::spawn_local(async move {
            let key = crate::topology_forms::new_key();
            let request = StudioJobRequest {
                key: key.clone(),
                job: StudioJob::DraftTest {
                    connection,
                    required_sources: sources,
                },
            };
            let result =
                crate::topology_studio_api::job(c.session, &request, JOB_DEADLINE_SECS, false)
                    .await;
            drop(request);
            if !alive.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            let current = at == epoch.get_untracked()
                && actor == c.session.get_untracked().map(|p| p.username);
            form.update(|f| f.password.clear());
            busy.set(false);
            if !current {
                message.set("The connection changed. Test the current draft again.".into());
                return;
            }
            match result {
                Ok(op) if op.state == "done" => {
                    match op.source_result {
                        Some(StudioJobResult::DraftTest(r)) => {
                            report.set(Some((sig, r)));
                            message.set("Draft test completed. Enter the credential again before registration.".into());
                        }
                        _ => message
                            .set("The draft test answer is not valid. Retry explicitly.".into()),
                    }
                }
                _ => message.set(
                    "The draft test failed or its result is unknown. Retry explicitly.".into(),
                ),
            }
        });
    };
    view! {<div class="studio-draft-test">
        <button type="button" class="router-icon secondary" aria-label="Test draft connection" title="Test draft connection" disabled=move||busy.get()||!supported on:click=move|_|test()>{crate::topology_canvas::icon("M5 3v5h14V3M5 16v5M19 16v5M3 8h18v8H3M12 11v2")}</button>
        <p class="field-help">{if supported{"Explicit one-shot test. It saves no router and enables no periodic collection."}else{"Draft testing is unavailable on this backend."}}</p>
        <p role="status">{move||if busy.get(){"Testing draft connection…".into()}else{message.get()}}</p>
        {move||report.get().map(|(_,r)|view!{<ul>{r.checks.into_iter().map(|check|view!{<li>{format!("{}: {} ({})",crate::topology_studio_model::key(&check.stage),crate::topology_studio_model::key(&check.state),check.code)}</li>}).collect_view()}</ul>})}
    </div>}
}
