use crate::{
    model::{self, Settings},
    normalizer,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const MAX_SELECTIONS: usize = 50_000;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub id: String,
    pub router: String,
    pub kind: String,
    pub selector: String,
    pub value: String,
    #[serde(default)]
    pub node: String,
    #[serde(default)]
    pub exclude: bool,
}
pub(crate) struct Binding<'a> {
    pub router: &'a str,
    pub service: &'static str,
    pub selector: &'a str,
    pub value: String,
    pub node: &'a str,
    pub exclude: bool,
}
pub(crate) fn bindings(settings: &Settings) -> Result<Vec<Binding<'_>>> {
    let mut bindings = vec![];
    for n in &settings.nodes {
        if let Some(service) = normalizer::service(&n.kind)
            && !n.router.is_empty()
            && !n.source.is_empty()
        {
            bindings.push(Binding {
                router: &n.router,
                service,
                selector: "group",
                value: n.source.clone(),
                node: &n.id,
                exclude: false,
            });
        }
    }
    for s in &settings.selections {
        let service = normalizer::service(&s.kind)
            .ok_or_else(|| anyhow::anyhow!("Unknown selection service"))?;
        let value = if service == "dhcp" && s.selector == "subscriber" {
            normalizer::canonical_mac(&s.value)?
        } else {
            s.value.clone()
        };
        bindings.push(Binding {
            router: &s.router,
            service,
            selector: &s.selector,
            value,
            node: if settings.dynamic.is_some() {
                &s.id
            } else {
                &s.node
            },
            exclude: s.exclude,
        });
    }
    Ok(bindings)
}
pub fn validate(settings: &Settings) -> Result<()> {
    ensure!(
        settings.selections.len() <= MAX_SELECTIONS,
        "Too many source selections"
    );
    let routers: HashMap<_, _> = settings
        .routers
        .iter()
        .map(|r| (r.id.as_str(), r))
        .collect();
    let nodes: HashMap<_, _> = settings.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut ids = HashSet::new();
    for s in &settings.selections {
        ensure!(
            model::valid_id(&s.id) && ids.insert(&s.id),
            "Selection IDs must be valid and unique"
        );
        ensure!(
            matches!(s.selector.as_str(), "group" | "subscriber"),
            "Unknown source selector type"
        );
        ensure!(
            !s.value.is_empty() && s.value.len() <= 256 && !s.value.chars().any(char::is_control),
            "Invalid source selector value"
        );
        if settings.dynamic.is_none() && (!s.exclude || !s.node.is_empty()) {
            let node = nodes
                .get(s.node.as_str())
                .ok_or_else(|| anyhow::anyhow!("Selection refers to an unknown node"))?;
            ensure!(
                node.kind == s.kind || matches!(node.kind.as_str(), "site" | "group" | "wan"),
                "Selection destination has incompatible service type"
            );
        }
    }
    let mut keys = HashSet::new();
    for b in bindings(settings)? {
        ensure!(
            routers.get(b.router).is_some_and(|r| r.role != "core"),
            "Subscriber source requires a distro or both-role router"
        );
        ensure!(
            keys.insert((b.router, b.service, b.selector, b.value)),
            "Duplicate source binding for {}",
            b.router
        );
    }
    Ok(())
}
