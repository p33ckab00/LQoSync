use super::*;
use crate::config::{RateLimitOrder, UnitlessUnit};
use crate::topology_manager::catalog::{SourceOrigin, tests::catalog};
use crate::topology_manager::identity::ReferenceAuthority;
use crate::topology_manager::source_policy::{ProfileBinding, RateRead, SourcePolicy};
use crate::topology_manager::source_rate::RateFormat;

pub(super) fn settings() -> SourceSettings {
    SourceSettings {
        selections: vec![SourceSelection {
            scope: catalog().required_scopes().unwrap()[0].clone(),
            service_names: vec!["pppoe".into()],
            authority: ReferenceAuthority::OperatorManagedComment,
            policy: SourcePolicy {
                profiles: vec![ProfileBinding {
                    source_ref: "provider-profile-id".into(),
                    current_name: "gold".into(),
                    rate: RateRead::RateLimit(RateFormat {
                        order: RateLimitOrder::RouterosRxTx,
                        unitless: UnitlessUnit::Bps,
                    }),
                }],
                unprofiled_rate: RateRead::Disabled,
                legacy_priority: None,
            },
        }],
    }
}

#[test]
fn strict_settings_roundtrip_keeps_explicit_modes_and_has_no_current_data() {
    let settings = settings();
    settings.validate(&catalog()).unwrap();
    let mut value = serde_json::to_value(&settings).unwrap();
    assert_eq!(
        serde_json::from_value::<SourceSettings>(value.clone()).unwrap(),
        settings
    );
    value["selections"][0]["policy"]["profiles"][0]["address"] = "192.0.2.1".into();
    assert!(serde_json::from_value::<SourceSettings>(value).is_err());
    let mut value = serde_json::to_value(settings).unwrap();
    value["selections"][0]
        .as_object_mut()
        .unwrap()
        .remove("authority");
    assert!(serde_json::from_value::<SourceSettings>(value).is_err());
}

#[test]
fn scope_coverage_and_duplicate_selections_are_rejected() {
    let mut settings = settings();
    settings.selections.clear();
    assert!(settings.validate(&catalog()).is_err());
    settings = self::settings();
    settings.selections.push(settings.selections[0].clone());
    assert!(settings.validate(&catalog()).is_err());
    settings.selections.pop();
    settings.selections[0].scope.router_id = "other".into();
    assert!(settings.validate(&catalog()).is_err());
}

#[test]
fn invalid_services_and_duplicate_profile_bindings_fail_before_reads() {
    for services in [vec![""], vec!["pppoe", "pppoe"], vec!["typo"]] {
        let mut settings = settings();
        settings.selections[0].service_names = services.into_iter().map(String::from).collect();
        assert!(settings.validate(&catalog()).is_err());
    }
    for duplicate_name in [true, false] {
        let mut settings = settings();
        let mut duplicate = settings.selections[0].policy.profiles[0].clone();
        if duplicate_name {
            duplicate.source_ref = "other".into();
        } else {
            duplicate.current_name = "other".into();
        }
        settings.selections[0].policy.profiles.push(duplicate);
        assert!(settings.validate(&catalog()).is_err());
    }
}

#[test]
fn selected_catalog_profiles_need_bindings_even_without_active_accounts() {
    let mut settings = settings();
    settings.selections[0].policy.profiles.clear();
    assert!(settings.validate(&catalog()).is_err());
    let mut catalog = catalog();
    if let SourceOrigin::Subscribers { profile_ref, .. } = &mut catalog.sources[2].origin {
        *profile_ref = None;
    }
    settings.validate(&catalog).unwrap();
}

#[test]
fn unconfigured_authority_can_be_saved_but_cannot_authorize_a_collection() {
    let mut settings = settings();
    settings.selections[0].authority = ReferenceAuthority::Unconfigured;
    settings.validate(&catalog()).unwrap();
    assert!(settings.ready_selections(&catalog()).is_err());
}

#[test]
fn all_explicit_rate_formats_roundtrip_and_missing_format_fields_fail() {
    for order in [RateLimitOrder::RouterosRxTx, RateLimitOrder::DownloadUpload] {
        for unitless in [UnitlessUnit::Bps, UnitlessUnit::Mbps] {
            let mut settings = settings();
            settings.selections[0].policy.profiles[0].rate =
                RateRead::RateLimit(RateFormat { order, unitless });
            let value = serde_json::to_value(&settings).unwrap();
            assert_eq!(
                serde_json::from_value::<SourceSettings>(value.clone()).unwrap(),
                settings
            );
            for field in ["order", "unitless"] {
                let mut missing = value.clone();
                missing["selections"][0]["policy"]["profiles"][0]["rate"]["RateLimit"]
                    .as_object_mut()
                    .unwrap()
                    .remove(field);
                assert!(serde_json::from_value::<SourceSettings>(missing).is_err());
            }
        }
    }
}

#[test]
fn overlapping_scopes_fail_in_both_orders_but_disjoint_services_are_valid() {
    let mut catalog = catalog();
    let mut source = catalog.sources[2].clone();
    source.id = "other-source".into();
    let SourceOrigin::Subscribers { scope, .. } = &mut source.origin else {
        unreachable!()
    };
    scope.source_id = "other-scope".into();
    let scope = scope.clone();
    catalog.sources.push(source);
    let mut settings = settings();
    let mut other = settings.selections[0].clone();
    other.scope = scope;
    other.service_names = vec!["sstp".into()];
    settings.selections.push(other);
    settings.validate(&catalog).unwrap();
    for whole in [false, true] {
        settings.selections[1].service_names = if whole { vec![] } else { vec!["pppoe".into()] };
        assert!(settings.validate(&catalog).is_err());
        settings.selections.reverse();
        assert!(settings.validate(&catalog).is_err());
        settings.selections.reverse();
    }
}

#[test]
fn configured_authorization_requires_settings_and_the_expected_revision() {
    use crate::topology_manager::identity::authorize_configured;
    use crate::topology_manager::model::RegistryDocument;
    let registry = RegistryDocument::default();
    let error = authorize_configured(&registry, 0, 1, vec![]).err().unwrap();
    assert!(error.to_string().contains("settings are unconfigured"));
    let error = authorize_configured(&registry, 1, 1, vec![]).err().unwrap();
    assert!(error.to_string().contains("revision conflict"));
}

#[test]
fn ten_thousand_profile_bindings_validate_and_canonicalize_independent_of_order() {
    let mut original = settings();
    original.selections[0]
        .policy
        .profiles
        .extend((0..10_000).map(|i| ProfileBinding {
            source_ref: format!("source-profile-{i}"),
            current_name: format!("plan-{i}"),
            rate: RateRead::Disabled,
        }));
    let mut reversed = original.clone();
    reversed.selections[0].policy.profiles.reverse();
    original.validate(&catalog()).unwrap();
    reversed.validate(&catalog()).unwrap();
    original.canonicalize();
    reversed.canonicalize();
    assert_eq!(original, reversed);
}
