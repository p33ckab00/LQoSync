//! One comparison panel. Pointer inspection never reads the network.
use crate::{
    dashboard::{axis_rate, local_time, rate},
    traffic_plot as plot,
};
use leptos::prelude::*;
use web_contract::{Overview, Point, WanCapacity};

const NAMES: [&str; 3] = ["Actual", "Mapped", "Unmapped"];
const CLASSES: [&str; 3] = ["actual", "mapped", "unmapped"];
#[derive(Clone, Default, PartialEq)]
struct Model {
    series: [Vec<Point>; 3],
    states: [&'static str; 3],
    capacity: Option<WanCapacity>,
    capped: bool,
    times: Vec<u64>,
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
        model.times = model.series.iter().flatten().map(|p| p.at).collect();
        model.times.sort_unstable();
        model.times.dedup();
        model
    }
}
#[derive(Clone, Default, PartialEq)]
struct Geometry {
    domain: (u64, u64),
    max: [f64; 2],
    ceiling: [f64; 2],
    paths: [[String; 2]; 3],
    areas: [String; 2],
}

#[component]
pub(crate) fn TrafficChart(data: RwSignal<Option<Overview>>) -> impl IntoView {
    let model = Memo::new(move |_| data.with(|d| Model::read(d.as_ref())));
    let enabled = RwSignal::new([true; 3]);
    let selected = RwSignal::new(None::<u64>);
    let pinned = RwSignal::new(false);
    let table_open = RwSignal::new(false);
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
            let capacities = [
                m.capacity.as_ref().map(|c| c.down_bps),
                m.capacity.as_ref().map(|c| c.up_bps),
            ];
            let max = std::array::from_fn(|d| plot::maximum(&active, d == 0, capacities[d]));
            let ceiling = std::array::from_fn(|d| {
                plot::y(
                    capacities[d].unwrap_or(0) as f64 / 1_000_000.0,
                    max[d],
                    d == 0,
                )
            });
            Geometry {
                domain,
                max,
                ceiling,
                paths: std::array::from_fn(|i| {
                    std::array::from_fn(|d| {
                        if visible[i] {
                            plot::path(&m.series[i], d == 0, max[d], false, domain)
                        } else {
                            String::new()
                        }
                    })
                }),
                areas: std::array::from_fn(|d| {
                    if visible[0] {
                        plot::path(&m.series[0], d == 0, max[d], true, domain)
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
        <div class="panel-title"><h2 id="aggregate-traffic-title">"Traffic (Aggregate)"</h2><span class="chip neutral">{move||model.with(|m|m.states[0])}</span></div>
        <Show when=move||model.with(|m|m.states[0]=="Stale" && !m.series[0].is_empty())><p class="sample-note">"Last known samples"</p></Show>
        <div class="traffic-toolbar"><div class="range-options" role="group" aria-label="Traffic window"><span class="range-selected">"Recent"</span>{["1h","6h","24h"].into_iter().map(|label|view!{<button disabled title="This history window is not available">{label}</button>}).collect_view()}</div>
          <button class="traffic-live" on:click=move |_|{selected.set(None);pinned.set(false);} aria-pressed=move||(!pinned.get()).to_string()>"Live"</button>
        </div>
        <div class="traffic-source-controls" role="group" aria-label="Traffic sources">
          {(0..3).map(|i|view!{
            <button class=CLASSES[i] aria-pressed=move||enabled.get()[i].to_string() on:click=move |_|enabled.update(|v|v[i] = !v[i])>
              <span class="source-key" aria-hidden="true"></span>{NAMES[i]}
              {[true,false].into_iter().map(move |down|view!{<span class="legend-rate">{if down{"↓\u{2009}"}else{"↑\u{2009}"}}{move||model.with(|m|{
                  let at=selected.get().or_else(||m.times.last().copied());
                  at.and_then(|at|plot::sample(&m.series[i],at)).map(|p|rate(if down{p.down_mbps}else{p.up_mbps})).unwrap_or("—".into())
              })}</span>}).collect_view()}
              <Show when=move||model.with(|m|!matches!(m.states[i],"Observed"|"Live"))><small>{move||model.with(|m|m.states[i])}</small></Show>
            </button>
          }).collect_view()}
        </div>
        <p class="capacity-note">{move||model.with(|m|match &m.capacity {
            Some(c)=>{
                let utilization=m.series[0].last().filter(|_| c.down_bps>0 && c.up_bps>0).map(|p|format!(" Actual ↓ {:.1}% ↑ {:.1}%.",p.down_mbps/(c.down_bps as f64/1_000_000.0)*100.0,p.up_mbps/(c.up_bps as f64/1_000_000.0)*100.0)).unwrap_or_default();
                format!("Capacity ({} WAN): ↓ {} ↑ {} · red line = 100%.{}",c.wan_count,rate(c.down_bps as f64/1_000_000.0),rate(c.up_bps as f64/1_000_000.0),utilization)
            },
            None=>"WAN capacity unavailable · axes use observed samples.".into()
        })}</p>
        <Show when=move||model.with(|m|m.series.iter().any(|s|!s.is_empty())) fallback=||view!{<div class="chart-empty">"No traffic samples available."<span>"Waiting for published traffic observations."</span></div>}>
          <div class="chart-frame mirrored-chart" class:rainbow=move||model.with(|m|m.capacity.is_some())>
            <div class="y-axis" aria-hidden="true">{move||geometry.with(|g|view!{<span>{format!("↓ {}",axis_rate(g.max[0]))}</span><span>{axis_rate(g.max[0]/2.0)}</span><span>"0"</span><span>{axis_rate(g.max[1]/2.0)}</span><span>{format!("↑ {}",axis_rate(g.max[1]))}</span>})}</div>
            <div class="chart-data">
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
                    let rainbow=model.with(|m|m.capacity.is_some());
                    let ceiling=g.ceiling;
                    let stops=[("0%","var(--traffic-red)"),("18%","var(--traffic-orange)"),("35%","var(--traffic-yellow)"),("52%","var(--traffic-green)"),("70%","var(--traffic-cyan)"),("85%","var(--traffic-blue)"),("100%","var(--traffic-violet)")];
                    view!{
                      <defs>{(0..2).map(|d|view!{
                        <linearGradient id=if d==0{"aggregate-download"}else{"aggregate-upload"} class=if d==0{"download-gradient"}else{"upload-gradient"} gradientUnits="userSpaceOnUse" x1="0" y1=g.ceiling[d].to_string() x2="0" y2="120">
                          {stops.into_iter().map(|(offset,color)|view!{<stop offset=offset stop-color=if rainbow{color}else{"currentColor"}/>}).collect_view()}
                        </linearGradient>
                      }).collect_view()}</defs>
                      {(0..2).map(|d|view!{<path class="traffic-area" d=g.areas[d].clone() fill=if d==0{"url(#aggregate-download)"}else{"url(#aggregate-upload)"} aria-hidden="true"/>}).collect_view()}
                      <g class="grid"><path d="M0 0H900M0 60H900M0 180H900M0 240H900M0 0V240M225 0V240M450 0V240M675 0V240M900 0V240"/></g>
                      <path class="traffic-zero" d="M0 120H900"/>
                      <Show when=move||rainbow><g class="capacity-lines" aria-hidden="true">{ceiling.into_iter().map(|y|view!{<path d=format!("M0,{y:.1}H900")/>}).collect_view()}</g></Show>
                      {(0..3).flat_map(|i|(0..2).map(move |d|(i,d))).map(|(i,d)|view!{
                        <path class=format!("traffic-line {} {}",CLASSES[i],if d==0{"download-line"}else{"upload-line"}) d=g.paths[i][d].clone()
                          style=if i==0 && rainbow {format!("stroke:url(#aggregate-{})",if d==0{"download"}else{"upload"})}else{String::new()}/>
                      }).collect_view()}
                    }
                })}
                {move||selected.get().and_then(|at|geometry.with(|g|(at>=g.domain.0 && at<=g.domain.1).then(||view!{<path class="traffic-cursor" d=format!("M{:.1} 0V240",plot::x(at,g.domain))/>})))}
              </svg>
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
        <details class="sample-table" on:toggle=move |ev|{let target=event_target::<web_sys::Element>(&ev);table_open.set(target.has_attribute("open"));}>
          <summary>"Sample table"</summary>
          <Show when=move||table_open.get()><div class="table-scroll" role="region" aria-label="Traffic samples" tabindex="0"><table><caption>"Published samples in local time"</caption><thead><tr><th scope="col">"Time"</th><th scope="col">"Source"</th><th scope="col">"Down"</th><th scope="col">"Up"</th></tr></thead><tbody>
            {move||model.with(|m|(0..3).flat_map(|i|m.series[i].iter().map(move |p|(i,p))).map(|(i,p)|view!{<tr><th scope="row">{local_time(p.at)}</th><td>{NAMES[i]}</td><td>{rate(p.down_mbps)}</td><td>{rate(p.up_mbps)}</td></tr>}).collect_view())}
          </tbody></table></div></Show>
        </details>
        <p class="panel-note">"↓ download · ↑ upload · Actual is delivered; Mapped and Unmapped are enqueued, so rates are not additive. Hover or tap the chart for samples."</p>
        <Show when=move||model.with(|m|m.capped)><p class="panel-note">"Older samples were omitted to keep the live response bounded."</p></Show>
      </section>
    }
}
