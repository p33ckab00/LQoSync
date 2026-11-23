//! Topology Studio source jobs. Only the core service reads RouterOS for them.
pub mod connection;
pub mod snapshots;

use web_contract::source_studio::{StudioCapabilities, StudioFeature, StudioLimits, VERSION};

/// The features whose plan task passed in this build. Add a feature only with its tests.
pub const FEATURES: [StudioFeature; 4] = [
    StudioFeature::DraftTest,
    StudioFeature::AccessProfiles,
    StudioFeature::SourceCollect,
    StudioFeature::WanEvidence,
];

/// API `source_studio.capabilities`. It reads nothing.
pub fn capabilities() -> StudioCapabilities {
    StudioCapabilities {
        version: VERSION,
        features: FEATURES.to_vec(),
        // The source fields come with the snapshot reader (plan task B2).
        fields: Vec::new(),
        limits: StudioLimits::default(),
    }
}
