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
#[cfg(any(target_arch = "wasm32", test))]
mod dpdk_history;
#[cfg(target_arch = "wasm32")]
mod live;
#[cfg(target_arch = "wasm32")]
mod traffic_chart;
#[cfg(any(target_arch = "wasm32", test))]
mod traffic_plot;
fn main() {
    #[cfg(target_arch = "wasm32")]
    leptos::mount::mount_to_body(app::App);
}
