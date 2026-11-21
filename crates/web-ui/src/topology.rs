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
    view! {
      <div class="page-heading">
        <div><h1 tabindex="-1">"Topology Manager"</h1><p>"Build the network hierarchy. The service applies the active policy each cycle."</p></div>
        <div class="router-actions">
          <button type="button" class="quiet" on:click=move|_|load.run(())>"Reload view"</button>
        </div>
      </div>
      <Evidence data/>
      <div class="topology-workflow" aria-label="Topology workflow">
        <span>"1. Discover sources"</span><span>"2. Build hierarchy"</span><span>"3. Review and save"</span><span>"4. Automatic sync"</span>
      </div>
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
      <Show when=move||!error.get().is_empty()><div class="notice" role="status">{move||error.get()}</div></Show>
      <nav class="router-tabs" aria-label="Topology views">
        {TOPOLOGY_TABS.iter().map(|(key,label)|{
          let key=*key;
          view!{<Link href=format!("/topology?tab={key}") current=Signal::derive(move||key==tab)>{*label}</Link>}
        }).collect_view()}
      </nav>
      <Show when=move||data.with(Option::is_some) fallback=||view!{<div class="panel loading" role="status">"Loading the topology view…"</div>}>
        {
          let subscriber=subscriber.clone();
          move||match tab {
            "assignments"=>view!{<Assignments session data selected=subscriber.clone() reload=load/>}.into_any(),
            "policies"=>view!{<Policies session data reload=load/>}.into_any(),
            "changes"=>view!{<Changes data/>}.into_any(),
            _=>view!{<Builder session data reload=load/>}.into_any(),
          }
        }
      </Show>
    }
}

#[component]
fn Builder(
    session: RwSignal<Option<Principal>>,
    data: RwSignal<Option<TopologySummary>>,
    reload: Callback<()>,
) -> impl IntoView {
    use crate::topology_forms::{ChangeDialog, blocked};
    let selected = RwSignal::new(None::<String>);
    let action = RwSignal::new(None::<Change>);
    let locked = move || crate::topology_forms::structural_blocked(data).is_some();
    let query = RwSignal::new(String::new());
    let collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    let forest = Memo::new(move |_| {
        data.with(|d| tree_forest(d.as_ref().map(|s| s.nodes.as_slice()).unwrap_or_default()))
    });
    let matches = Memo::new(move |_| {
        data.with(|d| {
            tree_matches(
                d.as_ref().map(|s| s.nodes.as_slice()).unwrap_or_default(),
                &query.get(),
            )
        })
    });
    let searching = Signal::derive(move || !query.get().trim().is_empty());
    let inventory_open = RwSignal::new(
        web_sys::window()
            .and_then(|w| w.match_media("(min-width: 901px)").ok().flatten())
            .is_some_and(|m| m.matches()),
    );
    let node = Memo::new(move |_| {
        let id = selected.get()?;
        data.with(|d| d.as_ref()?.nodes.iter().find(|n| n.id == id).cloned())
    });
    view! {
      <div class="topology-builder">
        <section class="panel topology-inventory" aria-labelledby="inventory-title">
          <div class="inventory-heading"><h2 id="inventory-title">"Inventory"</h2>
            <button type="button" class="quiet" aria-label="Toggle source inventory" aria-expanded=move||inventory_open.get().to_string()
              on:click=move|_|inventory_open.update(|open| *open = !*open)>{move||if inventory_open.get(){"Hide"}else{"Show"}}</button>
          </div>
          <div hidden=move||!inventory_open.get()>
          <p class="panel-note">"Attach a source beneath the selected parent."</p>
          <h3>"Sources"</h3>
          <ul class="plain-list">
            {move||data.with(|d|d.as_ref().map(|s|s.sources.iter().map(|src|view!{
              <li><div><strong>{src.name.clone()}</strong><small>{if src.kind=="wan"{"WAN source"}else{"Subscriber source"}}</small></div>
                {(src.kind=="subscribers").then(||{
                  let (id,name)=(src.id.clone(),src.name.clone());
                  view!{<button type="button" class="quiet" aria-label=format!("Attach {}",src.name) disabled=locked on:click=move|_|action.set(Some(Change::AttachSource(id.clone(),name.clone(),selected.get_untracked())))>"Attach"</button>}
                })}
              </li>
            }).collect_view()))}
          </ul>
          <h3>"Subscriber collection"</h3>
          <ul class="plain-list">
            {move||data.with(|d|d.as_ref().map(|s|s.collection.iter().map(|r|{
              let good=r.state=="online";
              let collects=r.sources.iter().any(|s|s.enabled);
              view!{<li><strong>{r.name.clone()}</strong>
                <span class=if !collects{"chip neutral"}else if good{"chip good"}else{"chip warn"}>{if collects{r.state.clone()}else{"No subscriber collector".into()}}</span>
                <small>{if collects{format!("Read {}",age(now(),r.observed_at))}else{"Use WAN discovery for interfaces.".into()}}</small></li>}
            }).collect_view()))}
          </ul>
          <Reads session action reload locked/>
          </div>
        </section>
        <section class="panel topology-hierarchy" aria-labelledby="tree-title">
          <div class="panel-title">
            <h2 id="tree-title">"Topology tree"</h2>
            <div class="node-actions form-actions">
              <button type="button" class="quiet" disabled=move||blocked(data).is_some() on:click=move|_|action.set(Some(Change::Rebalance(None,"all sources".into())))>"Rebalance all"</button>
              <button type="button" class="primary" disabled=locked on:click=move|_|action.set(Some(Change::Create))>"Create node"</button>
            </div>
          </div>
          <Show when=move||data.with(|d|d.as_ref().is_some_and(|s|!s.frontend_initialized && !s.nodes.is_empty()))>
            <p class="panel-note">"Existing saved policy. Opening this page does not create or import nodes."</p>
          </Show>
          <div class="hierarchy-toolbar">
            <label class="hierarchy-search">"Find a node"<input type="search" placeholder="Node name or type…" prop:value=move||query.get() on:input=move|ev|query.set(event_target_value(&ev))/></label>
            <button type="button" class="quiet" on:click=move|_|collapsed.set(std::collections::HashSet::new())>"Expand all"</button>
            <button type="button" class="quiet" on:click=move|_|collapsed.set(data.with(|d|d.as_ref().map(|s|s.nodes.iter().filter(|n|s.nodes.iter().any(|c|c.parent.as_deref()==Some(n.id.as_str()))).map(|n|n.id.clone()).collect()).unwrap_or_default()))>"Collapse all"</button>
          </div>
          {move||crate::topology_forms::structural_blocked(data).filter(|_|data.with(Option::is_some)).map(|why|view!{<p class="notice" role="note">{why}</p>})}
          <Show when=move||data.with(|d|d.as_ref().is_some_and(|s|!s.nodes.is_empty()))
            fallback=move||view!{<div class="router-empty"><p>"No topology nodes. Build the hierarchy from this workspace."</p>
              <Show when=move||data.with(|d|d.as_ref().is_some_and(|s|!s.frontend_initialized && !s.enrolled && s.writer=="legacy" && s.nodes.is_empty() && s.sources.is_empty()))>
                <button type="button" class="primary" on:click=move|_|action.set(Some(Change::Initialize))>"Start empty topology"</button>
              </Show></div>}>
          <div class="network-root"><strong>"Network"</strong><span>{move||data.with(|d|d.as_ref().map(|s|format!("{} WANs · {} active subscribers",s.nodes.iter().filter(|n|n.kind=="wan").count(),s.subscribers_active)))}</span></div>
          <div class="tree-labels"><span>"Node / policy"</span><span>"Down / Up Mbps"</span><span>"Members"</span></div>

            <ul class="topology-tree" aria-label="Topology nodes">
              {move||forest.get().into_iter().map(|branch|branch_view(branch,selected,collapsed,matches,searching)).collect_view()}
            </ul>
            <Show when=move||searching.get()&&matches.with(|m|m.is_empty())><p class="router-empty">"No node matches. Clear the search to show the hierarchy."</p></Show>
            <p class="panel-note hierarchy-caption">"Saved capacity policy, not measured traffic. Review changes before saving."</p>
            <Show when=move||data.with(|d|d.as_ref().is_some_and(|s|s.nodes_truncated))>
              <p class="notice" role="status">"The tree has more nodes than one reply holds. The first nodes are shown."</p>
            </Show>
          </Show>
        </section>
        <ChangeDialog session data action reload/>
        <section class="panel topology-details" aria-labelledby="details-title">
          <h2 id="details-title">"Details"</h2>
          {move||match node.get() {
            None=>view!{<p class="panel-note">"No node selected."</p>}.into_any(),
            Some(n)=>view!{
              <p class="chip neutral">{kind_label(&n.kind)}</p>
              <h3>{n.name.clone()}</h3>
              <dl class="router-facts">
                <div><dt>"Node ID"</dt><dd>{n.id.clone()}</dd></div>
                <div><dt>"Parent"</dt><dd>{n.parent.as_ref().map(|id|node_name(data,id)).unwrap_or("Network".into())}</dd></div>
                <div><dt>"Source"</dt><dd>{n.source_id.as_ref().map(|id|data.with(|d|d.as_ref().and_then(|s|s.sources.iter().find(|src|src.id==*id)).map(|src|src.name.clone()).unwrap_or(id.clone()))).unwrap_or("—".into())}</dd></div>
                <div><dt>"Capacity mode"</dt><dd>{capacity_label(&n.capacity)}</dd></div>
                <div><dt>"Calculated capacity"</dt><dd>{mbps(n.calculated_mbps)}</dd></div>
                <div><dt>"Exported capacity"</dt><dd>{mbps(n.exported_mbps)}</dd></div>
                <div><dt>"Direct subscribers"</dt><dd>{n.members.map(|m|m.to_string()).unwrap_or("No preview".into())}</dd></div>
              </dl>
              {n.source_id.as_ref().map(|source| {
                let names=data.with(|d|d.as_ref().map(|s|shared_parent_names(&s.nodes,source)).unwrap_or_default());
                view!{<p class="panel-note">{format!("Source placements: {}. Each subscriber has one effective placement.",names.join(", "))}</p>}
              })}
              {n.alias.then(||view!{<p class="panel-note">"This node keeps an adopted legacy name."</p>})}
              {
                let (e,m,r,child)=(n.clone(),n.clone(),n.clone(),n.id.clone());
                view!{<div class="form-actions node-actions">
                  <button type="button" class="quiet" disabled=locked on:click=move|_|action.set(Some(Change::CreateChild(child.clone())))>"Add child"</button>
                  <button type="button" class="quiet" disabled=locked on:click=move|_|action.set(Some(Change::Edit(e.clone())))>"Edit node"</button>
                  <button type="button" class="quiet" disabled=locked on:click=move|_|action.set(Some(Change::Move(m.clone())))>"Move"</button>
                  <button type="button" class="quiet danger" disabled=locked on:click=move|_|action.set(Some(Change::Remove(r.clone())))>"Remove node"</button>
                  {(n.kind=="group" && n.source_id.is_some()).then(||{
                    let candidate=n.clone();
                    let available=data.with(|d|d.as_ref().is_some_and(|s|!additional_parents(&s.nodes,&n).is_empty()));
                    view!{<button type="button" class="quiet" disabled=move||locked()||!available on:click=move|_|action.set(Some(Change::AddParent(candidate.clone())))>"Add parent"</button>}
                  })}
                  {(n.kind=="group").then(||n.source_id.clone()).flatten().map(|source|{
                    let change=Change::Rebalance(Some(source),n.name.clone());
                    view!{<button type="button" class="quiet" disabled=move||blocked(data).is_some() on:click=move|_|action.set(Some(change.clone()))>"Rebalance source"</button>}
                  })}
                </div>}
              }
            }.into_any(),
          }}
        </section>
      </div>
    }
}

/// Ordinary nested lists keep the hierarchy readable without an incomplete ARIA tree widget.
fn branch_view(
    branch: TreeBranch,
    selected: RwSignal<Option<String>>,
    collapsed: RwSignal<std::collections::HashSet<String>>,
    matches: Memo<std::collections::HashSet<String>>,
    searching: Signal<bool>,
) -> AnyView {
    let n = branch.node;
    let (id, current, visible_id, toggle_id, open_id) = (
        n.id.clone(),
        n.id.clone(),
        n.id.clone(),
        n.id.clone(),
        n.id.clone(),
    );
    let has_children = !branch.children.is_empty();
    let open = Signal::derive(move || searching.get() || !collapsed.with(|c| c.contains(&open_id)));
    let children = branch
        .children
        .into_iter()
        .map(|b| branch_view(b, selected, collapsed, matches, searching))
        .collect_view();
    view! {
      <li hidden=move||!matches.with(|m|m.contains(&visible_id))>
        <div class="tree-branch-row">
          {if has_children {view!{
            <button type="button" class="tree-disclosure" aria-label=format!("Toggle {} branch",n.name) aria-expanded=move||open.get().to_string()
              on:click=move|_|collapsed.update(|c|{if !c.remove(&toggle_id){c.insert(toggle_id.clone());}})><span aria-hidden="true">{move||if open.get(){"⌄"}else{"›"}}</span></button>
          }.into_any()}else{view!{<span class="tree-leaf" aria-hidden="true">"·"</span>}.into_any()}}
          <button type="button" class="tree-node" aria-pressed=move||(selected.get().as_deref()==Some(current.as_str())).to_string()
            on:click=move|_|selected.set(Some(id.clone()))>
            <span><strong>{n.name}</strong><small>{kind_label(&n.kind)}</small></span>
            <span class="num">{mbps(n.exported_mbps)}</span>
            <span class="num">{n.members.map(|m|m.to_string()).unwrap_or("—".into())}</span>
          </button>
        </div>
        {has_children.then(||view!{<ul hidden=move||!open.get()>{children}</ul>})}
      </li>
    }.into_any()
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

/// Refresh and discovery. Each button reads the enabled routers once; nothing else does.
/// A failed read keeps the last good answer beside the error.
#[component]
fn Reads(
    session: RwSignal<Option<Principal>>,
    action: RwSignal<Option<Change>>,
    reload: Callback<()>,
    locked: impl Fn() -> bool + Copy + Send + Sync + 'static,
) -> impl IntoView {
    use crate::topology_forms::run_read;
    let running = RwSignal::new(None::<ReadJob>);
    let wan = RwSignal::new(None::<Discovery>);
    let sources = RwSignal::new(None::<Discovery>);
    let message = RwSignal::new(String::new());
    let start = move |job: ReadJob| {
        if running.get_untracked().is_some() {
            return;
        }
        running.set(Some(job));
        message.set(String::new());
        leptos::task::spawn_local(async move {
            match run_read(session, job).await {
                Ok(op) => match job {
                    ReadJob::Refresh => {
                        message.set("Observations refreshed.".into());
                        reload.run(());
                    }
                    ReadJob::Wan => wan.set(op.discovery),
                    ReadJob::Sources => sources.set(op.discovery),
                },
                Err(e) => message.set(format!(
                    "Read failed: {e} The last good answer stays below."
                )),
            }
            running.set(None);
        });
    };
    let busy = move |job: ReadJob| running.get() == Some(job);
    view! {
      <h3>"Read the routers now"</h3>
      <p class="panel-note">"Each button reads every enabled router now. Opening this page reads none."</p>
      <div class="form-actions node-actions">
        <button type="button" class="quiet" disabled=move||running.get().is_some() on:click=move|_|start(ReadJob::Refresh)>
          {move||if busy(ReadJob::Refresh){"Refreshing…"}else{"Refresh observations"}}</button>
        <button type="button" class="quiet" disabled=move||running.get().is_some() on:click=move|_|start(ReadJob::Wan)>
          {move||if busy(ReadJob::Wan){"Discovering…"}else{"Discover WAN interfaces"}}</button>
        <button type="button" class="quiet" disabled=move||running.get().is_some() on:click=move|_|start(ReadJob::Sources)>
          {move||if busy(ReadJob::Sources){"Discovering…"}else{"Discover sources"}}</button>
      </div>
      <p role="status" class="field-help">{move||message.get()}</p>
      {move||wan.get().map(|d|view!{
        <h3>{format!("WAN interfaces · read {}",age(now(),d.collected_at))}</h3>
        <ul class="plain-list" aria-label="WAN interfaces">
          {d.routers.into_iter().map(|r|{
            if r.state!="ok" {
              return view!{<li><strong>{r.name.clone()}</strong><span class="chip warn">"Read failed"</span><small>{r.error.clone().unwrap_or_default()}</small></li>}.into_any();
            }
            let result = view!{<li class="discovery-router"><strong>{r.name.clone()}</strong><span class="chip good">"Read complete"</span><small>{format!("{} interfaces",r.candidates.len())}</small></li>};
            let candidates = r.candidates.into_iter().map(|c|{
              let change=Change::CreateWan(r.router_id.clone(),r.name.clone(),c.name.clone());
              let used=!c.used_by.is_empty();
              view!{<li><strong>{c.name.clone()}</strong><small>{format!("{} · {}",r.name,if c.running{"running"}else{"not running"})}</small>
                {if used {view!{<span class="chip neutral">"In use"</span>}.into_any()} else {view!{
                  <button type="button" class="quiet" disabled=locked on:click=move|_|action.set(Some(change.clone()))>"Create WAN"</button>}.into_any()}}
              </li>}
            }).collect_view();
            view!{<>{result}{candidates}</>}.into_any()
          }).collect_view()}
        </ul>
      })}
      {move||sources.get().map(|d|view!{
        <h3>{format!("New sources · read {}",age(now(),d.collected_at))}</h3>
        {d.sources.is_empty().then(||view!{<p class="field-help">"No new source was found."</p>})}
        <ul class="plain-list" aria-label="New sources">
          {d.sources.into_iter().map(|c|{
            let change=Change::AddSource(c.key.clone(),c.name.clone());
            view!{<li><strong>{c.name.clone()}</strong><small>{c.kind.to_uppercase()}</small>
              <button type="button" class="quiet" disabled=locked on:click=move|_|action.set(Some(change.clone()))>"Add source"</button></li>}
          }).collect_view()}
        </ul>
      })}
    }
}
