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
fn artwork(connector: &str) -> &'static str {
    match connector {
        "rj45" => "rj45",
        "sfp" | "sfp+" | "sfp28" => "sfp",
        "qsfp" | "qsfp+" | "qsfp28" => "qsfp",
        _ => "unknown",
    }
}
#[component]
fn PortTile(
    id: u16,
    snapshot: Memo<Option<Dpdk>>,
    clock: Memo<u64>,
    active_port: RwSignal<Option<u16>>,
) -> impl IntoView {
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
    let history = RwSignal::new(crate::nic_diagnostics::History::default());
    Effect::new(move |_| {
        let p = port.get();
        let fresh = live.get();
        let stale = snapshot.with(|d| d.as_ref().is_none_or(|d| d.nic.stale));
        history.update(|h| {
            if stale {
                h.invalidate();
            } else {
                h.observe(&p, fresh);
            }
        });
    });
    let open = Memo::new(move |_| active_port.get() == Some(id));
    let set_open = move |show| {
        if show {
            active_port.set(Some(id));
        } else if active_port.get_untracked() == Some(id) {
            active_port.set(None);
        }
    };
    let pinned = RwSignal::new(false);
    let hover = StoredValue::new_local(None::<gloo_timers::callback::Timeout>);
    Effect::new(move |_| {
        if !open.get() {
            pinned.set(false);
            hover.set_value(None);
        }
    });
    let tile = NodeRef::<leptos::html::Figure>::new();
    let position = RwSignal::new(String::new());
    let place = move || {
        if let Some(node) = tile.get_untracked() {
            let rect = node.get_bounding_client_rect();
            let window = web_sys::window().expect("browser window");
            let width = window
                .inner_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(320.0);
            let height = window
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(600.0);
            let panel = (width - 32.0).min(680.0);
            let left = rect.left().clamp(16.0, (width - panel - 16.0).max(16.0));
            if rect.bottom() < 0.0 || rect.top() > height {
                set_open(false);
                return;
            }
            let below = height - rect.bottom() - 24.0;
            let (top, available) = if below >= 240.0 {
                ((rect.bottom() + 8.0).max(16.0), below.min(height - 32.0))
            } else {
                (16.0, (rect.top() - 24.0).max(120.0).min(height - 32.0))
            };
            position.set(format!(
                "left:{left}px;top:{top}px;max-height:{available}px"
            ));
        }
    };
    let close = Callback::new(move |()| {
        hover.set_value(None);
        set_open(false);
        pinned.set(false);
        if let Some(n) = tile.get_untracked() {
            let _ = n.focus();
        }
    });
    let key_listener =
        leptos::leptos_dom::helpers::window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked() && ev.key() == "Escape" {
                ev.prevent_default();
                close.run(());
            }
        });
    let resize_listener =
        leptos::leptos_dom::helpers::window_event_listener(leptos::ev::resize, move |_| {
            if open.get_untracked() {
                place();
            }
        });
    let scroll_listener =
        leptos::leptos_dom::helpers::window_event_listener(leptos::ev::scroll, move |_| {
            if open.get_untracked() {
                place();
            }
        });
    on_cleanup(move || {
        hover.try_update_value(|v| *v = None);
        key_listener.remove();
        resize_listener.remove();
        scroll_listener.remove();
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
          <div class="nic-port-control"
            on:pointerenter=move |ev|{if ev.pointer_type()=="mouse"{hover.set_value(Some(gloo_timers::callback::Timeout::new(180,move||{place();set_open(true);})));}}
            on:pointerleave=move |_|{hover.set_value(None);if !pinned.get_untracked(){hover.set_value(Some(gloo_timers::callback::Timeout::new(220,move||set_open(false))));}}
    >
          <figure node_ref=tile class=move||format!("nic-physical-port link-{}",state.get()) tabindex="0" aria-label=move||description.get() role="button" aria-expanded=move||open.get().to_string()
            on:click=move|_|{hover.set_value(None);place();let next=!pinned.get_untracked();pinned.set(next);set_open(next);}
            on:keydown=move|ev|{if ev.key()=="Enter" || ev.key()==" "{ev.prevent_default();hover.set_value(None);place();pinned.set(true);set_open(true);}}>

            <figcaption><strong>{move||port.with(|p|p.physical.role.clone().unwrap_or("Role unknown".into()))}</strong><span>{move||port.with(|p|format!("{} · P{}",if live.get(){speed(p.speed_mbps)}else{"—".into()},p.id))}</span></figcaption>
            {move||port.with(|p|{
                let kind=artwork(p.physical.connector.as_deref().unwrap_or("unknown"));
                view!{<svg class="nic-port-art" data-connector=kind viewBox="0 0 100 88" role="img" aria-label=format!("{} physical port, link {}",p.physical.connector.as_deref().unwrap_or("Unknown"),state.get())>
                  {if kind=="unknown" {
                    view!{<g><rect class="nic-bezel" x="3" y="6" width="94" height="67" rx="9"/><rect class="nic-socket" x="12" y="16" width="76" height="46" rx="3"/><path class="nic-contacts" d="M40 30C40 22 58 22 58 32C58 38 49 38 49 44M49 49V51"/></g>}.into_any()
                  } else {
                    view!{<use href=format!("#lqosync-nic-{kind}")/>}.into_any()
                  }}
                  <rect class="nic-link-outline" x="1" y="5" width="98" height="69" rx="9"/>
                  <circle class="nic-link-led" cx="11" cy="80" r="3"/>
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
          <Show when=move||open.get()><div class="nic-hud-position" style=move||position.get()><crate::nic_hud::NicHud port live history on_close=close/></div></Show>
          </div>
        }
}
#[component]
pub(crate) fn NicPorts(snapshot: Memo<Option<Dpdk>>, clock: Memo<u64>) -> impl IntoView {
    let active_port = RwSignal::new(None::<u16>);
    view! {
      <div class="dpdk-nic">
        // This fragment is reviewed repository artwork, never API or user input.
        <svg class="nic-art-defs" width="0" height="0" aria-hidden="true" focusable="false" inner_html=include_str!("nic_art.svg")/>
        <div class="nic-faceplate-heading"><h2>"NIC ports"</h2><span>{move||snapshot.with(|d|{
        let Some(d)=d else {return "Unavailable".into();};
        if !d.nic.live(clock.get(),15){return "Link unknown · stale".into();}
        format!("{} / {} UP",d.ports.iter().filter(|p|p.state.eq_ignore_ascii_case("up")).count(),d.ports.len())
      })}</span></div>
        <div class="nic-faceplate" role="group" aria-label="Physical NIC ports">
          <For each=move||snapshot.with(|d|d.as_ref().map(|d|d.ports.clone()).unwrap_or_default()) key=|p| (p.id,p.name.clone(),p.physical.pci.clone()) children=move |p:Port|view!{<PortTile id=p.id snapshot clock active_port/>}/>
        </div>
        <Show when=move||snapshot.with(|d|d.as_ref().is_none_or(|d|d.ports.is_empty()))><p class="panel-note">"NIC telemetry unavailable."</p></Show>
        <p class="sr-only nic-cadence">"LED pulses show activity in each completed NIC sample, not individual packets."</p>
      </div>
    }
}
