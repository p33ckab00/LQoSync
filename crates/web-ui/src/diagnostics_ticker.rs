//! A stable animation track with permanent status and short-lived observed changes.
use crate::{
    dashboard::{badge, local_time},
    diagnostics_activity::{ACTIVITY_SECONDS, Feed},
};
use leptos::prelude::*;
use web_contract::Overview;

#[component]
fn Items(data: RwSignal<Option<Overview>>, activity: RwSignal<Feed>) -> impl IntoView {
    view! {
      {[("Service health", "health"), ("Runtime mode", "mode"), ("Output target", "target"),
        ("Publication phase", "publication"), ("Last cycle", "cycle")]
        .into_iter().map(|(label, field)| {
          let value = move || data.with(|d| d.as_ref().map(|v| match field {
            "health" => v.health.clone(), "publication" => v.publication.clone(),
            "mode" => v.mode.clone(), "target" => v.target.clone(),
            _ => v.cycle_ms.map(|ms|format!("{ms} ms")).unwrap_or("Unknown".into()),
          }).unwrap_or("Unknown".into()));
          view!{<span class="ticker-item" role="listitem"><span>{label}</span>
            <strong class=move||if matches!(field,"health"|"publication"){badge(&value())}else{""}>{value}</strong></span>}
        }).collect_view()}
      <For each=move||activity.with(|f|f.events.clone()) key=|event|event.id children=move|event| {
          let color = badge(&event.state);
          view!{<span class="ticker-item ticker-activity" role="listitem"><span>"Update "{local_time(event.at)}</span><strong class=color>{event.text}</strong></span>}
        }/>
    }
}

#[component]
pub(crate) fn DiagnosticsTicker(data: RwSignal<Option<Overview>>) -> impl IntoView {
    let activity = RwSignal::new(Feed::default());
    Effect::new(move |_| {
        data.with(|view| {
            if let Some(view) = view {
                activity.update(|feed| feed.observe(view, now()));
            }
        });
    });
    let timer = gloo_timers::callback::Interval::new(1000, move || {
        let at = now();
        if activity.with_untracked(|f| {
            f.events
                .iter()
                .any(|e| at.saturating_sub(e.at) >= ACTIVITY_SECONDS)
        }) {
            activity.update(|feed| feed.expire(at));
        }
    });
    let timer = StoredValue::new_local(Some(timer));
    on_cleanup(move || {
        timer.try_update_value(|timer| *timer = None);
    });
    view! {
      <div id="diagnostics-ticker" class="diagnostics-ticker"
        role="group" aria-label="Live service status banner">
        <div class="ticker-track" style=move||format!("--ticker-duration:{}s",activity.with(|f|45+f.events.len()*5))>
          <div class="ticker-group" role="list" aria-label="Service and component status"><Items data activity/></div>
          <div class="ticker-group ticker-clone" aria-hidden="true"><Items data activity/></div>
        </div>
      </div>
    }
}
fn now() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}
