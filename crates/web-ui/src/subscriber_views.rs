//! Source records use the service cache. Policy actions require a verified subscriber link.
use crate::route::Link;
use crate::route_model::assignments_href;
use crate::topology::{Evidence, PlansTable, SubscriberPanel, get, node_name, now, summary};
use crate::topology_model::*;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_contract::Principal;
use web_contract::topology::*;

#[component]
pub(crate) fn SourcePage(
    session: RwSignal<Option<Principal>>,
    kind: &'static str,
    plans: bool,
) -> impl IntoView {
    let (title, rows_label, base, states): (_, _, _, &[(&str, &str)]) = if kind == "ppp" {
        (
            "PPPoE",
            "Users",
            "/pppoe",
            &[
                ("online", "Online"),
                ("offline", "Offline"),
                ("disabled", "Disabled"),
                ("stale", "Stale"),
                ("unknown", "Unknown"),
            ],
        )
    } else {
        (
            "IPoE / DHCP",
            "Clients",
            "/ipoe",
            &[
                ("bound", "Bound"),
                ("unbound", "Unbound"),
                ("stale", "Stale"),
                ("unknown", "Unknown"),
            ],
        )
    };
    let (data, error, load) = summary(session);
    let refresh = RwSignal::new(0u64);
    let plan_action = RwSignal::new(None::<Change>);
    view! {
      <div class="page-heading">
        <div><h1 tabindex="-1">{title}</h1><p>"Inspect source records. Manage plans and placement through Topology."</p></div>
        <div class="router-actions">
          <button type="button" class="quiet" on:click=move|_|{load.run(());refresh.update(|r|*r+=1);}>"Reload view"</button>
          <button type="button" class="quiet" on:click=move|_|crate::route::go("/topology")>"Open Topology"</button>
        </div>
      </div>
      <Evidence data/>
      <Show when=move||!error.get().is_empty()><div class="notice" role="status">{move||error.get()}</div></Show>
      <nav class="router-tabs" aria-label=format!("{title} views")>
        <Link href=base.to_string() current=Signal::derive(move||!plans)>{rows_label}</Link>
        <Link href=format!("{base}?view=plans") current=Signal::derive(move||plans)>"Local plans"</Link>
        <Link href="/topology?tab=assignments".to_string()>"Assignments"</Link>
      </nav>
      <SourceHealth data kind/>
      {if plans {
        view!{
          <section class="panel" aria-labelledby="source-plans-title">
            <div class="panel-title"><h2 id="source-plans-title">"Local plans"</h2><button type="button" class="quiet" disabled=move||crate::topology_forms::blocked(data).is_some() on:click=move|_|plan_action.set(Some(Change::CreatePlan))>"Create plan"</button></div>
            <p class="panel-note">"The source view filters membership. It is not a separate plan catalog. Identity counts and subscriber counts are separate."</p>
            <p class="panel-note"><Link href="/topology?tab=policies".to_string()>"Manage shared plans"</Link></p>
            <PlansTable data kind=Some(kind) action=plan_action/>
            <crate::topology_forms::ChangeDialog session data action=plan_action reload=load/>
          </section>
        }.into_any()
      } else {
        view!{<Records session data kind states rows_label refresh reload=load/>}.into_any()
      }}
    }
}

/// Collection failure is source evidence, not subscriber disconnection.
#[component]
fn SourceHealth(data: RwSignal<Option<TopologySummary>>, kind: &'static str) -> impl IntoView {
    view! {
      <div class="source-context">
        <ul class="source-health-list" aria-label="Source collection status">
          {move||data.with(|d|d.as_ref().map(|s|s.collection.iter().filter_map(|r|{
            let src=r.sources.iter().find(|x|x.kind==kind)?;
            let (state,class)=if !src.enabled {("Collection disabled".to_string(),"neutral")}
              else if r.state!="online" {(format!("Read {} · rows are stale",r.state),"warn")}
              else if stale(now(),r.observed_at,s.fresh_for) {(format!("Stale · {}",age(now(),r.observed_at)),"warn")}
              else {(format!("Complete · {} records · {}",src.records,age(now(),r.observed_at)),"good")};
            Some(view!{<li><strong>{r.name.clone()}</strong><span class=class>{state}</span></li>})
          }).collect_view()))}
        </ul>
        <p><Link href="/routers".to_string()>"Manage source routers"</Link></p>
      </div>
    }
}

#[component]
fn Records(
    session: RwSignal<Option<Principal>>,
    data: RwSignal<Option<TopologySummary>>,
    kind: &'static str,
    states: &'static [(&'static str, &'static str)],
    rows_label: &'static str,
    refresh: RwSignal<u64>,
    reload: Callback<()>,
) -> impl IntoView {
    let action = RwSignal::new(None::<Change>);
    let search = RwSignal::new(String::new());
    let router = RwSignal::new(String::new());
    let state = RwSignal::new(String::new());
    let link = RwSignal::new(String::new());
    let plan = RwSignal::new(String::new());
    let rows = RwSignal::new(Vec::<Observation>::new());
    let total = RwSignal::new(0u64);
    let next = RwSignal::new(None::<String>);
    let error = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let loaded = RwSignal::new(false);
    let request = RwSignal::new(0u64);
    let detail_request = RwSignal::new(0u64);
    let detail = RwSignal::new(None::<SubscriberDetail>);
    let open = RwSignal::new(None::<String>);
    let snapshot = Memo::new(move |_| {
        data.with(|d| {
            d.as_ref()
                .map(|s| s.snapshot_id.clone())
                .unwrap_or_default()
        })
    });
    let rows_snapshot = RwSignal::new(String::new());
    let detail_snapshot = RwSignal::new(String::new());
    let detail_error = RwSignal::new(String::new());
    let focus_return = StoredValue::new_local(None::<web_sys::HtmlElement>);
    let close = move || {
        detail_request.update(|n| *n += 1);
        open.set(None);
        detail.set(None);
        detail_error.set(String::new());
        if let Some(el) = focus_return.get_value() {
            let current = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id(&el.id()))
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok());
            if let Some(current) = current {
                let _ = current.focus();
            }
        }
    };
    let fetch = move |cursor: Option<String>| {
        request.update(|n| *n += 1);
        let generation = request.get_untracked();
        loading.set(true);
        let q = PageQuery {
            kind: Some(kind.into()),
            router_id: Some(router.get_untracked()),
            state: Some(state.get_untracked()),
            link: Some(link.get_untracked()),
            plan_id: Some(plan.get_untracked()),
            q: Some(search.get_untracked()),
            cursor: cursor.clone(),
        };
        leptos::task::spawn_local(async move {
            let result =
                get::<Page<Observation>>(session, &page_url("/api/v1/topology/observations", &q))
                    .await;
            if request.try_get_untracked() != Some(generation) {
                return;
            }
            loading.set(false);
            match result {
                Ok(p) => {
                    rows_snapshot.set(p.snapshot_id.clone());
                    total.set(p.total);
                    next.set(p.next_cursor);
                    if cursor.is_some() {
                        rows.update(|r| r.extend(p.items));
                    } else {
                        rows.set(p.items);
                    }
                    loaded.set(true);
                    error.set(String::new());
                    if let Some(id) = open.get_untracked()
                        && action.with_untracked(Option::is_none)
                        && !rows.with_untracked(|r| {
                            r.iter().any(|o| o.subscriber_id.as_ref() == Some(&id))
                        })
                    {
                        close();
                    }
                }
                Err(e) => {
                    error.set(e.into());
                    next.set(None);
                }
            }
        });
    };
    Effect::new(move |_| {
        router.track();
        state.track();
        link.track();
        plan.track();
        refresh.track();
        if !snapshot.get().is_empty() {
            fetch(None);
        }
    });
    let select = move |id: String, focus: bool| {
        if !focus && action.with_untracked(Option::is_some) {
            return;
        }
        detail_request.update(|n| *n += 1);
        let generation = detail_request.get_untracked();
        let expected_snapshot = snapshot.get_untracked();
        open.set(Some(id.clone()));
        if detail.with_untracked(|d| d.as_ref().is_none_or(|d| d.subscriber.id != id)) {
            detail.set(None);
            detail_error.set(String::new());
        }
        leptos::task::spawn_local(async move {
            let result =
                get::<SubscriberDetail>(session, &format!("/api/v1/topology/subscribers/{id}"))
                    .await;
            if detail_request.try_get_untracked() != Some(generation)
                || open.try_get_untracked().flatten().as_deref() != Some(&id)
                || snapshot.try_get_untracked().as_ref() != Some(&expected_snapshot)
            {
                return;
            }
            match result {
                Ok(d) if action.with_untracked(Option::is_none) => {
                    detail.set(Some(d));
                    detail_snapshot.set(expected_snapshot);
                    detail_error.set(String::new());
                }
                Ok(_) => {}
                Err(e) => detail_error.set(e.into()),
            }
            if focus
                && let Some(el) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("source-detail-heading"))
                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let _ = el.focus();
            }
        });
    };
    Effect::new(move |_| {
        snapshot.track();
        refresh.track();
        action.track();
        if action.with_untracked(Option::is_none)
            && let Some(id) = open.get_untracked()
        {
            select(id, false);
        }
    });
    let plan_name = move |id: &Option<String>| {
        id.as_ref().map(|id| {
            data.with(|d| {
                d.as_ref()
                    .and_then(|s| s.plans.iter().find(|p| p.id == *id))
                    .map(|p| p.name.clone())
            })
            .unwrap_or(id.clone())
        })
    };
    let label = if kind == "ppp" {
        "PPPoE identity records"
    } else {
        "DHCP client records"
    };
    view! {
      <div class="subscriber-workspace" class:has-selection=move||open.with(Option::is_some)
        on:keydown=move|ev|{if ev.key()=="Escape" && open.with_untracked(Option::is_some){ev.prevent_default();close();}}>
        <section class="panel subscriber-records" aria-labelledby="records-title" aria-busy=move||loading.get().to_string()>
          <div class="panel-title"><div><h2 id="records-title">{if kind=="ppp"{"PPPoE users"}else{"IPoE clients"}}</h2>
            <p class="panel-note" role="status">{move||if loading.get(){"Loading source records…".to_string()}else if loaded.get(){format!("{} records match · {} shown",total.get(),rows.with(Vec::len))}else{"Source records have not loaded.".into()}}</p></div>
          </div>
          <form class="router-filters subscriber-filters" role="search" on:submit=move|ev|{ev.prevent_default();fetch(None);}>
            <label>"Search"<input type="search" name="subscriber-search" autocomplete="off" placeholder="Account, subscriber, IP or MAC…" maxlength="128" prop:value=move||search.get() on:input=move|ev|search.set(event_target_value(&ev))/></label>
            <label>"Router"<select aria-label="Router" prop:value=move||router.get() on:change=move|ev|router.set(event_target_value(&ev))>
              <option value="">"All routers"</option>
              {move||data.with(|d|d.as_ref().map(|s|s.collection.iter().filter(|r|r.sources.iter().any(|x|x.kind==kind)).map(|r|view!{<option value=r.router_id.clone()>{r.name.clone()}</option>}).collect_view()))}
            </select></label>
            <label>"State"<select aria-label="State" prop:value=move||state.get() on:change=move|ev|state.set(event_target_value(&ev))>
              <option value="">"All states"</option>{states.iter().map(|(v,l)|view!{<option value=*v>{*l}</option>}).collect_view()}
            </select></label>
            <label>"Local plan"<select aria-label="Local plan" prop:value=move||plan.get() on:change=move|ev|plan.set(event_target_value(&ev))>
              <option value="">"All local plans"</option>{move||data.with(|d|d.as_ref().map(|s|s.plans.iter().map(|p|view!{<option value=p.id.clone()>{p.name.clone()}</option>}).collect_view()))}
            </select></label>
            <label>"Identity link"<select aria-label="Identity link" prop:value=move||link.get() on:change=move|ev|link.set(event_target_value(&ev))>
              <option value="">"All links"</option><option value="verified">"Verified link"</option><option value="none">"Observation only"</option><option value="unknown">"Link not decided"</option>
            </select></label>
            <div class="subscriber-filter-actions"><button type="submit" class="quiet" disabled=move||loading.get()>"Apply"</button>
              <button type="button" class="quiet" on:click=move|_|{search.set(String::new());router.set(String::new());state.set(String::new());plan.set(String::new());link.set(String::new());refresh.update(|r|*r+=1);}>"Clear filters"</button></div>
          </form>
          <Show when=move||!error.get().is_empty()><div class="notice" role="status">{move||error.get()}</div></Show>
          <Show when=move||loaded.get()&&(!error.get().is_empty()||rows_snapshot.get()!=snapshot.get())><p class="notice" role="status">"Source rows are retained from the last successful snapshot. Their endpoints are not current until refresh succeeds."</p></Show>
          <Show when=move||loaded.get() && !loading.get() && error.with(String::is_empty) && rows.with(Vec::is_empty)><p class="router-empty">"No source records match. Clear filters or reload the source view."</p></Show>
          <div class="table-scroll" role="region" aria-label=label tabindex="0">
            <table class="router-table"><caption class="sr-only">{label}</caption>
              <thead><tr><th scope="col">"Subscriber"</th><th scope="col">{if kind=="ppp"{"Account / router"}else{"Client / router"}}</th><th scope="col">"State"</th><th scope="col">"IP / MAC"</th><th scope="col">"Local plan"</th><th scope="col">"Placement / pin"</th></tr></thead>
              <tbody>{move||{let rows_current=rows_snapshot.get()==snapshot.get()&&error.with(String::is_empty)&&data.with(|d|d.as_ref().is_some_and(|s|!stale(now(),s.updated,s.fresh_for)));rows.get().into_iter().map(|o|{
                let current=rows_current&&matches!(o.state.as_str(),"online"|"bound");
                let selected=o.subscriber_id.clone();
                let subscriber=match o.subscriber_id.clone(){Some(id)=>{let target=id.clone();let pressed_id=id.clone();view!{<button id=format!("source-row-{}-{}-{}",o.router_id,o.kind,o.name) type="button" class="link" aria-pressed=move||open.with(|id|id.as_ref()==Some(&pressed_id)).to_string() on:click=move|ev|{focus_return.set_value(ev.current_target().and_then(|e|e.dyn_into::<web_sys::HtmlElement>().ok()));select(target.clone(),true);}>{id}</button>}.into_any()},None=>view!{<span>{if o.link=="none"{"Unlinked"}else{"Link not decided"}}<br/><small>{link_label(&o.link)}</small></span>}.into_any()};
                view!{<tr class:subscriber-selected=move||selected.as_ref().is_some_and(|id|open.with(|s|s.as_ref()==Some(id)))>
                  <th scope="row">{subscriber}</th><td>{o.name.clone()}<br/><small>{o.router.clone()}</small></td>
                  <td><span class=match if rows_current{o.state.as_str()}else{"stale"}{"online"|"bound"=>"chip good","stale"|"unknown"=>"chip warn",_=>"chip neutral"}>{state_label(if rows_current{&o.state}else{"stale"})}</span></td>
                  <td>{if current&&!o.ipv4.is_empty(){o.ipv4.clone()}else{"—".into()}}<br/><small>{if current{if o.mac.is_empty(){if kind=="ppp"{"Current session".into()}else{"Current lease".into()}}else{o.mac.clone()}}else{"No current endpoint".into()}}</small>
                    {(!current&&(!o.ipv4.is_empty()||!o.mac.is_empty())).then(||view!{<br/><small>{format!("Past: {} {}",o.ipv4,o.mac)}</small>})}</td>
                  <td>{if o.subscriber_id.is_some(){plan_name(&o.plan_id).unwrap_or("Source rate".into())}else{"—".into()}}</td>
                  <td>{o.node.as_ref().map(|n|node_name(data,n)).unwrap_or(if o.subscriber_id.is_some(){"No active placement".into()}else{"—".into()})}<br/><small>{match (&o.subscriber_id,&o.pin){(None,_)=>"No policy actions".into(),(_,Some(p))=>format!("Pinned: {}",node_name(data,p)),_=>"Unpinned".into()}}</small></td>
                </tr>}
              }).collect_view()}}</tbody>
            </table>
          </div>
          <Show when=move||next.with(Option::is_some)><button type="button" class="quiet" disabled=move||loading.get() on:click=move|_|fetch(next.get_untracked())>{format!("Load more {}",rows_label.to_lowercase())}</button></Show>
        </section>
        <Show when=move||open.with(Option::is_some)>
          <aside class="subscriber-details" aria-label="Subscriber details">
            <div class="subscriber-detail-toolbar"><h2 id="source-detail-heading" tabindex="-1">"Subscriber details"</h2><button type="button" class="quiet" aria-label="Close subscriber details" on:click=move|_|close()>"Close"</button></div>
            <Show when=move||detail.with(Option::is_some)&&(detail_snapshot.get()!=snapshot.get()||!detail_error.get().is_empty())><p class="notice" role="status">"Details are from a previous snapshot. The current draft is kept. Refresh follows after the draft closes."</p></Show>
            <Show when=move||!detail_error.get().is_empty()><p class="notice" role="status">{move||detail_error.get()}</p></Show>
            <SubscriberPanel detail plans=data action/>
            <p class="panel-note">{move||open.get().map(|id|view!{<Link href=assignments_href(&id)>"View assignments"</Link>})}</p>
          </aside>
        </Show>
      </div>
      <crate::topology_forms::ChangeDialog session data action reload=Callback::new(move|()|{reload.run(());fetch(None);if let Some(id)=open.get_untracked(){select(id,false);}})/>
    }
}
