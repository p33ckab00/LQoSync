//! The `via-<WAN>` address-list entries that the placement of one pair needs on the core router.
use super::catalog::TopologyCatalog;
use super::layout::NodeKind;
use super::publication::PreparedPair;
use anyhow::ensure;
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

/// One entry for each IPv4 address of each subscriber of the pair, in the list of its routing
/// WAN (`routing`). A subscriber without a routing WAN and an IPv6 address have no entry. When
/// two subscribers report one address, the first by subscriber ID keeps it.
pub fn wanted(
    pair: &PreparedPair,
    catalog: &TopologyCatalog,
    routing: &super::routing_balance::RoutingState,
) -> anyhow::Result<BTreeMap<Ipv4Addr, ListEntry>> {
    let index: HashMap<&str, usize> = catalog
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let names: HashMap<&str, &str> = pair
        .rows()
        .iter()
        .flat_map(|r| {
            r.ipv4
                .split(',')
                .map(move |ip| (ip.trim(), r.circuit_name.as_str()))
        })
        .collect();
    let mut subscribers: Vec<_> = pair.view().subscribers.iter().collect();
    subscribers.sort_by(|a, b| a.id.cmp(&b.id));
    let mut out = BTreeMap::new();
    for s in subscribers {
        let Some(&wan) = routing
            .assigned
            .get(&s.id)
            .and_then(|w| index.get(w.as_str()))
        else {
            continue;
        };
        ensure!(
            catalog.nodes[wan].kind == NodeKind::Wan,
            "a routing WAN is not a WAN node"
        );
        let Some(list) = catalog.wan_list(wan) else {
            continue;
        };
        for ip in &s.ips {
            let Ok(address) = ip.parse::<Ipv4Addr>() else {
                continue;
            };
            let name = names.get(ip.as_str()).copied().unwrap_or(&s.id);
            let comment: String = format!("{OWNER}{name}").chars().take(64).collect();
            out.entry(address).or_insert(ListEntry {
                list: list.clone(),
                address,
                comment,
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology_manager::publication::{prepare, tests::input};
    use crate::topology_manager::wan_health::{WanSnapshot, WanState, WanStatus};

    /// The lists of a pair, with a routing balance from an empty state.
    fn routed(
        pair: &PreparedPair,
        catalog: &TopologyCatalog,
        wan: &WanSnapshot,
    ) -> BTreeMap<Ipv4Addr, ListEntry> {
        use crate::topology_manager::routing_balance::{
            RoutingState, balance, candidates, demands, placed_of,
        };
        let routing = balance(
            &demands(&placed_of(pair), catalog),
            &candidates(catalog, wan),
            &RoutingState::default(),
        );
        wanted(pair, catalog, &routing).unwrap()
    }

    fn lists(wan: WanSnapshot) -> BTreeMap<Ipv4Addr, ListEntry> {
        let mut observed = input(6);
        observed.reconciliation.wan = wan.clone();
        let catalog = observed.reconciliation.registry.catalog.clone().unwrap();
        routed(&prepare(observed).unwrap(), &catalog, &wan)
    }

    #[test]
    fn every_shaped_address_is_in_one_list() {
        let out = lists(WanSnapshot::default());
        assert_eq!(out.len(), 6);
        assert!(out.values().all(|e| e.list.starts_with("via-")));
    }

    #[test]
    fn an_unlinked_card_gets_lists_from_the_balance() {
        // Keep the WAN nodes, but place no subscriber below them.
        let mut observed = input(4);
        let c = observed.reconciliation.registry.catalog.as_mut().unwrap();
        c.nodes.retain(|n| n.kind == NodeKind::Wan);
        for s in &mut c.sources {
            s.targets.clear();
        }
        let catalog = c.clone();
        let out = routed(
            &prepare(observed).unwrap(),
            &catalog,
            &WanSnapshot::default(),
        );
        assert_eq!(out.len(), 4);
        let mut used: Vec<_> = out.values().map(|e| e.list.as_str()).collect();
        used.sort();
        used.dedup();
        assert_eq!(used, ["via-wan1", "via-wan2"]);
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
        // No WAN node: no routing candidate, so no entry.
        assert!(
            routed(
                &prepare(observed).unwrap(),
                &catalog,
                &WanSnapshot::default()
            )
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
            routed(
                &prepare(observed).unwrap(),
                &catalog,
                &WanSnapshot::default()
            )
            .is_empty()
        );
    }

    #[test]
    fn one_address_gets_one_entry() {
        // Two subscribers report one address (a stale session): one entry, never two lists.
        let mut observed = input(2);
        let ip = observed.reconciliation.active[0].endpoints[0]
            .endpoint
            .ips
            .clone();
        observed.reconciliation.active[1].endpoints[0].endpoint.ips = ip.clone();
        let catalog = observed.reconciliation.registry.catalog.clone().unwrap();
        let pair = prepare(observed).unwrap();
        let mut ids: Vec<String> = pair
            .view()
            .subscribers
            .iter()
            .map(|s| s.id.clone())
            .collect();
        ids.sort();
        let mut routing = crate::topology_manager::routing_balance::RoutingState::default();
        routing.assigned.insert(ids[0].clone(), "wan1".into());
        routing.assigned.insert(ids[1].clone(), "wan2".into());
        let out = wanted(&pair, &catalog, &routing).unwrap();
        assert_eq!(out.len(), 1);
        let address: Ipv4Addr = ip[0].to_string().parse().unwrap();
        assert_eq!(out[&address].list, "via-wan1");
    }
}
