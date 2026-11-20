//! A stable animation track. Live values change without restarting its motion.
use crate::dashboard::{badge, phase};
use leptos::prelude::*;
use web_contract::Overview;

#[component]
fn Items(data: RwSignal<Option<Overview>>) -> impl IntoView {
    view! {
      {[("Evidence", "evidence"), ("Service", "health"), ("Sync", "publication"),
        ("Mode", "mode"), ("Target", "target"), ("Cycle", "cycle")]
        .into_iter().map(|(label, field)| {
          let value = move || data.with(|d| d.as_ref().map(|v| match field {
            "evidence" => if v.stale { "Stale".into() } else { "Current".into() },
            "health" => v.health.clone(), "publication" => phase(&v.publication),
            "mode" => v.mode.clone(), "target" => v.target.clone(),
            _ => v.cycle_ms.map(|ms|format!("{ms} ms")).unwrap_or("Unknown".into()),
          }).unwrap_or("Unknown".into()));
          view!{<span class="ticker-item" role="listitem"><span>{label}</span>
            <strong class=move||if matches!(field,"health"|"publication"){badge(&value())}else{""}>{value}</strong></span>}
        }).collect_view()}
      <For each=move||data.with(|d|d.as_ref().map(|v|v.components.iter().enumerate().map(|(i,c)|(i,c.name.clone())).collect::<Vec<_>>()).unwrap_or_default())
        key=|(index,name)|(*index,name.clone()) children=move|(index,name)| {
          let label = name.replace('_', " ");
          let state = move||data.with(|d|d.as_ref().and_then(|v|v.components.get(index)).map(|c|c.state.replace('_'," ")).unwrap_or("UNKNOWN".into()));
          view!{<span class="ticker-item" role="listitem"><span>{label}</span><strong class=move||badge(&state())>{state}</strong></span>}
        }/>
    }
}

#[component]
pub(crate) fn DiagnosticsTicker(data: RwSignal<Option<Overview>>) -> impl IntoView {
    let paused = RwSignal::new(false);
    view! {
      <div class="diagnostics-heading">
        <div><h2 id="service-diagnostics-title">"Service diagnostics"</h2><p>"Synchronization and component health"</p></div>
        <button type="button" class="quiet ticker-toggle" aria-controls="diagnostics-ticker"
          on:click=move|_|paused.update(|v|*v = !*v)>
          {move||if paused.get(){"Resume banner"}else{"Pause banner"}}
        </button>
      </div>
      <div id="diagnostics-ticker" class="diagnostics-ticker" class:paused=move||paused.get()
        role="group" aria-label="Live service status banner">
        <div class="ticker-track" style=move||format!("--ticker-duration:{}s",data.with(|d|d.as_ref().map(|v|45+v.components.len()*5).unwrap_or(45)))>
          <div class="ticker-group" role="list" aria-label="Service and component status"><Items data/></div>
          <div class="ticker-group ticker-clone" aria-hidden="true"><Items data/></div>
        </div>
      </div>
    }
}
