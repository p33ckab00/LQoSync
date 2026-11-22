//! Durable identity and policy intent. Current endpoints do not belong here.

use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Ppp,
    Dhcp,
    Hotspot,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceScope {
    pub router_id: String,
    pub source_id: String,
    pub kind: SourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityRef {
    pub scope: SourceScope,
    /// A verified account/subscriber reference, never an address inferred from a past session.
    pub account_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubscriberPolicy {
    pub profile_id: Option<String>,
    pub assignment: Option<String>,
    pub pin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubscriberRecord {
    pub id: String,
    pub identities: Vec<IdentityRef>,
    pub policy: SubscriberPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryDocument {
    pub schema_version: u32,
    pub revision: u64,
    pub next_id: u64,
    pub subscribers: Vec<SubscriberRecord>,
    pub catalog: Option<super::catalog::TopologyCatalog>,
    pub source_settings: Option<super::settings::SourceSettings>,
    pub export_ownership: Option<super::ownership::ExportOwnership>,
    /// The operator explicitly created this policy in the frontend Builder.
    #[serde(default, skip_serializing_if = "is_false")]
    pub frontend_initialized: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// The current registry schema. Schema 5 adds node `geo` (type and location).
pub const SCHEMA: u32 = 7;

impl Default for RegistryDocument {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA,
            revision: 0,
            next_id: 1,
            subscribers: Vec::new(),
            catalog: None,
            source_settings: None,
            export_ownership: None,
            frontend_initialized: false,
        }
    }
}

impl RegistryDocument {
    pub fn validate(&self) -> anyhow::Result<()> {
        self.index().map(|_| ())
    }

    pub(super) fn index(&self) -> anyhow::Result<RegistryIndex<'_>> {
        ensure!(
            self.schema_version == SCHEMA,
            "unsupported subscriber registry schema"
        );
        ensure!(self.next_id > 0, "subscriber ID counter must be positive");
        let mut by_id = HashMap::with_capacity(self.subscribers.len());
        let mut by_identity =
            HashMap::with_capacity(self.subscribers.iter().map(|s| s.identities.len()).sum());
        for (index, subscriber) in self.subscribers.iter().enumerate() {
            ensure!(
                !subscriber.id.trim().is_empty(),
                "subscriber ID must not be empty"
            );
            ensure!(
                by_id.insert(subscriber.id.as_str(), index).is_none(),
                "duplicate subscriber ID"
            );
            if let Some(sequence) = subscriber.id.strip_prefix("tm-sub-") {
                ensure!(
                    sequence.len() == 16
                        && sequence
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                    "invalid generated subscriber ID"
                );
                let number =
                    u64::from_str_radix(sequence, 16).context("invalid generated subscriber ID")?;
                ensure!(
                    number > 0 && number < self.next_id,
                    "subscriber ID counter must exceed allocated IDs"
                );
            }
            ensure!(
                !subscriber.identities.is_empty(),
                "subscriber requires a verified identity reference"
            );
            for identity in &subscriber.identities {
                identity.validate()?;
                ensure!(
                    by_identity.insert(identity, index).is_none(),
                    "duplicate subscriber identity reference"
                );
            }
            for id in [
                &subscriber.policy.profile_id,
                &subscriber.policy.assignment,
                &subscriber.policy.pin,
            ]
            .into_iter()
            .flatten()
            {
                ensure!(
                    !id.trim().is_empty(),
                    "subscriber policy reference must not be empty"
                );
            }
            if let (Some(assignment), Some(pin)) =
                (&subscriber.policy.assignment, &subscriber.policy.pin)
            {
                ensure!(
                    assignment == pin,
                    "subscriber assignment conflicts with its pin"
                );
            }
        }
        let required_scopes = self
            .catalog
            .as_ref()
            .map(|catalog| catalog.checked_scopes(&self.subscribers))
            .transpose()?;
        if let Some(catalog) = &self.catalog {
            use super::ids::{IdKind, validate_id};
            for router in &catalog.routers {
                validate_id(&router.id, IdKind::Router, self.next_id)?;
            }
            for source in &catalog.sources {
                validate_id(&source.id, IdKind::Source, self.next_id)?;
            }
            for profile in &catalog.profiles {
                validate_id(&profile.id, IdKind::Profile, self.next_id)?;
            }
            for node in &catalog.nodes {
                validate_id(&node.id, IdKind::Node, self.next_id)?;
            }
        }
        if let Some(settings) = &self.source_settings {
            settings.validate(
                self.catalog
                    .as_ref()
                    .context("source settings require a catalog")?,
            )?;
            let mut override_rates = HashMap::new();
            for selection in &settings.selections {
                for binding in &selection.policy.dhcp_overrides {
                    let identity = IdentityRef {
                        scope: selection.scope.clone(),
                        account_ref: binding.account_ref.clone(),
                    };
                    let subscriber = by_identity
                        .get(&identity)
                        .context("DHCP override requires an enrolled independent identity")?;
                    ensure!(
                        override_rates
                            .insert(*subscriber, binding.rate)
                            .is_none_or(|rate| rate == binding.rate),
                        "subscriber aliases have conflicting DHCP overrides"
                    );
                }
            }
        }
        if let Some(ownership) = &self.export_ownership {
            ownership.validate(self)?;
        }
        Ok(RegistryIndex {
            by_id,
            by_identity,
            required_scopes,
        })
    }
}

pub(super) struct RegistryIndex<'a> {
    pub by_id: HashMap<&'a str, usize>,
    pub by_identity: HashMap<&'a IdentityRef, usize>,
    pub required_scopes: Option<Vec<SourceScope>>,
}

impl SourceScope {
    pub(super) fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            !self.router_id.trim().is_empty(),
            "source router ID must not be empty"
        );
        ensure!(
            !self.source_id.trim().is_empty(),
            "source ID must not be empty"
        );
        Ok(())
    }
}

impl IdentityRef {
    pub(super) fn validate(&self) -> anyhow::Result<()> {
        self.scope.validate()?;
        ensure!(
            !self.account_ref.trim().is_empty(),
            "subscriber requires an authoritative account reference"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn identity(account: &str) -> IdentityRef {
        IdentityRef {
            scope: SourceScope {
                router_id: "router-id".into(),
                source_id: "ppp-service-id".into(),
                kind: SourceKind::Ppp,
            },
            account_ref: account.into(),
        }
    }

    fn registry() -> RegistryDocument {
        RegistryDocument {
            next_id: 2,
            subscribers: vec![SubscriberRecord {
                id: "tm-sub-0000000000000001".into(),
                identities: vec![identity("account-1"), identity("explicit-alias")],
                policy: SubscriberPolicy {
                    profile_id: Some("plan-id".into()),
                    assignment: Some("instance-id".into()),
                    pin: Some("instance-id".into()),
                },
            }],
            ..RegistryDocument::default()
        }
    }

    #[test]
    fn valid_registry_keeps_explicit_aliases_and_policy_intent() {
        registry().validate().unwrap();
        RegistryDocument::default().validate().unwrap();
        let json = serde_json::to_string(&registry()).unwrap();
        let restored: RegistryDocument = serde_json::from_str(&json).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored, registry());
    }

    #[test]
    fn duplicate_subscriber_ids_and_identity_ownership_are_errors() {
        let mut duplicate = registry();
        duplicate.subscribers.push(duplicate.subscribers[0].clone());
        assert!(duplicate.validate().is_err());
        duplicate.subscribers[1].id = "another-stable-id".into();
        assert!(duplicate.validate().is_err());
        let mut alias = registry();
        alias.subscribers[0].identities.push(identity("account-1"));
        assert!(alias.validate().is_err());
    }

    #[test]
    fn source_scope_separates_equal_account_names() {
        let mut doc = registry();
        let mut another = doc.subscribers[0].clone();
        another.id = "legacy-stable-id".into();
        for reference in &mut another.identities {
            reference.scope.router_id = "other-router".into();
        }
        doc.subscribers.push(another);
        doc.validate().unwrap();
    }

    #[test]
    fn empty_identity_evidence_is_rejected() {
        for field in 0..4 {
            let mut doc = registry();
            match field {
                0 => doc.subscribers[0].id = " ".into(),
                1 => doc.subscribers[0].identities[0].scope.router_id.clear(),
                2 => doc.subscribers[0].identities[0].scope.source_id.clear(),
                _ => doc.subscribers[0].identities[0].account_ref = " ".into(),
            }
            assert!(doc.validate().is_err());
        }
        let mut doc = registry();
        doc.subscribers[0].identities.clear();
        assert!(doc.validate().is_err());
    }

    #[test]
    fn blank_policy_ids_and_conflicting_pin_are_rejected() {
        for field in 0..3 {
            let mut doc = registry();
            let policy = &mut doc.subscribers[0].policy;
            match field {
                0 => policy.profile_id = Some(" ".into()),
                1 => policy.assignment = Some(" ".into()),
                _ => policy.pin = Some(" ".into()),
            }
            assert!(doc.validate().is_err());
        }
        let mut doc = registry();
        doc.subscribers[0].policy.pin = Some("different-instance".into());
        assert!(doc.validate().is_err());
    }

    #[test]
    fn unsupported_schema_and_unsafe_id_counters_are_rejected() {
        for version in [0, 1, 2, 3, u32::MAX] {
            let mut doc = registry();
            doc.schema_version = version;
            assert!(doc.validate().is_err());
        }
        for counter in [0, 1] {
            let mut doc = registry();
            doc.next_id = counter;
            assert!(doc.validate().is_err());
        }
        for id in [
            "tm-sub-xyz",
            "tm-sub-0000000000000000",
            "tm-sub-000000000000000A",
        ] {
            let mut doc = registry();
            doc.subscribers[0].id = id.into();
            assert!(doc.validate().is_err());
        }
    }

    #[test]
    fn strict_json_rejects_endpoint_fields_and_duplicate_named_fields() {
        let mut json = serde_json::to_value(registry()).unwrap();
        json["subscribers"][0]["ipv4"] = serde_json::json!("192.0.2.1");
        assert!(serde_json::from_value::<RegistryDocument>(json).is_err());
        let text = r#"{"schema_version":1,"revision":0,"revision":1,"next_id":1,"subscribers":[]}"#;
        assert!(serde_json::from_str::<RegistryDocument>(text).is_err());
    }

    #[test]
    fn current_schema_checks_catalog_references_without_guessing_legacy_bindings() {
        assert_eq!(RegistryDocument::default().schema_version, SCHEMA);
        let mut doc = registry();
        doc.catalog = Some(super::super::catalog::tests::catalog());
        assert!(doc.validate().is_err());
        doc.subscribers[0].identities.clear();
        doc.subscribers[0].identities.push(IdentityRef {
            scope: SourceScope {
                router_id: "distro".into(),
                source_id: "ppp-service".into(),
                kind: SourceKind::Ppp,
            },
            account_ref: "account-id".into(),
        });
        doc.subscribers[0].policy = SubscriberPolicy {
            profile_id: Some("local-plan".into()),
            assignment: Some("group1".into()),
            pin: Some("group1".into()),
        };
        doc.validate().unwrap();
        let mut orphan = doc.clone();
        orphan.catalog.as_mut().unwrap().profiles.clear();
        orphan.catalog.as_mut().unwrap().sources[2].profile_id = None;
        assert!(orphan.validate().is_err());
    }
}
