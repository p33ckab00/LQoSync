//! The Topology Manager read view for the web (`topology.json`). No router I/O and no mutation:
//! it projects the registry, the cached preview, and the cycle collection into one snapshot.
use crate::collection::{self, Report};
use crate::topology_manager::catalog::{BaseRateAuthority, SourceOrigin};
use crate::topology_manager::edits::factor_text;
use crate::topology_manager::identity::{
    ReferenceAuthority, SourceSelection, dhcp_mac_reference, ppp_name_reference,
};
use crate::topology_manager::model::{IdentityRef, RegistryDocument, SourceKind};
use crate::topology_manager::policy_draft::router_id;
use crate::topology_manager::rates::RateRule;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use web_contract::topology::*;

pub struct Input<'a> {
    pub doc: Option<&'a RegistryDocument>,
    /// The cached preview of `topology.preview.refresh`, as the API returns it.
    pub preview: Option<&'a Value>,
    pub collection: Option<&'a Report>,
    pub mode: &'a str,
    pub writer: &'a str,
    pub cycle_seq: u64,
    pub now: u64,
    pub fresh_for: u64,
    /// The current shaping write receipt.
    pub publication: Option<&'a crate::write_receipt::Receipt>,
    /// Web change receipts, in any order.
    pub history: &'a [crate::topology_manager::web_receipts::Record],
}

/// Web changes kept in the view, newest first.
const HISTORY: usize = 50;

pub fn project(input: &Input<'_>) -> TopologyView {
    let doc = input.doc;
    let preview = input.preview.filter(|p| p.is_object());
    let observed = preview.map(|p| {
        let revision = p["registry_revision"].as_u64().unwrap_or(0);
        Observed {
            revision: revision.to_string(),
            collected_at: p["collected_at"].as_u64().unwrap_or(0),
            generation: p["generation"].as_u64(),
            state: p["state"].as_str().unwrap_or("unknown").into(),
            stale: doc.is_none_or(|d| d.revision != revision),
        }
    });
    let snapshot_id = format!(
        "r{}-c{}-o{}-g{}-p{}",
        doc.map_or("none".into(), |d| d.revision.to_string()),
        input.cycle_seq,
        observed.as_ref().map_or("none", |o| o.revision.as_str()),
        observed.as_ref().and_then(|o| o.generation).unwrap_or(0),
        observed.as_ref().map_or(0, |o| o.collected_at),
    );
    let list = |key: &str| -> Vec<Value> {
        preview
            .and_then(|p| p[key].as_array().cloned())
            .unwrap_or_default()
    };
    let tree = doc.map(crate::topology_manager::edits::tree);
    let field = |key: &str| tree.as_ref().map_or(Value::Null, |t| t[key].clone());

    let preview_nodes: HashMap<String, Value> = list("nodes")
        .into_iter()
        .filter_map(|n| Some((n["id"].as_str()?.to_owned(), n)))
        .collect();
    let mut nodes: Vec<Node> = serde_json::from_value(field("nodes")).unwrap_or_default();
    for node in &mut nodes {
        if let Some(p) = preview_nodes.get(&node.id) {
            node.calculated_mbps = serde_json::from_value(p["calculated_mbps"].clone()).ok();
            node.exported_mbps = serde_json::from_value(p["exported_mbps"].clone()).ok();
            node.members = p["subscribers"].as_u64();
        }
    }

    let active: HashMap<String, Active> = list("subscribers")
        .into_iter()
        .filter_map(|s| {
            Some((
                s["id"].as_str()?.to_owned(),
                Active {
                    node: s["node"].as_str()?.to_owned(),
                    ips: serde_json::from_value(s["ips"].clone()).ok()?,
                    min_mbps: serde_json::from_value(s["min_mbps"].clone()).ok()?,
                    max_mbps: serde_json::from_value(s["max_mbps"].clone()).ok()?,
                    fallback: s["fallback"].as_bool().unwrap_or(false),
                },
            ))
        })
        .collect();
    let records = doc.map_or(&[][..], |d| &d.subscribers[..]);
    let subscribers: Vec<Subscriber> = records
        .iter()
        .map(|r| Subscriber {
            id: r.id.clone(),
            identities: r.identities.iter().map(identity).collect(),
            plan_id: r.policy.profile_id.clone(),
            pin: r.policy.pin.clone(),
            assignment: r.policy.assignment.clone(),
            active: active.get(&r.id).cloned(),
        })
        .collect();

    let plans = doc
        .and_then(|d| d.catalog.as_ref())
        .map_or(&[][..], |c| &c.profiles[..])
        .iter()
        .filter(|p| crate::topology_manager::plans::is_plan(p))
        .filter_map(|p| {
            let BaseRateAuthority::Local(rate) = p.base else {
                return None;
            };
            let members = records
                .iter()
                .filter(|r| r.policy.profile_id.as_deref() == Some(p.id.as_str()));
            let count = |kind| {
                members
                    .clone()
                    .flat_map(|r| &r.identities)
                    .filter(|i| i.scope.kind == kind)
                    .count() as u64
            };
            Some(Plan {
                id: p.id.clone(),
                name: p.name.clone(),
                down_mbps: rate.down / 1_000_000,
                up_mbps: rate.up / 1_000_000,
                ppp_identities: count(SourceKind::Ppp),
                dhcp_identities: count(SourceKind::Dhcp),
                subscribers: members.count() as u64,
            })
        })
        .collect();

    let (mut observations, collection) = input
        .collection
        .map(|r| observe(doc, r))
        .unwrap_or_default();
    let by_id: HashMap<&str, &Subscriber> =
        subscribers.iter().map(|s| (s.id.as_str(), s)).collect();
    for o in &mut observations {
        if let Some(s) = o.subscriber_id.as_deref().and_then(|id| by_id.get(id)) {
            o.plan_id = s.plan_id.clone();
            o.pin = s.pin.clone();
            o.node = s.active.as_ref().map(|a| a.node.clone());
        }
    }

    TopologyView {
        version: VERSION,
        updated: input.now,
        fresh_for: input.fresh_for,
        snapshot_id,
        registry_revision: doc.map(|d| d.revision.to_string()),
        mode: input.mode.into(),
        writer: input.writer.into(),
        enrolled: doc.is_some_and(|d| d.export_ownership.is_some()),
        observed,
        nodes,
        sources: serde_json::from_value(field("sources")).unwrap_or_default(),
        fallback: serde_json::from_value(field("fallback")).unwrap_or_default(),
        plans,
        subscribers,
        observations,
        collection,
        conflicts: serde_json::from_value(Value::Array(list("conflicts"))).unwrap_or_default(),
        speed: doc.and_then(|d| d.catalog.as_ref()).map(|c| SpeedDefaults {
            down_min: rule(&c.defaults.down_min),
            up_min: rule(&c.defaults.up_min),
            down_max: rule(&c.defaults.down_max),
            up_max: rule(&c.defaults.up_max),
            overrides: c
                .profiles
                .iter()
                .filter(|p| p.speed != Default::default())
                .count() as u64,
        }),
        factors: doc
            .and_then(|d| d.catalog.as_ref())
            .map_or(&[][..], |c| &c.sources[..])
            .iter()
            .filter_map(|s| match &s.origin {
                SourceOrigin::Subscribers { group, .. } => Some(SourceFactor {
                    source_id: s.id.clone(),
                    name: s.name.clone(),
                    down: factor_text(group.factors.down),
                    up: factor_text(group.factors.up),
                }),
                SourceOrigin::Wan { .. } => None,
            })
            .collect(),
        publication: input.publication.map(publication),
        history: {
            let mut records: Vec<_> = input.history.iter().collect();
            records.sort_by_key(|r| std::cmp::Reverse(r.at));
            records
                .into_iter()
                .take(HISTORY)
                .map(|r| {
                    let receipt = r.view();
                    WebChange {
                        at: r.at,
                        actor: r.actor.clone(),
                        change: change_label(&r.command),
                        state: receipt.state,
                        base_revision: receipt.base_revision,
                        revision: receipt.revision,
                    }
                })
                .collect()
        },
    }
}

/// A lower-case name of a serialized enum value, for example `IMPORT_CONFIRMED`.
fn name<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_ascii_lowercase))
        .unwrap_or_else(|| "unknown".into())
}

/// The latest write without paths or error text: those stay in the service journal.
fn publication(r: &crate::write_receipt::Receipt) -> Publication {
    Publication {
        write_id: r.write_id.clone(),
        phase: name(&r.phase),
        started: r.started,
        updated: r.updated,
        policy_revision: r
            .meta
            .topology
            .as_ref()
            .map(|t| t.policy_revision.to_string()),
        files: r
            .files
            .iter()
            .map(|f| PublishedFile {
                name: f
                    .path
                    .file_name()
                    .map_or("file".into(), |n| n.to_string_lossy().into_owned()),
                outcome: name(&f.outcome),
            })
            .collect(),
        failed: r.error.is_some(),
        accepted: r.acceptance.is_some(),
        superseded_by: r.superseded_by.clone(),
    }
}

fn change_label(c: &WebCommand) -> String {
    match c {
        WebCommand::PlanCreate { name, .. } => format!("Create plan {name}"),
        WebCommand::PlanUpdate { plan_id, .. } => format!("Edit plan {plan_id}"),
        WebCommand::PlanDelete { plan_id, .. } => format!("Delete plan {plan_id}"),
        WebCommand::SubscriberPlan { identity, .. } => {
            format!("Plan of {}", identity.account_ref)
        }
        // The Builder review names the edit; the receipt keeps only the private token.
        WebCommand::EditConfirm { .. } => "Reviewed change".into(),
    }
}

/// `num / den` as exact decimal text with up to `places` decimals, or rounded when not exact.
fn decimal(num: u128, den: u128, places: u32) -> String {
    let scale = 10u128.pow(places);
    let den = den.max(1);
    let scaled = if (num * scale).is_multiple_of(den) {
        num * scale / den
    } else {
        (num * scale + den / 2) / den
    };
    let text = format!(
        "{}.{:0width$}",
        scaled / scale,
        scaled % scale,
        width = places as usize
    );
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn rule(r: &RateRule) -> SpeedRule {
    let (unit, value) = match *r {
        RateRule::FixedBps(bps) => ("mbps", decimal(u128::from(bps), 1_000_000, 3)),
        RateRule::BaseFraction(f) => (
            "percent",
            decimal(u128::from(f.numerator) * 100, u128::from(f.denominator), 1),
        ),
        RateRule::LegacyRoundedBase { factor, .. } => (
            "legacy",
            decimal(
                u128::from(factor.numerator) * 100,
                u128::from(factor.denominator),
                1,
            ),
        ),
    };
    SpeedRule {
        unit: unit.into(),
        value,
    }
}

fn identity(i: &IdentityRef) -> Identity {
    Identity {
        router_id: i.scope.router_id.clone(),
        source_id: i.scope.source_id.clone(),
        kind: kind_name(i.scope.kind).into(),
        account_ref: i.account_ref.clone(),
    }
}

fn kind_name(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Ppp => "ppp",
        SourceKind::Dhcp => "dhcp",
        SourceKind::Hotspot => "hotspot",
    }
}

/// PPP and DHCP rows with their link, and the health of each router read.
fn observe(
    doc: Option<&RegistryDocument>,
    report: &Report,
) -> (Vec<Observation>, Vec<CollectionRouter>) {
    let selections = doc
        .and_then(|d| d.source_settings.as_ref())
        .map_or(&[][..], |s| &s.selections[..]);
    let owners: HashMap<&IdentityRef, &str> = doc
        .map_or(&[][..], |d| &d.subscribers[..])
        .iter()
        .flat_map(|r| r.identities.iter().map(|i| (i, r.id.as_str())))
        .collect();
    let mut observations = Vec::new();
    let mut routers = Vec::new();
    for router in &report.routers {
        let rid = router_id(&router.name);
        let online = router.state == "online";
        for row in &router.subscribers {
            let kind = match row.source.as_str() {
                "ppp" => SourceKind::Ppp,
                "dhcp" => SourceKind::Dhcp,
                _ => continue,
            };
            let (link, subscriber_id) = link(selections, &owners, &rid, kind, row);
            let state = if online {
                row.state
            } else {
                collection::State::Stale
            };
            observations.push(Observation {
                router_id: rid.clone(),
                router: router.name.clone(),
                kind: kind_name(kind).into(),
                name: row.name.clone(),
                ipv4: row.ipv4.clone(),
                mac: row.mac.clone(),
                state: state.label().to_ascii_lowercase(),
                link: link.into(),
                subscriber_id,
                plan_id: None,
                pin: None,
                node: None,
            });
        }
        routers.push(CollectionRouter {
            router_id: rid,
            name: router.name.clone(),
            state: router.state.clone(),
            observed_at: router.observed_at,
            sources: router
                .sources
                .iter()
                .map(|(kind, s)| CollectionSource {
                    kind: kind.clone(),
                    enabled: s.enabled,
                    records: s.records as u64,
                    online: s.online.map(|n| n as u64),
                    offline: s.offline.map(|n| n as u64),
                })
                .collect(),
        });
    }
    (observations, routers)
}

/// Link a row only through the reference rule of the one source selection that holds it.
/// Never by IP, MAC alone, or display name.
fn link(
    selections: &[SourceSelection],
    owners: &HashMap<&IdentityRef, &str>,
    rid: &str,
    kind: SourceKind,
    row: &collection::Subscriber,
) -> (&'static str, Option<String>) {
    // The DHCP key is `["dhcp", server, mac, ip]`; the PPP row carries no service name.
    let server = serde_json::from_str::<(String, String, String, String)>(&row.key)
        .ok()
        .map(|k| k.1);
    let mut found: Vec<&SourceSelection> = Vec::new();
    for s in selections {
        if s.scope.router_id != rid || s.scope.kind != kind {
            continue;
        }
        if !s.service_names.is_empty() {
            match (kind, &server) {
                (SourceKind::Dhcp, Some(server)) if !s.service_names.contains(server) => continue,
                (SourceKind::Dhcp, Some(_)) => {}
                _ => return ("unknown", None),
            }
        }
        found.push(s);
    }
    let selection = match found[..] {
        [] => return ("none", None),
        [one] => one,
        _ => return ("unknown", None),
    };
    let account_ref = match (selection.authority, kind) {
        (ReferenceAuthority::PppSecretName, SourceKind::Ppp) if !row.name.is_empty() => {
            ppp_name_reference(&row.name)
        }
        (ReferenceAuthority::DhcpClientMac, SourceKind::Dhcp) => {
            match crate::config::normalize_mac(&row.mac) {
                Some(mac) => dhcp_mac_reference(&mac),
                None => return ("none", None),
            }
        }
        _ => return ("unknown", None),
    };
    let key = IdentityRef {
        scope: selection.scope.clone(),
        account_ref,
    };
    match owners.get(&key) {
        Some(id) => ("verified", Some((*id).to_owned())),
        None => ("none", None),
    }
}

/// Write `topology.json` with mode 0600 through a temporary file and a rename.
pub fn publish(directory: &Path, view: &TopologyView) -> anyhow::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let temp = directory.join(format!(".{CACHE}.tmp-{}", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temp)?;
    file.write_all(&serde_json::to_vec(view)?)?;
    file.sync_all()?;
    std::fs::rename(temp, directory.join(CACHE))?;
    Ok(())
}

#[cfg(test)]
#[path = "web_view_tests.rs"]
mod tests;
