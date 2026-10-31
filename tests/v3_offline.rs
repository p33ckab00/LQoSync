//! Manual tool: make `libreqos_network.json` from a LibreQoS pair, for the offline v3 parser check.
//! Run: LQ3_PAIR_DIR=/opt/libreqos/src LQ3_OUT=/path/libreqos_network.json \
//!      cargo test --test v3_offline -- --ignored
//! Then load the file with the v3 binding (docs/LIBREQOS-V3-INGRESS.md, "Simulation method").

#[test]
#[ignore = "manual: needs LQ3_PAIR_DIR and LQ3_OUT"]
fn pair_to_libreqos_network() {
    let dir = std::path::PathBuf::from(std::env::var("LQ3_PAIR_DIR").unwrap());
    let out = std::env::var("LQ3_OUT").unwrap();
    let csv = std::fs::read_to_string(dir.join("ShapedDevices.csv")).unwrap();
    let net: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("network.json")).unwrap()).unwrap();
    let rows = lqosync3::state::read_csv(&csv).unwrap();
    let (text, alerts) = lqosync3::writer::v3network::render_libreqos_network(&rows, &net).unwrap();
    std::fs::write(&out, text).unwrap();
    println!("rows {} alerts {alerts:?}", rows.len());
}
