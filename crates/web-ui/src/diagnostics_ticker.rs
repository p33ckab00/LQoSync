//! A stable animation track with permanent status and short-lived observed changes.
use crate::{
    dashboard::{badge, local_time},
    diagnostics_activity::{ACTIVITY_SECONDS, Feed, LABELS, values},
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

// Each moving item keeps its text and width until it exits the viewport.
#[derive(Clone)]
struct MovingItem {
    id: u64,
    field: Option<usize>,
    activity_id: Option<u64>,
    label: String,
    text: String,
    state: String,
}

fn status_item(id: u64, field: usize, data: Option<&Overview>) -> MovingItem {
    let text = data
        .map(|v| values(v)[field].clone())
        .unwrap_or("Unknown".into());
    MovingItem {
        id,
        field: Some(field),
        activity_id: None,
        label: LABELS[field].into(),
        state: if matches!(field, 0 | 3) {
            text.clone()
        } else {
            String::new()
        },
        text,
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

    let queue = RwSignal::new(Vec::<MovingItem>::new());
    let ready = RwSignal::new(false);
    let seen = StoredValue::new(0_u64);
    let next_id = StoredValue::new(10_u64);
    // Keep two sets of permanent fields. Append new activity only at the tail.
    Effect::new(move |_| {
        let events = activity.with(|f| f.events.clone());
        let length = queue.with(Vec::len);
        if length == 0 {
            // Do not cache loading placeholders for an entire lap.
            let initial = data.with(|v| {
                v.as_ref().filter(|v| v.observed_at.is_some()).map(|v| {
                    (0..10)
                        .map(|id| status_item(id, id as usize % 5, Some(v)))
                        .collect()
                })
            });
            if let Some(initial) = initial {
                queue.set(initial);
                ready.set(true);
            }
        }
        let available = 18_usize.saturating_sub(queue.with_untracked(Vec::len));
        let pending = events
            .into_iter()
            .filter(|e| e.id > seen.get_value())
            .take(available)
            .collect::<Vec<_>>();
        if let Some(last) = pending.last() {
            seen.set_value(last.id);
            queue.update(|items| {
                items.extend(pending.into_iter().map(|e| MovingItem {
                    id: take_id(next_id),
                    field: None,
                    activity_id: Some(e.id),
                    label: format!("Update {}", local_time(e.at)),
                    text: e.text,
                    state: e.state,
                }))
            });
        }
    });

    let track = NodeRef::<leptos::html::Div>::new();
    let step = RwSignal::new(0_i32);
    let alternate = RwSignal::new(false);
    let moving = RwSignal::new(false);
    let advance = move || {
        moving.set(false);
        // Remove expired waiting entries only beyond the visible right edge.
        // Finish any visible text before retiring it.
        use wasm_bindgen::JsCast;
        let mut retired = Vec::new();
        if let Some(node) = track.get_untracked() {
            let viewport = node.parent_element().map(|p| p.client_width()).unwrap_or(0);
            let mut x = -step.get_untracked();
            let mut child = node.first_element_child();
            queue.with_untracked(|items| {
                for item in items {
                    let Some(element) = child.take() else { break };
                    if x >= viewport
                        && item.activity_id.is_some()
                        && !activity.with_untracked(|f| {
                            f.events.iter().any(|e| Some(e.id) == item.activity_id)
                        })
                    {
                        retired.push(item.id);
                    }
                    x += element
                        .unchecked_ref::<web_sys::HtmlElement>()
                        .offset_width();
                    child = element.next_element_sibling();
                }
            });
        }
        queue.update(|items| {
            items.retain(|item| !retired.contains(&item.id));
            if items.is_empty() {
                return;
            }
            let old = items.remove(0);
            if let Some(field) = old.field {
                items.push(
                    data.with_untracked(|v| status_item(take_id(next_id), field, v.as_ref())),
                );
            } else if activity
                .with_untracked(|f| f.events.iter().any(|e| Some(e.id) == old.activity_id))
            {
                items.push(old);
            }
        });
        alternate.update(|v| *v = !*v);
    };
    Effect::new(move |_| {
        let _ = alternate.get();
        if !ready.get() || track.get().is_none() {
            return;
        }
        leptos::leptos_dom::helpers::request_animation_frame(move || {
            use wasm_bindgen::JsCast;
            if let Some(first) = track
                .try_get_untracked()
                .flatten()
                .and_then(|n| n.first_element_child())
            {
                let width = first
                    .unchecked_into::<web_sys::HtmlElement>()
                    .offset_width();
                step.set(width);
                moving.set(true);
            }
        });
    });
    view! {
      <div id="diagnostics-ticker" class="diagnostics-ticker" class:ready=move||ready.get()
        role="group" aria-label="Live service status banner">
        <p class="ticker-loading" hidden=move||ready.get()>"Waiting for service status…"</p>
        <div class="ticker-group ticker-static" role="list" aria-label="Service and component status"><Items data activity/></div>
        <div class="ticker-track ticker-clone" aria-hidden="true" node_ref=track
          class:moving=move||moving.get() class:alternate=move||alternate.get()
          style=move||format!("--ticker-step:{}px;--ticker-duration:{}s",step.get(),f64::from(step.get())/32.0)
          on:animationend=move |_| advance()>
          <For each=move||queue.get() key=|item|item.id children=move|item| {
              let color = if item.state.is_empty() { "" } else { badge(&item.state) };
              view!{<span class="ticker-item" class:ticker-activity=item.activity_id.is_some()>
                <span>{item.label}</span><strong class=color>{item.text}</strong>
              </span>}
          }/>
        </div>
      </div>
    }
}
fn take_id(next: StoredValue<u64>) -> u64 {
    let id = next.get_value();
    next.set_value(id + 1);
    id
}
fn now() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}
