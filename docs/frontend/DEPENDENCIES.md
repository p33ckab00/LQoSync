# Web dependency baseline

The workspace keeps `lqosync3` as its default member.
A normal root `cargo build` still builds the existing CLI and TUI.
Use `tools/frontend/build.sh` to build the separate web applications.
The build uses two Cargo jobs by default. Release builds use thin LTO and four codegen units.
These environment settings do not change the core release profile in `Cargo.toml`.

| Dependency | Selected version | Use |
|---|---|---|
| Leptos | 0.8.23 | Rust CSR browser components |
| wasm-bindgen | 0.2.108 | Generated browser bindings |
| js-sys / web-sys | 0.3.85 | Browser APIs |
| Axum | 0.8.9 | HTTP routing |
| tower-http | 0.6.6 | Static file responses |
| rusqlite | 0.37.0 | Bundled SQLite account and session stores |
| Argon2 | 0.5.3 | Argon2id password checks |
| Trunk | 0.22.2 | Browser build tool |

`Cargo.lock` fixes transitive versions. Use `--locked` for repeat builds.
Leptos 0.8.23 declares Rust 1.88. The project minimum remains Rust 1.89.

The 0.8.3 framework with current transitive macro versions did not compile.
The selected framework and binding versions passed dependency resolution together.

The workspace metadata has no selected package with a declared Rust minimum above 1.89.
Compilation checks are recorded separately in the evidence file.

The browser font is IBM Plex Sans, from the Google Fonts repository.
The repository includes its OFL license in `crates/web-ui/FONT-LICENSE.txt`.
The browser loads local assets. It does not contact a font CDN.

Trunk produces JavaScript bindings from Rust. The build moves its generated module to `bootstrap.js` for the script policy.
There is no handwritten JavaScript application logic. The `.cjs` file is a browser test only.

Sources: [Leptos manifest](https://docs.rs/crate/leptos/0.8.23/source/Cargo.toml),
[Google Fonts source](https://github.com/google/fonts/tree/main/ofl/ibmplexsans),
[Trunk](https://github.com/trunk-rs/trunk/releases/tag/v0.22.2).
