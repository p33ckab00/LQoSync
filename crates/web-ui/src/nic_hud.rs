//! Read-only, interval-aware diagnostics for a physical port.
use crate::{
    dashboard::{local_time, rate},
    nic_diagnostics::{self as figures, History},
};
use leptos::prelude::*;
use web_contract::dpdk::Port;
fn integer(value: Option<u64>) -> String {
    value.map(|v| v.to_string()).unwrap_or_else(|| "—".into())
}
fn bytes(value: Option<u64>) -> String {
    value
        .map(|n| {
            if n >= 1_000_000 {
                format!("{:.2} MB", n as f64 / 1_000_000.0)
            } else if n >= 1000 {
                format!("{:.2} kB", n as f64 / 1000.0)
            } else {
                format!("{n} B")
            }
        })
        .unwrap_or_else(|| "—".into())
}
#[component]
pub(crate) fn NicHud(
    port: Memo<Port>,
    live: Memo<bool>,
    history: RwSignal<History>,
    on_close: Callback<()>,
) -> impl IntoView {
    let valid = move || live.get() && !port.get().reset;
    view! {
      <section class="nic-hud" role="region" aria-label=move||port.with(|p|format!("Port {} NIC diagnostics",p.id))>
        <header><div><h3>{move||port.with(|p|format!("Port {} · NIC Diagnostics",p.id))}</h3><p>{move||port.with(|p|format!("{} · {}",p.physical.pci.as_deref().unwrap_or(&p.name),p.details.iter().find(|r|r.label=="Duplex").map(|r|r.value.as_str()).unwrap_or("Duplex unknown")))}</p></div>
          <span class=move||format!("chip {}",if !live.get(){"neutral"}else if port.get().state.eq_ignore_ascii_case("up"){"good"}else{"bad"})>{move||if !live.get(){"Stale / unknown".into()}else{format!("Link {}",port.get().state)}}</span>
          <button class="nic-hud-close" aria-label="Close NIC diagnostics" on:click=move|_|on_close.run(())>"×"</button>
        </header>
        <p class="nic-hud-identity">{move||port.with(|p|format!("{} · {} · {}",p.physical.connector.as_deref().unwrap_or("Connector unknown"),p.physical.confidence.as_deref().unwrap_or("unknown confidence"),p.physical.evidence.as_deref().unwrap_or("No physical connector evidence")))}</p>
        <div class="nic-hud-traffic">{["RX","TX"].into_iter().enumerate().map(|(i,label)|view!{
          <div class=if i==0{"nic-hud-rx"}else{"nic-hud-tx"}><h4>{format!("{label} sample")}</h4>
            <strong>{move||port.with(|p|bytes(if valid(){if i==0{p.activity.rx_bytes}else{p.activity.tx_bytes}}else{None}))}</strong>
            <p>{move||port.with(|p|format!("{} packets",integer(if valid(){if i==0{p.activity.rx_packets}else{p.activity.tx_packets}}else{None})))}</p>
            <p>{move||port.with(|p|figures::rates(p,live.get()).map(|r|format!("{} · {:.0} packets/s",rate(r[i]),r[i+2])).unwrap_or("Rate unavailable".into()))}</p>
            <svg viewBox="0 0 240 40" preserveAspectRatio="none" role="img" aria-label=format!("{label} throughput trend, independent scale")><path d=move||history.with(|h|h.path(i))/></svg>
          </div>
        }).collect_view()}</div>
        <div class="nic-hud-section"><h4>"Packet composition · RX"<small>"Cumulative"</small></h4>
          <Show when=move||port.with(|p|figures::composition(p).is_some()) fallback=||view!{<p>"Composition unavailable for these driver counters."</p>}>
            <div class="nic-composition" role="img" aria-label=move||port.with(|p|figures::composition(p).map(|c|format!("Approximate RX composition: {} other, {} broadcast, {} multicast packets",c[0],c[1],c[2])).unwrap_or_default())>
              {(0..3).map(|i|view!{<span style=move||port.with(|p|figures::composition(p).map(|c|format!("flex-grow:{}",c[i])).unwrap_or_default())></span>}).collect_view()}
            </div><p class="nic-composition-legend">"Other · Broadcast · Multicast"</p>
          </Show>
          <p class="panel-note">"Approximate shares. Driver counter semantics can differ."</p>
        </div>
        <div class="nic-hud-section"><h4>"Hardware health counters"<small>"Cumulative totals"</small></h4>
          <div class="nic-hud-health">{[("RX missed","imissed"),("RX errors","ierrors"),("TX errors","oerrors"),("No mbuf","rx_nombuf")].into_iter().map(|(label,key)|view!{
            <div><span>{label}</span><strong>{move||port.with(|p|integer(figures::counter(p,&format!("{key} (total)"))))}</strong><small>{move||port.with(|p|format!("{} / sample",integer(if valid(){figures::counter(p,&format!("{key}/sample"))}else{None})))}</small></div>
          }).collect_view()}</div>
        </div>
        <p class="nic-hud-stamp">{move||port.with(|p|format!("Sample {} · measured interval {} ms{}",p.activity.sampled_at.map(local_time).unwrap_or_else(||"unknown".into()),integer(p.activity.interval_ms),if p.reset{" · Counter reset"}else if !live.get(){" · Last known counters"}else{""}))}</p>
        <details class="nic-hud-raw"><summary>"NIC counter details"</summary><dl>{move||port.with(|p|p.details.iter().map(|r|view!{<dt>{r.label.clone()}</dt><dd>{r.value.clone()}</dd>}).collect_view())}</dl></details>
        <p class="panel-note">"Recent rates use up to 60 samples from this browser session. RX/TX are physical port directions. Published counters only; use the TUI for additional driver counters."</p>
      </section>
    }
}
