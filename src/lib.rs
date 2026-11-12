//! lqosync3: sync MikroTik subscribers to the LibreQoS shaping files.

/// `3.0.<commits>+g<hash>`: set by `build.rs`. The commit count orders the versions, the hash
/// names the source.
pub const VERSION: &str = env!("LQOSYNC_VERSION");

pub mod apply_evidence;
pub mod bridge;
pub mod circuit_queue;
pub mod cli;
pub mod collection;
pub mod config;
pub mod cycle;
pub mod detect;
pub mod diff;
pub mod files;
pub mod import_evidence;
pub mod mikrotik;
pub mod model;
pub mod nic_stats;
pub mod notify;
pub mod probe;
pub mod routeros;
pub mod routerstate;
pub mod runner;
pub mod secrets;
pub mod sources;
pub mod speed;
pub mod state;
pub mod status;
pub mod subscriber_stats;
pub mod system_health;
pub mod topology;
pub mod tui;
pub mod validate;
pub mod worker_util;
pub mod write_receipt;
pub mod writer;
