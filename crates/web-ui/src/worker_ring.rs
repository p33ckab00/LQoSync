//! Worker execution ring after the reviewed `worker-radial-profiler-v3.svg` reference.
//! Hover or focus keeps the view radial: the sector grows on the ring and the chip shows its counter.
use crate::worker_ring_model::{CX, CY, RADIUS, Sector, compact, grouped, pick, sectors};
use leptos::prelude::*;
use web_contract::dpdk::Dpdk;

fn pct(n: Option<u64>) -> String {
    n.map(|n| format!("{:.1}%", n as f64 / 10000.0))
        .unwrap_or("—".into())
}
fn value(n: Option<u64>) -> String {
    n.map(grouped).unwrap_or("—".into())
}

#[component]
pub(crate) fn WorkerRing(snapshot: Memo<Option<Dpdk>>, server_clock: Memo<u64>) -> impl IntoView {
    let selected = RwSignal::new(None::<u16>);
    let hovered = RwSignal::new(None::<usize>);
    let pinned = RwSignal::new(None::<usize>);
    let over = RwSignal::new(false);
    let focus = Memo::new(move |_| pinned.get().or(hovered.get()));
    let live = Memo::new(move |_| {
        snapshot.with(|d| {
            d.as_ref()
                .is_some_and(|d| d.analytics.live(server_clock.get(), 6))
        })
    });
    let worker = Memo::new(move |_| {
        snapshot.with(|d| {
            d.as_ref()
                .and_then(|d| pick(&d.workers, selected.get()).cloned())
        })
    });
    let ring = Memo::new(move |_| worker.with(|w| sectors(w.as_ref())));
    let ids = Memo::new(move |_| {
        snapshot.with(|d| {
            d.as_ref()
                .map(|d| d.workers.iter().map(|w| w.id).collect::<Vec<_>>())
                .unwrap_or_default()
        })
    });
    let toggle = move |i: usize| pinned.update(|p| *p = if *p == Some(i) { None } else { Some(i) });
    let name = move || {
        worker.with(|w| {
            w.as_ref()
                .map(|w| format!("W{}", w.id))
                .unwrap_or("W—".into())
        })
    };
    view! {
      <article class="panel worker-ring-card" aria-labelledby="worker-ring-title">
        <div class="worker-ring-head">
          <h2 id="worker-ring-title">"Worker"</h2>
          <span class=move||format!("chip {}",if live.get(){"good"}else{"neutral"})>{move||if live.get(){"Live"}else if snapshot.with(|d|d.is_none()){"Unavailable"}else{"Stale · last known"}}</span>
        </div>
        <Show when=move||ids.with(|v|v.len()>1)>
          <div class="worker-picker" role="group" aria-label="Select worker">
            {move||ids.get().into_iter().map(|id|view!{
              <button type="button" aria-pressed=move||(worker.with(|w|w.as_ref().map(|w|w.id))==Some(id)).to_string()
                on:click=move|_|{selected.set(Some(id));pinned.set(None);}>{format!("W{id}")}</button>
            }).collect_view()}
          </div>
        </Show>
        <svg class="worker-ring" class:active=move||over.get()||focus.with(Option::is_some) class:stale=move||!live.get()
          viewBox="0 0 520 392" role="group" aria-label=move||format!("Worker {} execution phases",name())
          on:pointerenter=move|_|over.set(true) on:pointerleave=move|_|{over.set(false);hovered.set(None);}
          on:focusin=move|_|over.set(true)
          on:focusout=move|ev|{
              let inside=ev.related_target().and_then(|t|wasm_bindgen::JsCast::dyn_into::<web_sys::Node>(t).ok())
                  .zip(ev.current_target().and_then(|t|wasm_bindgen::JsCast::dyn_into::<web_sys::Node>(t).ok()))
                  .is_some_and(|(to,me)|me.contains(Some(&to)));
              if !inside {over.set(false);hovered.set(None);}
          }
          on:keydown=move|ev|{if ev.key()=="Escape"{pinned.set(None);hovered.set(None);}}>
          <circle class="worker-ring-guide" cx=CX cy=CY r=RADIUS+34.0/>
          <g class="worker-ring-sectors">
            <For each=move||ring.get() key=|s:&Sector|s.index children=move|s:Sector|{
              let i=s.index;
              view!{<path class=format!("worker-sector phase-{i}") d=s.arc.clone()
                class:selected=move||focus.get()==Some(i) class:dimmed=move||focus.get().is_some_and(|f|f!=i)
                tabindex="0" role="button" aria-pressed=move||(pinned.get()==Some(i)).to_string()
                aria-label=move||ring.with(|r|format!("{}: {} cycles. {}",r[i].label,value(r[i].cycles),r[i].description))
                on:pointerenter=move|_|hovered.set(Some(i)) on:pointerleave=move|_|hovered.set(None)
                on:focus=move|_|hovered.set(Some(i)) on:click=move|_|toggle(i)
                on:keydown=move|ev|{if ev.key()=="Enter"||ev.key()==" "{ev.prevent_default();toggle(i);}}/>}
            }/>
          </g>
          <g class="worker-chip" aria-hidden="true">
            <circle cx=CX cy=CY r=RADIUS-24.0 class="worker-chip-well"/>
            <rect x=CX-56.0 y=CY-56.0 width="112" height="112" rx="10" class="worker-chip-die"/>
            <rect x=CX-46.0 y=CY-46.0 width="92" height="92" rx="6" class="worker-chip-core"/>
            {move||match focus.get() {
              None=>view!{
                <text x=CX y=CY-12.0 class="worker-chip-id">{name()}</text>
                <text x=CX y=CY+20.0 class="worker-chip-busy">{move||worker.with(|w|pct(w.as_ref().map(|w|w.busy_ppm)))}</text>
                <text x=CX y=CY+38.0 class="worker-chip-note">"BUSY"</text>
              }.into_any(),
              Some(i)=>view!{
                <text x=CX y=CY-14.0 class="worker-chip-phase">{move||ring.with(|r|r[i].label)}</text>
                <text x=CX y=CY+8.0 class="worker-chip-cycles">{move||ring.with(|r|r[i].cycles.map(compact).unwrap_or("—".into()))}</text>
                <text x=CX y=CY+26.0 class="worker-chip-note">"CYCLES"</text>
              }.into_any(),
            }}
          </g>
          <g class="worker-callouts" aria-hidden="true">
            <For each=move||ring.get() key=|s:&Sector|(s.index,s.cycles) children=move|s:Sector|{
              let i=s.index;
              let points=format!("{},{} {},{} {},{}",s.anchor.0,s.anchor.1,s.elbow.0,s.elbow.1,s.end.0,s.end.1);
              let (tx,anchor)=if s.left {(s.end.0-6.0,"end")} else {(s.end.0+6.0,"start")};
              view!{<g class=format!("worker-callout phase-{i}") class:dimmed=move||focus.get().is_some_and(|f|f!=i)>
                <polyline points=points/>
                <circle cx=s.anchor.0 cy=s.anchor.1 r="3"/>
                <text x=tx y=s.end.1-3.0 text-anchor=anchor class="worker-callout-name">{s.label}</text>
                <text x=tx y=s.end.1+11.0 text-anchor=anchor class="worker-callout-num">{s.cycles.map(compact).unwrap_or("unknown".into())}</text>
              </g>}
            }/>
          </g>
        </svg>
        <p class="worker-focus" aria-live="polite">{move||match focus.get() {
          Some(i)=>ring.with(|r|format!("{} · {} cumulative cycles · {}",r[i].label,value(r[i].cycles),r[i].description)),
          None=>"Hover or focus a phase · click to pin · Esc to clear".into(),
        }}</p>
        <dl class="worker-stats">
          <div><dt>"Idle (derived)"</dt><dd>{move||worker.with(|w|pct(w.as_ref().map(|w|1_000_000-w.busy_ppm.min(1_000_000))))}</dd></div>
          <div><dt>"Laps/s"</dt><dd>{move||worker.with(|w|value(w.as_ref().and_then(|w|w.laps_per_s)))}</dd></div>
          <div><dt>"Cycles/busy lap"</dt><dd>{move||worker.with(|w|value(w.as_ref().and_then(|w|w.cycles_per_busy_lap)))}</dd></div>
          <div><dt>"Mean backlog"</dt><dd>{move||worker.with(|w|w.as_ref().and_then(|w|w.backlog_mean).map(|v|format!("{} pkts",grouped(v))).unwrap_or("—".into()))}</dd></div>
        </dl>
        <p class="panel-note">{move||snapshot.with(|d|d.as_ref().map(|d|format!("{} workers · mean busy {} · equal-angle phases, cumulative cycles, not CPU-time shares",d.worker_count,pct(d.mean_busy_ppm))).unwrap_or("Worker telemetry unavailable".into()))}</p>
      </article>
    }
}
