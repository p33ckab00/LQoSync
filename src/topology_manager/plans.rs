//! LQoSync speed plans: `Local` profiles that the operator attaches to subscribers.

use super::catalog::{BaseRateAuthority, SubscriberProfile};
use super::model::{IdentityRef, RegistryDocument, SubscriberPolicy, SubscriberRecord};
use super::rates::{RateBps, SpeedOverrides};
use super::store::InvalidPolicy;

const PREFIX: &str = "plan-";

fn invalid(message: &str) -> anyhow::Error {
    anyhow::Error::new(InvalidPolicy(message.into()))
}

/// A plan is a profile with the `plan-` prefix and a fixed local base rate.
pub fn is_plan(p: &SubscriberProfile) -> bool {
    p.id.starts_with(PREFIX) && matches!(p.base, BaseRateAuthority::Local(_))
}

pub(crate) fn plan_profile(
    id: &str,
    name: &str,
    down_mbps: u64,
    up_mbps: u64,
) -> SubscriberProfile {
    SubscriberProfile {
        id: id.into(),
        name: name.into(),
        base: BaseRateAuthority::Local(RateBps {
            down: down_mbps * 1_000_000,
            up: up_mbps * 1_000_000,
        }),
        speed: SpeedOverrides::default(),
        eligible_targets: None,
    }
}

fn enrolled(doc: &RegistryDocument) -> anyhow::Result<RegistryDocument> {
    if doc.export_ownership.is_none() {
        return Err(invalid(
            "enroll first: the registry has no export ownership",
        ));
    }
    if doc.catalog.is_none() {
        return Err(invalid(
            "enroll first: the registry has no export ownership",
        ));
    }
    Ok(doc.clone())
}

fn check_rates(down: u64, up: u64) -> anyhow::Result<()> {
    if down == 0 || up == 0 || down > 1_000_000 || up > 1_000_000 {
        return Err(invalid(
            "a plan rate must be a positive whole number of Mbps",
        ));
    }
    Ok(())
}

fn check_name(doc: &RegistryDocument, name: &str, except: Option<&str>) -> anyhow::Result<()> {
    let len = name.chars().count();
    if len == 0 || len > 64 || name.trim() != name {
        return Err(invalid(
            "a plan name must have 1 to 64 characters without outer spaces",
        ));
    }
    let taken = doc
        .catalog
        .iter()
        .flat_map(|c| &c.profiles)
        .any(|p| is_plan(p) && p.name == name && Some(p.id.as_str()) != except);
    if taken {
        return Err(invalid("a plan with this name exists"));
    }
    Ok(())
}

fn validated(doc: RegistryDocument) -> anyhow::Result<RegistryDocument> {
    doc.validate().map_err(|e| invalid(&format!("{e:#}")))?;
    Ok(doc)
}

pub fn create(
    doc: &RegistryDocument,
    name: &str,
    down_mbps: u64,
    up_mbps: u64,
) -> anyhow::Result<(RegistryDocument, String)> {
    let mut doc = enrolled(doc)?;
    check_name(&doc, name, None)?;
    check_rates(down_mbps, up_mbps)?;
    let catalog = doc.catalog.as_mut().expect("checked above");
    let next = catalog
        .profiles
        .iter()
        .filter(|p| is_plan(p))
        .filter_map(|p| p.id[PREFIX.len()..].parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    let id = format!("{PREFIX}{next}");
    catalog
        .profiles
        .push(plan_profile(&id, name, down_mbps, up_mbps));
    Ok((validated(doc)?, id))
}

fn plan_mut<'a>(
    doc: &'a mut RegistryDocument,
    plan_id: &str,
) -> anyhow::Result<&'a mut SubscriberProfile> {
    doc.catalog
        .as_mut()
        .and_then(|c| {
            c.profiles
                .iter_mut()
                .find(|p| p.id == plan_id && is_plan(p))
        })
        .ok_or_else(|| invalid("the plan does not exist"))
}

pub fn update(
    doc: &RegistryDocument,
    plan_id: &str,
    name: Option<&str>,
    down_mbps: Option<u64>,
    up_mbps: Option<u64>,
) -> anyhow::Result<RegistryDocument> {
    let mut doc = enrolled(doc)?;
    if let Some(name) = name {
        check_name(&doc, name, Some(plan_id))?;
    }
    let plan = plan_mut(&mut doc, plan_id)?;
    let BaseRateAuthority::Local(rate) = plan.base else {
        return Err(invalid("the plan does not exist"));
    };
    let down = down_mbps.unwrap_or(rate.down / 1_000_000);
    let up = up_mbps.unwrap_or(rate.up / 1_000_000);
    check_rates(down, up)?;
    *plan = plan_profile(plan_id, name.unwrap_or(&plan.name), down, up);
    validated(doc)
}

pub fn delete(doc: &RegistryDocument, plan_id: &str) -> anyhow::Result<RegistryDocument> {
    let mut doc = enrolled(doc)?;
    plan_mut(&mut doc, plan_id)?;
    if doc
        .subscribers
        .iter()
        .any(|s| s.policy.profile_id.as_deref() == Some(plan_id))
    {
        return Err(invalid("the plan is in use"));
    }
    if let Some(c) = doc.catalog.as_mut() {
        c.profiles.retain(|p| p.id != plan_id);
    }
    validated(doc)
}

/// Attach a plan to the subscriber with `identity`, or detach with `None`.
/// A subscriber without a record gets one only when the latest preview shows the account.
pub fn attach(
    doc: &RegistryDocument,
    identity: &IdentityRef,
    plan_id: Option<&str>,
    in_preview: bool,
) -> anyhow::Result<(RegistryDocument, String)> {
    let mut doc = enrolled(doc)?;
    if let Some(id) = plan_id {
        plan_mut(&mut doc, id)?;
    }
    let bound = doc
        .source_settings
        .iter()
        .flat_map(|s| &s.selections)
        .any(|s| {
            s.scope == identity.scope
                && s.policy
                    .dhcp_overrides
                    .iter()
                    .any(|b| b.account_ref == identity.account_ref)
        });
    if bound {
        return Err(invalid("remove the DHCP client override first"));
    }
    let existing = doc
        .subscribers
        .iter_mut()
        .find(|s| s.identities.contains(identity));
    let subscriber_id = match existing {
        Some(record) => {
            record.policy.profile_id = plan_id.map(str::to_string);
            record.id.clone()
        }
        None if in_preview => {
            let id = format!("tm-sub-{:016x}", doc.next_id);
            doc.next_id += 1;
            doc.subscribers.push(SubscriberRecord {
                id: id.clone(),
                identities: vec![identity.clone()],
                policy: SubscriberPolicy {
                    profile_id: plan_id.map(str::to_string),
                    assignment: None,
                    pin: None,
                },
            });
            id
        }
        None => return Err(invalid("the account is not in the latest preview")),
    };
    Ok((validated(doc)?, subscriber_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RouterSnapshot;
    use crate::topology_manager::identity::authorize_configured;
    use crate::topology_manager::model::{IdentityRef, RegistryDocument, SourceKind, SourceScope};
    use crate::topology_manager::observation::ObservationGate;
    use crate::topology_manager::policy_draft::{document, propose, tests as draft};
    use crate::topology_manager::store::InvalidPolicy;

    fn enrolled() -> RegistryDocument {
        let f = crate::topology_manager::legacy_static::tests::fixture();
        crate::topology_manager::legacy_static::adopt(
            &f.pair,
            &f.rows,
            &f.network,
            &f.manifest,
            &f.roots,
            &f.attachments,
        )
        .unwrap()
        .registry
    }
    fn rule(e: anyhow::Error) -> String {
        e.chain()
            .find_map(|c| c.downcast_ref::<InvalidPolicy>())
            .map(|i| i.0.clone())
            .unwrap_or_else(|| format!("not a rule error: {e:#}"))
    }
    fn first_identity(doc: &RegistryDocument) -> IdentityRef {
        doc.subscribers[0].identities[0].clone()
    }

    #[test]
    fn create_adds_a_local_plan_with_the_next_number() {
        let (doc, a) = create(&enrolled(), "100M", 100, 100).unwrap();
        let (doc, b) = create(&doc, "50M", 50, 50).unwrap();
        assert_eq!((a.as_str(), b.as_str()), ("plan-1", "plan-2"));
        let p = doc
            .catalog
            .as_ref()
            .unwrap()
            .profiles
            .iter()
            .find(|p| p.id == "plan-1")
            .unwrap();
        assert!(is_plan(p));
        assert_eq!(p.name, "100M");
    }

    #[test]
    fn names_must_be_unique_and_rates_positive() {
        let (doc, _) = create(&enrolled(), "100M", 100, 100).unwrap();
        assert!(rule(create(&doc, "100M", 10, 10).unwrap_err()).contains("name"));
        assert!(rule(create(&doc, "zero", 0, 10).unwrap_err()).contains("rate"));
        assert!(rule(create(&doc, "", 10, 10).unwrap_err()).contains("name"));
        assert!(rule(create(&doc, &"x".repeat(65), 10, 10).unwrap_err()).contains("name"));
    }

    #[test]
    fn update_and_delete_touch_only_plans() {
        let base = enrolled();
        let (doc, id) = create(&base, "100M", 100, 100).unwrap();
        let doc = update(&doc, &id, Some("120M"), Some(120), None).unwrap();
        let p = doc
            .catalog
            .as_ref()
            .unwrap()
            .profiles
            .iter()
            .find(|p| p.id == id)
            .unwrap();
        assert_eq!(p.name, "120M");
        let other = base.catalog.as_ref().unwrap().profiles[0].id.clone();
        assert!(update(&doc, &other, Some("x"), None, None).is_err());
        assert!(delete(&doc, &other).is_err());
        let doc = delete(&doc, &id).unwrap();
        assert!(
            !doc.catalog
                .as_ref()
                .unwrap()
                .profiles
                .iter()
                .any(|p| p.id == id)
        );
    }

    #[test]
    fn deleting_a_used_plan_is_refused_and_changes_nothing() {
        let (doc, id) = create(&enrolled(), "100M", 100, 100).unwrap();
        let (doc, _) = attach(&doc, &first_identity(&doc), Some(&id), false).unwrap();
        assert_eq!(rule(delete(&doc, &id).unwrap_err()), "the plan is in use");
    }

    #[test]
    fn edits_before_enrollment_are_refused() {
        let mut doc = enrolled();
        doc.export_ownership = None;
        assert_eq!(
            rule(create(&doc, "100M", 100, 100).unwrap_err()),
            "enroll first: the registry has no export ownership"
        );
    }

    #[test]
    fn attach_creates_a_record_from_the_preview_with_the_next_id() {
        let (doc, id) = create(&enrolled(), "100M", 100, 100).unwrap();
        let mut identity = first_identity(&doc);
        identity.account_ref = "dhcp-mac/v1/02:00:00:00:99:01".into();
        let before = doc.next_id;
        let (doc, sub) = attach(&doc, &identity, Some(&id), true).unwrap();
        assert_eq!(sub, format!("tm-sub-{before:016x}"));
        assert_eq!(doc.next_id, before + 1);
        let rec = doc.subscribers.iter().find(|s| s.id == sub).unwrap();
        assert_eq!(rec.policy.profile_id.as_deref(), Some(id.as_str()));
        assert_eq!(rec.identities, vec![identity]);
    }

    #[test]
    fn attach_refuses_an_account_missing_from_the_preview() {
        let doc = enrolled();
        let mut identity = first_identity(&doc);
        identity.account_ref = "dhcp-mac/v1/02:00:00:00:99:02".into();
        assert_eq!(
            rule(attach(&doc, &identity, None, false).unwrap_err()),
            "the account is not in the latest preview"
        );
    }

    #[test]
    fn attach_refuses_a_dhcp_override_binding() {
        let (mut doc, id) = create(&enrolled(), "100M", 100, 100).unwrap();
        let identity = first_identity(&doc);
        // The legacy-adopted fixture has no source settings: add one selection with a binding.
        doc.source_settings = Some(crate::topology_manager::settings::SourceSettings {
            selections: vec![crate::topology_manager::identity::SourceSelection {
                scope: identity.scope.clone(),
                service_names: vec![],
                authority: crate::topology_manager::identity::ReferenceAuthority::DhcpClientMac,
                policy: crate::topology_manager::source_policy::SourcePolicy {
                    dhcp_overrides: vec![
                        crate::topology_manager::source_policy::DhcpOverrideBinding {
                            legacy_client_index: 0,
                            account_ref: identity.account_ref.clone(),
                            rate: crate::topology_manager::rates::RateBps {
                                down: 10_000_000,
                                up: 10_000_000,
                            },
                        },
                    ],
                    profiles: vec![],
                    unprofiled_rate: Default::default(),
                    legacy_priority: None,
                },
            }],
        });
        assert_eq!(
            rule(attach(&doc, &identity, Some(&id), false).unwrap_err()),
            "remove the DHCP client override first"
        );
    }

    async fn view_with(
        edit: impl Fn(&mut RegistryDocument),
        snap: &RouterSnapshot,
    ) -> crate::topology_manager::preview::PreviewView {
        let (cfg, _) = draft::scenario();
        let current = draft::current(&cfg, snap).await;
        let mut doc = document(&propose(&cfg, &[current]).unwrap()).unwrap();
        edit(&mut doc);
        let auth =
            authorize_configured(&doc, 0, 1, vec![draft::current(&cfg, snap).await]).unwrap();
        let pair = crate::topology_manager::publication::prepare(
            auth.reconcile(&mut ObservationGate::default()).unwrap(),
        )
        .unwrap();
        pair.view().clone()
    }
    fn with_plan(doc: &mut RegistryDocument, account_ref: &str, kind: SourceKind, source_id: &str) {
        let router_id = doc.catalog.as_ref().unwrap().routers[0].id.clone();
        doc.catalog
            .as_mut()
            .unwrap()
            .profiles
            .push(plan_profile("plan-1", "7M", 7, 7));
        doc.subscribers
            .push(crate::topology_manager::model::SubscriberRecord {
                id: format!("tm-sub-{:016x}", doc.next_id),
                identities: vec![IdentityRef {
                    scope: SourceScope {
                        router_id,
                        source_id: source_id.into(),
                        kind,
                    },
                    account_ref: account_ref.into(),
                }],
                policy: crate::topology_manager::model::SubscriberPolicy {
                    profile_id: Some("plan-1".into()),
                    assignment: None,
                    pin: None,
                },
            });
        doc.next_id += 1;
    }

    #[tokio::test(start_paused = true)]
    async fn a_dhcp_client_with_a_plan_gets_the_plan_rate_and_detach_restores_the_server_rate() {
        let (_, snap) = draft::scenario();
        let mac = "dhcp-mac/v1/02:00:00:00:02:01";
        let planned = view_with(|d| with_plan(d, mac, SourceKind::Dhcp, "dhcp-LAN"), &snap).await;
        assert_eq!(
            planned
                .subscribers
                .iter()
                .find(|s| s.account_ref == mac)
                .unwrap()
                .max_mbps,
            (7.0, 7.0)
        );
        let plain = view_with(|_| {}, &snap).await;
        assert_eq!(
            plain
                .subscribers
                .iter()
                .find(|s| s.account_ref == mac)
                .unwrap()
                .max_mbps,
            (50.0, 50.0)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_ppp_override_wins_over_the_router_speed() {
        let (_, snap) = draft::scenario();
        let v = view_with(
            |d| with_plan(d, "ppp-name/v1/g1", SourceKind::Ppp, "ppp"),
            &snap,
        )
        .await;
        assert_eq!(
            v.subscribers
                .iter()
                .find(|s| s.account_ref == "ppp-name/v1/g1")
                .unwrap()
                .max_mbps,
            (7.0, 7.0)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_new_mac_or_renamed_secret_is_a_new_identity_without_the_plan() {
        let (_, mut snap) = draft::scenario();
        snap.dhcp_lease[0].insert("mac-address".into(), "02:00:00:00:02:99".into());
        let v = view_with(
            |d| {
                with_plan(
                    d,
                    "dhcp-mac/v1/02:00:00:00:02:01",
                    SourceKind::Dhcp,
                    "dhcp-LAN",
                )
            },
            &snap,
        )
        .await;
        let new = v
            .subscribers
            .iter()
            .find(|s| s.account_ref == "dhcp-mac/v1/02:00:00:00:02:99")
            .unwrap();
        assert_ne!(new.profile.as_deref(), Some("plan-1"));
        assert!(
            !v.subscribers
                .iter()
                .any(|s| s.account_ref == "dhcp-mac/v1/02:00:00:00:02:01")
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_renamed_ppp_secret_is_a_new_identity_without_the_plan() {
        let (_, mut snap) = draft::scenario();
        let mut renamed = 0;
        for row in snap.ppp_secret.iter_mut().chain(snap.ppp_active.iter_mut()) {
            if row.get("name").map(String::as_str) == Some("g1") {
                row.insert("name".into(), "g1-renamed".into());
                renamed += 1;
            }
        }
        assert_eq!(renamed, 2, "one secret and one active session");
        let v = view_with(
            |d| with_plan(d, "ppp-name/v1/g1", SourceKind::Ppp, "ppp"),
            &snap,
        )
        .await;
        let new = v
            .subscribers
            .iter()
            .find(|s| s.account_ref == "ppp-name/v1/g1-renamed")
            .unwrap();
        assert_ne!(new.profile.as_deref(), Some("plan-1"));
        assert!(
            !v.subscribers
                .iter()
                .any(|s| s.account_ref == "ppp-name/v1/g1")
        );
    }
}
