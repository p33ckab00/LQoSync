#[cfg(target_arch = "wasm32")]
mod app;
fn main() {
    #[cfg(target_arch = "wasm32")]
    leptos::mount::mount_to_body(app::App);
}
