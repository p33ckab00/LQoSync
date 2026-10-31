//! Speed rules and rate values.

/// A download and upload rate in Mbps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rate {
    pub down: f64,
    pub up: f64,
}
