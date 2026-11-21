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
      <a href=href class:nav-active=move||active.get() aria-current=move||active.get().then_some("page")
        on:click=move|ev: web_sys::MouseEvent|{
          if ev.button()==0 && !(ev.ctrl_key()||ev.meta_key()||ev.shift_key()||ev.alt_key()) {
            ev.prevent_default();
            crate::route::go(href);
          }
        }><Icon name=icon/>{label}</a>
    }
}
#[component]
pub(crate) fn Navigation() -> impl IntoView {
    let page = crate::route::page();
    let groups = [
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
        <NavLink href="/" label="Dashboard" icon="dashboard" active=Signal::derive(move||page.get()==Page::Dashboard)/>
        <div class="nav-group"><p>"Network"</p>
          <NavLink href="/routers" label="Routers" icon="router" active=Signal::derive(move||matches!(page.get(),Page::Routers|Page::Router{..}))/>
        </div>
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
        <Metric data kind="ipoe" title="IPoE Subscribers" icon="people" note="Active DHCP observations"/>
        <Metric data kind="ppp_online" title="PPPoE Online" icon="wifi" note="Currently online users"/>
        <Metric data kind="ppp_offline" title="PPPoE Offline" icon="wifi-off" note="Configured users offline"/>
        <Metric data kind="routers" title="Routers" icon="router" note="Managed routers"/>
        <Metric data kind="publication" title="LibreQoS" icon="layers" note="Publication evidence"/>
        // The DPDK Health card has its own full row: worker ring, health band, telemetry, and NIC ports.
        <crate::worker_ring::WorkerRing snapshot server_clock/>
      </section>
      <div class="overview-grid">
        <crate::traffic_chart::TrafficChart data/>
        <WanDistribution/>
      </div>
      <Upstream/>
    }
}
