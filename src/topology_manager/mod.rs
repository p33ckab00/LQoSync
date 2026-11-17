//! Topology policy, durable identities, and current observations. Service activation remains gated.

pub mod aggregate;
pub mod catalog;
pub mod identity;
pub mod ids;
pub mod layout;
pub mod model;
pub mod normalize;
pub mod observation;
pub mod rates;
pub mod reader;
pub mod resolver;
pub use resolver::rebalance;
pub mod admission;
pub mod migration;
pub mod parity;
pub mod publication;
pub mod settings;
pub mod source_policy;
pub mod source_rate;
pub mod store;

pub mod ownership;

pub mod legacy_rate;

mod compiler_sandbox;

pub mod compiler;

pub(crate) mod runtime;

pub(crate) mod coordinator;

pub mod preflight;

mod dhcp_overrides;

pub mod legacy_bounds;

pub mod static_input;

mod shared_static;

pub mod legacy_static;

pub mod staging;

pub(crate) mod enrollment;
