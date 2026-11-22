//! Writes the difference between the wanted `via-` address-list entries and the owned entries on
//! the core router. It changes only entries in `via-` lists whose comment starts with `lqosync: `.
use crate::config::WanWriter;
use crate::model::RosRow;
use crate::routeros::{ListWriter, RouterSession};
use crate::topology_manager::wan_lists::{ListEntry, OWNER};
use std::collections::{BTreeMap, HashSet};
use std::net::Ipv4Addr;

const PATH: &str = "/ip/firewall/address-list";
const FIELDS: &str = ".id,list,address,comment,dynamic,disabled";

/// One entry on the router that the writer owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owned {
    pub id: String,
    pub list: String,
    /// `None` for an address that is not one IPv4 address. The writer removes such an entry.
    pub address: Option<Ipv4Addr>,
    pub comment: String,
    pub disabled: bool,
}

/// The owned entries: static, in a `via-` list, with the owner comment.
pub fn read_owned(rows: &[RosRow]) -> Vec<Owned> {
    let get = |r: &RosRow, k: &str| r.get(k).cloned().unwrap_or_default();
    rows.iter()
        .filter(|r| {
            get(r, "list").starts_with("via-")
                && get(r, "comment").starts_with(OWNER)
                && get(r, "dynamic") != "true"
        })
        .map(|r| Owned {
            id: get(r, ".id"),
            list: get(r, "list"),
            address: get(r, "address").parse().ok(),
            comment: get(r, "comment"),
            disabled: get(r, "disabled") == "true",
        })
        .collect()
}

#[derive(Debug, Default, PartialEq)]
pub struct Plan {
    pub add: Vec<ListEntry>,
    pub set: Vec<(String, Ipv4Addr)>,
    pub remove: Vec<String>,
}

/// The smallest set of writes. An entry that only changed its address keeps its ID.
pub fn plan(owned: &[Owned], wanted: &BTreeMap<Ipv4Addr, ListEntry>) -> Plan {
    let mut out = Plan::default();
    let mut kept = HashSet::new();
    for entry in wanted.values() {
        let exact = owned.iter().position(|o| {
            !kept.contains(&o.id)
                && !o.disabled
                && o.list == entry.list
                && o.address == Some(entry.address)
        });
        if let Some(i) = exact {
            kept.insert(owned[i].id.clone());
        } else {
            out.add.push(entry.clone());
        }
    }
    // A client that reconnected with a new address: change the address of its old entry.
    out.add.retain(|entry| {
        let stale = owned.iter().find(|o| {
            !kept.contains(&o.id)
                && !o.disabled
                && o.list == entry.list
                && o.comment == entry.comment
                && o.address.is_none_or(|a| !wanted.contains_key(&a))
        });
        match stale {
            Some(o) => {
                kept.insert(o.id.clone());
                out.set.push((o.id.clone(), entry.address));
                false
            }
            None => true,
        }
    });
    out.remove = owned
        .iter()
        .filter(|o| !kept.contains(&o.id))
        .map(|o| o.id.clone())
        .collect();
    out
}

#[derive(Debug, PartialEq, Eq)]
pub enum WriteError {
    /// The safety limit holds the write. Nothing changed. The same `fingerprint` in the next
    /// call lets the write go, as the removal guard of the cycle does.
    Held { reason: String, fingerprint: u64 },
    /// The router refused or did not answer. Some writes can be done.
    Router(String),
}

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct WriteReport {
    pub added: usize,
    pub changed: usize,
    pub removed: usize,
    pub owned_after: usize,
}

fn router(e: impl std::fmt::Display) -> WriteError {
    WriteError::Router(e.to_string())
}

/// Read, write the difference (add, then change, then remove), and read again to verify.
pub async fn apply<W: ListWriter + RouterSession>(
    w: &mut W,
    wanted: &BTreeMap<Ipv4Addr, ListEntry>,
    cfg: &WanWriter,
    allow: Option<u64>,
) -> Result<WriteReport, WriteError> {
    let owned = read_owned(&w.print(PATH, FIELDS).await.map_err(router)?);
    let p = plan(&owned, wanted);
    let removed = p.remove.len();
    // Only clients that leave every list count; a move or a duplicate is no loss.
    let mut lost: Vec<String> = owned
        .iter()
        .filter(|o| o.address.is_some_and(|a| !wanted.contains_key(&a)))
        .filter(|o| p.remove.contains(&o.id))
        .map(|o| format!("{}|{:?}", o.list, o.address))
        .collect();
    lost.sort();
    let too_many = lost.len() > cfg.max_removed
        || lost.len() as f64 * 100.0 > owned.len() as f64 * cfg.max_removed_percent;
    if !lost.is_empty() && too_many {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        lost.hash(&mut h);
        let fingerprint = h.finish();
        if allow != Some(fingerprint) {
            return Err(WriteError::Held {
                reason: format!(
                    "{} of {} clients would leave the routing lists; held until the next cycle",
                    lost.len(),
                    owned.len()
                ),
                fingerprint,
            });
        }
    }
    for entry in &p.add {
        w.add(entry).await.map_err(router)?;
    }
    for (id, address) in &p.set {
        w.set_address(id, *address).await.map_err(router)?;
    }
    for id in &p.remove {
        w.remove(id).await.map_err(router)?;
    }
    let after = read_owned(&w.print(PATH, FIELDS).await.map_err(router)?);
    let got: HashSet<_> = after
        .iter()
        .filter(|o| !o.disabled)
        .filter_map(|o| Some((o.list.as_str(), o.address?)))
        .collect();
    let want: HashSet<_> = wanted
        .values()
        .map(|e| (e.list.as_str(), e.address))
        .collect();
    if got != want || after.len() != wanted.len() {
        return Err(WriteError::Router(
            "the routing lists differ from the placement after the write".into(),
        ));
    }
    Ok(WriteReport {
        added: p.add.len(),
        changed: p.set.len(),
        removed,
        owned_after: after.len(),
    })
}

type Job = (
    BTreeMap<Ipv4Addr, ListEntry>,
    WanWriter,
    tokio::sync::oneshot::Sender<Result<WriteReport, WriteError>>,
);

/// The writer task. It keeps one write session and the last held removal.
#[derive(Clone)]
pub struct Handle(tokio::sync::mpsc::Sender<Job>);

/// One write must finish in this time, or the cycle keeps the previous pair.
const LIMIT: std::time::Duration = std::time::Duration::from_secs(30);

impl Handle {
    pub async fn write(
        &self,
        wanted: BTreeMap<Ipv4Addr, ListEntry>,
        cfg: &WanWriter,
    ) -> Result<WriteReport, WriteError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.0
            .send((wanted, cfg.clone(), tx))
            .await
            .map_err(|_| router("the writer task stopped"))?;
        tokio::time::timeout(LIMIT, rx)
            .await
            .map_err(|_| router("the routing list write timed out"))?
            .map_err(|_| router("the writer task stopped"))?
    }
}

/// Start the writer task. `connect` opens a write session; a failed write drops the session.
pub fn start<W, F, Fut>(connect: F) -> Handle
where
    W: ListWriter + RouterSession + 'static,
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<W, crate::routeros::ReadError>> + Send,
{
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Job>(1);
    tokio::spawn(async move {
        let mut session: Option<W> = None;
        let mut held = None;
        while let Some((wanted, cfg, reply)) = rx.recv().await {
            let result = async {
                if session.is_none() {
                    session = Some(connect().await.map_err(router)?);
                }
                let w = session.as_mut().expect("connected above");
                apply(w, &wanted, &cfg, held).await
            }
            .await;
            held = match &result {
                Err(WriteError::Held { fingerprint, .. }) => Some(*fingerprint),
                _ => None,
            };
            if matches!(result, Err(WriteError::Router(_))) {
                session = None;
            }
            let _ = reply.send(result);
        }
    });
    Handle(tx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routeros::ReadError;

    /// A router with one address-list table. It logs each write.
    #[derive(Default)]
    struct Fake {
        rows: Vec<RosRow>,
        log: Vec<String>,
        next: u32,
    }
    fn row(id: &str, list: &str, address: &str, comment: &str) -> RosRow {
        [
            (".id", id),
            ("list", list),
            ("address", address),
            ("comment", comment),
            ("dynamic", "false"),
            ("disabled", "false"),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
    }
    impl RouterSession for Fake {
        fn print(
            &mut self,
            _: &str,
            _: &str,
        ) -> impl Future<Output = Result<Vec<RosRow>, ReadError>> + Send {
            let rows = self.rows.clone();
            async move { Ok(rows) }
        }
    }
    impl ListWriter for Fake {
        fn add(&mut self, e: &ListEntry) -> impl Future<Output = Result<(), ReadError>> + Send {
            self.next += 1;
            let id = format!("*N{}", self.next);
            self.rows
                .push(row(&id, &e.list, &e.address.to_string(), &e.comment));
            self.log.push(format!("add {} {}", e.list, e.address));
            async { Ok(()) }
        }
        fn remove(&mut self, id: &str) -> impl Future<Output = Result<(), ReadError>> + Send {
            self.rows.retain(|r| r[".id"] != id);
            self.log.push(format!("remove {id}"));
            async { Ok(()) }
        }
        fn set_address(
            &mut self,
            id: &str,
            address: Ipv4Addr,
        ) -> impl Future<Output = Result<(), ReadError>> + Send {
            for r in &mut self.rows {
                if r[".id"] == id {
                    r.insert("address".into(), address.to_string());
                }
            }
            self.log.push(format!("set {id} {address}"));
            async { Ok(()) }
        }
    }

    fn cfg() -> WanWriter {
        WanWriter {
            enabled: true,
            max_removed: 50,
            max_removed_percent: 20.0,
        }
    }

    fn want(entries: &[(&str, &str, &str)]) -> BTreeMap<Ipv4Addr, ListEntry> {
        entries
            .iter()
            .map(|(list, address, name)| {
                let address: Ipv4Addr = address.parse().unwrap();
                let entry = ListEntry {
                    list: list.to_string(),
                    address,
                    comment: format!("{OWNER}{name}"),
                };
                (address, entry)
            })
            .collect()
    }

    #[tokio::test]
    async fn foreign_entries_are_never_touched() {
        let mut f = Fake {
            rows: vec![
                row("*1", "via-WAN1", "10.0.0.1", "manual entry"),
                row("*2", "WAN1-CUSTOMERS", "10.0.0.2", "lqosync: old"),
                row("*3", "via-WAN1", "10.0.0.3", "lqosync: A"),
            ],
            ..Fake::default()
        };
        let r = apply(
            &mut f,
            &want(&[("via-WAN1", "10.0.0.3", "A")]),
            &cfg(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            r,
            WriteReport {
                owned_after: 1,
                ..Default::default()
            }
        );
        assert!(f.log.is_empty(), "{:?}", f.log);
        assert_eq!(f.rows.len(), 3);
    }

    #[tokio::test]
    async fn a_move_adds_before_it_removes() {
        let mut f = Fake {
            rows: vec![row("*1", "via-WAN1", "10.0.0.5", "lqosync: A")],
            ..Fake::default()
        };
        apply(
            &mut f,
            &want(&[("via-WAN2", "10.0.0.5", "A")]),
            &cfg(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(f.log, ["add via-WAN2 10.0.0.5", "remove *1"]);
    }

    #[tokio::test]
    async fn a_new_address_changes_the_entry() {
        let mut f = Fake {
            rows: vec![row("*1", "via-WAN1", "10.0.0.5", "lqosync: A")],
            ..Fake::default()
        };
        let r = apply(
            &mut f,
            &want(&[("via-WAN1", "10.0.0.9", "A")]),
            &cfg(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(f.log, ["set *1 10.0.0.9"]);
        assert_eq!(r.changed, 1);
    }

    #[tokio::test]
    async fn an_unchanged_world_writes_nothing() {
        let wanted = want(&[("via-WAN1", "10.0.0.1", "A"), ("via-WAN2", "10.0.0.2", "B")]);
        let mut f = Fake::default();
        apply(&mut f, &wanted, &cfg(), None).await.unwrap();
        f.log.clear();
        apply(&mut f, &wanted, &cfg(), None).await.unwrap();
        assert!(f.log.is_empty(), "{:?}", f.log);
    }

    #[tokio::test]
    async fn a_mass_removal_holds() {
        let rows = (1..=100)
            .map(|i| {
                row(
                    &format!("*{i}"),
                    "via-WAN1",
                    &format!("10.0.1.{i}"),
                    "lqosync: x",
                )
            })
            .collect();
        let mut f = Fake {
            rows,
            ..Fake::default()
        };
        // 30 of 100 entries go: more than 20%.
        let keep: Vec<(String, String)> = (1..=70)
            .map(|i| ("via-WAN1".to_string(), format!("10.0.1.{i}")))
            .collect();
        let keep: Vec<(&str, &str, &str)> = keep
            .iter()
            .map(|(l, a)| (l.as_str(), a.as_str(), "x"))
            .collect();
        let r = apply(&mut f, &want(&keep), &cfg(), None).await;
        let Err(WriteError::Held { fingerprint, .. }) = r else {
            panic!("{r:?}")
        };
        assert!(f.log.is_empty());
        assert_eq!(f.rows.len(), 100);
        // The same removal in the next cycle goes through.
        apply(&mut f, &want(&keep), &cfg(), Some(fingerprint))
            .await
            .unwrap();
        assert_eq!(f.rows.len(), 70);
    }

    #[tokio::test]
    async fn duplicate_owned_entries_are_reduced_to_one() {
        let mut f = Fake {
            rows: vec![
                row("*1", "via-WAN1", "10.0.0.5", "lqosync: A"),
                row("*2", "via-WAN1", "10.0.0.5", "lqosync: A"),
            ],
            ..Fake::default()
        };
        apply(
            &mut f,
            &want(&[("via-WAN1", "10.0.0.5", "A")]),
            &cfg(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(f.log, ["remove *2"]);
    }
}
