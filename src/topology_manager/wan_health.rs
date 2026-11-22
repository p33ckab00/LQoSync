//! WAN availability and quality as the placement sees them. The WAN monitor produces them.
use super::rates::Fraction;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WanState {
    Up,
    /// No current probe result. Placement treats it as available, without quality.
    #[default]
    Unknown,
    /// A problem shorter than the move delay. Subscribers stay.
    Degraded,
    Unavailable,
    /// Healthy again, but not yet stable for the failback time.
    Recovering,
    /// Stable again with automatic failback off. Only a Rebalance releases it.
    WaitingForRebalance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WanStatus {
    pub state: WanState,
    /// 0 to 1. `None` when the data is stale: quality mode uses capacity.
    pub quality: Option<Fraction>,
}

impl WanStatus {
    /// True when placement must not use this WAN.
    pub fn excluded(&self) -> bool {
        matches!(
            self.state,
            WanState::Unavailable | WanState::Recovering | WanState::WaitingForRebalance
        )
    }
}

/// WAN node ID to its status. A WAN without an entry is `Unknown`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WanSnapshot(pub HashMap<String, WanStatus>);
