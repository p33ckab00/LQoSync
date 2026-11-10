//! A subscriber-only projection. Observation rows never enter LibreQoS shaping previews.
use super::data::Data;
use crate::{
    collection::{State, Subscriber},
    model::CsvRow,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct Evidence {
    pub subscriber: Subscriber,
    pub router: String,
    pub observed_at: u64,
    pub complete: bool,
    pub enabled: bool,
}
impl Evidence {
    pub fn state(&self, d: &Data, now: u64) -> State {
        if !self.enabled {
            return State::Disabled;
        }
        if !self.complete || !fresh(d, now) {
            return State::Stale;
        }
        self.subscriber.state
    }
}
pub fn fresh(d: &Data, now: u64) -> bool {
    d.collection.as_ref().is_some_and(|c| {
        c.fresh(now)
            && d.status
                .as_ref()
                .and_then(|s| s["cycle_seq"].as_u64())
                .is_none_or(|seq| seq <= c.cycle_seq)
            && d.status
                .as_ref()
                .and_then(|s| s["pid"].as_u64())
                .is_none_or(|pid| pid == u64::from(c.pid))
    })
}
pub fn state(d: &Data, r: &CsvRow, now: u64) -> State {
    d.presence
        .get(&r.circuit_id)
        .map_or(State::Unknown, |p| p.state(d, now))
}
pub fn source<'a>(d: &'a Data, r: &'a CsvRow) -> (&'a str, &'a str) {
    d.presence
        .get(&r.circuit_id)
        .map(|p| {
            (
                match p.subscriber.source.as_str() {
                    "ppp" => "PPPoE",
                    "dhcp" => "DHCP",
                    "hotspot" => "Hotspot",
                    _ => "unknown",
                },
                p.router.as_str(),
            )
        })
        .unwrap_or_else(|| super::devices::source_of(d, &r.circuit_name))
}
fn raw_source<'a>(d: &'a Data, name: &str) -> (&'a str, &'a str) {
    d.sources
        .get(name)
        .map_or(("unknown", "-"), |(s, r)| (s.as_str(), r.as_str()))
}
fn mac(s: &str) -> String {
    s.replace([':', '-'], "").to_ascii_uppercase()
}
#[derive(Default)]
pub struct Projection {
    pub view: Data,
    base: Option<Data>,
}
impl Projection {
    pub fn update(&mut self, d: &Data) {
        let same = self.base.as_ref().is_some_and(|old| {
            Arc::ptr_eq(&old.rows, &d.rows)
                && Arc::ptr_eq(&old.sources, &d.sources)
                && match (&old.collection, &d.collection) {
                    (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                    (None, None) => true,
                    _ => false,
                }
        });
        if same {
            let old = std::mem::replace(&mut self.view, d.clone());
            self.view.rows = old.rows;
            self.view.row_search = old.row_search;
            self.view.flags = old.flags;
            self.view.presence = old.presence;
            self.view.ip_owners = old.ip_owners;
            return;
        }
        self.base = Some(d.clone());
        self.view = d.clone();
        let Some(report) = d.collection.as_ref() else {
            return;
        };
        let mut rows = (*d.rows).clone();
        let mut search = (*d.row_search).clone();
        let mut flags = (*d.flags).clone();
        search.resize(rows.len(), String::new());
        flags.resize(rows.len(), 0);
        let mut circuit_sizes = HashMap::new();
        for row in &rows {
            *circuit_sizes
                .entry(row.circuit_id.clone())
                .or_insert(0usize) += 1;
        }
        let mut by_name: HashMap<_, Vec<usize>> = HashMap::new();
        let mut by_endpoint: HashMap<_, Vec<usize>> = HashMap::new();
        for (i, r) in rows.iter().enumerate() {
            let (source, router) = raw_source(d, &r.circuit_name);
            by_name
                .entry((router.to_owned(), source.to_owned(), r.circuit_name.clone()))
                .or_default()
                .push(i);
            if !r.mac.is_empty() && !r.ipv4.is_empty() {
                by_endpoint
                    .entry((
                        router.to_owned(),
                        source.to_owned(),
                        mac(&r.mac),
                        r.ipv4.clone(),
                    ))
                    .or_default()
                    .push(i);
            }
        }
        let mut observed_endpoints = HashMap::new();
        for router in &report.routers {
            for entry in &router.subscribers {
                if entry.source == "dhcp" {
                    *observed_endpoints
                        .entry((router.name.clone(), mac(&entry.mac), entry.ipv4.clone()))
                        .or_insert(0usize) += 1;
                }
            }
        }
        let mut presence = HashMap::new();
        let mut used = HashSet::new();
        for router in &report.routers {
            for entry in &router.subscribers {
                let enabled = router.sources.get(&entry.source).is_some_and(|s| s.enabled);
                let endpoint = by_endpoint
                    .get(&(
                        router.name.clone(),
                        entry.source.clone(),
                        mac(&entry.mac),
                        entry.ipv4.clone(),
                    ))
                    .filter(|v| v.len() == 1);
                // DHCP host names are not unique. Match leases by router, MAC, and address.
                let candidates = if entry.source == "dhcp" {
                    endpoint.filter(|_| {
                        observed_endpoints.get(&(
                            router.name.clone(),
                            mac(&entry.mac),
                            entry.ipv4.clone(),
                        )) == Some(&1)
                    })
                } else {
                    by_name
                        .get(&(
                            router.name.clone(),
                            entry.source.clone(),
                            entry.circuit.clone(),
                        ))
                        .filter(|v| v.len() == 1)
                        .or(endpoint)
                };
                let matched = candidates
                    .and_then(|v| v.first())
                    .copied()
                    .filter(|i| circuit_sizes.get(&rows[*i].circuit_id) == Some(&1))
                    .filter(|i| used.insert(*i));
                let id = if let Some(index) = matched {
                    rows[index].circuit_id.clone()
                } else {
                    let id = format!(
                        "observed:{}",
                        serde_json::to_string(&(&router.name, &entry.key)).unwrap()
                    );
                    rows.push(CsvRow {
                        circuit_id: id.clone(),
                        device_id: id.clone(),
                        circuit_name: entry.circuit.clone(),
                        device_name: entry.name.clone(),
                        ipv4: entry.ipv4.clone(),
                        mac: entry.mac.clone(),
                        ..Default::default()
                    });
                    search.push(
                        format!(
                            "{} {} {} {} {} {}",
                            entry.name,
                            entry.circuit,
                            entry.ipv4,
                            entry.mac,
                            entry.source,
                            router.name
                        )
                        .to_lowercase(),
                    );
                    flags.push(0);
                    id
                };
                presence.insert(
                    id,
                    Evidence {
                        subscriber: entry.clone(),
                        router: router.name.clone(),
                        observed_at: router.observed_at,
                        complete: router.state == "online",
                        enabled,
                    },
                );
            }
        }
        let observed_names: HashSet<_> = report
            .routers
            .iter()
            .flat_map(|router| {
                router
                    .subscribers
                    .iter()
                    .map(move |s| (router.name.as_str(), s.source.as_str(), s.circuit.as_str()))
            })
            .collect();
        for r in d.rows.iter() {
            if presence.contains_key(&r.circuit_id) {
                continue;
            }
            let (source, name) = raw_source(d, &r.circuit_name);
            if let Some(router) = report.routers.iter().find(|r| r.name == name)
                && let Some(src) = router.sources.get(source)
            {
                presence.insert(
                    r.circuit_id.clone(),
                    Evidence {
                        subscriber: Subscriber {
                            key: r.circuit_id.clone(),
                            source: source.into(),
                            name: r.device_name.clone(),
                            circuit: r.circuit_name.clone(),
                            ipv4: r.ipv4.clone(),
                            mac: r.mac.clone(),
                            state: if (source == "ppp" || source == "hotspot")
                                && circuit_sizes.get(&r.circuit_id) == Some(&1)
                                && !observed_names.contains(&(
                                    router.name.as_str(),
                                    source,
                                    r.circuit_name.as_str(),
                                )) {
                                State::Offline
                            } else {
                                State::Unknown
                            },
                        },
                        router: name.into(),
                        observed_at: router.observed_at,
                        complete: router.state == "online",
                        enabled: src.enabled,
                    },
                );
            }
        }
        if rows.len() != d.rows.len() {
            self.view.ip_owners = Arc::new(crate::subscriber_stats::owners(&rows));
        }
        self.view.rows = Arc::new(rows);
        self.view.row_search = Arc::new(search);
        self.view.flags = Arc::new(flags);
        self.view.presence = Arc::new(presence);
    }
}

/// Sum a source only when every enabled router has complete current evidence.
pub fn count(d: &Data, now: u64, source: &str, offline: bool) -> String {
    let Some(report) = d.collection.as_ref().filter(|_| fresh(d, now)) else {
        return "unknown".into();
    };
    let mut total = 0;
    let mut enabled = false;
    for router in &report.routers {
        let Some(stats) = router.sources.get(source) else {
            return "unknown".into();
        };
        if !stats.enabled {
            continue;
        }
        enabled = true;
        if router.state != "online" {
            return "unknown".into();
        }
        let Some(n) = (if offline { stats.offline } else { stats.online }) else {
            return "unknown".into();
        };
        total += n;
    }
    if enabled {
        total.to_string()
    } else {
        "disabled".into()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::Report;
    #[test]
    fn ambiguous_dhcp_observations_do_not_assign_a_false_state_to_shaping() {
        let cfg=crate::config::Config::from_toml_str("[[routers]]\nname='R1'\naddress='192.0.2.1'\nusername='u'\npassword_file='/fixture/password'\n[routers.dhcp]\nenabled=true\n").unwrap();
        let snapshot=serde_json::from_value(serde_json::json!({"dhcp_lease":[{"server":"A","host-name":"same","mac-address":"00:11:22:33:44:55","address":"192.0.2.10","status":"bound"},{"server":"B","host-name":"same","mac-address":"00:11:22:33:44:55","address":"192.0.2.10","status":"waiting"}]})).unwrap();
        let report = Report::collect(
            &cfg,
            &[("R1".into(), crate::cycle::RouterResult::Online(snapshot))],
            None,
            100,
        );
        let d = Data {
            rows: Arc::new(vec![CsvRow {
                circuit_id: "shaped".into(),
                circuit_name: "same".into(),
                mac: "00:11:22:33:44:55".into(),
                ipv4: "192.0.2.10".into(),
                ..Default::default()
            }]),
            sources: Arc::new(
                [("same".into(), ("dhcp".into(), "R1".into()))]
                    .into_iter()
                    .collect(),
            ),
            collection: Some(Arc::new(report)),
            ..Default::default()
        };
        let mut projection = Projection::default();
        projection.update(&d);
        assert_eq!(
            state(&projection.view, &projection.view.rows[0], 100),
            State::Unknown
        );
        assert_eq!(projection.view.rows.len(), 3);
        assert_eq!(
            projection
                .view
                .ip_owners
                .get(&"192.0.2.10".parse().unwrap()),
            Some(&3)
        );
        let owners = projection.view.ip_owners.clone();
        projection.update(&d);
        assert!(Arc::ptr_eq(&owners, &projection.view.ip_owners));
    }

    #[test]
    fn shared_shaping_identity_keeps_ambiguous_devices_unknown_not_offline() {
        let entry = Subscriber {
            key: "alice".into(),
            source: "ppp".into(),
            name: "alice".into(),
            circuit: "alice".into(),
            ipv4: "192.0.2.1".into(),
            mac: String::new(),
            state: State::Online,
        };
        let report = Report {
            version: 1,
            pid: 1,
            cycle_seq: 1,
            updated: 100,
            fresh_for: 150,
            routers: vec![crate::collection::Router {
                name: "R1".into(),
                state: "online".into(),
                observed_at: 100,
                sources: [(
                    "ppp".into(),
                    crate::collection::Source {
                        enabled: true,
                        records: 1,
                        online: Some(1),
                        offline: Some(0),
                    },
                )]
                .into_iter()
                .collect(),
                subscribers: vec![entry],
            }],
        };
        let rows = (0..2)
            .map(|i| CsvRow {
                circuit_id: "same-circuit".into(),
                device_id: format!("device-{i}"),
                circuit_name: "alice".into(),
                ..Default::default()
            })
            .collect();
        let d = Data {
            rows: Arc::new(rows),
            sources: Arc::new(
                [("alice".into(), ("ppp".into(), "R1".into()))]
                    .into_iter()
                    .collect(),
            ),
            collection: Some(Arc::new(report)),
            ..Default::default()
        };
        let mut p = Projection::default();
        p.update(&d);
        for row in p.view.rows.iter().take(2) {
            assert_eq!(state(&p.view, row, 100), State::Unknown);
        }
        assert_eq!(
            state(&p.view, p.view.rows.last().unwrap(), 100),
            State::Online
        );
    }

    #[test]
    fn cache_from_previous_cycle_or_process_is_never_live() {
        let mut report = Report {
            version: 1,
            pid: 7,
            updated: 100,
            fresh_for: 120,
            cycle_seq: 4,
            routers: vec![],
        };
        let mut d = Data {
            collection: Some(Arc::new(report.clone())),
            status: Some(Arc::new(serde_json::json!({"pid":7,"cycle_seq":4}))),
            ..Default::default()
        };
        assert!(fresh(&d, 100));
        d.status = Some(Arc::new(serde_json::json!({"pid":7,"cycle_seq":5})));
        assert!(!fresh(&d, 100));
        report.cycle_seq = 5;
        d.collection = Some(Arc::new(report));
        d.status = Some(Arc::new(serde_json::json!({"pid":8,"cycle_seq":1})));
        assert!(!fresh(&d, 100));
    }

    #[test]
    fn projection_adds_inactive_inventory_without_changing_shaping_and_marks_stale() {
        let cfg=crate::config::Config::from_toml_str("[[routers]]\nname='R1'\naddress='192.0.2.1'\nusername='u'\npassword_file='/fixture/password'\n[routers.pppoe]\nenabled=true\n").unwrap();
        let snapshot =
            serde_json::from_value(serde_json::json!({"ppp_secret":[{"name":"offline"}]})).unwrap();
        let report = Report::collect(
            &cfg,
            &[("R1".into(), crate::cycle::RouterResult::Online(snapshot))],
            None,
            100,
        );
        let d = Data {
            collection: Some(Arc::new(report)),
            ..Default::default()
        };
        let mut p = Projection::default();
        p.update(&d);
        assert!(d.rows.is_empty());
        assert_eq!(p.view.rows.len(), 1);
        assert_eq!(state(&p.view, &p.view.rows[0], 100), State::Offline);
        assert_eq!(source(&p.view, &p.view.rows[0]), ("PPPoE", "R1"));
        assert_eq!(state(&p.view, &p.view.rows[0], 10000), State::Stale);
        assert_eq!(count(&d, 100, "ppp", true), "1");
        let failed = Report::collect(
            &cfg,
            &[("R1".into(), crate::cycle::RouterResult::Failed)],
            d.collection.as_deref(),
            110,
        );
        let mut failed_data = d.clone();
        failed_data.collection = Some(Arc::new(failed));
        let mut failure_projection = Projection::default();
        failure_projection.update(&failed_data);
        assert_eq!(
            state(
                &failure_projection.view,
                &failure_projection.view.rows[0],
                110
            ),
            State::Stale
        );
        assert_eq!(count(&failed_data, 110, "ppp", true), "unknown");

        let old = p.view.rows.clone();
        p.update(&d);
        assert!(Arc::ptr_eq(&old, &p.view.rows));
    }
}
