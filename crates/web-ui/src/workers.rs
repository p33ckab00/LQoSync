//! The Workers screen: every DPDK worker in the bridge snapshot, with its session trend and its
//! phase mix. It reads the same live overview as the Dashboard; it has no request of its own.
use crate::workers_model::{History, KEEP, pct, shares, sparkline};
use leptos::prelude::*;
use web_contract::Overview;
use web_contract::dpdk::Worker;

fn count(n: Option<u64>) -> String {
    n.map(crate::worker_ring_model::grouped)
        .unwrap_or("—".into())
}

#[component]
pub(crate) fn WorkersPage(data: RwSignal<Option<Overview>>) -> impl IntoView {
    let (snapshot, server_clock) = crate::dpdk::monitoring(data);
    let history = RwSignal::new(History::default());
    Effect::new(move |_| {
        snapshot.with(|d| {
            if let Some(d) = d {
                history.update(|h| h.observe(d.analytics.updated.unwrap_or(0), &d.workers));
            }
        })
    });
    let live = Memo::new(move |_| {
        snapshot.with(|d| {
            d.as_ref()
                .is_some_and(|d| d.analytics.live(server_clock.get(), 6))
        })
    });
    let open = RwSignal::new(None::<u16>);
    view! {
      <div class="page-heading">
        <div><h1 tabindex="-1">"Workers"</h1><p>"DPDK worker load from the bridge analytics. The page follows each new sample."</p></div>
        <div class="freshness">{move||snapshot.with(|d|match d {
          None=>"Waiting for worker data…".to_string(),
          Some(_) if live.get()=>"Live".into(),
          Some(_)=>"Stale: last known values".into(),
        })}</div>
      </div>
      <section class="panel workers-panel" aria-labelledby="workers-title">
        <h2 id="workers-title" class="sr-only">"Worker table"</h2>
        <p class="workers-summary">{move||snapshot.with(|d|d.as_ref().map(|d|{
          let busiest=d.workers.iter().max_by_key(|w|w.busy_ppm);
          format!("{} workers. Mean busy {}.{}",d.worker_count,pct(d.mean_busy_ppm),
            busiest.map(|w|format!(" Busiest: W{} at {}.",w.id,pct(Some(w.busy_ppm)))).unwrap_or_default())
        }).unwrap_or("Worker telemetry unavailable.".into()))}</p>
        <Show when=move||snapshot.with(|d|d.as_ref().is_some_and(|d|!d.workers.is_empty()))
          fallback=||view!{<p class="router-empty">"No worker data yet. The bridge analytics publish it while the DPDK bridge runs."</p>}>
          <div class="table-scroll" role="region" aria-label="Worker table" tabindex="0">
            <table class="router-table workers-table" class:stale=move||!live.get()>
              <thead><tr>
                <th scope="col">"Worker"</th><th scope="col">"Busy"</th>
                <th scope="col">{format!("Trend, last {KEEP} samples")}</th>
                <th scope="col" class="num">"Idle"</th><th scope="col" class="num">"Laps/s"</th>
                <th scope="col" class="num">"Cycles/busy lap"</th><th scope="col" class="num">"Mean backlog"</th>
                <th scope="col">"Phase mix"</th>
              </tr></thead>
              <For each=move||snapshot.with(|d|d.as_ref().map(|d|d.workers.clone()).unwrap_or_default()) key=|w:&Worker|w.id children=move|w:Worker|{
                let id=w.id;
                let worker=Memo::new(move|_|snapshot.with(|d|d.as_ref().and_then(|d|d.workers.iter().find(|w|w.id==id).cloned())).unwrap_or_default());
                let expanded=move||open.get()==Some(id);
                view!{
                  <tbody>
                    <tr>
                      <th scope="row">
                        <button type="button" class="link" aria-expanded=move||expanded().to_string() aria-controls=format!("worker-detail-{id}")
                          on:click=move|_|open.update(|o|*o=if *o==Some(id){None}else{Some(id)})>{format!("W{id}")}</button>
                      </th>
                      <td><span class="busy-cell"><span class="busy-bar" aria-hidden="true"><i style=move||format!("inline-size:{:.1}%",worker.with(|w|w.busy_ppm.min(1_000_000)) as f64/10_000.0)></i></span>
                        {move||worker.with(|w|pct(Some(w.busy_ppm)))}</span></td>
                      <td><svg class="sparkline" viewBox=format!("0 0 {KEEP} 20") preserveAspectRatio="none" aria-hidden="true">
                        <path d=move||history.with(|h|h.points.get(&id).map(sparkline).unwrap_or_default())/></svg></td>
                      <td class="num">{move||worker.with(|w|pct(Some(1_000_000-w.busy_ppm.min(1_000_000))))}</td>
                      <td class="num">{move||worker.with(|w|count(w.laps_per_s))}</td>
                      <td class="num">{move||worker.with(|w|count(w.cycles_per_busy_lap))}</td>
                      <td class="num">{move||worker.with(|w|w.backlog_mean.map(|b|format!("{} pkts",crate::worker_ring_model::grouped(b))).unwrap_or("—".into()))}</td>
                      <td>{move||{let s=worker.with(shares);let label=s.iter().map(|x|format!("{} {:.0}%",x.label,x.share*100.0)).collect::<Vec<_>>().join(", ");if s.is_empty(){view!{<span class="field-help">"No phase data"</span>}.into_any()}else{view!{
                        <span class="phase-mix" role="img" aria-label=label>
                          {s.iter().filter(|x|x.share>0.0).map(|x|view!{<i class=format!("phase-{}",x.index) style=format!("flex-grow:{:.4}",x.share)></i>}).collect_view()}
                        </span>}.into_any()}}}</td>
                    </tr>
                    <Show when=expanded>
                      <tr class="worker-detail" id=format!("worker-detail-{id}")><td colspan="8">
                        <p class="field-help">"Cycle shares since the bridge started, not the CPU time now. Idle polling (Empty RX) is not in the total."</p>
                        <dl class="router-facts">{move||worker.with(|w|shares(w).into_iter().map(|x|view!{
                          <div><dt><i class=format!("swatch phase-{}",x.index) aria-hidden="true"></i>{x.label}</dt>
                            <dd>{format!("{:.1}% · {} cycles",x.share*100.0,crate::worker_ring_model::grouped(x.cycles))}</dd></div>
                        }).collect_view())}</dl>
                        <dl class="router-facts">{move||worker.with(|w|w.details.iter().map(|r|view!{<div><dt>{r.label.clone()}</dt><dd>{r.value.clone()}</dd></div>}).collect_view())}</dl>
                      </td></tr>
                    </Show>
                  </tbody>
                }
              }/>
            </table>
          </div>
        </Show>
        <Show when=move||snapshot.with(|d|d.as_ref().is_some_and(|d|d.details_capped))>
          <p class="notice">{move||snapshot.with(|d|d.as_ref().map(|d|format!("The bridge reports {} workers; this view shows the {} busiest, to keep the response bounded.",d.worker_count,d.workers.len())).unwrap_or_default())}</p>
        </Show>
        <p class="panel-note">"The trend starts when this page opens. Busy is the share of loop time with work; laps per second count loop passes, not packets."</p>
      </section>
    }
}
