//! Compact physical faceplate. Source identity owns the keyed port instances.
use crate::nic_activity::{Activity, active};
use leptos::prelude::*;
use web_contract::dpdk::{Dpdk, Port};

fn speed(value: Option<u64>) -> String {
    match value.filter(|v| *v > 0) {
        Some(v) if v >= 1000 => format!("{}G", v as f64 / 1000.0),
        Some(v) => format!("{v}M"),
        None => "—".into(),
    }
}
fn artwork(connector: &str) -> (&'static str, &'static str) {
    match connector {
        "rj45" => (
            "rj45",
            "M23 17H67V24H74V52H16V24H23Z M26 22V41M33 22V41M40 22V41M47 22V41M54 22V41M61 22V41",
        ),
        "sfp" | "sfp+" | "sfp28" => (
            "sfp",
            "M18 19H72V51H18Z M24 23H43V45H24ZM47 23H66V45H47Z M15 15H75M20 55H70",
        ),
        "qsfp" | "qsfp+" | "qsfp28" => (
            "qsfp",
            "M13 19H77V49H13Z M20 24V44M30 24V44M40 24V44M50 24V44M60 24V44M70 24V44 M16 54H74 M35 54V58H55V54",
        ),
        _ => (
            "unknown",
            "M18 19H72V51H18Z M37 28C37 21 54 21 54 30C54 35 45 35 45 40 M45 44V46",
        ),
    }
}
#[component]
fn PortTile(id: u16, snapshot: Memo<Option<Dpdk>>, clock: Memo<u64>) -> impl IntoView {
    let port = Memo::new(move |_| {
        snapshot.with(|d| {
            d.as_ref()
                .and_then(|d| d.ports.iter().find(|p| p.id == id).cloned())
                .unwrap_or_default()
        })
    });
    let live = Memo::new(move |_| {
        snapshot.with(|d| d.as_ref().is_some_and(|d| d.nic.live(clock.get(), 15)))
    });
    let state = Memo::new(move |_| {
        port.with(|p| {
            if !live.get() {
                "unknown"
            } else if p.state.eq_ignore_ascii_case("up") {
                "up"
            } else if p.state.eq_ignore_ascii_case("down") {
                "down"
            } else {
                "unknown"
            }
        })
    });
    let tracker = StoredValue::new_local(Activity::default());
    let pulse = RwSignal::new([false; 2]);
    let timeout = StoredValue::new_local(None::<gloo_timers::callback::Timeout>);
    Effect::new(move |_| {
        let current = port.get();
        let mut result = None;
        tracker.update_value(|t| {
            result = t.observe(&current, live.get());
        });
        if let Some(next) = result {
            timeout.update_value(|t| *t = None);
            pulse.set(next);
            if next.iter().any(|v| *v) {
                timeout.set_value(Some(gloo_timers::callback::Timeout::new(180, move || {
                    pulse.set([false; 2])
                })));
            }
        }
    });
    on_cleanup(move || {
        timeout.try_update_value(|t| *t = None);
    });
    let description = Memo::new(move |_| {
        port.with(|p| {
            format!(
                "Port {} · {} · {} · {} · {} · {}. {}. Interface {}. Reported speed {}",
                p.id,
                p.physical.role.as_deref().unwrap_or("Role unknown"),
                p.name,
                p.physical
                    .connector
                    .as_deref()
                    .unwrap_or("Connector unknown"),
                p.physical
                    .confidence
                    .as_deref()
                    .unwrap_or("unknown confidence"),
                state.get(),
                p.physical
                    .evidence
                    .as_deref()
                    .unwrap_or("No physical connector evidence"),
                p.physical.netdev.as_deref().unwrap_or("unknown"),
                if live.get() {
                    speed(p.speed_mbps)
                } else {
                    "unknown".into()
                }
            )
        })
    });
    view! {
      <figure class=move||format!("nic-physical-port link-{}",state.get()) tabindex="0" title=move||description.get() aria-label=move||description.get()>
        <figcaption><strong>{move||port.with(|p|p.physical.role.clone().unwrap_or("Role unknown".into()))}</strong><span>{move||port.with(|p|format!("{} · P{}",if live.get(){speed(p.speed_mbps)}else{"—".into()},p.id))}</span></figcaption>
        {move||port.with(|p|{
            let (kind,path)=artwork(p.physical.connector.as_deref().unwrap_or("unknown"));
            view!{<svg class="nic-port-art" data-connector=kind viewBox="0 0 90 70" role="img" aria-label=format!("{} physical port, link {}",p.physical.connector.as_deref().unwrap_or("Unknown"),state.get())>
              <rect class="nic-bezel" x="5" y="7" width="80" height="56" rx="6"/>
              <rect class="nic-socket" x="12" y="14" width="66" height="42" rx="2"/>
              <path class="nic-contacts" d=path/>
              <circle class="nic-link-led" cx="11" cy="59" r="3"/>
            </svg>}
        })}
        <div class="nic-activity-leds" aria-label="Traffic activity in last counter interval">
          {["RX","TX"].into_iter().enumerate().map(|(i,label)|view!{
            <span class=if i==0{"nic-activity rx"}else{"nic-activity tx"} title=move||port.with(|p|{
                let count=if i==0{p.activity.rx_packets}else{p.activity.tx_packets};
                let bytes=if i==0{p.activity.rx_bytes}else{p.activity.tx_bytes};
                if state.get()!="up" || p.reset {format!("{label}: activity unavailable")}
                else {format!("{label}: {} packets, {} bytes over {} ms",count.map(|v|v.to_string()).unwrap_or("unknown".into()),bytes.map(|v|v.to_string()).unwrap_or("unknown".into()),p.activity.interval_ms.map(|v|v.to_string()).unwrap_or("unknown".into()))}
            })>
              <i class="nic-activity-led" class:pulsing=move||pulse.get()[i] data-active=move||port.with(|p|(state.get()=="up" && !p.reset && active(p)[i]).to_string()) aria-hidden="true"></i><span>{label}</span>
            </span>
          }).collect_view()}
        </div>
        <small class="nic-pci">{move||port.with(|p|p.physical.pci.clone().unwrap_or_else(||p.name.clone()))}</small>
        <span class="sr-only">{move||port.with(|p|if p.reset{"Counter reset"}else{""})}</span>
      </figure>
    }
}
#[component]
pub(crate) fn NicPorts(snapshot: Memo<Option<Dpdk>>, clock: Memo<u64>) -> impl IntoView {
    view! {
      <div class="dpdk-nic"><div class="nic-faceplate-heading"><h3>"NIC ports"</h3><span>{move||snapshot.with(|d|{
        let Some(d)=d else {return "Unavailable".into();};
        if !d.nic.live(clock.get(),15){return "Link unknown · stale".into();}
        format!("{} / {} UP",d.ports.iter().filter(|p|p.state.eq_ignore_ascii_case("up")).count(),d.ports.len())
      })}</span></div>
        <div class="nic-faceplate" role="group" aria-label="Physical NIC ports">
          <For each=move||snapshot.with(|d|d.as_ref().map(|d|d.ports.clone()).unwrap_or_default()) key=|p| (p.id,p.name.clone(),p.physical.pci.clone()) children=move |p:Port|view!{<PortTile id=p.id snapshot clock/>}/>
        </div>
        <Show when=move||snapshot.with(|d|d.as_ref().is_none_or(|d|d.ports.is_empty()))><p class="panel-note">"NIC telemetry unavailable."</p></Show>
        <p class="panel-note nic-cadence">"LED pulses show activity in each completed NIC sample, not individual packets."</p>
      </div>
    }
}
