//! Topology Studio source jobs. Only the core service reads RouterOS for them.
pub mod candidate_cache;
pub mod connection;
pub mod evaluate;
pub mod snapshots;

use web_contract::source_studio::{
    FieldKey, FieldSpec, FilterOp, SourceKind, StudioCapabilities, StudioFeature, StudioLimits,
    VERSION,
};

/// The features whose plan task passed in this build. Add a feature only with its tests.
pub const FEATURES: [StudioFeature; 5] = [
    StudioFeature::DraftTest,
    StudioFeature::AccessProfiles,
    StudioFeature::SourceCollect,
    StudioFeature::WanEvidence,
    StudioFeature::PolicyPreview,
];

/// API `source_studio.capabilities`. It reads nothing. The fields are the ones that the
/// version 1 translation supports (`evaluate::translate`), not every reader field.
pub fn capabilities() -> StudioCapabilities {
    StudioCapabilities {
        version: VERSION,
        features: FEATURES.to_vec(),
        fields: vec![
            FieldSpec {
                field: FieldKey::AccountProfile,
                source: SourceKind::Ppp,
                operators: vec![],
                groupable: true,
            },
            FieldSpec {
                field: FieldKey::LeaseServer,
                source: SourceKind::Dhcp,
                operators: vec![FilterOp::Eq, FilterOp::In],
                groupable: true,
            },
        ],
        limits: StudioLimits::default(),
    }
}
