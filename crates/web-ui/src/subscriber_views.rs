//! PPPoE Users and IPoE / DHCP Clients, with their Local plans view. Read-only release: rows come
//! from the service cache. A row links to a saved subscriber only through a verified identity.
use crate::route::Link;
use crate::route_model::assignments_href;
use crate::topology::{Evidence, PlansTable, SubscriberPanel, get, node_name, now, summary};
use crate::topology_model::*;
use leptos::prelude::*;
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
    view! {
      <div class="page-heading">
        <div><h1 tabindex="-1">{title}</h1><p>"Inspect source records. Manage plans and placement through Topology."</p></div>
        <div class="router-actions">
          <button type="button" class="quiet" on:click=move|_|load.run(())>"Reload view"</button>
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
            <h2 id="source-plans-title">"Local plans"</h2>
            <p class="panel-note">"The source view filters membership. It is not a separate plan catalog. Identity counts and subscriber counts are separate."</p>
            <PlansTable data kind=Some(kind)/>
          </section>
        }.into_any()
      } else {
        view!{<Records session data kind states rows_label reload=load/>}.into_any()
      }}
    }
}

/// Collection state per router for this source kind. A failed read is not a disconnect.
#[component]
fn SourceHealth(data: RwSignal<Option<TopologySummary>>, kind: &'static str) -> impl IntoView {
    view! {
      <p class="source-context">
        {move||data.with(|d|d.as_ref().map(|s|{
          let parts:Vec<String>=s.collection.iter().filter_map(|r|{
            let src=r.sources.iter().find(|x|x.kind==kind)?;
            Some(if !src.enabled {
              format!("{}: collection disabled",r.name)
            } else if r.state!="online" {
              format!("{}: read {} · rows are stale",r.name,r.state)
            } else {
              format!("{}: complete · {}",r.name,age(now(),r.observed_at))
            })
          }).collect();
          if parts.is_empty(){"No router collects this source.".to_string()}else{parts.join(" · ")}
        }))}
        " · Source settings: "<Link href="/routers".to_string()>"Routers"</Link>
      </p>
    }
}

#[component]
fn Records(
    session: RwSignal<Option<Principal>>,
    data: RwSignal<Option<TopologySummary>>,
    kind: &'static str,
    states: &'static [(&'static str, &'static str)],
    rows_label: &'static str,
    reload: Callback<()>,
) -> impl IntoView {
    let action = RwSignal::new(None::<Change>);
    let search = RwSignal::new(String::new());
    let router = RwSignal::new(String::new());
    let state = RwSignal::new(String::new());
    let link = RwSignal::new(String::new());
    let rows = RwSignal::new(Vec::<Observation>::new());
    let total = RwSignal::new(0u64);
    let next = RwSignal::new(None::<String>);
    let error = RwSignal::new(String::new());
    let fetch = move |cursor: Option<String>| {
        let q = PageQuery {
            kind: Some(kind.into()),
            router_id: Some(router.get_untracked()),
            state: Some(state.get_untracked()),
            link: Some(link.get_untracked()),
            q: Some(search.get_untracked()),
            cursor: cursor.clone(),
            ..Default::default()
        };
        leptos::task::spawn_local(async move {
            match get::<Page<Observation>>(session, &page_url("/api/v1/topology/observations", &q))
                .await
            {
                Ok(p) => {
                    total.set(p.total);
                    next.set(p.next_cursor);
                    if cursor.is_some() {
                        rows.update(|r| r.extend(p.items));
                    } else {
                        rows.set(p.items);
                    }
                    error.set(String::new());
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
        fetch(None);
    });
    let detail = RwSignal::new(None::<SubscriberDetail>);
    let open = RwSignal::new(None::<String>);
    let select = move |id: String| {
        open.set(Some(id.clone()));
        detail.set(None);
        leptos::task::spawn_local(async move {
            match get::<SubscriberDetail>(session, &format!("/api/v1/topology/subscribers/{id}"))
                .await
            {
                Ok(d) if open.get_untracked().as_deref() == Some(d.subscriber.id.as_str()) => {
                    detail.set(Some(d))
                }
                Ok(_) => {}
                Err(e) => error.set(e.into()),
            }
        });
    };
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
      <section class="panel" aria-labelledby="records-title">
        <h2 id="records-title">{if kind=="ppp"{"PPPoE users"}else{"IPoE clients"}}</h2>
        <p class="panel-note">{move||format!("{} records match. Counts describe this source; they are not global subscriber totals.",total.get())}</p>
        <form class="router-filters" role="search" on:submit=move|ev|{ev.prevent_default();fetch(None);}>
          <label>"Search"<input type="search" placeholder="Account or subscriber" maxlength="128" prop:value=move||search.get() on:input=move|ev|search.set(event_target_value(&ev))/></label>
          <label>"Router"<select on:change=move|ev|router.set(event_target_value(&ev))>
            <option value="">"All routers"</option>
            {move||data.with(|d|d.as_ref().map(|s|s.collection.iter().filter(|r|r.sources.iter().any(|x|x.kind==kind&&x.enabled)).map(|r|view!{<option value=r.router_id.clone()>{r.name.clone()}</option>}).collect_view()))}
          </select></label>
          <label>"State"<select on:change=move|ev|state.set(event_target_value(&ev))>
            <option value="">"All states"</option>
            {states.iter().map(|(v,l)|view!{<option value=*v>{*l}</option>}).collect_view()}
          </select></label>
          <label>"Identity link"<select on:change=move|ev|link.set(event_target_value(&ev))>
            <option value="">"All links"</option><option value="verified">"Verified link"</option><option value="none">"Observation only"</option><option value="unknown">"Link not decided"</option>
          </select></label>
          <button type="submit" class="quiet">"Apply"</button>
        </form>
        <Show when=move||!error.get().is_empty()><div class="notice" role="status">{move||error.get()}</div></Show>
        <div class="table-scroll" role="region" aria-label=label tabindex="0">
          <table class="router-table">
            <caption class="sr-only">{label}</caption>
            <thead><tr><th scope="col">"Subscriber"</th><th scope="col">{if kind=="ppp"{"Account / router"}else{"Client / router"}}</th><th scope="col">"State"</th><th scope="col">"Endpoint"</th><th scope="col">"Local plan"</th><th scope="col">"Placement / pin"</th></tr></thead>
            <tbody>
              {move||rows.get().into_iter().map(|o|{
                let current=matches!(o.state.as_str(),"online"|"bound");
                let subscriber=match o.subscriber_id.clone() {
                  Some(id)=>{let target=id.clone();view!{<button type="button" class="link" on:click=move|_|select(target.clone())>{id}</button>}.into_any()},
                  None=>view!{<span>{if o.link=="none"{"Unlinked"}else{"Link not decided"}}<br/><small>{link_label(&o.link)}</small></span>}.into_any(),
                };
                view!{<tr>
                  <th scope="row">{subscriber}</th>
                  <td>{o.name.clone()}<br/><small>{o.router.clone()}</small></td>
                  <td><span class="chip neutral">{state_label(&o.state)}</span></td>
                  <td>{if current && !o.ipv4.is_empty(){o.ipv4.clone()}else{"—".into()}}<br/>
                    <small>{if current{if kind=="ppp"{"Current session"}else{"Current lease"}}else{"No current endpoint"}}</small></td>
                  <td>{if o.subscriber_id.is_some(){plan_name(&o.plan_id).unwrap_or("Source rate".into())}else{"—".into()}}</td>
                  <td>{o.node.as_ref().map(|n|node_name(data,n)).unwrap_or(if o.subscriber_id.is_some(){"No active placement".into()}else{"—".into()})}<br/>
                    <small>{match (&o.subscriber_id,&o.pin){(None,_)=>"No policy actions".to_string(),(_,Some(p))=>format!("Pinned: {}",node_name(data,p)),_=>"Unpinned".into()}}</small></td>
                </tr>}
              }).collect_view()}
            </tbody>
          </table>
        </div>
        <Show when=move||next.with(Option::is_some)>
          <button type="button" class="quiet" on:click=move|_|fetch(next.get_untracked())>{format!("Load more {}",rows_label.to_lowercase())}</button>
        </Show>
      </section>
      <crate::topology_forms::ChangeDialog session data action reload=Callback::new(move|()|{
        reload.run(());
        fetch(None);
        if let Some(id)=open.get_untracked(){select(id);}
      })/>
      <Show when=move||open.with(Option::is_some)>
        <SubscriberPanel detail plans=data action/>
        <p>{move||open.get().map(|id|view!{<Link href=assignments_href(&id)>"View assignments"</Link>})}</p>
      </Show>
    }
}
