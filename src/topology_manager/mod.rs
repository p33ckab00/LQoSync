//! Topology policy, durable identities, and current observations. No live path calls this module.

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
