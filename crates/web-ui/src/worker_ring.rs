//! DPDK health ring: the worker execution ring after `worker-radial-profiler-v3.svg`, with an outer
//! band for analytics, classification, and bridge health. Hover or focus keeps the view radial:
//! the sector or band grows and the chip shows its value. A new band value pulses and shows briefly.
use crate::dpdk_console_model::{Segment, count, details, segments, uptime};
use crate::worker_ring_model::{CX, CY, RADIUS, Sector, band, compact, grouped, pick, sectors};
use leptos::prelude::*;
use web_contract::dpdk::{Console, Dpdk};

/// The focused instrument: a phase sector or a health band.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Phase(usize),
    Health(usize),
}

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
    let hovered = RwSignal::new(None::<Target>);
    let pinned = RwSignal::new(None::<Target>);
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
    let health = Memo::new(move |_| {
        let empty = Console::default();
        snapshot.with(|d| segments(d.as_ref().map_or(&empty, |d| &d.console)))
    });
    let ages = Memo::new(move |_| {
        let now = server_clock.get();
        snapshot.with(|d| {
            d.as_ref()
                .map(|d| {
                    (
                        d.analytics.updated.and_then(|t| now.checked_sub(t)),
                        d.health.updated.and_then(|t| now.checked_sub(t)),
                    )
                })
                .unwrap_or_default()
        })
    });
    let ticks = move || {
        health
            .get()
            .into_iter()
            .enumerate()
            .collect::<Vec<(usize, Segment)>>()
    };
    let tick_key = |(k, s): &(usize, Segment)| (*k, s.value.clone());
    // A second click on the pinned instrument resets the view. Hover stays muted on it until the pointer leaves.
    let muted = RwSignal::new(None::<Target>);
    let enter = move |t: Target| {
        if muted.get_untracked() != Some(t) {
            hovered.set(Some(t));
        }
    };
    let toggle = move |i: Target| {
        if pinned.get_untracked() == Some(i) {
            pinned.set(None);
            hovered.set(None);
            muted.set(Some(i));
        } else {
            pinned.set(Some(i));
        }
    };
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
          <h2 id="worker-ring-title">"DPDK Health"</h2>
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
        <div class="worker-ring-body">
        <svg class="worker-ring" class:active=move||over.get()||focus.with(Option::is_some) class:stale=move||!live.get()
          viewBox="-28 0 576 400" role="group" aria-label=move||format!("DPDK health: worker {} execution phases and bridge instruments",name())
          on:pointerenter=move|_|over.set(true) on:pointerleave=move|_|{over.set(false);hovered.set(None);}
          on:focusin=move|_|over.set(true)
          on:focusout=move|ev|{
              let inside=ev.related_target().and_then(|t|wasm_bindgen::JsCast::dyn_into::<web_sys::Node>(t).ok())
                  .zip(ev.current_target().and_then(|t|wasm_bindgen::JsCast::dyn_into::<web_sys::Node>(t).ok()))
                  .is_some_and(|(to,me)|me.contains(Some(&to)));
              if !inside {over.set(false);hovered.set(None);}
          }
          on:keydown=move|ev|{if ev.key()=="Escape"{pinned.set(None);hovered.set(None);}}>
          <circle class="worker-ring-guide" cx=CX cy=CY r=RADIUS+22.0/>
          <g class="health-ring">
            {(0..9).map(|k|{let t=Target::Health(k);let s=move||health.with(|h|h[k].clone());view!{
              <path id=format!("health-band-{k}") d=band(k) class=move||format!("health-band {}",s().tone.class())
                class:selected=move||focus.get()==Some(t) class:dimmed=move||focus.get().is_some_and(|f|f!=t)
                tabindex="0" role="button" aria-pressed=move||(pinned.get()==Some(t)).to_string()
                aria-label=move||{let s=s();format!("{}: {}",s.name,s.value)}
                on:pointerenter=move|_|enter(t) on:pointerleave=move|_|{hovered.set(None);muted.set(None);}
                on:focus=move|_|enter(t) on:click=move|_|toggle(t)
                on:keydown=move|ev|{if ev.key()=="Enter"||ev.key()==" "{ev.prevent_default();toggle(t);}}/>
            }}).collect_view()}
            // Keyed by value: a new value recreates the group, so the pulse and the value flash run again.
            <For each=ticks key=tick_key
              children=move|(k,s):(usize,Segment)|view!{
                <g class=format!("health-tick {}",s.tone.class()) class:show=move||focus.get()==Some(Target::Health(k)) aria-hidden="true">
                  <path class="health-pulse" d=band(k)/>
                  <text class="health-label"><textPath href=format!("#health-band-{k}") startOffset="50%">{s.label}</textPath></text>
                  <text class="health-value"><textPath href=format!("#health-band-{k}") startOffset="50%">{s.value}</textPath></text>
                </g>
              }/>
          </g>
          <g class="worker-ring-sectors">
            <For each=move||ring.get() key=|s:&Sector|s.index children=move|s:Sector|{
              let i=s.index;
              let t=Target::Phase(i);
              view!{<path class=format!("worker-sector phase-{i}") d=s.arc.clone()
                class:selected=move||focus.get()==Some(t) class:dimmed=move||focus.get().is_some_and(|f|f!=t)
                tabindex="0" role="button" aria-pressed=move||(pinned.get()==Some(t)).to_string()
                aria-label=move||ring.with(|r|format!("{}: {} cycles. {}",r[i].label,value(r[i].cycles),r[i].description))
                on:pointerenter=move|_|enter(t)
                on:pointerleave=move|_|{hovered.set(None);muted.set(None);}
                on:focus=move|_|enter(t) on:click=move|_|toggle(t)
                on:keydown=move|ev|{if ev.key()=="Enter"||ev.key()==" "{ev.prevent_default();toggle(t);}}/>}
            }/>
          </g>
          <g class="worker-chip" aria-hidden="true">
            <defs><linearGradient id="worker-chip-metal" x1="0" x2="0" y1="0" y2="1"><stop offset="0" stop-color="#466d8b"/><stop offset=".48" stop-color="#162b40"/><stop offset="1" stop-color="#071523"/></linearGradient></defs>
            <circle cx=CX cy=CY r=RADIUS-24.0 class="worker-chip-well"/>
            <circle cx=CX cy=CY r=RADIUS-31.0 class="worker-busy-track"/>
            <circle cx=CX cy=CY r=RADIUS-31.0 class="worker-busy-arc" transform=format!("rotate(-90 {CX} {CY})")
              stroke-dasharray=move||worker.with(|w|{let c=2.0*std::f64::consts::PI*(RADIUS-31.0);let f=w.as_ref().map(|w|w.busy_ppm.min(1_000_000) as f64/1_000_000.0).unwrap_or(0.0);format!("{:.1} {:.1}",c*f,c)})/>
            {(0..6).flat_map(|k|{let c=-42.5+17.0*k as f64;[(CX+c-4.0,CY-72.0,8.0,16.0),(CX+c-4.0,CY+56.0,8.0,16.0),(CX-72.0,CY+c-4.0,16.0,8.0),(CX+56.0,CY+c-4.0,16.0,8.0)]})
              .map(|(x,y,w,h)|view!{<rect class="worker-chip-pin" x=x y=y width=w height=h rx="2"/>}).collect_view()}
            <rect x=CX-58.0 y=CY-58.0 width="116" height="116" rx="10" class="worker-chip-package" fill="url(#worker-chip-metal)"/>
            <rect x=CX-50.0 y=CY-50.0 width="100" height="100" rx="5" class="worker-chip-core"/>
            {[(-44.0,-44.0),(44.0,-44.0),(-44.0,44.0),(44.0,44.0)].into_iter().map(|(dx,dy)|view!{<circle class="worker-chip-dot" cx=CX+dx cy=CY+dy r="2.2"/>}).collect_view()}
            <path class="worker-chip-traces" d=format!("M{a} {t}h10m56 0h10M{a} {u}h10m56 0h10M{a} {v}h10m56 0h10M{a} {w}h10m56 0h10",a=CX-43.0,t=CY-36.0,u=CY-28.0,v=CY+28.0,w=CY+36.0)/>
            <rect x=CX-33.0 y=CY-38.0 width="66" height="78" rx="5" class="worker-chip-die"/>
            {move||match focus.get() {
              None=>view!{
                <text x=CX y=CY-12.0 class="worker-chip-id">{name()}</text>
                <text x=CX y=CY+20.0 class="worker-chip-busy">{move||worker.with(|w|pct(w.as_ref().map(|w|w.busy_ppm)))}</text>
                <text x=CX y=CY+38.0 class="worker-chip-note">"BUSY"</text>
              }.into_any(),
              Some(Target::Health(k))=>view!{
                <text x=CX y=CY-14.0 class="worker-chip-phase">{move||health.with(|h|h[k].label)}</text>
                <text x=CX y=CY+8.0 class="worker-chip-cycles">{move||health.with(|h|h[k].value.clone())}</text>
                <text x=CX y=CY+26.0 class="worker-chip-note">"HEALTH"</text>
              }.into_any(),
              Some(Target::Phase(i))=>view!{
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
              view!{<g class=format!("worker-callout phase-{i}") class:dimmed=move||focus.get().is_some_and(|f|f!=Target::Phase(i))>
                <polyline points=points/>
                <circle cx=s.anchor.0 cy=s.anchor.1 r="3"/>
                <text x=tx y=s.end.1-3.0 text-anchor=anchor class="worker-callout-name">{s.label}</text>
                <text x=tx y=s.end.1+11.0 text-anchor=anchor class="worker-callout-num">{s.cycles.map(compact).unwrap_or("unknown".into())}</text>
              </g>}
            }/>
          </g>
        </svg>
        <div class="worker-ring-side">
        <p class="worker-focus" aria-live="polite">{move||match focus.get() {
          Some(Target::Phase(i))=>ring.with(|r|format!("{} · {} cumulative cycles · {}",r[i].label,value(r[i].cycles),r[i].description)),
          Some(Target::Health(k))=>{let (a,b)=ages.get();let t=health.with(|h|h[k].topic);snapshot.with(|d|{let empty=Console::default();details(t,d.as_ref().map_or(&empty,|d|&d.console),a,b).join(" · ")})}
          None=>"Hover or focus a phase or a health band · click to pin · Esc to clear".into(),
        }}</p>
        <dl class="worker-stats">
          <div><dt>"Idle (derived)"</dt><dd>{move||worker.with(|w|pct(w.as_ref().map(|w|1_000_000-w.busy_ppm.min(1_000_000))))}</dd></div>
          <div><dt>"Laps/s"</dt><dd>{move||worker.with(|w|value(w.as_ref().and_then(|w|w.laps_per_s)))}</dd></div>
          <div><dt>"Cycles/busy lap"</dt><dd>{move||worker.with(|w|value(w.as_ref().and_then(|w|w.cycles_per_busy_lap)))}</dd></div>
          <div><dt>"Mean backlog"</dt><dd>{move||worker.with(|w|w.as_ref().and_then(|w|w.backlog_mean).map(|v|format!("{} pkts",grouped(v))).unwrap_or("—".into()))}</dd></div>
        </dl>
        <p class="health-line">{move||{let (a,b)=ages.get();snapshot.with(|d|d.as_ref().map(|d|{let c=&d.console;format!("Bridge {}{} · up {} · {} rollups missing · analytics {} · bridge {}",
          c.status.as_deref().unwrap_or("unknown"),c.version.as_deref().map(|v|format!(" {v}")).unwrap_or_default(),uptime(c.uptime_ms),count(c.missing_rollups),
          a.map(|a|format!("{a}s")).unwrap_or("—".into()),b.map(|b|format!("{b}s")).unwrap_or("—".into()))}).unwrap_or("Bridge health unavailable".into()))}}</p>
        <p class="panel-note">{move||snapshot.with(|d|d.as_ref().map(|d|format!("{} workers · mean busy {} · equal-angle phases, cumulative cycles, not CPU-time shares",d.worker_count,pct(d.mean_busy_ppm))).unwrap_or("Worker telemetry unavailable".into()))}</p>
        </div>
        </div>
      </article>
    }
}
