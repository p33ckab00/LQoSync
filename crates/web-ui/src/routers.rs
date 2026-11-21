//! The Routers screen: the cached inventory, the router detail, and local edits.
//! The page reads `/api/v1/routers` (a service cache). It never connects to a router.
use crate::dashboard::local_time;
use crate::route::{Link, page};
use crate::route_model::{Page, Tab, router_href};
use crate::router_form::{
    ActivityTab, AdminAction, Ctx, EditableSection, NewRouter, SavedNotice, SyncNow,
};
use crate::router_form_model::Section;
use crate::routers_model::{
    Filter, matches, pinning_prerequisites, read_state, role_label, state, transport,
};
use gloo_net::http::Request;
use leptos::prelude::*;
use web_contract::routers::{RouterInventory, RouterSummary};
use web_contract::{Overview, Principal};

fn count(n: Option<u64>) -> String {
    n.map(|n| n.to_string()).unwrap_or("—".into())
}

#[component]
pub(crate) fn RoutersPage(
    data: RwSignal<Option<Overview>>,
    session: RwSignal<Option<Principal>>,
    filter: RwSignal<Filter>,
) -> impl IntoView {
    let inventory = RwSignal::new(None::<RouterInventory>);
    let refresh = RwSignal::new(0u32);
    provide_context(Ctx {
        inventory,
        refresh,
        saved: RwSignal::new(None),
        session,
    });
    let loaded = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    // Load on entry and again after each service cycle; the overview time changes once per cycle.
    let cycle = Memo::new(move |_| data.with(|d| d.as_ref().and_then(|v| v.observed_at)));
    Effect::new(move |_| {
        cycle.track();
        refresh.track();
        let expected = session.get_untracked().map(|p| p.csrf);
        leptos::task::spawn_local(async move {
            let (signal, _timer) = crate::app::deadline();
            let response = Request::get("/api/v1/routers")
                .abort_signal(Some(&signal))
                .send()
                .await;
            if expected != session.get_untracked().map(|p| p.csrf) {
                return;
            }
            let failure = match response {
                Ok(r) if r.ok() => match r.json::<RouterInventory>().await {
                    Ok(v) => {
                        inventory.try_set(Some(v));
                        None
                    }
                    Err(_) => Some("The router inventory response is not valid."),
                },
                Ok(r) if r.status() == 401 => {
                    session.set(None);
                    return;
                }
                Ok(r) if r.status() == 503 => Some(
                    "The router inventory is not available yet. The service writes it at the end of each cycle.",
                ),
                _ => Some(
                    "The router inventory could not load. The page tries again after the next cycle.",
                ),
            };
            let kept = inventory.with_untracked(Option::is_some);
            error.try_set(
                failure
                    .map(|f| {
                        if kept {
                            format!("{f} The last loaded inventory remains below.")
                        } else {
                            f.to_string()
                        }
                    })
                    .unwrap_or_default(),
            );
            loaded.try_set(true);
        });
    });
    let route = Memo::new(move |_| page().get());
    view! {
      <Show when=move||matches!(route.get(),Page::Router{..}) fallback=move||view!{<Inventory inventory loaded error filter data/>}>
        {move||match route.get() {
          Page::Router{id,tab}=>view!{<Detail id tab inventory loaded error/>}.into_any(),
          _=>().into_any(),
        }}
      </Show>
      <Show when=move||route.get()==Page::NewRouter><NewRouter/></Show>
    }
}

#[component]
fn Inventory(
    inventory: RwSignal<Option<RouterInventory>>,
    loaded: RwSignal<bool>,
    error: RwSignal<String>,
    filter: RwSignal<Filter>,
    data: RwSignal<Option<Overview>>,
) -> impl IntoView {
    let sync_message = RwSignal::new(None::<(&'static str, &'static str)>);
    let rows = Memo::new(move |_| {
        let f = filter.get();
        inventory.with(|i| {
            i.as_ref()
                .map(|i| {
                    i.routers
                        .iter()
                        .filter(|r| matches(r, &f))
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    });
    let total = Memo::new(move |_| inventory.with(|i| i.as_ref().map_or(0, |i| i.routers.len())));
    let set = move |f: fn(&mut Filter, String), ev: leptos::ev::Event| {
        let v = event_target_value(&ev);
        filter.update(|x| f(x, v));
    };
    view! {
      <div class="page-heading router-heading">
        <div><h1 tabindex="-1">"Routers"</h1><p>"MikroTik routers in the configuration, their roles, and the latest read."</p></div>
        <div class="router-actions">
          <SyncNow message=sync_message/>
          <button id="add-router" type="button" class="router-icon primary" aria-label="Add router" title="Add router" aria-haspopup="dialog"
            on:click=move|_|crate::route::go("/routers/new")>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>
          </button>
        </div>
        <div class="freshness">{move||inventory.with(|i|match i {
          Some(i) if data.with(|d|d.as_ref().is_some_and(|v|v.stale))=>format!("Service evidence stale · inventory from {}",local_time(i.updated)),
          Some(i)=>format!("Inventory from {}",local_time(i.updated)),
          None=>"Loading router inventory…".into(),
        })}</div>
      </div>
      <div class="sync-message" role="status">{move||sync_message.get().map(|(text,tone)|view!{<p class=format!("router-state {tone}")>{text}</p>})}</div>
      <Show when=move||!error.get().is_empty()><div class="notice" role="status">{move||error.get()}</div></Show>
      <section class="panel routers-panel" aria-labelledby="routers-title">
        <h2 id="routers-title" class="sr-only">"Router inventory"</h2>
        <div class="router-filters" role="search" aria-label="Filter routers">
          <label class="router-search">"Search"
            <input type="search" placeholder="Name or address" autocomplete="off" spellcheck="false" maxlength="64"
              prop:value=move||filter.with(|f|f.text.clone()) on:input=move|ev|set(|f,v|f.text=v,ev)/>
          </label>
          <label>"Role"
            <select prop:value=move||filter.with(|f|f.role.clone()) on:change=move|ev|set(|f,v|f.role=v,ev)>
              <option value="">"All roles"</option><option value="wan">"WAN"</option><option value="ppp">"PPPoE"</option>
              <option value="dhcp">"DHCP"</option><option value="hotspot">"Hotspot"</option>
            </select>
          </label>
          <label>"Read state"
            <select prop:value=move||filter.with(|f|f.state.clone()) on:change=move|ev|set(|f,v|f.state=v,ev)>
              <option value="">"All states"</option><option value="online">"Online"</option>
              <option value="problem">"Needs attention"</option><option value="not_read">"Not read"</option>
              <option value="disabled">"Disabled"</option>
            </select>
          </label>
          <span class="router-count" aria-live="polite">{move||format!("{} of {} routers",rows.with(Vec::len),total.get())}</span>
        </div>
        <Show when=move||loaded.get()||inventory.with(Option::is_some) fallback=||view!{<p class="router-empty" role="status">"Loading router inventory…"</p>}>
          <Show when=move||{total.get()>0} fallback=move||view!{
            <p class="router-empty">{move||if inventory.with(Option::is_some){"No routers are in the configuration. Select Add router to register the first one."}else{"No router inventory yet."}}</p>
          }>
            <Show when=move||rows.with(|r|!r.is_empty()) fallback=move||view!{
              <div class="router-empty"><p>"No router matches these filters."</p>
                <button type="button" class="quiet" on:click=move|_|filter.set(Filter::default())>"Clear filters"</button></div>
            }>
              <div class="table-scroll" role="region" aria-label="Router table" tabindex="0">
                <table class="router-table">
                  <thead><tr><th scope="col">"Router"</th><th scope="col">"Roles"</th><th scope="col">"Read state"</th><th scope="col" class="num">"Subscribers"</th><th scope="col">"Connection"</th></tr></thead>
                  <tbody>
                    <For each=move||rows.get() key=|r:&RouterSummary|r.id.clone() children=move|r:RouterSummary|{
                      let (state,tone)=state(&r);
                      let (link,link_tone)=transport(&r);
                      view!{<tr>
                        <th scope="row"><Link href=router_href(&r.id,Tab::Overview) class="router-name">{r.name.clone()}</Link>
                          <small>{format!("{}:{}",r.address,r.port)}</small></th>
                        <td><Roles roles=r.roles.clone()/></td>
                        <td><span class=format!("router-state {tone}")><i aria-hidden="true"></i>{state}</span>
                          <small>{if !r.enabled {"No reads or writes".into()} else {r.collection.as_ref().map(|c|if c.observed_at>0{format!("Last read {}",local_time(c.observed_at))}else{"No successful read yet".into()}).unwrap_or("No subscriber role".into())}}</small></td>
                        <td class="num">{count(r.collection.as_ref().map(|c|c.subscribers))}</td>
                        <td><span class=format!("router-transport {link_tone}")>{link}</span>
                          {(!r.credential).then(||view!{<small class="router-warn">"Credential missing"</small>})}</td>
                      </tr>}
                    }/>
                  </tbody>
                </table>
              </div>
            </Show>
          </Show>
        </Show>
        <Show when=move||inventory.with(|i|i.as_ref().is_some_and(|i|i.truncated))>
          <p class="notice">"Some routers are not shown, to keep the response bounded. Use the TUI for the full list."</p>
        </Show>
        <p class="panel-note">"This page reads the service cache. Opening it does not connect to any router."</p>
      </section>
    }
}

#[component]
fn Roles(roles: Vec<String>) -> impl IntoView {
    view! {
      <span class="router-roles">{if roles.is_empty(){
        view!{<span class="router-role none">"No role"</span>}.into_any()
      }else{
        roles.into_iter().map(|r|view!{<span class=format!("router-role {r}")>{role_label(&r)}</span>}).collect_view().into_any()
      }}</span>
    }
}

#[component]
fn Detail(
    id: String,
    tab: Tab,
    inventory: RwSignal<Option<RouterInventory>>,
    loaded: RwSignal<bool>,
    error: RwSignal<String>,
) -> impl IntoView {
    let key = id.clone();
    let key2 = id.clone();
    let saved = expect_context::<Ctx>().saved;
    let router = Memo::new(move |_| {
        inventory.with(|i| {
            i.as_ref()
                .and_then(|i| i.routers.iter().find(|r| r.id == key).cloned())
        })
    });
    view! {
      <nav class="breadcrumb" aria-label="Breadcrumb"><Link href="/routers".to_string()>"Routers"</Link><span aria-hidden="true">"/"</span>
        <span aria-current="page">{move||router.with(|r|r.as_ref().map(|r|r.name.clone()).unwrap_or("Router".into()))}</span></nav>
      <Show when=move||!error.get().is_empty()><div class="notice" role="status">{move||error.get()}</div></Show>
      {move||match router.get() {
        None if saved.with(|s|s.as_ref().is_some_and(|s|s.router_id==key2))=>view!{
          <div class="page-heading"><div><h1 tabindex="-1">"New router"</h1><p role="status">"Saved. It appears here when the service loads the configuration."</p></div></div>
          <SavedNotice router_id=key2.clone()/>
        }.into_any(),
        None if !loaded.get()=>view!{<div class="page-heading"><div><h1 tabindex="-1">"Router"</h1><p role="status">"Loading router…"</p></div></div>}.into_any(),
        None=>view!{<div class="page-heading"><div><h1 tabindex="-1">"Router not found"</h1>
          <p>"This router is not in the configuration. It may have a new name."</p></div></div>
          <p><Link href="/routers".to_string()>"Back to all routers"</Link></p>}.into_any(),
        Some(r)=>{
          let (state,tone)=state(&r);
          let id=r.id.clone();
          view!{
            <div class="page-heading">
              <div><h1 tabindex="-1">{r.name.clone()}</h1><p>{format!("{}:{}",r.address,r.port)}</p></div>
              <div class="router-actions">
                <span class=format!("router-state big {tone}")><i aria-hidden="true"></i>{state}</span>
                <AdminAction router=r.clone()/>
              </div>
            </div>
            <SavedNotice router_id=id.clone()/>
            <nav class="router-tabs" aria-label="Router sections">
              {Tab::ALL.into_iter().map(|t|view!{<Link href=router_href(&id,t) current=Signal::derive(move||t==tab)>{t.label()}</Link>}).collect_view()}
            </nav>
            <section class="panel router-section" aria-label=tab.label()>
              {match tab {
                Tab::Overview=>overview(&r).into_any(),
                Tab::Connection=>view!{<EditableSection router=r.clone() section=Section::Connection/>}.into_any(),
                Tab::Sources=>view!{<EditableSection router=r.clone() section=Section::Sources/>}.into_any(),
                Tab::Pinning=>pinning(&r).into_any(),
                Tab::Activity=>view!{<ActivityTab router_id=r.id.clone()/>}.into_any(),
              }}
              <p class="panel-note">"A save changes the configuration only. It does not connect to the router."</p>
            </section>
          }.into_any()
        }
      }}
    }
}

fn overview(r: &RouterSummary) -> impl IntoView {
    let read = match &r.collection {
        None if !r.enabled => "Not read: the router is disabled.".to_string(),
        None => "The cycle does not read this router: it has no subscriber role.".to_string(),
        Some(c) if c.observed_at > 0 => format!(
            "{}, last successful read {}",
            read_state(Some(c)).0,
            local_time(c.observed_at)
        ),
        Some(c) => format!("{}, no successful read yet", read_state(Some(c)).0),
    };
    view! {
      <dl class="router-facts">
        <div><dt>"Administrative state"</dt><dd>{if r.enabled{"Enabled"}else{"Disabled: LQoSync makes no connection to this router"}}</dd></div>
        <div><dt>"Roles"</dt><dd><Roles roles=r.roles.clone()/></dd></div>
        <div><dt>"Read state"</dt><dd>{read}</dd></div>
        <div><dt>"Subscribers"</dt><dd class="num">{count(r.collection.as_ref().map(|c|c.subscribers))}</dd></div>
        <div><dt>"WAN links"</dt><dd>{if r.wans.is_empty(){"None in the topology catalog".to_string()}else{r.wans.join(", ")}}</dd></div>
        <div><dt>"Location"</dt><dd>{r.location.map(|[a,b]|format!("{a:.5}, {b:.5}")).unwrap_or("Not set".into())}</dd></div>
        <div><dt>"Router ID"</dt><dd><code>{r.id.clone()}</code></dd></div>
      </dl>
    }
}

/// The future WAN pinning surface: the prerequisites and the planned form, as text. There is no
/// input and no action, so nothing here can look operational (review P6, owner decision D3).
fn pinning(r: &RouterSummary) -> impl IntoView {
    let outline = [
        (
            "Target",
            "This core router, one of its WAN links, and the routing rule that sends marked traffic to that WAN.",
        ),
        (
            "Inputs",
            "Subscriber IPs as a source match, application IPs from DPI as a destination match, or both. With both, both must match.",
        ),
        (
            "Address lists",
            "Lists that LQoSync owns and names. LQoSync never changes other lists or rules.",
        ),
        (
            "Review",
            "Additions, removals, conflicts, stale evidence, and the matched traffic, before any write.",
        ),
        (
            "Result",
            "The write receipt, a list readback, and a separate routing check.",
        ),
    ];
    view! {
      <div class="pinning">
        <p class="pinning-status"><b>"Not available yet. "</b>
          "WAN pinning will put chosen subscriber or application addresses in an address list on the core router, so their traffic uses a chosen WAN. Nothing on this tab changes the router or the shaping."</p>
        <h3>"Prerequisites"</h3>
        <ul class="prereqs">
          {pinning_prerequisites(r).into_iter().map(|p|{
            let (mark,tone)=match p.met {Some(true)=>("Met","good"),Some(false)=>("Not met","warn"),None=>("Not available","neutral")};
            view!{<li><span class=format!("router-state {tone}")><i aria-hidden="true"></i>{mark}</span>
              <div><b>{p.label}</b><p>{p.note}</p></div></li>}
          }).collect_view()}
        </ul>
        <h3>"Planned form"</h3>
        <dl class="router-facts outline">
          {outline.into_iter().map(|(k,v)|view!{<div><dt>{k}</dt><dd>{v}</dd></div>}).collect_view()}
        </dl>
      </div>
    }
}
