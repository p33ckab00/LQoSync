//! One comparison panel. Pointer inspection never reads the network.
use crate::{
    dashboard::{axis_rate, local_time, rate},
    traffic_plot as plot,
};
use leptos::prelude::*;
use web_contract::{Overview, Point, TrafficWindow, WanCapacity};

const NAMES: [&str; 3] = ["Actual", "Mapped", "Unmapped"];
const CLASSES: [&str; 3] = ["actual", "mapped", "unmapped"];
#[derive(Clone, Default, PartialEq)]
struct Model {
    series: [Vec<Point>; 3],
    states: [&'static str; 3],
    capacity: Option<WanCapacity>,
    capped: bool,
    times: Vec<u64>,
    /// The largest spacing that still joins two samples: 3 s live, two buckets in a history window.
    step: u64,
}
impl Model {
    fn read(view: Option<&Overview>) -> Self {
        let Some(v) = view else {
            return Self::default();
        };
        let mut model = Self {
            series: [v.traffic.clone(), vec![], vec![]],
            states: [
                if v.traffic_stale { "Stale" } else { "Observed" },
                "Unavailable",
                "Unavailable",
            ],
            capacity: v.wan_capacity.clone(),
            capped: false,
            times: Vec::new(),
            step: 3,
        };
        if let Some(c) = &v.classification {
            for (i, s) in [(1, &c.mapped), (2, &c.unmapped)] {
                model.series[i] = s.points.clone();
                model.states[i] = if !s.available {
                    "Unavailable"
                } else if s.stale {
                    "Stale / gap"
                } else {
                    "Observed"
                };
            }
            model.capped = c.history_capped;
        }
        model.index();
        model
    }
    /// Use a bucketed history window in place of the live samples. Source states stay live.
    fn with_window(mut self, w: Option<&TrafficWindow>) -> Self {
        self.series = w
            .map(|w| [w.actual.clone(), w.mapped.clone(), w.unmapped.clone()])
            .unwrap_or_default();
        self.capped = false;
        self.step = w.map_or(3, |w| w.bucket_secs.saturating_mul(2));
        self.index();
        self
    }
    fn index(&mut self) {
        self.times = self.series.iter().flatten().map(|p| p.at).collect();
        self.times.sort_unstable();
        self.times.dedup();
    }
}
const RANGES: [&str; 4] = ["Recent", "1h", "6h", "24h"];
/// Read one history window. A failed read keeps the previous window and its time.
async fn fetch_window(window: &'static str) -> Option<TrafficWindow> {
    let response = gloo_net::http::Request::get(&format!("/api/v1/traffic?window={window}"))
        .send()
        .await
        .ok()?;
    if !response.ok() {
        return None;
    }
    response.json::<TrafficWindow>().await.ok()
}
#[derive(Clone, Default, PartialEq)]
struct Geometry {
    domain: (u64, u64),
    max: [f64; 2],
    /// Visible peak in Mbps and its plot y. Red marks this peak.
    peak: [f64; 2],
    peak_y: [f64; 2],
    /// Capacity references: label, Mbps, and plot y, or `None` above the observed scale.
    marks: [Vec<(String, f64, Option<f64>)>; 2],
    paths: [[String; 2]; 3],
    areas: [String; 2],
}

#[component]
pub(crate) fn TrafficChart(data: RwSignal<Option<Overview>>) -> impl IntoView {
    let range = RwSignal::new("Recent");
    let history = RwSignal::new(None::<TrafficWindow>);
    let model = Memo::new(move |_| {
        let live = data.with(|d| Model::read(d.as_ref()));
        if range.get() == "Recent" {
            live
        } else {
            history.with(|w| live.with_window(w.as_ref()))
        }
    });
    // History windows refresh every 15 s while the page is visible. A newer selection wins.
    let generation = StoredValue::new(0u32);
    let load = move |window: &'static str| {
        generation.update_value(|g| *g = g.wrapping_add(1));
        let mine = generation.get_value();
        leptos::task::spawn_local(async move {
            if let Some(w) = fetch_window(window).await
                && generation.get_value() == mine
                && range.get_untracked() == window
            {
                history.set(Some(w));
            }
        });
    };
    let refresh = StoredValue::new_local(Some(gloo_timers::callback::Interval::new(
        15_000,
        move || {
            let window = range.get_untracked();
            let hidden = web_sys::window()
                .and_then(|w| w.document())
                .is_some_and(|d| d.hidden());
            if window != "Recent" && !hidden {
                load(window);
            }
        },
    )));
    on_cleanup(move || {
        refresh.try_update_value(|t| *t = None);
    });
    let enabled = RwSignal::new([true; 3]);
    let selected = RwSignal::new(None::<u64>);
    let pinned = RwSignal::new(false);
    let svg = NodeRef::<leptos::svg::Svg>::new();
    let geometry = Memo::new(move |_| {
        model.with(|m| {
            let all = m.series.iter().map(Vec::as_slice).collect::<Vec<_>>();
            let domain = plot::domain(&all).unwrap_or((0, 1));
            let visible = enabled.get();
            let active = all
                .iter()
                .enumerate()
                .filter(|(i, _)| visible[*i])
                .map(|(_, p)| *p)
                .collect::<Vec<_>>();
            let max: [f64; 2] = std::array::from_fn(|d| plot::maximum(&active, d == 0));
            let peak: [f64; 2] = std::array::from_fn(|d| plot::peak(&active, d == 0));
            let refs = m
                .capacity
                .as_ref()
                .map(plot::references)
                .unwrap_or_default();
            Geometry {
                domain,
                max,
                peak,
                peak_y: std::array::from_fn(|d| plot::y(peak[d], max[d], d == 0)),
                marks: std::array::from_fn(|d| {
                    refs.iter()
                        .map(|(label, r)| (label.clone(), r[d], plot::placed(r[d], max[d], d == 0)))
                        .collect()
                }),
                paths: std::array::from_fn(|i| {
                    std::array::from_fn(|d| {
                        if visible[i] {
                            plot::path(&m.series[i], d == 0, max[d], false, domain, m.step)
                        } else {
                            String::new()
                        }
                    })
                }),
                areas: std::array::from_fn(|d| {
                    if visible[0] {
                        plot::path(&m.series[0], d == 0, max[d], true, domain, m.step)
                    } else {
                        String::new()
                    }
                }),
            }
        })
    });
    let pointer = move |event: leptos::ev::PointerEvent| {
        if let Some(node) = svg.get_untracked() {
            let rect = node.get_bounding_client_rect();
            if rect.width() <= 0.0 {
                return;
            }
            let fraction = ((event.client_x() as f64 - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let g = geometry.get_untracked();
            let target = g.domain.0 + (fraction * (g.domain.1 - g.domain.0) as f64).round() as u64;
            let at = model
                .with_untracked(|m| m.times.iter().copied().min_by_key(|at| at.abs_diff(target)));
            if selected.get_untracked() != at {
                selected.set(at);
            }
        }
    };
    view! {
      <section class="panel traffic-panel" aria-labelledby="aggregate-traffic-title">
        <div class="panel-title traffic-head">
          <h2 id="aggregate-traffic-title">"Traffic (Aggregate)"</h2>
          <span class="traffic-state" data-state=move||model.with(|m|m.states[0].to_lowercase())><i aria-hidden="true"></i>{move||model.with(|m|m.states[0])}</span>
          <div class="traffic-controls"><div class="range-options" role="group" aria-label="Traffic window">{RANGES.into_iter().map(|label|view!{
            <button type="button" class:range-selected=move||range.get()==label aria-pressed=move||(range.get()==label).to_string()
              on:click=move|_|{if range.get_untracked()!=label{selected.set(None);pinned.set(false);history.set(None);range.set(label);if label!="Recent"{load(label);}}}>{label}</button>
          }).collect_view()}</div>
          <button class="traffic-live" on:click=move |_|{selected.set(None);pinned.set(false);} aria-pressed=move||(!pinned.get()).to_string()><i class="live-dot" aria-hidden="true"></i>"Live"</button>
          </div>
        </div>
        <Show when=move||model.with(|m|m.states[0]=="Stale" && !m.series[0].is_empty())><p class="sample-note">"Last known samples"</p></Show>
        <div class="traffic-source-controls" role="group" aria-label="Traffic sources">
          {(0..3).map(|i|view!{
            <button class=CLASSES[i] aria-pressed=move||enabled.get()[i].to_string() on:click=move |_|enabled.update(|v|v[i] = !v[i])>
              <span class="source-name"><span class="source-key" aria-hidden="true"></span>{NAMES[i]}
              <Show when=move||model.with(|m|!matches!(m.states[i],"Observed"|"Live"))><small>{move||model.with(|m|m.states[i])}</small></Show></span>
              {[true,false].into_iter().map(move |down|view!{<span class="legend-rate">{if down{"↓\u{2009}"}else{"↑\u{2009}"}}{move||model.with(|m|{
                  let at=selected.get().or_else(||m.times.last().copied());
                  at.and_then(|at|plot::sample(&m.series[i],at)).map(|p|rate(if down{p.down_mbps}else{p.up_mbps})).unwrap_or("—".into())
              })}</span>}).collect_view()}
            </button>
          }).collect_view()}
        </div>
        <Show when=move||model.with(|m|m.series.iter().any(|s|!s.is_empty())) fallback=move||if range.get()=="Recent"{view!{<div class="chart-empty">"No traffic samples available."<span>"Waiting for published traffic observations."</span></div>}.into_any()}else{view!{<div class="chart-empty">{move||format!("No {} history yet.",range.get())}<span>"The web service collects this history while it runs. It starts with the last hour from the core."</span></div>}.into_any()}>
          <div class="chart-frame mirrored-chart rainbow">
            <div class="y-axis" aria-hidden="true">{move||geometry.with(|g|view!{<span>{format!("↓ {}",axis_rate(g.max[0]))}</span><span>{axis_rate(g.max[0]/2.0)}</span><span>"0"</span><span>{axis_rate(g.max[1]/2.0)}</span><span>{format!("↑ {}",axis_rate(g.max[1]))}</span>})}</div>
            <div class="chart-data">
              <div class="plot-box">
              <svg node_ref=svg viewBox="0 0 900 240" preserveAspectRatio="none" role="img" tabindex="0" aria-label="Mirrored Actual, Mapped, and Unmapped traffic. Download above zero, upload below. Use Left and Right to inspect samples, Escape for Live." aria-describedby="traffic-inspector-help"
                on:pointermove=move |ev|{if !pinned.get_untracked() && ev.pointer_type()=="mouse" {pointer(ev);}}
                on:pointerdown=move |ev|{pointer(ev);pinned.set(true);}
                on:pointerleave=move |_|{if !pinned.get_untracked(){selected.set(None);}}
                on:focus=move |_|{if selected.get_untracked().is_none(){selected.set(model.with_untracked(|m|m.times.last().copied()));}}
                on:keydown=move |ev|{
                    let key=ev.key();
                    if key=="Escape" {selected.set(None);pinned.set(false);ev.prevent_default();}
                    else if key=="ArrowLeft" || key=="ArrowRight" {
                        ev.prevent_default();
                        let times=model.with_untracked(|m|m.times.clone());
                        if !times.is_empty(){
                            let current=selected.get_untracked().unwrap_or(*times.last().unwrap());
                            let index=times.partition_point(|at|*at<current).min(times.len()-1);
                            let next=if key=="ArrowLeft"{index.saturating_sub(1)}else{(index+1).min(times.len()-1)};
                            selected.set(Some(times[next]));pinned.set(true);
                        }
                    }
                }>
                {move||geometry.with(|g|{
                    let stops=[("0%","var(--traffic-red)"),("18%","var(--traffic-orange)"),("35%","var(--traffic-yellow)"),("52%","var(--traffic-green)"),("70%","var(--traffic-cyan)"),("85%","var(--traffic-blue)"),("100%","var(--traffic-violet)")];
                    view!{
                      <defs>{(0..2).map(|d|view!{
                        <linearGradient id=if d==0{"aggregate-download"}else{"aggregate-upload"} class=if d==0{"download-gradient"}else{"upload-gradient"} gradientUnits="userSpaceOnUse" x1="0" y1=format!("{:.1}",g.peak_y[d]) x2="0" y2="120">
                          {stops.into_iter().map(|(offset,color)|view!{<stop offset=offset stop-color=color/>}).collect_view()}
                        </linearGradient>
                      }).collect_view()}</defs>
                      {(0..2).map(|d|view!{<path class="traffic-area" d=g.areas[d].clone() fill=if d==0{"url(#aggregate-download)"}else{"url(#aggregate-upload)"} aria-hidden="true"/>}).collect_view()}
                      <g class="grid"><path d="M0 0H900M0 60H900M0 180H900M0 240H900M0 0V240M225 0V240M450 0V240M675 0V240M900 0V240"/></g>
                      <path class="traffic-zero" d="M0 120H900"/>
                      <g class="capacity-lines" aria-hidden="true">{g.marks.iter().flatten().filter_map(|m|m.2).map(|y|view!{<path d=format!("M0,{y:.1}H900")/>}).collect_view()}</g>
                      <g class="peak-lines" aria-hidden="true">{(0..2).filter(|d|g.peak[*d]>0.0).map(|d|view!{<path d=format!("M0,{:.1}H900",g.peak_y[d])/>}).collect_view()}</g>
                      {(0..3).flat_map(|i|(0..2).map(move |d|(i,d))).map(|(i,d)|view!{
                        <path class=format!("traffic-line {} {}",CLASSES[i],if d==0{"download-line"}else{"upload-line"}) d=g.paths[i][d].clone()
                          style=if i==0 {format!("stroke:url(#aggregate-{})",if d==0{"download"}else{"upload"})}else{String::new()}/>
                      }).collect_view()}
                    }
                })}
                {move||selected.get().and_then(|at|geometry.with(|g|(at>=g.domain.0 && at<=g.domain.1).then(||view!{<path class="traffic-cursor" d=format!("M{:.1} 0V240",plot::x(at,g.domain))/>})))}
              </svg>
              // shortcut: labels at close values can overlap; offset them if operators report it.
              <div class="chart-marks" aria-hidden="true">{move||geometry.with(|g|(0..2).map(|d|{
                  let (arrow,side)=if d==0{("↓","down")}else{("↑","up")};
                  let above=g.marks[d].iter().filter(|m|m.2.is_none()).map(|m|format!("{} {arrow} {}",m.0,rate(m.1))).collect::<Vec<_>>();
                  view!{
                    {(g.peak[d]>0.0).then(||view!{<span class=format!("mark peak {side}") style=format!("top:{:.2}%",g.peak_y[d]/2.4)>{format!("Peak {arrow} {}",rate(g.peak[d]))}</span>})}
                    {g.marks[d].iter().filter_map(|m|m.2.map(|y|view!{<span class=format!("mark capacity {side}") style=format!("top:{:.2}%",y/2.4)>{format!("{} {arrow} {}",m.0,rate(m.1))}</span>})).collect_view()}
                    {(!above.is_empty()).then(||view!{<span class=format!("mark edge {side}")>{format!("{} {} above scale",if d==0{"▲"}else{"▼"},above.join(" · "))}</span>})}
                  }
              }).collect_view())}</div>
              </div>
                    <div class="traffic-inspector" aria-label="Traffic figures at selected timestamp" class:open=move||selected.get().is_some()
                      class:flip=move||selected.get().is_some_and(|at|geometry.with(|g|plot::x(at,g.domain)>540.0))
                      style=move||selected.get().map(|at|geometry.with(|g|format!("left:{:.2}%",plot::x(at,g.domain)/9.0))).unwrap_or_default()>
                <p>{move||model.with(|m|{
                    let at=selected.get().or_else(||m.times.last().copied());
                    format!("{} · {}",if pinned.get(){"Pinned"}else if selected.get().is_some(){"Inspect"}else{"Latest"},at.map(local_time).unwrap_or("—".into()))
                })}</p>
                <div class="table-scroll"><table><caption class="sr-only">"All sources at one timestamp; missing samples show —"</caption><thead><tr><th scope="col">"Source"</th><th scope="col"><abbr title="Download">"↓"</abbr></th><th scope="col"><abbr title="Upload">"↑"</abbr></th></tr></thead><tbody>
                  {(0..3).map(|i|view!{
                    <tr class=CLASSES[i]><th scope="row">{NAMES[i]}<small>{move||model.with(|m|m.states[i])}</small></th>
                      { [true,false].into_iter().map(move |down|view!{<td>{move||model.with(|m|{
                          let at=selected.get().or_else(||m.times.last().copied());
                          at.and_then(|at|plot::sample(&m.series[i],at)).map(|p|rate(if down{p.down_mbps}else{p.up_mbps})).unwrap_or("—".into())
                      })}</td>}).collect_view()}
                    </tr>
                  }).collect_view()}
                </tbody></table></div>
              </div>
              <div class="x-axis" aria-label="Sample times in local time">{move||geometry.with(|g|(0..5).map(|i|view!{<span>{local_time(g.domain.0+(g.domain.1-g.domain.0)*i/4)}</span>}).collect_view())}</div>
            </div>
          </div>
        </Show>
        <p id="traffic-inspector-help" class="sr-only">"Download above zero, upload below. Hover to inspect. Tap to pin. Use Left/Right on the chart; Escape or Live clears selection."</p>
        <p class="panel-note">"Mapped and Unmapped are enqueue rates. They do not add up to Actual. Hover or tap the chart to inspect."</p>
        <Show when=move||model.with(|m|m.capped)><p class="panel-note">"Older samples were omitted to keep the live response bounded."</p></Show>
      </section>
    }
}
