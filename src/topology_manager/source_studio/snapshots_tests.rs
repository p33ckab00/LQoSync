use super::*;
use crate::config::RouterAccess;
use crate::routeros::ReadError;
use std::sync::{Arc, Mutex};

type Script = Arc<Mutex<HashMap<&'static str, Result<Vec<RosRow>, ReadError>>>>;

/// A fake router: one answer for each command path. It counts connects.
#[derive(Clone, Default)]
struct Fake {
    connects: Arc<Mutex<usize>>,
    answers: Script,
}
struct Session(Fake);

impl Connector for Fake {
    type Session = Session;
    fn connect(
        &self,
        _r: &RouterCfg,
        _a: &RouterAccess,
    ) -> impl Future<Output = Result<Session, ReadError>> + Send {
        *self.connects.lock().unwrap() += 1;
        let me = self.clone();
        async move { Ok(Session(me)) }
    }
}
impl RouterSession for Session {
    fn print(
        &mut self,
        path: &str,
        _proplist: &str,
    ) -> impl Future<Output = Result<Vec<RosRow>, ReadError>> + Send {
        let answer = self
            .0
            .answers
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .unwrap_or(Ok(vec![]));
        async move { answer }
    }
}

impl Fake {
    fn answer(&self, path: &'static str, a: Result<Vec<RosRow>, ReadError>) {
        self.answers.lock().unwrap().insert(path, a);
    }
}

fn row(pairs: &[(&str, &str)]) -> RosRow {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn cfg() -> Config {
    Config::from_toml_str(
        "[[routers]]\nname='Core'\naddress='192.0.2.1'\nusername='u'\npassword_file='/x'\n\
         access_profiles=[{id='reader',label='Reader',username='ro',password_file='/nonexistent'}]\n\
         [[routers]]\nname='Spare'\nenabled=false\naddress='192.0.2.9'\nusername='u'\npassword_file='/y'\n",
    )
    .unwrap()
}

const D: &str = "digest-1";

fn state(r: &SnapshotRef, t: TableId) -> (ReadState, u64, Option<String>) {
    let e = r.tables.iter().find(|e| e.table == t).unwrap();
    (e.state, e.records, e.issue_code.clone())
}

#[tokio::test]
async fn route_failure_keeps_last_evidence() {
    let (c, fake, mut studio) = (cfg(), Fake::default(), Studio::default());
    let core = router_id("Core");
    fake.answer(
        "/interface",
        Ok(vec![
            row(&[
                (".id", "*1"),
                ("name", "pppoe-out1"),
                ("type", "pppoe-out"),
                ("running", "true"),
            ]),
            row(&[(".id", "*2"), ("name", "pppoe-in1"), ("type", "pppoe-in")]),
        ]),
    );
    fake.answer(
        "/ip/route",
        Ok(vec![row(&[
            ("dst-address", "0.0.0.0/0"),
            ("gateway", "pppoe-out1"),
            ("active", "true"),
            ("routing-table", "wan1"),
        ])]),
    );
    let first = studio
        .collect(&fake, &c, D, &core, D, &[SourceKind::Wan])
        .await
        .unwrap();
    assert_eq!(state(&first, TableId::Routes).0, ReadState::Complete);
    let view = |s: &Studio| {
        s.router_config(&c.routers[0], D, D, None, now_secs())
            .wan_candidates
    };
    let good = view(&studio);
    assert_eq!(good.len(), 1, "a -in tunnel is no WAN candidate");
    assert_eq!(good[0].routes[0].routing_table, "wan1");
    assert_eq!(
        good[0].routes[0].resolved_interface.as_deref(),
        Some("pppoe-out1")
    );
    // A trap on the route table keeps the last routes and says the read failed.
    fake.answer("/ip/route", Err(ReadError::Trap("no permission".into())));
    let failed = studio
        .collect(&fake, &c, D, &core, D, &[SourceKind::Wan])
        .await
        .unwrap();
    assert_eq!(
        state(&failed, TableId::Routes),
        (ReadState::Failed, 0, Some("trap".into()))
    );
    let kept = view(&studio);
    assert_eq!(kept[0].routes, good[0].routes);
    assert_eq!(kept[0].read_state, ReadState::Failed);
    // A complete empty route table is a different answer: no routes, complete.
    fake.answer("/ip/route", Ok(vec![]));
    let empty = studio
        .collect(&fake, &c, D, &core, D, &[SourceKind::Wan])
        .await
        .unwrap();
    assert_eq!(
        state(&empty, TableId::Routes),
        (ReadState::Complete, 0, None)
    );
    let now_empty = view(&studio);
    assert!(now_empty[0].routes.is_empty());
    assert_eq!(now_empty[0].read_state, ReadState::Complete);
}

#[tokio::test]
async fn collection_limit_is_partial() {
    let (c, fake, mut studio) = (cfg(), Fake::default(), Studio::default());
    let rows: Vec<RosRow> = (0..=MAX_TABLE_ROWS)
        .map(|i| row(&[(".id", &format!("*{i}")), ("name", "a")]))
        .collect();
    fake.answer("/ppp/secret", Ok(rows));
    let r = studio
        .collect(&fake, &c, D, &router_id("Core"), D, &[SourceKind::Ppp])
        .await
        .unwrap();
    assert_eq!(
        state(&r, TableId::PppSecrets),
        (ReadState::Partial, MAX_TABLE_ROWS, Some("row_limit".into()))
    );
    assert_eq!(state(&r, TableId::PppActive).0, ReadState::Complete);
    // Tables of other sources are not requested, and not shown as empty complete tables.
    let (s, _, _) = state(&r, TableId::DhcpLeases);
    assert_eq!(s, ReadState::NotRequested);
    assert!(
        r.tables
            .iter()
            .find(|e| e.table == TableId::DhcpLeases)
            .unwrap()
            .collected_at
            .is_none()
    );
    let kept = studio.snapshot(&r.id, now_secs()).unwrap();
    assert_eq!(
        kept.rows(Table::PppSecrets).unwrap().len() as u64,
        MAX_TABLE_ROWS
    );
    // An unrequested field is a protocol failure, not data.
    fake.answer(
        "/ppp/profile",
        Ok(vec![row(&[(".id", "*1"), ("password", "x")])]),
    );
    let r = studio
        .collect(&fake, &c, D, &router_id("Core"), D, &[SourceKind::Ppp])
        .await
        .unwrap();
    assert_eq!(
        state(&r, TableId::PppProfiles),
        (ReadState::Failed, 0, Some("protocol".into()))
    );
    assert!(
        studio
            .snapshot(&r.id, now_secs())
            .unwrap()
            .rows(Table::PppProfiles)
            .is_none()
    );
}

#[tokio::test]
async fn disabled_router_is_not_collected() {
    let (c, fake, mut studio) = (cfg(), Fake::default(), Studio::default());
    for (id, sources) in [
        (router_id("Spare"), vec![SourceKind::Ppp]),
        (router_id("Unknown"), vec![SourceKind::Ppp]),
        (router_id("Core"), vec![]),
        (router_id("Core"), vec![SourceKind::Ppp, SourceKind::Ppp]),
    ] {
        assert!(
            studio
                .collect(&fake, &c, D, &id, D, &sources)
                .await
                .is_err()
        );
    }
    assert_eq!(*fake.connects.lock().unwrap(), 0);
    assert!(
        studio
            .router_config(&c.routers[1], D, D, None, now_secs())
            .snapshots
            .is_empty()
    );
}

#[tokio::test]
async fn snapshot_digest_matches_loaded_config() {
    let (c, fake, mut studio) = (cfg(), Fake::default(), Studio::default());
    let core = router_id("Core");
    let stale = studio
        .collect(&fake, &c, D, &core, "digest-0", &[SourceKind::Dhcp])
        .await;
    assert!(stale.unwrap_err().contains("configuration changed"));
    assert_eq!(*fake.connects.lock().unwrap(), 0);
    let r = studio
        .collect(&fake, &c, D, &core, D, &[SourceKind::Dhcp])
        .await
        .unwrap();
    assert_eq!(r.config_digest, D);
    assert!(r.expires_at > r.collected_to && r.collected_to >= r.collected_from);
    let view = studio.router_config(&c.routers[0], "digest-2", D, Some(42), now_secs());
    assert_eq!(
        (
            view.config_digest.as_str(),
            view.loaded_config_digest.as_str()
        ),
        ("digest-2", D)
    );
    assert_eq!(view.policy_revision.as_deref(), Some("42"));
    assert_eq!(view.snapshots, [r]);
}

#[tokio::test]
async fn the_config_view_shows_profiles_without_secret_paths() {
    let (c, mut studio) = (cfg(), Studio::default());
    let fake = Fake::default();
    let core = router_id("Core");
    for _ in 0..3 {
        studio
            .collect(&fake, &c, D, &core, D, &[SourceKind::Ppp])
            .await
            .unwrap();
    }
    let cache = studio.cache(&c, D, D, None, now_secs());
    assert_eq!(cache.routers.len(), 2);
    let core_view = &cache.routers[0];
    assert_eq!(
        core_view.snapshots.len(),
        KEEP,
        "older snapshots expire first"
    );
    let ids: Vec<_> = core_view
        .access_profiles
        .iter()
        .map(|p| (p.id.as_str(), p.credential_set))
        .collect();
    assert_eq!(ids, [("default", false), ("reader", false)]);
    assert!(core_view.policy_revision.is_none() && core_view.policies.is_empty());
    let json = serde_json::to_string(&cache).unwrap();
    assert!(
        !json.contains("/nonexistent") && !json.contains("password"),
        "{json}"
    );
}
