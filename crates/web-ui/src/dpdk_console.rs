//! DPDK health console after the reviewed `dpdk-radial-console.svg` reference.
//! Hover or focus highlights every instrument of one topic and shows its details. Click pins; a second click resets.
use crate::dpdk_console_model::{
    Tone, Topic, count, details, per_sample_tone, pool_free_pct, pool_tone, pressure_tone,
    ring_pct, status_tone, uptime,
};
use crate::worker_ring_model::grouped;
use leptos::prelude::*;
use web_contract::dpdk::{Console, Dpdk};

const C: f64 = 160.0;
const R_POOL: f64 = 136.0;
const R_RING: f64 = 114.0;

#[derive(Clone, Copy)]
struct Focus {
    hovered: RwSignal<Option<Topic>>,
    pinned: RwSignal<Option<Topic>>,
    muted: RwSignal<Option<Topic>>,
}
impl Focus {
    fn current(self) -> Option<Topic> {
        self.pinned.get().or(self.hovered.get())
    }
    fn enter(self, t: Topic) {
        if self.muted.get_untracked() != Some(t) {
            self.hovered.set(Some(t));
        }
    }
    fn leave(self) {
        self.hovered.set(None);
        self.muted.set(None);
    }
    fn toggle(self, t: Topic) {
        if self.pinned.get_untracked() == Some(t) {
            self.pinned.set(None);
            self.hovered.set(None);
            self.muted.set(Some(t));
        } else {
            self.pinned.set(Some(t));
        }
    }
}

#[component]
fn Tile(
    focus: Focus,
    topic: Topic,
    kind: &'static str,
    label: &'static str,
    #[prop(into)] value: Signal<String>,
    #[prop(into)] note: Signal<String>,
    #[prop(into)] tone: Signal<Tone>,
) -> impl IntoView {
    view! {
      <button type="button" class=move||format!("console-tile {kind} {}",tone.get().class())
        class:selected=move||focus.current()==Some(topic) class:dimmed=move||focus.current().is_some_and(|f|f!=topic)
        aria-pressed=move||(focus.pinned.get()==Some(topic)).to_string()
        aria-label=move||format!("{label}: {}. {}",value.get(),note.get())
        on:pointerenter=move|_|focus.enter(topic) on:pointerleave=move|_|focus.leave()
        on:focus=move|_|focus.enter(topic) on:blur=move|_|focus.hovered.set(None)
        on:click=move|_|focus.toggle(topic)>
        <span class="tile-label">{label}</span>
        <b class="tile-value">{value}</b>
        <small class="tile-note">{note}</small>
      </button>
    }
}

/// Dash pattern and offset for an arc that starts at `from` and covers `len`, both as shares of the circle.
fn arc(r: f64, from: f64, len: f64) -> (String, String) {
    let circumference = 2.0 * std::f64::consts::PI * r;
    (
        format!("{:.2} {:.2}", circumference * len, circumference),
        format!("{:.2}", -circumference * from),
    )
}
fn age(a: Option<u64>) -> String {
    a.map(|a| format!("{a}s")).unwrap_or("—".into())
}
fn ms(m: Option<u64>) -> String {
    m.map(|m| format!("{} ms", grouped(m)))
        .unwrap_or("—".into())
}

#[component]
pub(crate) fn DpdkConsole(snapshot: Memo<Option<Dpdk>>, server_clock: Memo<u64>) -> impl IntoView {
    let console = Memo::new(move |_| {
        snapshot.with(|d| d.as_ref().map(|d| d.console.clone()).unwrap_or_default())
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
    let focus = Focus {
        hovered: RwSignal::new(None),
        pinned: RwSignal::new(None),
        muted: RwSignal::new(None),
    };
    let c = move |f: fn(&Console) -> String| Signal::derive(move || console.with(f));
    let tone = move |f: fn(&Console) -> Tone| Signal::derive(move || console.with(f));
    let fixed = |s: &'static str| Signal::derive(move || s.to_string());
    let pool = move || console.with(|c| pool_free_pct(c).map(|p| p / 100.0).unwrap_or(0.0));
    let rings = move || console.with(|c| ring_pct(c).map(|p| p / 100.0));
    view! {
      <section class="panel console-card" aria-labelledby="console-title"
        on:keydown=move|ev|{if ev.key()=="Escape"{focus.pinned.set(None);focus.hovered.set(None);}}>
        <div class="console-head">
          <h2 id="console-title">"DPDK health console"</h2>
          <span class=move||format!("console-status {}",console.with(status_tone).class())><i aria-hidden="true"></i>
            {move||console.with(|c|match c.status.as_deref(){Some(s)=>format!("Bridge {}",s.to_ascii_lowercase()),None=>"Bridge unknown".into()})}</span>
        </div>
        <p class="console-sub">{move||{let (a,b)=ages.get();format!("Analytics age {} · Bridge age {} · hover, focus, or click an instrument",age(a),age(b))}}</p>
        <div class="console-core">
          <div class="console-side">
            <Tile focus topic=Topic::Buffer kind="side" label="Buffer pool" tone=tone(pool_tone)
              value=c(|c|pool_free_pct(c).map(|p|format!("{p:.2}% free")).unwrap_or("—".into()))
              note=c(|c|format!("{} / {}",count(c.pool_free),count(c.pool_total)))/>
            <Tile focus topic=Topic::Rings kind="side" label="Handoff rings"
              tone=tone(|c|match ring_pct(c){Some(p) if p>=75.0=>Tone::Warn,Some(_)=>Tone::Info,None=>Tone::Neutral})
              value=c(|c|ring_pct(c).map(|p|format!("{p:.1}% occupancy")).unwrap_or("N/A occupancy".into()))
              note=c(|c|format!("{} rings · {} slots / {} capacity",count(c.ring_count),count(c.ring_used),count(c.ring_capacity)))/>
          </div>
          <div class="console-center">
            <svg viewBox="0 0 320 320" class="console-rings" role="img"
              aria-label=move||console.with(|c|format!("Buffer pool {} free. Handoff ring occupancy {}.",pool_free_pct(c).map(|p|format!("{p:.2}%")).unwrap_or("unknown".into()),ring_pct(c).map(|p|format!("{p:.1}%")).unwrap_or("not available".into())))>
              <defs><linearGradient id="console-chip-metal" x2="1" y2="1"><stop stop-color="#355772"/><stop offset="1" stop-color="#101d2b"/></linearGradient></defs>
              <circle cx=C cy=C r="152" class="console-guide"/>
              <circle cx=C cy=C r=R_POOL class="console-track"/>
              <circle cx=C cy=C r=R_RING class="console-track"/>
              <g transform=format!("rotate(-90 {C} {C})")>
                <circle cx=C cy=C r=R_POOL class="console-arc tone-good" class:active=move||focus.current()==Some(Topic::Buffer)
                  stroke-dasharray=move||arc(R_POOL,0.0,pool()).0
                  on:pointerenter=move|_|focus.enter(Topic::Buffer) on:pointerleave=move|_|focus.leave() on:click=move|_|focus.toggle(Topic::Buffer)/>
                <circle cx=C cy=C r=R_POOL class="console-arc tone-warn" class:active=move||focus.current()==Some(Topic::Buffer)
                  stroke-dasharray=move||{let p=pool();arc(R_POOL,p,if p>0.0{1.0-p}else{0.0}).0} stroke-dashoffset=move||arc(R_POOL,pool(),0.0).1
                  on:pointerenter=move|_|focus.enter(Topic::Buffer) on:pointerleave=move|_|focus.leave() on:click=move|_|focus.toggle(Topic::Buffer)/>
                <Show when=move||rings().is_some()>
                  <circle cx=C cy=C r=R_RING class="console-arc tone-info" class:active=move||focus.current()==Some(Topic::Rings)
                    stroke-dasharray=move||arc(R_RING,0.0,rings().unwrap_or(0.0)).0
                    on:pointerenter=move|_|focus.enter(Topic::Rings) on:pointerleave=move|_|focus.leave() on:click=move|_|focus.toggle(Topic::Rings)/>
                </Show>
              </g>
              <circle cx=C cy=C r="100" class="console-dots"/>
              <g class="console-chip" aria-hidden="true">
                {(0..6).flat_map(|k|{let o=C-36.0+12.0*k as f64;[(o,C-73.0,6.0,13.0),(o,C+60.0,6.0,13.0),(C-73.0,o,13.0,6.0),(C+60.0,o,13.0,6.0)]})
                  .map(|(x,y,w,h)|view!{<rect class="console-pin" x=x y=y width=w height=h rx="1"/>}).collect_view()}
                <rect x=C-61.0 y=C-61.0 width="122" height="122" rx="15" fill="url(#console-chip-metal)" class="console-package"/>
                <rect x=C-48.0 y=C-48.0 width="96" height="96" rx="10" class="console-die"/>
                <path class="console-traces" d=format!("M{a} {b}h80M{a} {d}h80M{e} {f}v80M{g} {f}v80",a=C-40.0,b=C-22.0,d=C+24.0,e=C-25.0,f=C-40.0,g=C+25.0)/>
                <text x=C y=C-22.0 class="console-chip-label">"BRIDGE CORE"</text>
                <text x=C y=C+5.0 class=move||format!("console-chip-status {}",console.with(status_tone).class())>{move||console.with(|c|c.status.clone().map(|s|s.to_ascii_uppercase()).unwrap_or("UNKNOWN".into()))}</text>
                <text x=C y=C+25.0 class="console-chip-workers">{move||console.with(|c|match c.workers{Some(1)=>"1 worker".into(),Some(n)=>format!("{n} workers"),None=>"workers unknown".into()})}</text>
                <text x=C y=C+41.0 class="console-chip-version">{move||console.with(|c|c.version.as_deref().map(|v|if v.starts_with(|c:char|c.is_ascii_digit()){format!("v{v}")}else{v.into()}).unwrap_or_default())}</text>
              </g>
            </svg>
            <div class="console-detail" class:show=move||focus.current().is_some() aria-live="polite">
              {move||focus.current().map(|t|{let (a,b)=ages.get();let d=console.with(|c|details(t,c,a,b));view!{
                <b>{d[0].clone()}</b><span>{d[1].clone()}</span><span>{d[2].clone()}</span><span>{d[3].clone()}</span>
              }})}
            </div>
          </div>
          <div class="console-side">
            <Tile focus topic=Topic::Queue kind="side" label="Queue backlog" tone=Tone::Info
              value=c(|c|c.backlog_bytes.map(|b|format!("{} B",grouped(b))).unwrap_or("—".into()))
              note=c(|c|format!("{} drops / sample",count(c.queue_drops_per_sample)))/>
            <Tile focus topic=Topic::Rollup kind="side" label="Rollup freshness" tone=Tone::Violet
              value=c(|c|ms(c.oldest_rollup_ms))
              note=Signal::derive(move||format!("{} missing · bridge age {}",console.with(|c|count(c.missing_rollups)),age(ages.get().1)))/>
          </div>
        </div>
        <div class="console-row-head"><h3>"Live event & queue instruments"</h3><span>"/sample = producer sampling interval, not per second"</span></div>
        <div class="console-grid">
          <Tile focus topic=Topic::Queue kind="event" label="Queue drops" tone=tone(|c|per_sample_tone(c.queue_drops_per_sample)) value=c(|c|format!("{} /sample",count(c.queue_drops_per_sample))) note=fixed("current sample")/>
          <Tile focus topic=Topic::Queue kind="event" label="AQM marks" tone=tone(|c|per_sample_tone(c.aqm_marks_per_sample)) value=c(|c|format!("{} /sample",count(c.aqm_marks_per_sample))) note=fixed("current sample")/>
          <Tile focus topic=Topic::Buffer kind="event" label="RX exhaust" tone=tone(|c|per_sample_tone(c.rx_nombuf_per_sample)) value=c(|c|format!("{} /sample",count(c.rx_nombuf_per_sample))) note=fixed("current sample")/>
          <Tile focus topic=Topic::Buffer kind="event" label="Pool drops" tone=tone(|c|per_sample_tone(c.pool_drops_per_sample)) value=c(|c|format!("{} /sample",count(c.pool_drops_per_sample))) note=fixed("current sample")/>
          <Tile focus topic=Topic::Rings kind="event" label="Pressure" tone=tone(pressure_tone) value=c(|c|c.pressure.clone().unwrap_or("UNKNOWN".into())) note=fixed("buffer and ring verdict")/>
        </div>
        <div class="console-row-head"><h3>"Bridge & classification"</h3></div>
        <div class="console-grid six">
          <Tile focus topic=Topic::Bridge kind="fact" label="Uptime" tone=tone(status_tone) value=c(|c|uptime(c.uptime_ms)) note=c(|c|c.uptime_ms.map(|m|format!("{} ms",grouped(m))).unwrap_or_default())/>
          <Tile focus topic=Topic::Bridge kind="fact" label="Workers" tone=tone(status_tone) value=c(|c|count(c.workers)) note=fixed("reported by the bridge")/>
          <Tile focus topic=Topic::Rollup kind="fact" label="Missing rollups" tone=tone(|c|per_sample_tone(c.missing_rollups)) value=c(|c|count(c.missing_rollups)) note=fixed("per-worker summaries")/>
          <Tile focus topic=Topic::Rollup kind="fact" label="Oldest rollup" tone=Tone::Violet value=c(|c|ms(c.oldest_rollup_ms)) note=fixed("age of the oldest")/>
          <Tile focus topic=Topic::Bridge kind="fact" label="AQM degraded" tone=tone(|c|per_sample_tone(c.aqm_degraded_workers)) value=c(|c|count(c.aqm_degraded_workers)) note=fixed("workers")/>
          <Tile focus topic=Topic::Bridge kind="fact" label="Classification" tone=tone(|c|match c.classification_empty{Some(false)=>Tone::Good,Some(true)=>Tone::Warn,None=>Tone::Neutral})
            value=c(|c|match c.classification_empty{Some(false)=>"populated".into(),Some(true)=>"empty".into(),None=>"—".into()}) note=fixed("mapped hosts")/>
        </div>
        <div class="console-row-head"><h3>"Cumulative counters"</h3></div>
        <div class="console-grid">
          <Tile focus topic=Topic::Queue kind="counter" label="Queue drops total" tone=Tone::Neutral value=c(|c|count(c.queue_drops_total)) note=fixed("")/>
          <Tile focus topic=Topic::Queue kind="counter" label="AQM marks total" tone=Tone::Neutral value=c(|c|count(c.aqm_marks_total)) note=fixed("")/>
          <Tile focus topic=Topic::Rings kind="counter" label="Rings above 75%" tone=Tone::Neutral value=c(|c|count(c.rings_over_75)) note=fixed("")/>
          <Tile focus topic=Topic::Bridge kind="counter" label="Bridge source age" tone=Tone::Neutral value=Signal::derive(move||age(ages.get().1)) note=fixed("")/>
          <Tile focus topic=Topic::Buffer kind="counter" label="Analytics source age" tone=Tone::Neutral value=Signal::derive(move||age(ages.get().0)) note=fixed("")/>
        </div>
      </section>
    }
}
