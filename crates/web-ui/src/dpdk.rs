//! The shared DPDK snapshot and server-time clock for the DPDK Health card.
use leptos::prelude::*;
use web_contract::{Overview, dpdk::Dpdk};

fn now() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}
// Share one snapshot and server-time clock between the ring, the bands, and the NIC ports.
pub(crate) fn monitoring(data: RwSignal<Option<Overview>>) -> (Memo<Option<Dpdk>>, Memo<u64>) {
    let snapshot = Memo::new(move |_| data.with(|v| v.as_ref().and_then(|v| v.dpdk.clone())));
    let clock = RwSignal::new(now());
    let anchor = RwSignal::new((0_u64, now()));
    Effect::new(move |_| {
        snapshot.with(|d| {
            if let Some(d) = d {
                anchor.set((d.sampled_at, now()));
            }
        });
    });
    // Compare source timestamps with server time, not the remote device's clock.
    let server_clock = Memo::new(move |_| {
        let (server, received) = anchor.get();
        if server == 0 {
            clock.get()
        } else {
            server.saturating_add(clock.get().saturating_sub(received))
        }
    });
    let timer = StoredValue::new_local(Some(gloo_timers::callback::Interval::new(
        1000,
        move || clock.set(now()),
    )));
    on_cleanup(move || {
        timer.try_update_value(|t| *t = None);
    });
    (snapshot, server_clock)
}
