//! One-way authenticated observations. The browser owns reconnect and backoff.
use leptos::prelude::*;
use wasm_bindgen::{JsCast, closure::Closure};
use web_contract::{Overview, Principal};

pub(crate) fn mark_stale(data: RwSignal<Option<Overview>>) {
    data.update(|current| {
        if let Some(view) = current {
            view.stale = true;
            view.traffic_stale = true;
            if let Some(c) = &mut view.classification {
                c.mapped.stale = true;
                c.unmapped.stale = true;
            }
            view.wan_capacity = None;
            view.health = "UNKNOWN".into();
            if let Some(dpdk) = &mut view.dpdk {
                dpdk.analytics.stale = true;
                dpdk.health.stale = true;
                dpdk.nic.stale = true;
            }
            for component in &mut view.components {
                component.state = "UNKNOWN".into();
            }
        }
    });
}

struct LiveSource {
    source: web_sys::EventSource,
    _message: Closure<dyn FnMut(web_sys::MessageEvent)>,
    _error: Closure<dyn FnMut(web_sys::Event)>,
}
impl Drop for LiveSource {
    fn drop(&mut self) {
        self.source.close();
        self.source.set_onmessage(None);
        self.source.set_onerror(None);
    }
}

pub(crate) fn attach(
    session: RwSignal<Option<Principal>>,
    data: RwSignal<Option<Overview>>,
    error: RwSignal<String>,
) -> RwSignal<f64> {
    let received = RwSignal::new(0.0);
    let active = StoredValue::new_local(None::<LiveSource>);
    let connect = move || {
        active.update_value(|slot| {
            if web_sys::window().unwrap().document().unwrap().hidden() {
                *slot = None;
                received.set(0.0);
                return;
            }
            if slot.is_some() {
                return;
            }
            let expected = session.get_untracked().map(|s| s.csrf);
            let Ok(source) = web_sys::EventSource::new("/api/v1/overview/live") else {
                return;
            };
            let expected_error = expected.clone();
            let message = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(
                move |event: web_sys::MessageEvent| {
                    if expected != session.get_untracked().map(|s| s.csrf) {
                        return;
                    }
                    let Some(text) = event.data().as_string() else {
                        return;
                    };
                    if text == "unauthorized" {
                        session.set(None);
                        return;
                    }
                    let Ok(view) = serde_json::from_str::<Overview>(&text) else {
                        mark_stale(data);
                        error.set("Live traffic unavailable. Retrying connection.".into());
                        received.set(0.0);
                        return;
                    };
                    received.set(js_sys::Date::now());
                    if data.with_untracked(|old| old.as_ref() != Some(&view)) {
                        data.set(Some(view));
                    }
                    error.set(String::new());
                },
            );
            let failed = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
                if expected_error != session.get_untracked().map(|s| s.csrf) {
                    return;
                }
                mark_stale(data);
                received.set(0.0);
                error.set("Live traffic unavailable. Retrying connection.".into());
            });
            source.set_onmessage(Some(message.as_ref().unchecked_ref()));
            source.set_onerror(Some(failed.as_ref().unchecked_ref()));
            *slot = Some(LiveSource {
                source,
                _message: message,
                _error: failed,
            });
        });
    };
    connect();
    let visibility = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| connect());
    let document = web_sys::window().unwrap().document().unwrap();
    let _ = document
        .add_event_listener_with_callback("visibilitychange", visibility.as_ref().unchecked_ref());
    let visibility = StoredValue::new_local(visibility);
    on_cleanup(move || {
        active.try_update_value(|slot| *slot = None);
        visibility.try_with_value(|callback| {
            let document = web_sys::window().unwrap().document().unwrap();
            let _ = document.remove_event_listener_with_callback(
                "visibilitychange",
                callback.as_ref().unchecked_ref(),
            );
        });
    });
    received
}
