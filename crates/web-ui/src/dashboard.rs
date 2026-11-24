//! Dashboard composition follows the approved picture and its written corrections.
use crate::route_model::Page;
use leptos::prelude::*;
use web_contract::Overview;

pub(crate) fn local_time(at: u64) -> String {
    if at == 0 {
        return "—".into();
    }
    js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(at as f64 * 1000.0))
        .to_locale_time_string("en-GB")
        .as_string()
        .unwrap_or_default()
        .chars()
        .take(8)
        .collect()
}
fn number(value: Option<u64>) -> String {
    value.map(|v| v.to_string()).unwrap_or("—".into())
}
pub(crate) fn rate(value: f64) -> String {
    if value == 0.0 {
        "0 bps".into()
    } else if value < 0.001 {
        format!("{:.1} bps", value * 1_000_000.0)
    } else if value < 1.0 {
        format!("{:.1} kbps", value * 1000.0)
    } else if value >= 1000.0 {
        format!("{:.2} Gbps", value / 1000.0)
    } else {
        format!("{value:.1} Mbps")
    }
}
pub(crate) fn axis_rate(value: f64) -> String {
    if (10.0..1000.0).contains(&value) {
        format!("{value:.0} Mbps")
    } else {
        rate(value)
    }
}
pub(crate) fn phase(value: &str) -> String {
    match value {
        "IMPORT_CONFIRMED" => "Imported".into(),
        "Applied" | "APPLIED" => "Applied".into(),
        "unknown" | "UNKNOWN" | "" => "Unknown".into(),
        _ => value.replace('_', " "),
    }
}
pub(crate) fn badge(state: &str) -> &'static str {
    match state.to_ascii_lowercase().as_str() {
        "healthy" | "applied" | "import_confirmed" => "good",
        "failed" | "applyfailed" | "writefailed" => "bad",
        "unknown" | "stale" => "neutral",
        _ => "warn",
    }
}
#[component]
fn Icon(name: &'static str) -> impl IntoView {
    let path = match name {
        "people" => {
            "M3 20v-2a5 5 0 0 1 10 0v2M8 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6M15 5a3 3 0 0 1 0 6m2 3a5 5 0 0 1 4 5v1"
        }
        "wifi" => "M2 8a16 16 0 0 1 20 0M5 12a11 11 0 0 1 14 0M8 16a6 6 0 0 1 8 0M11 20h2",
        "wifi-off" => {
            "M2 8a16 16 0 0 1 20 0M5 12a11 11 0 0 1 14 0M8 16a6 6 0 0 1 8 0M10 19l4 4m0-4-4 4"
        }
        "router" => "M5 4v8m14-8v8M3 12h18v8H3ZM7 16h1m4 0h1",
        "layers" => "m2 7 10-5 10 5-10 5Zm0 5 10 5 10-5M2 17l10 5 10-5",
        "settings" => "M4 6h16M4 12h16M4 18h16M8 3v6m8 0v6m-6 0v6",
        "workers" => "M5 5h14v14H5ZM9 9h6v6H9ZM9 2v3m6-3v3M9 19v3m6-3v3M2 9h3m-3 6h3m14-6h3m-3 6h3",
        "journal" => "M5 3h14v18H5ZM8 7h8M8 12h8M8 17h5",
        _ => "M3 3h7v7H3Zm11 0h7v7h-7ZM3 14h7v7H3Zm11 0h7v7h-7Z",
    };
    view! {<svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d=path/></svg>}
}
/// A main navigation link. A plain click changes the page without a reload.
#[component]
fn NavLink(
    href: &'static str,
    label: &'static str,
    icon: &'static str,
    active: Signal<bool>,
) -> impl IntoView {
    view! {
      <a href=href data-label=label class:nav-active=move||active.get() aria-current=move||active.get().then_some("page")
        on:click=move|ev: web_sys::MouseEvent|{
          if ev.button()==0 && !(ev.ctrl_key()||ev.meta_key()||ev.shift_key()||ev.alt_key()) {
            ev.prevent_default();
            crate::route::go(href);
          }
        }><Icon name=icon/><span class="nav-text">{label}</span></a>
    }
}
#[component]
pub(crate) fn Navigation() -> impl IntoView {
    let page = crate::route::page();
    let groups = [(
        "System",
        vec![
            ("LibreQoS", "layers", None),
            ("Workers", "workers", Some(("/workers", Page::Workers))),
            ("Settings", "settings", None),
            ("Journal", "journal", Some(("/journal", Page::Journal))),
        ],
    )];
    view! {
      <nav class="dashboard-nav" aria-label="Main navigation">
        <NavLink href="/" label="Dashboard" icon="dashboard" active=Signal::derive(move||page.get()==Page::Dashboard)/>
        <div class="nav-group"><p>"Network"</p>
          <NavLink href="/routers" label="Routers" icon="router" active=Signal::derive(move||matches!(page.get(),Page::Routers|Page::NewRouter|Page::Router{..}))/>
          <NavLink href="/topology" label="Topology" icon="layers" active=Signal::derive(move||matches!(page.get(),Page::Topology{..}))/>
          <NavLink href="/pppoe" label="PPPoE" icon="people" active=Signal::derive(move||matches!(page.get(),Page::Source{kind:"ppp",..}))/>
          <NavLink href="/ipoe" label="IPoE / DHCP" icon="people" active=Signal::derive(move||matches!(page.get(),Page::Source{kind:"dhcp",..}))/>
        </div>
        {groups.into_iter().map(|(group,items)|view!{
          <div class="nav-group"><p>{group}</p>{items.into_iter().map(|(label,icon,target)|match target {
            Some((href,target))=>view!{<NavLink href label icon active=Signal::derive(move||page.get()==target)/>}.into_any(),
            None=>view!{<button class="nav-planned" disabled title="This screen is not available yet"><Icon name=icon/><span>{label}</span><small>"Soon"</small></button>}.into_any(),
          }).collect_view()}</div>
        }).collect_view()}
      </nav>
    }
}
#[component]
fn Metric(
    data: RwSignal<Option<Overview>>,
    kind: &'static str,
    title: &'static str,
    icon: &'static str,
    note: &'static str,
) -> impl IntoView {
    let color = match kind {
        "ppp_online" => "good",
        "ppp_offline" => "bad",
        _ => "accent",
    };
    view! {
      <article class="metric panel">
        <h4><span class=color><Icon name=icon/></span>{title}</h4>
        <strong class=move||if kind=="publication"{data.with(|d|format!("metric-state {}",d.as_ref().map(|v|badge(&v.publication)).unwrap_or("neutral")))}else{String::new()}>{move||data.with(|d|d.as_ref().map(|v|match kind{
          "ipoe"=>number(v.ipoe_active),"ppp_online"=>number(v.ppp_online),"ppp_offline"=>number(v.ppp_offline),"routers"=>number(v.routers),_=>phase(&v.publication)
        }).unwrap_or("—".into()))}</strong>
        <p>{note}</p>
      </article>
    }
}
const WAN_COLORS: [&str; 6] = ["blue", "cyan", "green", "yellow", "orange", "violet"];
/// One ring: an empty track, then one arc for each WAN in WAN order.
fn ring(radius: u32, class: &'static str, values: Vec<f64>) -> impl IntoView {
    view! {
      <g class=format!("wan-arcs {class}")>
        <circle class="wan-track" cx="90" cy="90" r=radius/>
        {crate::wan_ring::arcs(&values).into_iter().enumerate().filter(|(_, (_, len))| *len > 0.0).map(|(i, (start, len))| view!{
          <circle cx="90" cy="90" r=radius pathLength="100" transform="rotate(-90 90 90)"
            stroke-dasharray=format!("{len:.2} 100") stroke-dashoffset=format!("{:.2}", -start)
            style=format!("stroke:var(--traffic-{})", WAN_COLORS[i % WAN_COLORS.len()])/>
        }).collect_view()}
      </g>
    }
}
#[component]
fn WanDistribution(data: RwSignal<Option<Overview>>) -> impl IntoView {
    let capacity = move || data.with(|d| d.as_ref().and_then(|v| v.wan_capacity.clone()));
    view! {
      <section class="panel wan-panel" aria-labelledby="wan-distribution-title">
        <div class="panel-title"><h2 id="wan-distribution-title">"WAN Allocation Distribution"</h2>
          <span class="chip neutral">{move||if capacity().is_some(){"Partial"}else{"Unavailable"}}</span></div>
        <div class="wan-distribution">
          {move||{
              let c=capacity();
              // A core build without per-WAN rates still has the combined total: show it as one arc.
              let wans=c.as_ref().map(|c|if c.wans.is_empty(){vec![web_contract::WanLink{id:String::new(),name:"Combined WAN".into(),down_bps:c.down_bps,up_bps:c.up_bps,subscribers:None}]}else{c.wans.clone()}).unwrap_or_default();
              let label=c.as_ref().map_or("WAN allocation is unavailable".to_string(),|c|format!("Rings, inside out: capacity of {} WAN, traffic share (unavailable), and subscribers. The table lists each value.",c.wan_count));
              view!{
                <div class="wan-ring" role="img" aria-label=label>
                  <svg viewBox="0 0 180 180" aria-hidden="true">
                    {ring(46,"capacity",wans.iter().map(|w|w.down_bps as f64).collect())}
                    {ring(64,"share",Vec::new())}
                    {ring(81,"members",wans.iter().map(|w|w.subscribers.unwrap_or(0) as f64).collect())}
                  </svg>
                  <div class="wan-total">{match c.as_ref(){
                    Some(c)=>view!{<strong>{format!("↓ {}",axis_rate(c.down_bps as f64/1_000_000.0))}</strong><strong>{format!("↑ {}",axis_rate(c.up_bps as f64/1_000_000.0))}</strong><span>"Combined"</span>}.into_any(),
                    None=>view!{<strong>"—"</strong><span>"WAN capacity"</span>}.into_any(),
                  }}</div>
                </div>
                <div class="table-scroll" role="region" aria-label="WAN allocation by WAN" tabindex="0"><table>
                  <caption class="sr-only">"Capacity, traffic share, and subscribers for each WAN"</caption>
                  <thead><tr><th scope="col">"WAN"</th><th scope="col">"Capacity"</th><th scope="col">"Share"</th><th scope="col">"Subs"</th></tr></thead>
                  <tbody>
                    {if wans.is_empty(){view!{<tr><td colspan="4" class="empty-cell">"WAN capacity is not available."</td></tr>}.into_any()}else{
                      wans.iter().enumerate().map(|(i,w)|view!{<tr>
                        <th scope="row"><span class="wan-swatch" aria-hidden="true" style=format!("background:var(--traffic-{})",WAN_COLORS[i%WAN_COLORS.len()])></span>{w.name.clone()}</th>
                        <td><span class="cap-line">{format!("↓ {}",axis_rate(w.down_bps as f64/1_000_000.0))}</span><span class="cap-line">{format!("↑ {}",axis_rate(w.up_bps as f64/1_000_000.0))}</span></td>
                        <td title="Per-WAN traffic is not available yet">"—"</td>
                        <td>{number(w.subscribers)}</td>
                      </tr>}).collect_view().into_any()
                    }}
                  </tbody>
                </table></div>
              }
          }}
        </div>
      </section>
    }
}
#[component]
fn Upstream() -> impl IntoView {
    view! {
      <section class="panel upstream-panel" aria-labelledby="upstream-title">
        <div class="upstream-heading"><div><h2 id="upstream-title">"Upstream Health & Allocation"</h2><p>"WAN capacity, shaping membership, and observed traffic."</p></div><div class="upstream-statuses" aria-label="WAN health totals"><span>"Up "<b>"—"</b></span><span>"Recovering "<b>"—"</b></span><span>"Unknown "<b>"—"</b></span></div></div>
        <div class="table-scroll" role="region" aria-labelledby="upstream-title" tabindex="0"><table><caption class="sr-only">"WAN health, capacity, and allocation"</caption><thead><tr><th scope="col">"WAN"</th><th scope="col">"State"</th><th scope="col">"Capacity (Down / Up)"</th><th scope="col">"Members"</th><th scope="col">"RX (Down)"</th><th scope="col">"TX (Up)"</th><th scope="col">"Utilization"</th></tr></thead><tbody><tr><td colspan="7" class="empty-cell">"Per-WAN health, capacity, and traffic evidence is not available. Aggregate totals do not establish per-WAN utilization or routing."</td></tr></tbody></table></div>
      </section>
    }
}
#[component]
pub(crate) fn DashboardBody(data: RwSignal<Option<Overview>>) -> impl IntoView {
    let (snapshot, server_clock) = crate::dpdk::monitoring(data);
    view! {
      <Show when=move||data.with(|d|d.as_ref().is_some_and(|v|v.stale))><div class="notice">"Some source evidence is stale or unavailable. Unknown counts are shown as —."</div></Show>
      <section class="metrics" aria-label="Service summary">
        // The NOC Command Deck: service status, the radial profiler, worker telemetry, NIC ports, and the status ticker.
        <crate::worker_ring::WorkerRing snapshot server_clock data>
          <Metric data kind="ipoe" title="IPoE Subscribers" icon="people" note="Active DHCP observations"/>
          <Metric data kind="ppp_online" title="PPPoE Online" icon="wifi" note="Currently online users"/>
          <Metric data kind="ppp_offline" title="PPPoE Offline" icon="wifi-off" note="Configured users offline"/>
          <Metric data kind="routers" title="Routers" icon="router" note="Managed routers"/>
          <Metric data kind="publication" title="LibreQoS" icon="layers" note="Publication evidence"/>
        </crate::worker_ring::WorkerRing>
      </section>
      <div class="overview-grid">
        <crate::traffic_chart::TrafficChart data/>
        <WanDistribution data/>
      </div>
      <Upstream/>
    }
}
