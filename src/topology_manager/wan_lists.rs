//! The `via-<WAN>` address-list entries that the placement of one pair needs on the core router.
use super::catalog::TopologyCatalog;
use super::layout::NodeKind;
use super::publication::PreparedPair;
use anyhow::{Context, ensure};
use std::collections::{BTreeMap, HashMap};
use std::net::Ipv4Addr;

/// The comment prefix of every entry that the writer owns.
pub const OWNER: &str = "lqosync: ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListEntry {
    pub list: String,
    pub address: Ipv4Addr,
    pub comment: String,
}

/// One entry for each IPv4 address of each placed subscriber, in the list of the WAN above its
/// current node. A flat placement and an IPv6 address have no entry.
pub fn wanted(
    pair: &PreparedPair,
    catalog: &TopologyCatalog,
) -> anyhow::Result<BTreeMap<Ipv4Addr, ListEntry>> {
    let index: HashMap<&str, usize> = catalog
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let wan_list = |node: &str| -> anyhow::Result<Option<String>> {
        let mut cursor = *index.get(node).context("placement node is absent")?;
        // The parent graph is validated, so the walk ends at a root.
        while let Some(parent) = catalog.nodes[cursor].parent.as_deref() {
            cursor = index[parent];
        }
        Ok((catalog.nodes[cursor].kind == NodeKind::Wan)
            .then(|| catalog.wan_list(cursor))
            .flatten())
    };
    let names: HashMap<&str, &str> = pair
        .rows()
        .iter()
        .flat_map(|r| {
            r.ipv4
                .split(',')
                .map(move |ip| (ip.trim(), r.circuit_name.as_str()))
        })
        .collect();
    let mut out = BTreeMap::new();
    for s in &pair.view().subscribers {
        if s.node.is_empty() {
            continue;
        }
        let Some(list) = wan_list(&s.node)? else {
            continue;
        };
        for ip in &s.ips {
            let Ok(address) = ip.parse::<Ipv4Addr>() else {
                continue;
            };
            let name = names.get(ip.as_str()).copied().unwrap_or(&s.id);
            let comment: String = format!("{OWNER}{name}").chars().take(64).collect();
            let entry = ListEntry {
                list: list.clone(),
                address,
                comment,
            };
            if let Some(old) = out.insert(address, entry.clone()) {
                ensure!(
                    old.list == entry.list,
                    "address {address} is placed below two WANs"
                );
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology_manager::publication::{prepare, tests::input};
    use crate::topology_manager::wan_health::{WanSnapshot, WanState, WanStatus};

    fn lists(wan: WanSnapshot) -> BTreeMap<Ipv4Addr, ListEntry> {
        let mut observed = input(6);
        observed.reconciliation.wan = wan;
        let catalog = observed.reconciliation.registry.catalog.clone().unwrap();
        wanted(&prepare(observed).unwrap(), &catalog).unwrap()
    }

    #[test]
    fn each_client_address_gets_the_list_of_its_wan() {
        let out = lists(WanSnapshot::default());
        assert_eq!(out.len(), 6);
        let mut used: Vec<_> = out.values().map(|e| e.list.as_str()).collect();
        used.sort();
        used.dedup();
        assert_eq!(used, ["via-wan1", "via-wan2"]);
        let e = &out[&Ipv4Addr::new(10, 0, 0, 1)];
        assert!(e.comment.starts_with(OWNER), "{e:?}");
    }

    #[test]
    fn a_failover_uses_the_current_wan() {
        let down = WanSnapshot(HashMap::from([(
            "wan1".to_string(),
            WanStatus {
                state: WanState::Unavailable,
                quality: None,
            },
        )]));
        assert!(lists(down).values().all(|e| e.list == "via-wan2"));
    }

    #[test]
    fn a_flat_placement_has_no_entry() {
        let mut observed = input(3);
        let c = observed.reconciliation.registry.catalog.as_mut().unwrap();
        c.nodes.clear();
        for s in &mut c.sources {
            s.targets.clear();
        }
        let catalog = c.clone();
        assert!(
            wanted(&prepare(observed).unwrap(), &catalog)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn ipv6_addresses_have_no_entry() {
        let mut observed = input(1);
        observed.reconciliation.active[0].endpoints[0].endpoint.ips =
            vec!["2001:db8::1".parse().unwrap()];
        let catalog = observed.reconciliation.registry.catalog.clone().unwrap();
        assert!(
            wanted(&prepare(observed).unwrap(), &catalog)
                .unwrap()
                .is_empty()
        );
    }
}
