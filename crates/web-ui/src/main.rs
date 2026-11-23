#[cfg(target_arch = "wasm32")]
mod app;
#[cfg(target_arch = "wasm32")]
mod dashboard;
#[cfg(any(target_arch = "wasm32", test))]
mod diagnostics_activity;
#[cfg(target_arch = "wasm32")]
mod diagnostics_ticker;
#[cfg(target_arch = "wasm32")]
mod dpdk;
#[cfg(target_arch = "wasm32")]
mod live;
#[cfg(target_arch = "wasm32")]
mod traffic_chart;
#[cfg(any(target_arch = "wasm32", test))]
mod traffic_plot;
#[cfg(any(target_arch = "wasm32", test))]
mod wan_ring;
fn main() {
    #[cfg(target_arch = "wasm32")]
    leptos::mount::mount_to_body(app::App);
}

#[cfg(any(target_arch = "wasm32", test))]
mod nic_activity;
#[cfg(target_arch = "wasm32")]
mod nic_ports;

#[cfg(any(target_arch = "wasm32", test))]
mod nic_diagnostics;
#[cfg(target_arch = "wasm32")]
mod nic_hud;

#[cfg(target_arch = "wasm32")]
mod worker_ring;
#[cfg(any(target_arch = "wasm32", test))]
mod worker_ring_model;

#[cfg(any(target_arch = "wasm32", test))]
mod dpdk_console_model;

#[cfg(any(target_arch = "wasm32", test))]
mod canvas_layout;
#[cfg(target_arch = "wasm32")]
mod journal;
#[cfg(any(target_arch = "wasm32", test))]
mod journal_model;
#[cfg(target_arch = "wasm32")]
mod route;
#[cfg(any(target_arch = "wasm32", test))]
mod route_model;
#[cfg(target_arch = "wasm32")]
mod router_form;
#[cfg(any(target_arch = "wasm32", test))]
mod router_form_model;
#[cfg(target_arch = "wasm32")]
mod router_lists;
#[cfg(target_arch = "wasm32")]
mod routers;
#[cfg(any(target_arch = "wasm32", test))]
mod routers_model;
#[cfg(target_arch = "wasm32")]
mod subscriber_views;
#[cfg(target_arch = "wasm32")]
mod topology;
#[cfg(target_arch = "wasm32")]
mod topology_canvas;
#[cfg(any(target_arch = "wasm32", test))]
mod topology_canvas_model;
#[cfg(target_arch = "wasm32")]
mod topology_forms;
#[cfg(target_arch = "wasm32")]
mod topology_inventory;
#[cfg(any(target_arch = "wasm32", test))]
mod topology_inventory_model;
#[cfg(any(target_arch = "wasm32", test))]
mod topology_model;
#[cfg(target_arch = "wasm32")]
mod topology_studio_help;
#[cfg(target_arch = "wasm32")]
mod workers;
#[cfg(any(target_arch = "wasm32", test))]
mod workers_model;

#[cfg(any(target_arch = "wasm32", test))]
mod topology_studio_model;

#[cfg(target_arch = "wasm32")]
mod topology_source_editor;
#[cfg(target_arch = "wasm32")]
mod topology_studio;

#[cfg(target_arch = "wasm32")]
mod topology_studio_api;
#[cfg(target_arch = "wasm32")]
mod topology_studio_preview;
