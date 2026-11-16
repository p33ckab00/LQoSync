//! Topology policy, durable identities, and current observations. No live path calls this module.

pub mod aggregate;
pub mod catalog;
pub mod ids;
pub mod layout;
pub mod model;
pub mod observation;
pub mod rates;
pub mod reader;
pub mod resolver;
pub use resolver::rebalance;
pub mod store;
