//! Dashboard summary of the TUI's worker analytics, bridge health, and NIC caches.
use crate::{dashboard::local_time, dpdk_history::History};
use leptos::prelude::*;
use web_contract::{
    Overview,
    dpdk::{Dpdk, Reading, Source},
};

fn now() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}
fn color(s: &str) -> &'static str {
    match s.to_ascii_lowercase().as_str() {
        "healthy" | "ok" | "up" | "clear" | "live" => "good",
        "down" | "failed" | "error" => "bad",
        "degraded" | "alert" => "warn",
        _ => "neutral",
    }
}
fn pct(n: Option<u64>) -> String {
    n.map(|n| format!("{:.1}%", n as f64 / 10000.0))
        .unwrap_or("—".into())
}
fn number(n: Option<u64>) -> String {
    n.map(|n| n.to_string()).unwrap_or("Unknown".into())
}
fn stamp(s: &Source, at: u64, age: u64) -> String {
    let state = if s.updated.is_none() {
        "Unavailable"
    } else if s.live(at, age) {
        "Live"
    } else {
        "Stale"
    };
    let age = s
        .updated
        .and_then(|t| at.checked_sub(t))
        .map(|v| format!("{v}s"))
        .unwrap_or("unknown".into());
    format!("{state} · age {age}")
}
fn rows(readings: Vec<Reading>) -> impl IntoView {
    view! {<dl class="dpdk-readings">{readings.into_iter().map(|r|view!{<dt>{r.label}</dt><dd>{r.value}</dd>}).collect_view()}</dl>}
}
fn source(d: &Dpdk, kind: &str) -> Source {
    match kind {
        "health" => d.health.clone(),
        "nic" => d.nic.clone(),
        _ => d.analytics.clone(),
    }
}
// Share one snapshot and server-time clock between the summary and diagnostics.
pub(crate) fn monitoring(data: RwSignal<Option<Overview>>) -> (Memo<Option<Dpdk>>, Memo<u64>) {
    let snapshot = Memo::new(move |_| data.with(|v| v.as_ref().and_then(|v| v.dpdk.clone())));
    let clock = RwSignal::new(now());
    let anchor = RwSignal::new((0_u64, now()));
    Effect::new(move |_| {
        snapshot.with(|d| {
            if let Some(d) = d {
                anchor.set((d.sampled_at, now()));
            }
        });
    });
    // Compare source timestamps with server time, not the remote device's clock.
    let server_clock = Memo::new(move |_| {
        let (server, received) = anchor.get();
        if server == 0 {
            clock.get()
        } else {
            server.saturating_add(clock.get().saturating_sub(received))
        }
    });
    let timer = StoredValue::new_local(Some(gloo_timers::callback::Interval::new(
        1000,
        move || clock.set(now()),
    )));
    on_cleanup(move || {
        timer.try_update_value(|t| *t = None);
    });
    (snapshot, server_clock)
}
#[component]
pub(crate) fn DpdkCard(snapshot: Memo<Option<Dpdk>>, server_clock: Memo<u64>) -> impl IntoView {
    let history = RwSignal::new(History::default());
    let selected = RwSignal::new(None::<u16>);
    Effect::new(move |_| {
        let at = server_clock.get();
        snapshot.with(|d| history.update(|h| h.observe(d.as_ref(), at)));
    });
    view! {
      <section class="panel dpdk-panel" aria-labelledby="dpdk-title">
        <div class="panel-title"><div><h2 id="dpdk-title">"DPDK stats & health"</h2>
          <p class="panel-note">"Whole bridge · same published sources as the TUI"</p></div>
          <span class=move||format!("chip {}",snapshot.with(|d|d.as_ref().map(|d|if d.health.live(server_clock.get(),15){color(&d.health.state)}else{"neutral"}).unwrap_or("neutral")))>
            {move||snapshot.with(|d|d.as_ref().filter(|d|d.health.live(server_clock.get(),15)).map(|d|d.health.state.clone()).unwrap_or("Unknown health".into()))}
          </span>
        </div>
        <div class="dpdk-sources">{[("analytics","Workers",6),("health","Bridge health",15),("nic","NIC telemetry",15)].into_iter().map(|(kind,label,age)|view!{
          <span><b>{label}</b>{move||snapshot.with(|d|d.as_ref().map(|d|stamp(&source(d,kind),server_clock.get(),age)).unwrap_or("Unavailable".into()))}</span>
        }).collect_view()}</div>
        <div class="dpdk-trend"><div><h3>"Recent mean worker busy"</h3><span class="panel-note">"Browser session · up to 60 distinct samples · fixed 0–100%"</span></div>
          <Show when=move||history.with(|h|h.points.len()>=2) fallback=||view!{<p class="dpdk-wait">"Waiting for two fresh worker samples."</p>}>
            <div class="dpdk-trend-frame"><div class="dpdk-trend-axis"><span>"100%"</span><span>"50%"</span><span>"0%"</span></div>
              <svg viewBox="0 0 600 100" preserveAspectRatio="none" role="img" aria-label="Mean worker busy percent during this browser session">
                <path class="dpdk-chart-grid" d="M0 0H600M0 50H600M0 100H600"/>
                <path class="dpdk-busy-line" d=move||history.with(History::path)/>
              </svg>
            </div>
            <div class="dpdk-trend-times"><span>{move||history.with(|h|h.points.front().map(|p|local_time(p.at)).unwrap_or_default())}</span><span>{move||history.with(|h|h.points.back().map(|p|local_time(p.at)).unwrap_or_default())}</span></div>
          </Show>
        </div>
        <details class="dpdk-details"><summary>"View DPDK diagnostics"</summary>
          <p class="panel-note">"Optional measurements stay unknown when absent. Laps/s is producer-reported, not packets/s. Counters are cumulative unless marked /sample."</p>
          <div class="dpdk-detail-grid">{[("analytics","Analytics & classification",6),("health","Bridge health",15),("nic","NIC source",15)].into_iter().map(|(kind,label,age)|view!{
            <div><h3>{label}</h3><p class="panel-note">{move||snapshot.with(|d|d.as_ref().map(|d|stamp(&source(d,kind),server_clock.get(),age)).unwrap_or("Unavailable".into()))}</p>
            {move||snapshot.with(|d|d.as_ref().map(|d|rows(source(d,kind).details)))}</div>
          }).collect_view()}</div>
          <h3>"Worker measurements"</h3>
          <p class="panel-note">{move||snapshot.with(|d|d.as_ref().map(|d|format!("Showing {} of {} workers, ranked by busy percent.",d.workers.len(),d.worker_count)).unwrap_or_default())}</p>
          <div class="table-scroll" role="region" aria-label="DPDK worker measurements" tabindex="0"><table><caption>"Latest worker snapshot · last known when stale"</caption>
            <thead><tr><th scope="col">"Worker"</th><th scope="col">"Busy"</th><th scope="col">"Idle (derived)"</th><th scope="col">"Mean backlog (packets)"</th></tr></thead>
            <tbody>{move||snapshot.with(|d|d.as_ref().map(|d|d.workers.iter().map(|w|view!{<tr><th scope="row">{w.id}</th><td>{pct(Some(w.busy_ppm))}</td><td>{pct(Some(1_000_000-w.busy_ppm))}</td><td>{number(w.backlog_mean)}</td></tr>}).collect_view()))}</tbody>
          </table></div>
          <label class="dpdk-worker-select">"Worker details"<select aria-label="DPDK worker details" on:change=move|ev|selected.set(event_target_value(&ev).parse().ok())>
            {move||snapshot.with(|d|d.as_ref().map(|d|d.workers.iter().map(|w|{let id=w.id;view!{<option value=id.to_string() selected=move||selected.get()==Some(id)>{format!("Worker {id}")}</option>}}).collect_view()))}
          </select></label>
          {move||snapshot.with(|d|d.as_ref().and_then(|d|selected.get().and_then(|id|d.workers.iter().find(|w|w.id==id)).or_else(||d.workers.first())).map(|w|{
            let mut all=w.details.clone();
            all.extend(w.phases.iter().map(|p|web_contract::dpdk::Reading{label:format!("{} cycles (total)",p.name),value:p.cycles.to_string()}));
            rows(all)
          }))}
          <h3>"NIC counters"</h3>
          {move||snapshot.with(|d|d.as_ref().map(|d|d.ports.iter().map(|p|view!{<div><h4>{format!("Port {} · {}",p.id,p.name)}</h4>{rows(p.details.clone())}</div>}).collect_view()))}
          <h3>"Worker chart samples"</h3>
          <div class="table-scroll" role="region" aria-label="DPDK chart samples" tabindex="0"><table><caption>"Browser-session samples in local time"</caption><thead><tr><th scope="col">"Time"</th><th scope="col">"Mean busy"</th><th scope="col">"Gap before sample"</th></tr></thead>
            <tbody>{move||history.with(|h|h.points.iter().map(|p|view!{<tr><th scope="row">{local_time(p.at)}</th><td>{pct(Some(p.busy_ppm))}</td><td>{if p.gap{"Yes"}else{"No"}}</td></tr>}).collect_view())}</tbody>
          </table></div>
          <Show when=move||snapshot.with(|d|d.as_ref().is_some_and(|d|d.details_capped))><p class="notice">"Optional diagnostics are capped to keep the monitoring response bounded. Use the TUI for the full worker and NIC inspector."</p></Show>
        </details>
      </section>
    }
}
