//! Dashboard composition follows the approved picture and its written corrections.
use leptos::prelude::*;
use web_contract::{Overview, Point};

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
fn rate(value: f64) -> String {
    if value == 0.0 {
        "0 bps".into()
    } else if value >= 1000.0 {
        format!("{:.2} Gbps", value / 1000.0)
    } else {
        format!("{value:.1} Mbps")
    }
}
fn axis_rate(value: f64) -> String {
    if (10.0..1000.0).contains(&value) {
        format!("{value:.0} Mbps")
    } else {
        rate(value)
    }
}
fn phase(value: &str) -> String {
    match value {
        "IMPORT_CONFIRMED" => "Imported".into(),
        "Applied" | "APPLIED" => "Applied".into(),
        "unknown" | "UNKNOWN" | "" => "Unknown".into(),
        _ => value.replace('_', " "),
    }
}
fn badge(state: &str) -> &'static str {
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
#[component]
pub(crate) fn Navigation() -> impl IntoView {
    let groups = [
        ("Network", vec![("Routers", "router")]),
        (
            "IPoE",
            vec![("Subscribers", "people"), ("Profiles", "layers")],
        ),
        ("PPPoE", vec![("Users", "people"), ("Profiles", "layers")]),
        (
            "System",
            vec![
                ("LibreQoS", "layers"),
                ("WAN topology", "router"),
                ("Workers", "workers"),
                ("Settings", "settings"),
                ("Journal", "journal"),
            ],
        ),
    ];
    view! {
      <nav class="dashboard-nav" aria-label="Main navigation">
        <a class="nav-active" href="/" aria-current="page"><Icon name="dashboard"/>"Dashboard"</a>
        {groups.into_iter().map(|(group,items)|view!{
          <div class="nav-group"><p>{group}</p>{items.into_iter().map(|(label,icon)|view!{
            <button class="nav-planned" disabled title="This screen is not available yet"><Icon name=icon/><span>{label}</span><small>"Soon"</small></button>
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
        <h2><span class=color><Icon name=icon/></span>{title}</h2>
        <strong class=move||if kind=="publication"{data.with(|d|format!("metric-state {}",d.as_ref().map(|v|badge(&v.publication)).unwrap_or("neutral")))}else{String::new()}>{move||data.with(|d|d.as_ref().map(|v|match kind{
          "ipoe"=>number(v.ipoe_active),"ppp_online"=>number(v.ppp_online),"ppp_offline"=>number(v.ppp_offline),"routers"=>number(v.routers),_=>phase(&v.publication)
        }).unwrap_or("—".into()))}</strong>
        <p>{note}</p>
      </article>
    }
}
#[component]
fn Chart(points: Vec<Point>, stale: bool) -> impl IntoView {
    if points.len() < 2 {
        return view!{<div class="chart-empty">"No traffic samples available."<span>"Waiting for published traffic observations."</span></div>}.into_any();
    }
    let max = points
        .iter()
        .map(|p| p.down_mbps.max(p.up_mbps))
        .fold(1.0_f64, f64::max)
        * 1.15;
    let first = points.first().unwrap().at;
    let last = points.last().unwrap();
    let duration = last.at.saturating_sub(first).max(1) as f64;
    let line = |down: bool| {
        points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                format!(
                    "{}{:.1},{:.1}",
                    if i == 0 { "M" } else { " L" },
                    p.at.saturating_sub(first) as f64 / duration * 900.0,
                    240.0 - (if down { p.down_mbps } else { p.up_mbps }) / max * 240.0
                )
            })
            .collect::<String>()
    };
    let down = line(true);
    let up = line(false);
    let down_area = format!("{down} L900,240 L0,240 Z");
    let up_area = format!("{up} L900,240 L0,240 Z");
    static NEXT_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let down_gradient = format!("traffic-download-{id}");
    let up_gradient = format!("traffic-upload-{id}");
    let down_fill = format!("url(#{down_gradient})");
    let up_fill = format!("url(#{up_gradient})");
    // Keep stops as direct gradient children; static SVG fragments add a group.
    let stop_color = "currentColor";
    let ticks = (0..5)
        .map(|i| local_time(first + (duration * i as f64 / 4.0) as u64))
        .collect::<Vec<_>>();
    view!{
      <Show when=move||stale><p class="sample-note">"Last known samples"</p></Show>
      <div class="chart-frame">
        <div class="y-axis" aria-hidden="true">{(0..5).map(|i|view!{<span>{axis_rate(max*(4-i) as f64/4.0)}</span>}).collect_view()}</div>
        <div class="chart-data"><svg viewBox="0 0 900 240" preserveAspectRatio="none" role="img" aria-label="Recent download and upload traffic. Times use the browser's local timezone.">
          <defs>
            <linearGradient id=down_gradient class="download-gradient" x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stop-color=stop_color stop-opacity="0.52"/><stop offset="60%" stop-color=stop_color stop-opacity="0.16"/><stop offset="100%" stop-color=stop_color stop-opacity="0.01"/>
            </linearGradient>
            <linearGradient id=up_gradient class="upload-gradient" x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stop-color=stop_color stop-opacity="0.52"/><stop offset="60%" stop-color=stop_color stop-opacity="0.16"/><stop offset="100%" stop-color=stop_color stop-opacity="0.01"/>
            </linearGradient>
          </defs>
          <path class="traffic-area" d=down_area fill=down_fill aria-hidden="true"/><path class="traffic-area" d=up_area fill=up_fill aria-hidden="true"/>
          <g class="grid"><path d="M0 0H900M0 60H900M0 120H900M0 180H900M0 240H900M0 0V240M225 0V240M450 0V240M675 0V240M900 0V240"/></g>
          <path class="download-line" d=down/><path class="upload-line" d=up/>
        </svg><div class="x-axis" aria-label="Sample times in local time">{ticks.into_iter().map(|t|view!{<span>{t}</span>}).collect_view()}</div></div>
      </div>
      <div class="chart-legend"><span class="down">"━ Down (RX)"</span><span class="up">"━ Up (TX)"</span></div>

    }.into_any()
}
#[component]
fn WanDistribution() -> impl IntoView {
    view! {
      <section class="panel wan-panel" aria-labelledby="wan-distribution-title">
        <div class="panel-title"><h2 id="wan-distribution-title">"WAN Allocation Distribution"</h2><span class="chip neutral">"Unavailable"</span></div>
        <div class="wan-distribution">
          <div class="wan-ring" role="img" aria-label="WAN traffic allocation is unavailable"><svg viewBox="0 0 180 180" aria-hidden="true"><circle cx="90" cy="90" r="69"/></svg><div><strong>"—"</strong><span>"WAN traffic"</span></div></div>
          <div class="table-scroll" role="region" aria-label="WAN traffic shares" tabindex="0"><table><caption class="sr-only">"Observed traffic by WAN"</caption><thead><tr><th scope="col">"WAN"</th><th scope="col">"Traffic"</th><th scope="col">"Share"</th></tr></thead><tbody><tr><td colspan="3" class="empty-cell">"Per-WAN traffic is not available."</td></tr></tbody></table></div>
        </div>
        <p class="panel-note">"Aggregate traffic cannot establish each WAN's share."</p>
      </section>
    }
}
#[component]
fn Upstream() -> impl IntoView {
    view! {
      <section class="panel upstream-panel" aria-labelledby="upstream-title">
        <div class="upstream-heading"><div><h2 id="upstream-title">"Upstream Health & Allocation"</h2><p>"WAN capacity, shaping membership, and observed traffic."</p></div><div class="upstream-statuses" aria-label="WAN health totals"><span>"Up "<b>"—"</b></span><span>"Recovering "<b>"—"</b></span><span>"Unknown "<b>"—"</b></span></div></div>
        <div class="table-scroll" role="region" aria-labelledby="upstream-title" tabindex="0"><table><caption class="sr-only">"WAN health, capacity, and allocation"</caption><thead><tr><th scope="col">"WAN"</th><th scope="col">"State"</th><th scope="col">"Capacity (Down / Up)"</th><th scope="col">"Members"</th><th scope="col">"RX (Down)"</th><th scope="col">"TX (Up)"</th><th scope="col">"Utilization"</th></tr></thead><tbody><tr><td colspan="7" class="empty-cell">"WAN health and capacity evidence is not available. No utilization or routing state can be inferred."</td></tr></tbody></table></div>
      </section>
    }
}
#[component]
pub(crate) fn DashboardBody(data: RwSignal<Option<Overview>>) -> impl IntoView {
    view! {
      <Show when=move||data.with(|d|d.as_ref().is_some_and(|v|v.stale))><div class="notice">"Some source evidence is stale or unavailable. Unknown counts are shown as —."</div></Show>
      <section class="metrics" aria-label="Service summary">
        <Metric data kind="ipoe" title="IPoE Subscribers" icon="people" note="Active DHCP observations"/>
        <Metric data kind="ppp_online" title="PPPoE Online" icon="wifi" note="Currently online users"/>
        <Metric data kind="ppp_offline" title="PPPoE Offline" icon="wifi-off" note="Configured users offline"/>
        <Metric data kind="routers" title="Routers" icon="router" note="Managed routers"/>
        <Metric data kind="publication" title="LibreQoS" icon="layers" note="Publication evidence"/>
      </section>
      <div class="overview-grid">
        <section class="panel traffic-panel"><div class="panel-title"><h2>"Traffic (Aggregate)"</h2><span class="chip neutral">{move||data.with(|d|if d.as_ref().is_none_or(|v|v.traffic_stale){"Stale"}else{"Observed"})}</span></div>
          <div class="traffic-toolbar"><div class="range-options" role="group" aria-label="Traffic window"><span class="range-selected">"Recent"</span>{["1h","6h","24h"].into_iter().map(|label|view!{<button disabled title="This history window is not available">{label}</button>}).collect_view()}</div><div class="traffic-values"><span class="down"><i></i>"Down (RX) "<b>{move||data.with(|d|d.as_ref().and_then(|v|v.traffic.last()).map(|p|rate(p.down_mbps)).unwrap_or("—".into()))}</b></span><span class="up"><i></i>"Up (TX) "<b>{move||data.with(|d|d.as_ref().and_then(|v|v.traffic.last()).map(|p|rate(p.up_mbps)).unwrap_or("—".into()))}</b></span></div></div>
          {move||data.with(|d|d.as_ref().map(|v|view!{<Chart points=v.traffic.clone() stale=v.traffic_stale/>}))}
          <details class="sample-table"><summary>"Traffic samples"</summary><div class="table-scroll" role="region" aria-label="Traffic samples" tabindex="0"><table><caption>"Published samples in local time"</caption><thead><tr><th scope="col">"Time"</th><th scope="col">"Down (RX)"</th><th scope="col">"Up (TX)"</th></tr></thead><tbody>{move||data.with(|d|d.as_ref().map(|v|v.traffic.iter().map(|p|view!{<tr><th scope="row">{local_time(p.at)}</th><td>{rate(p.down_mbps)}</td><td>{rate(p.up_mbps)}</td></tr>}).collect_view()))}</tbody></table></div></details>
          <p class="panel-note">"Recent worker samples. Longer windows and per-WAN attribution are unavailable."</p>
        </section>
        <WanDistribution/>
      </div>
      <Upstream/>
      <details class="panel diagnostics"><summary>"Service diagnostics"<span>"Synchronization and component health"</span></summary>
        <dl><dt>"Service health"</dt><dd>{move||data.with(|d|d.as_ref().map(|v|v.health.clone()))}</dd><dt>"Runtime mode"</dt><dd>{move||data.with(|d|d.as_ref().map(|v|v.mode.clone()))}</dd><dt>"Output target"</dt><dd>{move||data.with(|d|d.as_ref().map(|v|v.target.clone()))}</dd><dt>"Publication phase"</dt><dd>{move||data.with(|d|d.as_ref().map(|v|v.publication.clone()))}</dd><dt>"Last cycle"</dt><dd>{move||data.with(|d|d.as_ref().and_then(|v|v.cycle_ms).map(|v|format!("{v} ms")).unwrap_or("Unknown".into()))}</dd></dl>
        <p class="panel-note">"A written input or confirmed import does not prove dataplane application."</p>
        <h2>"System health"</h2><div class="health-grid">{move||data.with(|d|d.as_ref().map(|v|v.components.iter().map(|c|view!{<div class="health-item"><span>{c.name.replace('_'," ")}</span><span class=format!("chip {}",badge(&c.state))>{c.state.replace('_'," ")}</span></div>}).collect_view()))}</div>
      </details>
    }
}
