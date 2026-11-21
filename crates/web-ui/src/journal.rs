//! The Journal screen: read-only service logs, newest first. The page reads one bounded page at a
//! time; Follow adds the newest page every 5 s while the page is visible.
use crate::dashboard::local_time;
use crate::journal_model::{LEVELS, level, matches, merge_newest, merge_older};
use gloo_net::http::Request;
use leptos::prelude::*;
use web_contract::Principal;
use web_contract::journal::{JournalEntry, JournalPage, UNITS};

async fn fetch(
    session: RwSignal<Option<Principal>>,
    unit: &str,
    priority: u8,
    before: Option<u64>,
) -> Result<JournalPage, &'static str> {
    let (signal, _timer) = crate::app::deadline();
    let mut url = format!("/api/v1/journal?unit={unit}&priority={priority}");
    if let Some(b) = before {
        url.push_str(&format!("&before={b}"));
    }
    match Request::get(&url).abort_signal(Some(&signal)).send().await {
        Ok(r) if r.ok() => r
            .json::<JournalPage>()
            .await
            .map_err(|_| "The log response is not valid."),
        Ok(r) if r.status() == 401 => {
            session.set(None);
            Err("Your session ended. Sign in again.")
        }
        _ => Err("The log could not load. The page tries again with Follow on."),
    }
}

fn stamp(at_usec: u64) -> String {
    format!(
        "{}.{:03}",
        local_time(at_usec / 1_000_000),
        at_usec % 1_000_000 / 1000
    )
}

#[component]
pub(crate) fn JournalPage(session: RwSignal<Option<Principal>>) -> impl IntoView {
    let unit = RwSignal::new(UNITS[0].0.to_string());
    let priority = RwSignal::new(6u8);
    let search = RwSignal::new(String::new());
    let follow = RwSignal::new(true);
    let entries = RwSignal::new(Vec::<JournalEntry>::new());
    let more = RwSignal::new(false);
    let cut = RwSignal::new(false);
    let loading = RwSignal::new(true);
    let error = RwSignal::new(String::new());
    // A new unit or level starts again from the newest page.
    Effect::new(move |_| {
        let (u, p) = (unit.get(), priority.get());
        loading.set(true);
        leptos::task::spawn_local(async move {
            let r = fetch(session, &u, p, None).await;
            if unit.get_untracked() != u || priority.get_untracked() != p {
                return;
            }
            match r {
                Ok(page) => {
                    entries.try_set(page.entries);
                    more.try_set(page.more);
                    cut.try_set(page.cut);
                    error.try_set(String::new());
                }
                Err(e) => {
                    error.try_set(e.into());
                }
            }
            loading.try_set(false);
        });
    });
    let alive = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let stop = alive.clone();
    on_cleanup(move || stop.store(false, std::sync::atomic::Ordering::Relaxed));
    leptos::task::spawn_local(async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(5000).await;
            if !alive.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            let hidden = web_sys::window()
                .and_then(|w| w.document())
                .is_some_and(|d| d.hidden());
            if hidden || !follow.get_untracked() || loading.get_untracked() {
                continue;
            }
            let (u, p) = (unit.get_untracked(), priority.get_untracked());
            if let Ok(page) = fetch(session, &u, p, None).await
                && unit.get_untracked() == u
                && priority.get_untracked() == p
            {
                entries.try_update(|e| *e = merge_newest(e, &page.entries));
                cut.try_update(|c| *c |= page.cut);
            }
        }
    });
    let older = move |_| {
        let Some(first) = entries.with_untracked(|e| e.first().map(|x| x.at_usec)) else {
            return;
        };
        let (u, p) = (unit.get_untracked(), priority.get_untracked());
        loading.set(true);
        leptos::task::spawn_local(async move {
            match fetch(session, &u, p, Some(first)).await {
                Ok(page) => {
                    entries.try_update(|e| *e = merge_older(&page.entries, e));
                    more.try_set(page.more);
                    cut.try_update(|c| *c |= page.cut);
                }
                Err(e) => {
                    error.try_set(e.into());
                }
            }
            loading.try_set(false);
        });
    };
    let shown = Memo::new(move |_| {
        let text = search.get();
        entries.with(|e| {
            e.iter()
                .rev()
                .filter(|x| matches(x, &text))
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    view! {
      <div class="page-heading">
        <div><h1 tabindex="-1">"Journal"</h1><p>"Service logs, newest first. Read-only."</p></div>
        <div class="freshness">{move||if follow.get(){"Following new lines"}else{"Paused"}}</div>
      </div>
      <Show when=move||!error.get().is_empty()><div class="notice" role="status">{move||error.get()}</div></Show>
      <section class="panel journal-panel" aria-labelledby="journal-title">
        <h2 id="journal-title" class="sr-only">"Log lines"</h2>
        <div class="router-filters" role="search" aria-label="Filter the log">
          <label>"Service"
            <select prop:value=move||unit.get() on:change=move|ev|unit.set(event_target_value(&ev))>
              {UNITS.iter().map(|(u,l)|view!{<option value=*u>{*l}</option>}).collect_view()}
            </select>
          </label>
          <label>"Level"
            <select prop:value=move||priority.get().to_string() on:change=move|ev|priority.set(event_target_value(&ev).parse().unwrap_or(6))>
              {LEVELS.iter().map(|(p,l)|view!{<option value=p.to_string()>{*l}</option>}).collect_view()}
            </select>
          </label>
          <label class="router-search">"Search in the loaded lines"
            <input type="search" autocomplete="off" spellcheck="false" maxlength="128" prop:value=move||search.get() on:input=move|ev|search.set(event_target_value(&ev))/>
          </label>
          <label class="check follow">
            <input type="checkbox" prop:checked=move||follow.get() on:change=move|ev|follow.set(event_target_checked(&ev))/>
            "Follow new lines"
          </label>
          <span class="router-count" aria-live="polite">{move||format!("{} of {} lines",shown.with(Vec::len),entries.with(Vec::len))}</span>
        </div>
        <Show when=move||cut.get()><p class="field-help">"Some long messages are cut to 2000 characters."</p></Show>
        <Show when=move||!(loading.get()&&entries.with(Vec::is_empty)) fallback=||view!{<p class="router-empty" role="status">"Loading the log…"</p>}>
          <Show when=move||!shown.with(Vec::is_empty) fallback=move||view!{
            <p class="router-empty">{move||if entries.with(Vec::is_empty){"No lines at this level for this service."}else{"No loaded line matches the search."}}</p>
          }>
            <ol class="journal">
              <For each=move||shown.get() key=|e:&JournalEntry|(e.at_usec,e.message.len(),e.priority) children=move|e:JournalEntry|{
                let (label,tone)=level(e.priority);
                view!{<li class=format!("journal-line {tone}")>
                  <time class="journal-time">{stamp(e.at_usec)}</time>
                  <span class=format!("router-state {tone}")><i aria-hidden="true"></i>{label}</span>
                  <span class="journal-message">{e.message}</span>
                </li>}
              }/>
            </ol>
          </Show>
        </Show>
        <div class="form-actions start">
          <Show when=move||more.get()>
            <button type="button" class="secondary" disabled=move||loading.get() on:click=older>"Load older lines"</button>
          </Show>
        </div>
        <p class="panel-note">"Search covers the loaded lines only. Load older lines to search further back."</p>
      </section>
    }
}
