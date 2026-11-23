//! The router inventory of the Topology canvas: one section for each router with its WAN
//! interfaces and subscriber sources. A read starts only from a button here.
use crate::topology::now;
use crate::topology_forms::run_read;
use crate::topology_inventory_model::*;
use crate::topology_model::{Change, age};
use leptos::prelude::*;
use web_contract::Principal;
use web_contract::topology::{
    BuilderEdit, DiscoveredRouter, ReadJob, SourceCandidate, TopologySummary,
};

fn router_state(state: &str) -> (&'static str, &'static str) {
    match state {
        "online" => ("Online", "good"),
        "failed" => ("Read failed", "bad"),
        "unreachable" => ("Unreachable", "bad"),
        "offline" => ("Offline", "warn"),
        _ => ("Not collected yet", "muted"),
    }
}

fn access_label(access: &str) -> &str {
    match access {
        "ppp" => "PPPoE",
        "dhcp" => "DHCP",
        "hotspot" => "Hotspot",
        other => other,
    }
}

#[component]
pub(crate) fn Inventory(
    session: RwSignal<Option<Principal>>,
    data: RwSignal<Option<TopologySummary>>,
    reload: Callback<()>,
    locked: impl Fn() -> bool + Copy + Send + Sync + 'static,
    /// A WAN form or a new source opens through the house actions.
    action: RwSignal<Option<Change>>,
    /// Select a card on the canvas and close the inventory.
    show: Callback<String>,
    /// Put a source on the canvas below the selected card.
    attach: Callback<String>,
    send: Callback<BuilderEdit>,
) -> impl IntoView {
    // The running read and its router (WAN discovery of one router).
    let running = RwSignal::new(None::<(ReadJob, Option<String>)>);
    let wan_reads = RwSignal::new(Vec::<DiscoveredRouter>::new());
    let source_reads = RwSignal::new(None::<(u64, Vec<SourceCandidate>)>);
    let wan_read_at = RwSignal::new(std::collections::HashMap::<String, u64>::new());
    let message = RwSignal::new(String::new());
    let start = move |job: ReadJob, router: Option<String>| {
        if running.get_untracked().is_some() {
            return;
        }
        running.set(Some((job, router.clone())));
        message.set(String::new());
        leptos::task::spawn_local(async move {
            match run_read(session, job, router.clone().into_iter().collect()).await {
                Ok(op) => match (job, op.discovery) {
                    (ReadJob::Refresh, _) => {
                        message.set("Observations refreshed.".into());
                        reload.run(());
                    }
                    (ReadJob::Wan, Some(d)) => {
                        // One answer replaces the last answer of each router that it read.
                        wan_reads.update(|all| {
                            for r in d.routers {
                                wan_read_at.update(|t| {
                                    t.insert(r.router_id.clone(), d.collected_at);
                                });
                                all.retain(|x| x.router_id != r.router_id);
                                all.push(r);
                            }
                        });
                        message.set("WAN interfaces read.".into());
                    }
                    (ReadJob::Sources, Some(d)) => {
                        message.set(if d.sources.is_empty() {
                            "No new source was found.".into()
                        } else {
                            format!(
                                "{} new source{} found.",
                                d.sources.len(),
                                if d.sources.len() == 1 { "" } else { "s" }
                            )
                        });
                        source_reads.set(Some((d.collected_at, d.sources)));
                    }
                    _ => message.set("The read gave no answer.".into()),
                },
                Err(e) => message.set(format!("Read failed: {e} The last good answer stays.")),
            }
            running.set(None);
        });
    };
    let busy = move |job: ReadJob, router: Option<&str>| {
        running.with(|r| {
            r.as_ref()
                .is_some_and(|(j, x)| *j == job && x.as_deref() == router)
        })
    };
    let routers = move || {
        data.with(|d| {
            d.as_ref()
                .map(|s| {
                    inventory(
                        s,
                        &wan_reads.get(),
                        &source_reads.with(|r| r.as_ref().map(|r| r.1.clone()).unwrap_or_default()),
                    )
                })
                .unwrap_or_default()
        })
    };
    let chip = |text: String, tone: &'static str| view! {<span class="inv-chip" data-tone=tone>{text}</span>};
    let router_view = move |r: InvRouter| {
        let (state, tone) = router_state(&r.state);
        let head = format!("inv-{}", r.id);
        let (wan_cap, src_cap) = (format!("{head}-wans"), format!("{head}-sources"));
        let (wan_id, src_id) = (wan_cap.clone(), src_cap.clone());
        let read = r
            .observed_at
            .map(|t| format!("Collected {}", age(now(), t)));
        let wan_read = wan_read_at
            .with(|t| t.get(&r.id).copied())
            .map(|t| format!("WANs read {}", age(now(), t)));
        let error = wan_reads.with(|w| {
            w.iter()
                .find(|x| x.router_id == r.id && x.state != "ok")
                .map(|x| {
                    x.error
                        .clone()
                        .unwrap_or_else(|| "The router did not answer.".into())
                })
        });
        let (rid, rid2, rname) = (r.id.clone(), r.id.clone(), r.name.clone());
        let wans = r.wans.clone();
        let no_wans = wans.is_empty();
        let wan_rows = wans
            .into_iter()
            .map(|w| {
                let iface = w.interface.clone();
                let (text, tone) = match &w.state {
                    WanState::OnCanvas { name, .. } => (format!("On canvas as {name}"), "good"),
                    WanState::Saved => ("Saved, not on canvas".to_string(), "muted"),
                    WanState::NotConfigured => ("Default route, not configured".to_string(), "warn"),
                    WanState::Found { running: true } => ("Found, running".to_string(), "muted"),
                    WanState::Found { running: false } => ("Found, not running".to_string(), "muted"),
                };
                let act = match w.state {
                    WanState::OnCanvas { card, name } => Some(view! {
                      <button type="button" class="quiet" aria-label=format!("Show {name} on the canvas")
                        on:click=move|_|show.run(card.clone())>"Show"</button>}.into_any()),
                    WanState::Saved => None,
                    _ => {
                        let change = Change::CreateWan(rid.clone(), rname.clone(), iface.clone());
                        Some(view! {
                          <button type="button" class="quiet" disabled=locked aria-label=format!("Add {iface} as a WAN")
                            on:click=move|_|action.set(Some(change.clone()))>"Add WAN"</button>}.into_any())
                    }
                };
                view! {<tr>
                  <th scope="row">{w.interface}</th>
                  <td data-label="State">{chip(text, tone)}</td>
                  <td class="inv-action">{act}</td>
                </tr>}
            })
            .collect_view();
        let no_sources = r.sources.is_empty() && r.found.is_empty();
        let source_rows = r
            .sources
            .into_iter()
            .map(|src| {
                let (text, tone) = match &src.state {
                    SourceState::OnCanvas { .. } => ("On canvas".to_string(), "good"),
                    SourceState::Idle => ("No active subscribers".to_string(), "muted"),
                    SourceState::Auto { parents } => (format!("Auto: {}", parents.join(", ")), "accent"),
                    SourceState::Unplaced => ("Not on canvas".to_string(), "muted"),
                    SourceState::Excluded => ("Not read".to_string(), "bad"),
                };
                let (id, name) = (src.id.clone(), src.name.clone());
                let act = match src.state {
                    SourceState::OnCanvas { card } => Some(view! {
                      <button type="button" class="quiet" aria-label=format!("Show {name} on the canvas")
                        on:click=move|_|show.run(card.clone())>"Show"</button>}.into_any()),
                    SourceState::Idle => None,
                    SourceState::Auto { .. } | SourceState::Unplaced => Some(view! {
                      <button type="button" class="quiet" disabled=locked aria-label=format!("Add {name} to canvas")
                        on:click=move|_|attach.run(id.clone())>"Add to canvas"</button>}.into_any()),
                    SourceState::Excluded => Some(view! {
                      <button type="button" class="quiet" disabled=locked aria-label=format!("Read {name} again")
                        on:click=move|_|send.run(BuilderEdit::SourceExclude{source_id:id.clone(),excluded:false})>"Read again"</button>}.into_any()),
                };
                view! {<tr>
                  <th scope="row">{src.name}</th>
                  <td data-label="Type">{access_label(&src.access).to_string()}</td>
                  <td data-label="Subscribers" class="inv-num">{src.subscribers.map_or("—".to_string(), |n| n.to_string())}</td>
                  <td data-label="State">{chip(text, tone)}</td>
                  <td class="inv-action">{act}</td>
                </tr>}
            })
            .collect_view();
        let found_rows = r
            .found
            .into_iter()
            .map(|c| {
                let change = Change::AddSource(c.key.clone(), c.name.clone());
                let label = format!("Add source {}", c.name);
                view! {<tr>
                  <th scope="row">{c.name.clone()}</th>
                  <td data-label="Type">{access_label(&c.kind).to_string()}</td>
                  <td data-label="Subscribers" class="inv-num">"—"</td>
                  <td data-label="State">{chip(if c.rebind {"Found, changes binding".into()} else {"Found by discovery".into()}, "accent")}</td>
                  <td class="inv-action"><button type="button" class="quiet" disabled=locked aria-label=label
                    on:click=move|_|action.set(Some(change.clone()))>"Add source"</button></td>
                </tr>}
            })
            .collect_view();
        let counts = r
            .counts
            .iter()
            .map(|(kind, records, online)| {
                let text = match online {
                    Some(o) => format!("{} {o} of {records} online", access_label(kind)),
                    None => format!("{} {records}", access_label(kind)),
                };
                view! {<span>{text}</span>}
            })
            .collect_view();
        view! {
          <section class="inv-router" data-tone=tone aria-labelledby=head.clone()>
            <header class="inv-head">
              <div class="inv-title">
                <h3 id=head.clone()>{r.name.clone()}</h3>
                <p class="inv-meta">{chip(state.to_string(), tone)}{read.map(|t|view!{<span>{t}</span>})}{counts}{wan_read.map(|t|view!{<span>{t}</span>})}</p>
              </div>
              <button type="button" class="quiet" disabled=move||locked()||running.with(Option::is_some)
                on:click=move|_|start(ReadJob::Wan,Some(rid2.clone()))>
                {let id=r.id.clone(); move||if busy(ReadJob::Wan,Some(&id)){"Reading…"}else{"Find WAN interfaces"}}</button>
            </header>
            {error.map(|e|view!{<p class="field-error" role="alert">{format!("WAN read failed: {e}")}</p>})}
            <div class="inv-wrap" role="region" tabindex="0" aria-labelledby=wan_cap>
              <table class="inv-table">
                <caption id=wan_id>"WAN interfaces"</caption>
                <thead><tr><th scope="col">"Interface"</th><th scope="col">"State"</th><th scope="col"><span class="sr-only">"Action"</span></th></tr></thead>
                <tbody>
                  {wan_rows}
                  {no_wans.then(||view!{<tr><td colspan="3" class="inv-empty">"No WAN interface is known. Find WAN interfaces reads this router."</td></tr>})}
                </tbody>
              </table>
            </div>
            <div class="inv-wrap" role="region" tabindex="0" aria-labelledby=src_cap>
              <table class="inv-table">
                <caption id=src_id>"Subscriber sources"</caption>
                <thead><tr><th scope="col">"Source"</th><th scope="col">"Type"</th><th scope="col" class="inv-num">"Subscribers"</th><th scope="col">"State"</th><th scope="col"><span class="sr-only">"Action"</span></th></tr></thead>
                <tbody>
                  {source_rows}{found_rows}
                  {no_sources.then(||view!{<tr><td colspan="5" class="inv-empty">"No subscriber source is saved. Discover sources reads every router."</td></tr>})}
                </tbody>
              </table>
            </div>
          </section>
        }
    };
    view! {
      <div class="inv-bar">
        <p class="field-help">"Reads start only from these buttons. Opening the inventory reads no router."</p>
        <div class="form-actions">
          <button type="button" class="quiet" disabled=move||running.with(Option::is_some) on:click=move|_|start(ReadJob::Refresh,None)>
            {move||if busy(ReadJob::Refresh,None){"Refreshing…"}else{"Refresh observations"}}</button>
          <button type="button" class="quiet" disabled=move||locked()||running.with(Option::is_some) on:click=move|_|start(ReadJob::Sources,None)>
            {move||if busy(ReadJob::Sources,None){"Discovering…"}else{"Discover sources"}}</button>
        </div>
        <p role="status" class="field-help">{move||message.get()}</p>
      </div>
      {move||{
        let all=routers();
        if all.is_empty() {
          view!{<p class="inv-empty">"No router is collected yet. Add a router on the Routers page."</p>}.into_any()
        } else {
          all.into_iter().map(router_view).collect_view().into_any()
        }
      }}
    }
}
