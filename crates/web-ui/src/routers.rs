//! The Routers screen: the cached inventory and a read-only router detail.
//! The page reads `/api/v1/routers` (a service cache). It never connects to a router.
use crate::dashboard::local_time;
use crate::route::{Link, page};
use crate::route_model::{Page, Tab, router_href};
use crate::routers_model::{Filter, matches, read_state, role_label, transport};
use gloo_net::http::Request;
use leptos::prelude::*;
use web_contract::routers::{RouterInventory, RouterSummary};
use web_contract::{Overview, Principal};

fn count(n: Option<u64>) -> String {
    n.map(|n| n.to_string()).unwrap_or("—".into())
}
fn yes(b: bool) -> &'static str {
    if b { "Yes" } else { "No" }
}

#[component]
pub(crate) fn RoutersPage(
    data: RwSignal<Option<Overview>>,
    session: RwSignal<Option<Principal>>,
    filter: RwSignal<Filter>,
) -> impl IntoView {
    let inventory = RwSignal::new(None::<RouterInventory>);
    let loaded = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    // Load on entry and again after each service cycle; the overview time changes once per cycle.
    let cycle = Memo::new(move |_| data.with(|d| d.as_ref().and_then(|v| v.observed_at)));
    Effect::new(move |_| {
        cycle.track();
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
      {move||match route.get() {
        Page::Router{id,tab}=>view!{<Detail id tab inventory loaded error/>}.into_any(),
        _=>view!{<Inventory inventory loaded error filter data/>}.into_any(),
      }}
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
      <div class="page-heading">
        <div><h1 tabindex="-1">"Routers"</h1><p>"MikroTik routers in the configuration, their roles, and the latest read."</p></div>
        <div class="freshness">{move||inventory.with(|i|match i {
          Some(i) if data.with(|d|d.as_ref().is_some_and(|v|v.stale))=>format!("Service evidence stale · inventory from {}",local_time(i.updated)),
          Some(i)=>format!("Inventory from {}",local_time(i.updated)),
          None=>"Loading router inventory…".into(),
        })}</div>
      </div>
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
            </select>
          </label>
          <span class="router-count" aria-live="polite">{move||format!("{} of {} routers",rows.with(Vec::len),total.get())}</span>
        </div>
        <Show when=move||loaded.get()||inventory.with(Option::is_some) fallback=||view!{<p class="router-empty" role="status">"Loading router inventory…"</p>}>
          <Show when=move||{total.get()>0} fallback=move||view!{
            <p class="router-empty">{move||if inventory.with(Option::is_some){"No routers are in the configuration. Add a router in the TUI Settings for now; web registration comes in a later update."}else{"No router inventory yet."}}</p>
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
                      let (state,tone)=read_state(r.collection.as_ref());
                      let (link,link_tone)=transport(&r);
                      view!{<tr>
                        <th scope="row"><Link href=router_href(&r.id,Tab::Overview) class="router-name">{r.name.clone()}</Link>
                          <small>{format!("{}:{}",r.address,r.port)}</small></th>
                        <td><Roles roles=r.roles.clone()/></td>
                        <td><span class=format!("router-state {tone}")><i aria-hidden="true"></i>{state}</span>
                          <small>{r.collection.as_ref().map(|c|if c.observed_at>0{format!("Last read {}",local_time(c.observed_at))}else{"No successful read yet".into()}).unwrap_or("No subscriber role".into())}</small></td>
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
        None if !loaded.get()=>view!{<div class="page-heading"><div><h1 tabindex="-1">"Router"</h1><p role="status">"Loading router…"</p></div></div>}.into_any(),
        None=>view!{<div class="page-heading"><div><h1 tabindex="-1">"Router not found"</h1>
          <p>"This router is not in the configuration. It may have a new name."</p></div></div>
          <p><Link href="/routers".to_string()>"Back to all routers"</Link></p>}.into_any(),
        Some(r)=>{
          let (state,tone)=read_state(r.collection.as_ref());
          let id=r.id.clone();
          view!{
            <div class="page-heading">
              <div><h1 tabindex="-1">{r.name.clone()}</h1><p>{format!("{}:{}",r.address,r.port)}</p></div>
              <span class=format!("router-state big {tone}")><i aria-hidden="true"></i>{state}</span>
            </div>
            <nav class="router-tabs" aria-label="Router sections">
              {Tab::ALL.into_iter().map(|t|view!{<Link href=router_href(&id,t) current=Signal::derive(move||t==tab)>{t.label()}</Link>}).collect_view()}
            </nav>
            <section class="panel router-section" aria-label=tab.label()>
              {match tab {Tab::Overview=>overview(&r).into_any(),Tab::Connection=>connection(&r).into_any(),Tab::Sources=>sources(&r).into_any()}}
              <p class="panel-note">"Read-only. Router editing comes in a later update; use the TUI Settings for changes now."</p>
            </section>
          }.into_any()
        }
      }}
    }
}

fn overview(r: &RouterSummary) -> impl IntoView {
    let read = match &r.collection {
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
        <div><dt>"Roles"</dt><dd><Roles roles=r.roles.clone()/></dd></div>
        <div><dt>"Read state"</dt><dd>{read}</dd></div>
        <div><dt>"Subscribers"</dt><dd class="num">{count(r.collection.as_ref().map(|c|c.subscribers))}</dd></div>
        <div><dt>"WAN links"</dt><dd>{if r.wans.is_empty(){"None in the topology catalog".to_string()}else{r.wans.join(", ")}}</dd></div>
        <div><dt>"Location"</dt><dd>{r.location.map(|[a,b]|format!("{a:.5}, {b:.5}")).unwrap_or("Not set".into())}</dd></div>
        <div><dt>"Router ID"</dt><dd><code>{r.id.clone()}</code></dd></div>
      </dl>
    }
}

fn connection(r: &RouterSummary) -> impl IntoView {
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

fn sources(r: &RouterSummary) -> impl IntoView {
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
      </dl>
    }
}
