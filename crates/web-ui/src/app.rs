use crate::dashboard::{DashboardBody, Navigation, local_time};
use gloo_net::http::Request;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_contract::{Overview, Principal};
// Drop the timer when a request completes. Abort a stalled browser request after 15 seconds.
pub(crate) fn deadline() -> (web_sys::AbortSignal, gloo_timers::callback::Timeout) {
    let controller = web_sys::AbortController::new().expect("browser AbortController");
    let signal = controller.signal();
    let timer = gloo_timers::callback::Timeout::new(15_000, move || controller.abort());
    (signal, timer)
}
#[component]
fn Brand(#[prop(default = false)] compact: bool) -> impl IntoView {
    view! {<div class:compact=compact class="brand"><svg viewBox="0 0 80 80" aria-hidden="true"><path class="brand-blue" d="M6 23 40 7 74 23 40 39Z"/><path d="m6 39 34 16 34-16M6 55l34 16 34-16"/></svg><span>"LQoSync"<sup>"v3"</sup></span></div>}
}
#[component]
fn Theme() -> impl IntoView {
    let initial = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
        .and_then(|e| e.get_attribute("data-theme"))
        .unwrap_or("system".into());
    view! {<label class="theme"><span>"Theme"</span><select aria-label="Color theme" prop:value=initial on:change=move|ev|{
    let value=event_target_value(&ev);if !matches!(value.as_str(),"light"|"dark"|"system"){return;}
    if let Some(document)=web_sys::window().and_then(|w|w.document()){
    if let Some(root)=document.document_element(){let _=root.set_attribute("data-theme",&value);}
    let age=if value=="system"{0}else{31536000};let cookie=format!("lqosync_theme={value}; Path=/; SameSite=Strict; Max-Age={age}{}",if web_sys::window().unwrap().location().protocol().unwrap_or_default()=="https:"{"; Secure"}else{""});
    if let Some(html)=document.dyn_ref::<web_sys::HtmlDocument>(){let _=html.set_cookie(&cookie);}
    }
    }><option value="light">"Light"</option><option value="dark">"Dark"</option><option value="system">"System"</option></select></label>}
}
#[component]
fn NetworkArt() -> impl IntoView {
    view! {<svg class="network-art" viewBox="0 0 500 360" aria-hidden="true"><g fill="none" stroke="currentColor" stroke-width="2.5"><path d="M250 65v45M90 160v-50h320v50M250 110v50M90 200v40H35v50m55-50h55v50M250 200v90M410 200v40h-55v50m55-50h55v50"/>
    <rect x="205" y="25" width="90" height="40" rx="7"/><rect x="45" y="160" width="90" height="40" rx="7"/><rect x="205" y="160" width="90" height="40" rx="7"/><rect x="365" y="160" width="90" height="40" rx="7"/>
    <rect x="15" y="290" width="40" height="30" rx="3"/><rect x="125" y="290" width="40" height="30" rx="3"/><rect x="210" y="290" width="80" height="40" rx="7"/><rect x="335" y="290" width="40" height="30" rx="3"/><rect x="445" y="290" width="40" height="30" rx="3"/>
    <path d="M25 330h20m-10-10v10m100 0h20m-10-10v10m200 10h20m-10-10v10m100 10h20m-10-10v10M225 277q25-30 50 0m-42 5q17-20 34 0"/>
    </g><g fill="currentColor">{[225,240,255,270].into_iter().map(|x|view!{<rect x=x y="40" width="7" height="6" rx="2"/>}).collect_view()}{[65,85,105,225,245,265,385,405,425].into_iter().map(|x|view!{<rect x=x y="177" width="9" height="6" rx="2"/>}).collect_view()}</g></svg>}
}
#[component]
fn SignIn(session: RwSignal<Option<Principal>>) -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let show = RwSignal::new(false);
    let pending = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    view! {<div class="signin-page"><header class="signin-top"><Theme/></header><main class="signin-layout" id="main"><section class="identity"><Brand/><p>"Network configuration and orchestration"</p><NetworkArt/></section><section class="signin-panel glass"><Brand compact=true/><h1>"Sign in"</h1><p class="intro">"Use your administrator account."</p><form on:submit=move|ev|{ev.prevent_default();if pending.get_untracked(){return;}pending.set(true);error.set(String::new());let body=serde_json::json!({"username":username.get_untracked(),"password":password.get_untracked()});leptos::task::spawn_local(async move{
    let (signal,_timer)=deadline();let response=Request::post("/api/v1/session").abort_signal(Some(&signal)).json(&body).expect("serializable login").send().await;
    match response {Ok(r)if r.ok()=>match r.json::<Principal>().await{Ok(p)=>{password.set(String::new());session.set(Some(p));},Err(_)=>error.set("The sign-in response is invalid. Try again.".into())},Ok(r)if r.status()==401=>error.set("Sign-in details are incorrect.".into()),_=>error.set("Sign-in service unavailable. Try again shortly.".into())};pending.set(false);
    });}><label for="username">"Username"</label><input id="username" name="username" type="text" autocomplete="username" spellcheck="false" autocapitalize="none" placeholder="Enter your username" maxlength="64" required prop:value=move||username.get() on:input=move|ev|username.set(event_target_value(&ev))/><label for="password">"Password"</label><div class="password-field"><input id="password" name="password" type=move||if show.get(){"text"}else{"password"} autocomplete="current-password" placeholder="Enter your password" maxlength="256" required prop:value=move||password.get() on:input=move|ev|password.set(event_target_value(&ev))/><button type="button" aria-label=move||if show.get(){"Hide password"}else{"Show password"} on:click=move|_|show.update(|v|*v = !*v)>{move||if show.get(){"Hide"}else{"Show"}}</button></div><p class="form-error" role="alert">{move||error.get()}</p><button type="submit" class="primary signin-button" disabled=move||pending.get()>{move||if pending.get(){"Signing in…"}else{"Sign in"}}</button></form><p class="access-note">"Contact your system administrator for account access."</p></section></main><footer>"LQoSync · Web control panel"</footer></div>}
}
#[component]
fn Dashboard(session: RwSignal<Option<Principal>>) -> impl IntoView {
    let page = crate::route::provide();
    // Kept here, so Back from a router detail restores the inventory filters.
    let filter = RwSignal::new(crate::routers_model::Filter::default());
    let section = Memo::new(move |_| match page.get() {
        crate::route_model::Page::Dashboard => 0,
        crate::route_model::Page::Routers
        | crate::route_model::Page::NewRouter
        | crate::route_model::Page::Router { .. } => 1,
        crate::route_model::Page::Unknown => 2,
    });
    let data = RwSignal::new(None::<Overview>);
    let error = RwSignal::new(String::new());
    let alive = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let received = crate::live::attach(session, data, error);
    let stop = alive.clone();
    on_cleanup(move || stop.store(false, std::sync::atomic::Ordering::Relaxed));
    leptos::task::spawn_local(async move {
        while alive.load(std::sync::atomic::Ordering::Relaxed) {
            if !web_sys::window().unwrap().document().unwrap().hidden()
                && js_sys::Date::now() - received.get_untracked() > 2500.0
            {
                let expected = session.get_untracked().map(|p| p.csrf);
                let (signal, _timer) = deadline();
                let response = Request::get("/api/v1/overview")
                    .abort_signal(Some(&signal))
                    .send()
                    .await;
                if !alive.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                if expected == session.get_untracked().map(|p| p.csrf) {
                    match response {
                        Ok(r) if r.ok() => match r.json::<Overview>().await {
                            Ok(v) => {
                                data.set(Some(v));
                                error.set(String::new());
                            }
                            Err(_) => error.set(
                                "Invalid monitoring response. Last known data remains below."
                                    .into(),
                            ),
                        },
                        Ok(r) if r.status() == 401 => session.set(None),
                        _ => error.set(
                            "Monitoring service unavailable. Last known data remains below.".into(),
                        ),
                    }
                    if !error.get_untracked().is_empty() {
                        crate::live::mark_stale(data);
                    }
                }
            }
            gloo_timers::future::TimeoutFuture::new(5000).await;
        }
    });
    let signing_out = RwSignal::new(false);
    view! {
      <div class="app-shell">
        <aside class="sidebar">
          <Brand compact=true/>
          <Navigation/>
          <div class="sidebar-bottom">
            <span class="service-dot" class:current=move||data.with(|d|d.as_ref().is_some_and(|v|!v.stale))></span>
            {move||data.with(|d|match d {Some(v) if !v.stale=>"Service evidence current",Some(_)=>"Service evidence stale",None=>"Waiting for service evidence"})}
            <small>"LQoSync3 synchronization service"</small>
          </div>
        </aside>
        <div class="workspace">
          <header class="main-top">
            <span class="top-context">"Network operations"</span>
            <div class="top-actions">
              <span class="header-time" title="Time in your browser's local timezone">{move||{data.with(|_|format!("{} Local",local_time(js_now())))}}</span>
              <span class="header-update">{move||data.with(|d|d.as_ref().and_then(|v|v.observed_at).map(|t|format!("Updated {}s ago",js_now().saturating_sub(t))).unwrap_or("Waiting for update".into()))}</span>
              <Theme/>
              <span class="username">{move||session.get().map(|p|p.username).unwrap_or_default()}</span>
              <button class="quiet" disabled=move||signing_out.get() on:click=move|_|{
                if signing_out.get_untracked(){return;}
                signing_out.set(true);
                let csrf=session.get_untracked().map(|p|p.csrf).unwrap_or_default();
                leptos::task::spawn_local(async move{
                  let (signal,_timer)=deadline();
                  match Request::delete("/api/v1/session").abort_signal(Some(&signal)).header("x-csrf-token",&csrf).send().await{
                    Ok(r)if r.ok()||r.status()==401=>session.set(None),
                    _=>error.set("Sign out could not complete. Try again.".into())
                  };
                  signing_out.set(false);
                });
              }>"Sign out"</button>
            </div>
          </header>
          <main class="dashboard" id="main">
            <section class="service-diagnostics status-banner" aria-label="Service diagnostics">
              <crate::diagnostics_ticker::DiagnosticsTicker data/>
            </section>
            {move||match section.get() {
              0=>view!{
                <div class="page-heading">
                  <div><h1 tabindex="-1">"Dashboard"</h1><p>"Network, subscriber, and traffic overview."</p></div>
                  <div class="freshness">{move||data.with(|d|match d {Some(v) if v.stale=>"Stale or partial data",Some(_)=>"Current data",None=>"Loading monitoring data…"})}</div>
                </div>
                <Show when=move||!error.get().is_empty()><div class="notice" role="status">{move||error.get()}</div></Show>
                <Show when=move||data.with(Option::is_some) fallback=||view!{<div class="panel loading" role="status">"Loading service evidence…"</div>}>
                  <DashboardBody data/>
                </Show>
              }.into_any(),
              1=>view!{<crate::routers::RoutersPage data session filter/>}.into_any(),
              _=>view!{
                <div class="page-heading"><div><h1 tabindex="-1">"Page not found"</h1><p>"This address is not a page of LQoSync."</p></div></div>
                <p><crate::route::Link href="/".to_string()>"Go to the Dashboard"</crate::route::Link></p>
              }.into_any(),
            }}
          </main>
        </div>
      </div>
    }
}
fn js_now() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}
#[component]
pub fn App() -> impl IntoView {
    let session = RwSignal::new(None::<Principal>);
    let loading = RwSignal::new(true);
    leptos::task::spawn_local(async move {
        let (signal, _timer) = deadline();
        if let Ok(r) = Request::get("/api/v1/session")
            .abort_signal(Some(&signal))
            .send()
            .await
            && r.ok()
            && let Ok(p) = r.json::<Principal>().await
        {
            session.set(Some(p));
        }
        loading.set(false);
    });
    view! {<a class="skip" href="#main">"Skip to content"</a><Show when=move||!loading.get() fallback=||view!{<main id="main" class="boot" role="status">"Loading LQoSync…"</main>}><Show when=move||session.get().is_some() fallback=move||view!{<SignIn session/>}><Dashboard session/></Show></Show>}
}
