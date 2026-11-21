//! The current page from the browser address, with the History API. No router crate.
use crate::route_model::{Page, parse};
use leptos::prelude::*;

fn current() -> Page {
    let location = web_sys::window().map(|w| w.location());
    let get = |f: fn(&web_sys::Location) -> Result<String, wasm_bindgen::JsValue>| {
        location
            .as_ref()
            .and_then(|l| f(l).ok())
            .unwrap_or_default()
    };
    parse(
        &get(web_sys::Location::pathname),
        &get(web_sys::Location::search),
    )
}

/// The page signal for the signed-in shell. Back and Forward update it.
pub(crate) fn provide() -> RwSignal<Page> {
    let page = RwSignal::new(current());
    let listener = window_event_listener(leptos::ev::popstate, move |_| page.set(current()));
    on_cleanup(move || listener.remove());
    provide_context(page);
    page
}

pub(crate) fn page() -> RwSignal<Page> {
    expect_context::<RwSignal<Page>>()
}

/// Go to `href` inside the page. Then move focus to the page heading, for screen readers.
pub(crate) fn go(href: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    if let Ok(history) = window.history() {
        let _ = history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(href));
    }
    page().set(current());
    window.scroll_to_with_x_and_y(0.0, 0.0);
    request_animation_frame(|| {
        if let Some(h) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.query_selector("#main h1").ok().flatten())
            .and_then(|h| wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(h).ok())
        {
            let _ = h.focus();
        }
    });
}

/// A same-page link: a plain click stays in the page; a modified click keeps the browser default.
#[component]
pub(crate) fn Link(
    href: String,
    #[prop(optional, into)] class: String,
    #[prop(optional)] current: Option<Signal<bool>>,
    children: Children,
) -> impl IntoView {
    let target = href.clone();
    view! {
      <a href=href class=class aria-current=move||current.and_then(|c|c.get().then_some("page"))
        on:click=move|ev: web_sys::MouseEvent|{
          if ev.button()==0 && !(ev.ctrl_key()||ev.meta_key()||ev.shift_key()||ev.alt_key()) {
            ev.prevent_default();
            go(&target);
          }
        }>{children()}</a>
    }
}
