//! Cached topology views and explicit, reviewed policy changes. The service owns orchestration.
use crate::route::Link;
use crate::route_model::{TOPOLOGY_TABS, assignments_href};
use crate::topology_model::*;
use gloo_net::http::Request;
use leptos::prelude::*;
use web_contract::Principal;
use web_contract::topology::*;

pub(crate) fn now() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}

/// One GET of a topology route. The error is a sentence for the page.
pub(crate) async fn get<T: serde::de::DeserializeOwned>(
    session: RwSignal<Option<Principal>>,
    url: &str,
) -> Result<T, &'static str> {
    let (signal, _timer) = crate::app::deadline();
    match Request::get(url).abort_signal(Some(&signal)).send().await {
        Ok(r) if r.ok() => r
            .json::<T>()
            .await
            .map_err(|_| "The topology response is not valid."),
        Ok(r) if r.status() == 401 => {
            session.set(None);
            Err("Your session ended. Sign in again.")
        }
        Ok(r) if r.status() == 409 => {
            Err("The view changed. The list loads again from its first page.")
        }
        Ok(r) if r.status() == 404 => Err("This subscriber is not in the current view."),
        Ok(r) if r.status() == 503 => Err(
            "The topology view is not available yet. The service writes it at the end of each cycle.",
        ),
        _ => Err("The topology view could not load. Try Reload."),
    }
}

/// The display name of a node ID. An ID outside the summary shows as itself.
pub(crate) fn node_name(data: RwSignal<Option<TopologySummary>>, id: &str) -> String {
    data.with(|d| {
        d.as_ref()
            .and_then(|s| s.nodes.iter().find(|n| n.id == id))
            .map(|n| n.name.clone())
    })
    .unwrap_or(id.into())
}

/// Refresh the cache while visible. Polling never reads routers or changes policy.
pub(crate) fn summary(
    session: RwSignal<Option<Principal>>,
) -> (
    RwSignal<Option<TopologySummary>>,
    RwSignal<String>,
    Callback<()>,
) {
    let data = RwSignal::new(None::<TopologySummary>);
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let alive = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let stop = alive.clone();
    on_cleanup(move || stop.store(false, std::sync::atomic::Ordering::Relaxed));
    let active = alive.clone();
    let load = Callback::new(move |()| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        let active = active.clone();
        let expected = session.get_untracked().map(|p| p.csrf);
        leptos::task::spawn_local(async move {
            let result = get::<TopologySummary>(session, "/api/v1/topology").await;
            if !active.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            busy.set(false);
            if expected != session.get_untracked().map(|p| p.csrf) {
                return;
            }
            match result {
                Ok(s) => {
                    data.set(Some(s));
                    error.set(String::new());
                }
                Err(e) => error.set(e.into()),
            }
        });
    });
    load.run(());
    leptos::task::spawn_local(async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(5000).await;
            if !alive.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            if !web_sys::window().unwrap().document().unwrap().hidden() {
                load.run(());
            }
        }
    });
    (data, error, load)
}

/// Saved policy, observation, writer, and publication evidence. Each item keeps its own state.
#[component]
pub(crate) fn Evidence(data: RwSignal<Option<TopologySummary>>) -> impl IntoView {
    let item = |label: &'static str, value: Signal<String>, class: Signal<&'static str>| {
        view! {<div><dt>{label}</dt><dd class=class>{value}</dd></div>}
    };
    let observed = Signal::derive(move || {
        data.with(|d| match d.as_ref().and_then(|s| s.observed.as_ref()) {
            None => "No preview since the service started".into(),
            Some(o) if o.state == "conflicts" => format!("Conflicts · r{}", o.revision),
            Some(o) if o.stale => format!("r{} · older than saved policy", o.revision),
            Some(o) => format!("r{} · {}", o.revision, age(now(), o.collected_at)),
        })
    });
    let observed_class = Signal::derive(move || {
        data.with(|d| match d.as_ref().and_then(|s| s.observed.as_ref()) {
            Some(o) if o.state == "ok" && !o.stale => "good",
            Some(_) => "warn",
            None => "neutral",
        })
    });
    let saved = Signal::derive(move || {
        data.with(|d| {
            d.as_ref()
                .map(|s| match &s.registry_revision {
                    Some(r) if s.enrolled => format!("Revision {r}"),
                    Some(r) => format!("Revision {r} · not enrolled"),
                    None => "No registry".into(),
                })
                .unwrap_or("—".into())
        })
    });
    let writer = Signal::derive(move || {
        data.with(|d| {
            d.as_ref()
                .map(|s| {
                    let w = match s.writer.as_str() {
                        "topology" => "Topology",
                        "legacy" => "Legacy",
                        _ => "Unknown",
                    };
                    format!("{w} · {}", s.mode)
                })
                .unwrap_or("—".into())
        })
    });
    let writer_class = Signal::derive(move || {
        data.with(|d| match d.as_ref().map(|s| s.writer.as_str()) {
            Some("topology") => "good",
            Some("unknown") => "bad",
            _ => "neutral",
        })
    });
    let view_age = Signal::derive(move || {
        data.with(|d| {
            d.as_ref()
                .map(|s| {
                    let a = age(now(), s.updated);
                    if stale(now(), s.updated, s.fresh_for) {
                        format!("Stale · {a}")
                    } else {
                        a
                    }
                })
                .unwrap_or("Loading…".into())
        })
    });
    let view_class = Signal::derive(move || {
        data.with(|d| match d {
            Some(s) if stale(now(), s.updated, s.fresh_for) => "warn",
            Some(_) => "good",
            None => "neutral",
        })
    });
    let neutral = Signal::derive(|| "neutral");
    let labels = move || {
        data.with(|d| {
            let s = d.as_ref();
            publication_labels(
                s.and_then(|s| s.publication.as_ref()),
                s.and_then(|s| s.registry_revision.as_deref()),
            )
        })
    };
    let files = Signal::derive(move || labels().0);
    let files_class = Signal::derive(move || labels().1);
    let native = Signal::derive(move || labels().2);
    let native_class = Signal::derive(move || labels().3);
    view! {
      <dl class="evidence-strip" aria-label="Topology evidence">
        {item("View", view_age, view_class)}
        {item("Observations", observed, observed_class)}
        {item("Saved policy", saved, neutral)}
        {item("Writer", writer, writer_class)}
        {item("Files / import", files, files_class)}
        {item("Native verification", native, native_class)}
      </dl>
      {move||labels().4.map(|note|view!{<p class="panel-note evidence-note">{note}</p>})}
    }
}

#[component]
pub(crate) fn TopologyPage(
    session: RwSignal<Option<Principal>>,
    tab: &'static str,
    subscriber: Option<String>,
) -> impl IntoView {
    let (data, error, load) = summary(session);
    let tabs = move || {
        view! {
          <nav class="router-tabs" aria-label="Topology views">
            {TOPOLOGY_TABS.iter().map(|(key,label)|{
              let key=*key;
              let canvas = !matches!(tab, "assignments" | "policies" | "changes");
              let path = match key {
                  "assignments" => "M8 6h12M8 12h12M8 18h12M3 6h1M3 12h1M3 18h1",
                  "policies" => "M4 7h10M18 7h2M4 17h4M12 17h8M14 5v4M8 15v4",
                  "changes" => "M4 5h16v15H4zM8 2v6M16 2v6M8 12h8M8 16h5",
                  _ => "M3 3h6v6H3zM15 15h6v6h-6zM6 9v9h9",
              };
              view!{<Link href=format!("/topology?tab={key}") current=Signal::derive(move||key==tab)>
                <span class=if canvas {"canvas-tab-icon"} else {""} title=*label>
                  {canvas.then(||crate::topology_canvas::icon(path))}
                  <span>{*label}</span>
                </span>
              </Link>}
            }).collect_view()}
          </nav>
        }
    };
    let notices = move || {
        view! {
          {move||data.with(|d|d.as_ref().filter(|s|s.writer!="topology"||s.mode!="apply").map(|s|view!{
            <p class="notice" role="note">{if s.writer!="topology"{"Policy editor ready. Automatic topology publication needs an enrolled policy and active topology writer. The current writer remains in control."}else{"Topology owns the hierarchy. The current mode does not apply shaping files."}}</p>
          }))}
          {move||data.with(|d|d.as_ref().filter(|s|s.enrolled&&s.writer=="legacy").map(|_|view!{
            <details class="topology-activation"><summary>"Enable automatic topology sync"</summary>
              <p>"An operator must activate the enrolled policy once. Stop the service first, then preview with its configuration."</p>
              <code>"lqosync3 topology activate --config PATH"</code>
              <p>"Review the preview. Repeat with --yes to publish and select the topology writer, then start the service. It follows saved Builder changes each cycle."</p>
              <code>"lqosync3 topology activate --yes --config PATH"</code>
              <p>"Activation requires apply mode and the v3_pair target. It keeps the enrollment, lease, and file admission checks."</p>
            </details>
          }))}
        }
    };
    // The Builder fills the window: one compact heading row, and the status inside the canvas.
    if !matches!(tab, "assignments" | "policies" | "changes") {
        let summary_line = move || {
            data.with(|d| {
                d.as_ref().map(|s| {
                    let fresh = !stale(now(), s.updated, s.fresh_for);
                    let revision = s
                        .registry_revision
                        .as_deref()
                        .map_or("No policy".to_string(), |r| format!("Revision {r}"));
                    let wans = s.nodes.iter().filter(|n| n.kind == "wan").count();
                    let cards = s.nodes.iter().filter(|n| n.active).count();
                    (
                        fresh,
                        revision,
                        format!("{} writer", s.writer),
                        format!("{wans} WANs, {cards} cards"),
                    )
                })
            })
        };
        return view! {
          <div class="topology-canvas-page">
            <div class="topology-stage">
            <div class="topology-head">
              <h1 tabindex="-1">"Topology Manager"</h1>
              {tabs()}
              <button type="button" class="quiet router-icon topology-reload" aria-label="Reload view" title="Reload view" on:click=move|_|load.run(())>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7"/></svg>
              </button>
            </div>
            <Show when=move||data.with(Option::is_some)&&!error.get().is_empty()><div class="notice" role="status">{move||error.get()}</div></Show>
              <Show when=move||data.with(Option::is_some)><details class="canvas-evidence">
                <summary>{move||summary_line().map(|(fresh,revision,writer,counts)|view!{
                  <span class="canvas-evidence-dot" data-fresh=fresh.to_string() aria-hidden="true"></span>
                  <strong>{revision}</strong><span>{writer}</span><span>{counts}</span>
                })}</summary>
                <div class="canvas-evidence-body"><Evidence data/>{notices()}</div>
              </details></Show>
              <Show when=move||data.with(Option::is_some) fallback=move||view!{<div class="studio-initial" role="status">{move||if error.get().is_empty(){"Loading the topology view…".to_string()}else{error.get()}}</div>}>
                <crate::topology_canvas::Canvas session data reload=load/>
              </Show>
            </div>
          </div>
        }
        .into_any();
    }
    view! {
      <div class="page-heading">
        <div><h1 tabindex="-1">"Topology Manager"</h1><p>"Arrange the network on the canvas. Valid edits save automatically; the service applies the active policy each cycle."</p></div>
        <div class="router-actions">
          <button type="button" class="quiet" on:click=move|_|load.run(())>"Reload view"</button>
        </div>
      </div>
      <Evidence data/>
      {notices()}
      <Show when=move||!error.get().is_empty()><div class="notice" role="status">{move||error.get()}</div></Show>
      {tabs()}
      <Show when=move||data.with(Option::is_some) fallback=||view!{<div class="panel loading" role="status">"Loading the topology view…"</div>}>
        {
          let subscriber=subscriber.clone();
          move||match tab {
            "assignments"=>view!{<Assignments session data selected=subscriber.clone() reload=load/>}.into_any(),
            "policies"=>view!{<Policies session data reload=load/>}.into_any(),
            "changes"=>view!{<Changes data/>}.into_any(),
            _=>().into_any(),
          }
        }
      </Show>
    }
    .into_any()
}

#[component]
fn Assignments(
    session: RwSignal<Option<Principal>>,
    data: RwSignal<Option<TopologySummary>>,
    selected: Option<String>,
    reload: Callback<()>,
) -> impl IntoView {
    let action = RwSignal::new(None::<Change>);
    let version = RwSignal::new(0u32);
    let search = RwSignal::new(String::new());
    let state = RwSignal::new(String::new());
    let plan = RwSignal::new(String::new());
    let rows = RwSignal::new(Vec::<Subscriber>::new());
    let total = RwSignal::new(0u64);
    let next = RwSignal::new(None::<String>);
    let error = RwSignal::new(String::new());
    let query = move || PageQuery {
        state: Some(state.get_untracked()),
        plan_id: Some(plan.get_untracked()),
        q: Some(search.get_untracked()),
        ..Default::default()
    };
    let fetch = move |cursor: Option<String>| {
        let q = PageQuery {
            cursor: cursor.clone(),
            ..query()
        };
        leptos::task::spawn_local(async move {
            match get::<Page<Subscriber>>(session, &page_url("/api/v1/topology/assignments", &q))
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
        state.track();
        plan.track();
        fetch(None);
    });
    let detail = RwSignal::new(None::<SubscriberDetail>);
    // The detail loads on entry and again after each saved change.
    let wanted = selected.clone();
    Effect::new(move |_| {
        version.track();
        let Some(id) = wanted.clone() else { return };
        leptos::task::spawn_local(async move {
            match get::<SubscriberDetail>(session, &format!("/api/v1/topology/subscribers/{id}"))
                .await
            {
                Ok(d) => detail.set(Some(d)),
                Err(e) => error.set(e.into()),
            }
        });
    });
    let after_save = Callback::new(move |()| {
        reload.run(());
        version.update(|v| *v += 1);
        fetch(None);
    });
    let plan_name = move |id: &Option<String>| {
        id.as_ref()
            .map(|id| {
                data.with(|d| {
                    d.as_ref()
                        .and_then(|s| s.plans.iter().find(|p| p.id == *id))
                        .map(|p| p.name.clone())
                })
                .unwrap_or(id.clone())
            })
            .unwrap_or("Source rate".into())
    };
    view! {
      <section class="panel" aria-labelledby="assign-title">
        <h2 id="assign-title">"Assignments"</h2>
        <p class="panel-note">{move||format!("{} subscribers match. Saved records include offline subscribers.",total.get())}</p>
        <form class="router-filters" role="search" on:submit=move|ev|{ev.prevent_default();fetch(None);}>
          <label>"Search"<input type="search" placeholder="Subscriber ID or account" maxlength="128" prop:value=move||search.get() on:input=move|ev|search.set(event_target_value(&ev))/></label>
          <label>"State"<select on:change=move|ev|state.set(event_target_value(&ev))>
            <option value="">"All states"</option><option value="active">"Active in preview"</option><option value="offline">"Not active"</option></select></label>
          <label>"Local plan"<select on:change=move|ev|plan.set(event_target_value(&ev))>
            <option value="">"All plans"</option>
            {move||data.with(|d|d.as_ref().map(|s|s.plans.iter().map(|p|view!{<option value=p.id.clone()>{p.name.clone()}</option>}).collect_view()))}
          </select></label>
          <button type="submit" class="quiet">"Apply"</button>
        </form>
        <Show when=move||!error.get().is_empty()><div class="notice" role="status">{move||error.get()}</div></Show>
        <div class="table-scroll" role="region" aria-label="Subscriber assignments" tabindex="0">
          <table class="router-table">
            <thead><tr><th scope="col">"Subscriber"</th><th scope="col">"Identities"</th><th scope="col">"Local plan"</th><th scope="col">"Placement / pin"</th><th scope="col">"Endpoint"</th></tr></thead>
            <tbody>
              {move||rows.get().into_iter().map(|s|{
                let ids=s.identities.iter().map(|i|format!("{} · {}",i.kind.to_uppercase(),account(&i.account_ref))).collect::<Vec<_>>().join(", ");
                view!{<tr>
                  <th scope="row"><Link href=assignments_href(&s.id)>{s.id.clone()}</Link></th>
                  <td>{ids}</td>
                  <td>{plan_name(&s.plan_id)}</td>
                  <td>{s.active.as_ref().map(|a|node_name(data,&a.node)).unwrap_or("No active placement".into())}<br/>
                    <small>{s.pin.as_ref().map(|p|format!("Pinned: {}",node_name(data,p))).unwrap_or("Unpinned".into())}</small></td>
                  <td>{s.active.as_ref().map(|a|a.ips.join(", ")).filter(|v|!v.is_empty()).unwrap_or("—".into())}</td>
                </tr>}
              }).collect_view()}
            </tbody>
          </table>
        </div>
        <Show when=move||next.with(Option::is_some)>
          <button type="button" class="quiet" on:click=move|_|fetch(next.get_untracked())>"Load more"</button>
        </Show>
      </section>
      {selected.map(|_|view!{<SubscriberPanel detail plans=data action/>})}
      <crate::topology_forms::ChangeDialog session data action reload=after_save/>
    }
}

/// One saved subscriber with all verified identities. Shared by Assignments, PPPoE, and IPoE.
#[component]
pub(crate) fn SubscriberPanel(
    detail: RwSignal<Option<SubscriberDetail>>,
    plans: RwSignal<Option<TopologySummary>>,
    /// Set when the page hosts the change dialog.
    #[prop(optional)]
    action: Option<RwSignal<Option<Change>>>,
) -> impl IntoView {
    let locked = move || crate::topology_forms::blocked(plans).is_some();
    view! {
      <section class="panel subscriber-panel" aria-live="polite" aria-label="Selected subscriber">
        {move||match detail.get() {
          None=>view!{<p class="panel-note">"Loading the subscriber…"</p>}.into_any(),
          Some(d)=>{
            let s=d.subscriber;
            let plan=s.plan_id.as_ref().map(|id|plans.with(|p|p.as_ref().and_then(|p|p.plans.iter().find(|x|x.id==*id)).map(|x|format!("{} · {} / {} Mbps",x.name,x.down_mbps,x.up_mbps))).unwrap_or(id.clone())).unwrap_or("Source rate (no local plan)".into());
            view!{
              <h2>{s.id.clone()}</h2>
              <p>{format!("{} verified identities · local plan: {plan} · topology pin: {}",s.identities.len(),s.pin.as_ref().map(|p|node_name(plans,p)).unwrap_or("none".into()))}</p>
              <ul class="plain-list">
                {s.identities.iter().map(|i|view!{<li><strong>{i.kind.to_uppercase()}</strong>" "{account(&i.account_ref)}<small>{format!(" · source {}",i.source_id)}</small></li>}).collect_view()}
              </ul>
              <p class="panel-note">{match &s.active {
                Some(a)=>format!("Active in the preview at {} · {}",node_name(plans,&a.node),a.ips.join(", ")),
                None=>"Not active in the preview. No current endpoint.".into(),
              }}</p>
              {action.map(|action|{
                let (plan_change,pin_change)=(Change::SubscriberPlan(s.id.clone(),s.plan_id.clone()),Change::SubscriberPin(s.id.clone(),s.pin.clone()));
                view!{<div class="form-actions node-actions">
                  <button type="button" class="quiet" disabled=locked on:click=move|_|action.set(Some(plan_change.clone()))>"Change plan"</button>
                  <button type="button" class="quiet" disabled=locked on:click=move|_|action.set(Some(pin_change.clone()))>"Pin to WAN"</button>
                </div>}
              })}
              <p class="panel-note">"Every source view opens this same record. A DHCP lease does not grant permission to shape traffic."</p>
            }.into_any()
          }
        }}
      </section>
    }
}

#[component]
fn Policies(
    session: RwSignal<Option<Principal>>,
    data: RwSignal<Option<TopologySummary>>,
    reload: Callback<()>,
) -> impl IntoView {
    let action = RwSignal::new(None::<Change>);
    let locked = move || crate::topology_forms::blocked(data).is_some();
    view! {
      <section class="panel" aria-labelledby="plans-title">
        <div class="panel-title">
          <h2 id="plans-title">"Local plans"</h2>
          <button type="button" class="primary" disabled=locked on:click=move|_|action.set(Some(Change::CreatePlan))>"Create plan"</button>
        </div>
        <p class="panel-note">"One shared plan catalog for PPPoE and IPoE. Subscriber counts are unique over all sources."</p>
        <PlansTable data kind=None action/>
      </section>
      <crate::topology_forms::ChangeDialog session data action reload/>
      <section class="panel" aria-labelledby="speed-title">
        <div class="panel-title">
          <h2 id="speed-title">"Speed rules"</h2>
          <button type="button" class="quiet" disabled=move||locked()||data.with(|d|d.as_ref().is_none_or(|s|s.speed.is_none()))
            on:click=move|_|if let Some(sd)=data.with_untracked(|d|d.as_ref().and_then(|s|s.speed.clone())){action.set(Some(Change::SpeedDefaults(sd)));}>"Edit speed rules"</button>
        </div>
        {move||data.with(|d|d.as_ref().and_then(|s|s.speed.clone())).map(|sd|{
          let show=|r:&web_contract::topology::SpeedRule|match r.unit.as_str() {
            "mbps"=>format!("{} Mbps",r.value),
            "legacy"=>format!("{} percent, imported rounded rule",r.value),
            _=>format!("{} percent of the plan",r.value),
          };
          view!{
            <dl class="router-facts">
              <div><dt>{RULES[0]}</dt><dd>{show(&sd.down_min)}</dd></div>
              <div><dt>{RULES[1]}</dt><dd>{show(&sd.up_min)}</dd></div>
              <div><dt>{RULES[2]}</dt><dd>{show(&sd.down_max)}</dd></div>
              <div><dt>{RULES[3]}</dt><dd>{show(&sd.up_max)}</dd></div>
            </dl>
            {(sd.overrides>0).then(||view!{<p class="panel-note">{format!("{} profiles have their own rules. Editing the speed rules resets them.",sd.overrides)}</p>})}
          }
        })}
      </section>
      <section class="panel" aria-labelledby="factor-title">
        <h2 id="factor-title">"Source factors"</h2>
        <p class="panel-note">"A factor multiplies plan rates when the service sizes a source group. Node factor overrides stay as they are."</p>
        <div class="table-scroll" role="region" aria-label="Source factors" tabindex="0">
          <table class="router-table">
            <thead><tr><th scope="col">"Source"</th><th scope="col" class="num">"Download"</th><th scope="col" class="num">"Upload"</th><th scope="col">"Actions"</th></tr></thead>
            <tbody>
              {move||data.with(|d|d.as_ref().map(|s|s.factors.clone()).unwrap_or_default()).into_iter().map(|f|{
                let (label,change)=(format!("Edit factor of {}",f.name),Change::GroupFactor(f.clone()));
                view!{<tr><th scope="row">{f.name.clone()}</th><td class="num">{f.down.clone()}</td><td class="num">{f.up.clone()}</td>
                  <td><button type="button" class="quiet" disabled=locked aria-label=label on:click=move|_|action.set(Some(change.clone()))>"Edit"</button></td></tr>}
              }).collect_view()}
            </tbody>
          </table>
        </div>
      </section>
      <section class="panel" aria-labelledby="fallback-title">
        <div class="panel-title">
          <h2 id="fallback-title">"Fallback"</h2>
          <div class="node-actions form-actions">
            <button type="button" class="quiet" disabled=locked
              on:click=move|_|action.set(Some(Change::SetFallback(data.with_untracked(|d|d.as_ref().and_then(|s|s.fallback.clone())))))>"Set fallback"</button>
            <button type="button" class="quiet danger" disabled=move||locked()||data.with(|d|d.as_ref().is_none_or(|s|s.fallback.is_none()))
              on:click=move|_|action.set(Some(Change::ClearFallback))>"Clear fallback"</button>
          </div>
        </div>
        <p>{move||data.with(|d|match d.as_ref().and_then(|s|s.fallback.as_ref()) {
          Some(f)=>format!("Fallback groups: {}. Default rate {} / {} Mbps.",f.node_ids.iter().map(|id|node_name(data,id)).collect::<Vec<_>>().join(", "),f.default_mbps[0],f.default_mbps[1]),
          None=>"No fallback group.".into(),
        })}</p>
        <p class="panel-note">"Schedules, per-profile overrides, backstops, and formulas are not supported here."</p>
      </section>
    }
}

/// The plan table. `kind`: `ppp` or `dhcp` adds that source's identity count.
#[component]
pub(crate) fn PlansTable(
    data: RwSignal<Option<TopologySummary>>,
    kind: Option<&'static str>,
    /// Set when the page hosts the change dialog: each row gets Edit and Delete.
    #[prop(optional)]
    action: Option<RwSignal<Option<Change>>>,
) -> impl IntoView {
    let locked = move || crate::topology_forms::blocked(data).is_some();
    view! {
      <Show when=move||data.with(|d|d.as_ref().is_some_and(|s|!s.plans.is_empty()))
        fallback=||view!{<p class="router-empty">"No local plans. Subscribers use the rate that their source reads."</p>}>
        <div class="table-scroll" role="region" aria-label="Local plans" tabindex="0">
          <table class="router-table">
            <thead><tr><th scope="col">"Plan"</th><th scope="col" class="num">"Down / up Mbps"</th>
              {kind.map(|k|view!{<th scope="col" class="num">{if k=="ppp"{"PPPoE identities"}else{"DHCP identities"}}</th>})}
              <th scope="col" class="num">"Unique subscribers, all sources"</th>
              {action.map(|_|view!{<th scope="col">"Actions"</th>})}</tr></thead>
            <tbody>
              {move||data.with(|d|d.as_ref().map(|s|s.plans.iter().map(|p|view!{<tr>
                <th scope="row">{p.name.clone()}<br/><small>{p.id.clone()}</small></th>
                <td class="num">{format!("{} / {}",p.down_mbps,p.up_mbps)}</td>
                {kind.map(|k|view!{<td class="num">{if k=="ppp"{p.ppp_identities}else{p.dhcp_identities}}</td>})}
                <td class="num">{p.subscribers}</td>
                {action.map(|action|{
                  let (edit,delete)=(Change::EditPlan(p.clone()),Change::DeletePlan(p.clone()));
                  view!{<td><div class="node-actions form-actions">
                    <button type="button" class="quiet" disabled=locked aria-label=format!("Edit plan {}",p.name) on:click=move|_|action.set(Some(edit.clone()))>"Edit"</button>
                    <button type="button" class="quiet danger" disabled=locked aria-label=format!("Delete plan {}",p.name) on:click=move|_|action.set(Some(delete.clone()))>"Delete"</button>
                  </div></td>}
                })}
              </tr>}).collect_view()))}
            </tbody>
          </table>
        </div>
      </Show>
    }
}

#[component]
fn Changes(data: RwSignal<Option<TopologySummary>>) -> impl IntoView {
    view! {
      <section class="panel" aria-labelledby="changes-title">
        <h2 id="changes-title">"Conflicts"</h2>
        <p class="panel-note">{move||data.with(|d|d.as_ref().map(|s|match &s.observed {
          None=>"No preview since the service started. Conflicts appear after an observation refresh.".to_string(),
          Some(_) if s.conflicts_total==0=>"The latest preview has no conflicts.".into(),
          Some(_)=>format!("{} conflicts block preparation. A conflict preview has empty node and subscriber lists; this does not mean the network is gone.",s.conflicts_total),
        }).unwrap_or_default())}</p>
        <ul class="plain-list">
          {move||data.with(|d|d.as_ref().map(|s|s.conflicts.iter().map(|c|view!{
            <li><span class="chip warn">{c.kind.clone()}</span>" "<strong>{c.subject.clone()}</strong><small>{format!(" · {}",c.detail)}</small></li>
          }).collect_view()))}
        </ul>
        <Show when=move||data.with(|d|d.as_ref().is_some_and(|s|s.conflicts_total>s.conflicts.len() as u64))>
          <p class="notice" role="status">{move||data.with(|d|d.as_ref().map(|s|format!("{} more conflicts are not shown here.",s.conflicts_total-s.conflicts.len() as u64)).unwrap_or_default())}</p>
        </Show>
      </section>
      <section class="panel" aria-labelledby="publication-title">
        <h2 id="publication-title">"Latest write"</h2>
        {move||match data.with(|d|d.as_ref().and_then(|s|s.publication.clone())) {
          None=>view!{<p class="panel-note">"The service has no write record yet."</p>}.into_any(),
          Some(p)=>{
            let phase=p.phase.replace('_'," ");
            view!{
              <dl class="router-facts">
                <div><dt>"Write"</dt><dd>{p.write_id.clone()}</dd></div>
                <div><dt>"Phase"</dt><dd>{phase}</dd></div>
                <div><dt>"Policy revision"</dt><dd>{p.policy_revision.clone().unwrap_or("None: legacy writer".into())}</dd></div>
                <div><dt>"Native acceptance"</dt><dd>{if p.accepted{"Accepted"}else{"Not accepted yet"}}</dd></div>
                <div><dt>"Started"</dt><dd>{crate::dashboard::local_time(p.started)}</dd></div>
                <div><dt>"Updated"</dt><dd>{crate::dashboard::local_time(p.updated)}</dd></div>
              </dl>
              <ul class="plain-list" aria-label="Written files">
                {p.files.iter().map(|f|view!{<li><strong>{f.name.clone()}</strong><span class="chip neutral">{f.outcome.clone()}</span></li>}).collect_view()}
              </ul>
              {p.failed.then(||view!{<p class="notice" role="note">"The write recorded an error. Read the lqosync3 service journal for the details."</p>})}
              {p.superseded_by.clone().map(|by|view!{<p class="panel-note">{format!("A newer write {by} replaced this write.")}</p>})}
            }.into_any()
          }
        }}
      </section>
      <section class="panel" aria-labelledby="history-title">
        <h2 id="history-title">"Web changes"</h2>
        <p class="panel-note">"Changes saved from this web page, newest first. Changes from the TUI or the CLI are not recorded here."</p>
        <Show when=move||data.with(|d|d.as_ref().is_some_and(|s|!s.history.is_empty()))
          fallback=||view!{<p class="router-empty">"No web change yet."</p>}>
          <div class="table-scroll" role="region" aria-label="Web change history" tabindex="0">
            <table class="router-table">
              <thead><tr><th scope="col">"Time"</th><th scope="col">"Change"</th><th scope="col">"Actor"</th><th scope="col">"Result"</th><th scope="col">"Revision"</th></tr></thead>
              <tbody>
                {move||data.with(|d|d.as_ref().map(|s|s.history.clone()).unwrap_or_default()).into_iter().map(|h|view!{<tr>
                  <td>{crate::dashboard::local_time(h.at)}</td>
                  <th scope="row">{h.change.clone()}</th>
                  <td>{h.actor.clone()}</td>
                  <td><span class="chip neutral">{h.state.replace('_'," ")}</span></td>
                  <td>{h.revision.clone().map(|r|format!("{} from {}",r,h.base_revision)).unwrap_or(format!("base {}",h.base_revision))}</td>
                </tr>}).collect_view()}
              </tbody>
            </table>
          </div>
        </Show>
      </section>
    }
}
