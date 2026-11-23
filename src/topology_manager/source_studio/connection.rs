//! The explicit one-shot test of an unsaved router connection (`StudioJob::DraftTest`).
//! It saves no router, enables no read, and writes nothing. The secrets stay in memory.
use crate::config::{Config, DraftSecret, RouterCfg};
use crate::routeros::{Connector, ReadError, RouterSession};
use std::time::{Duration, Instant};
use web_contract::routers::{MAX_CA_PEM, MAX_PASSWORD};
use web_contract::source_studio::{
    CheckState, DraftConnection, DraftTestReport, JOB_DEADLINE_SECS, SourceKind, TestCheck,
    TestStage,
};

/// A refused draft: the field and a safe reason.
#[derive(Debug, PartialEq, Eq)]
pub struct Invalid {
    pub field: &'static str,
    pub message: String,
}

const STAGES: [TestStage; 5] = [
    TestStage::Transport,
    TestStage::Tls,
    TestStage::Authentication,
    TestStage::Identity,
    TestStage::TablePermission,
];

/// The reader tables of each source. The test reads only their row IDs.
fn tables(kind: SourceKind) -> &'static [&'static str] {
    match kind {
        SourceKind::Ppp => &["/ppp/secret", "/ppp/active", "/ppp/profile"],
        SourceKind::Dhcp => &["/ip/dhcp-server", "/ip/dhcp-server/lease"],
        SourceKind::Hotspot => &["/ip/hotspot/user", "/ip/hotspot/active"],
        SourceKind::Wan => &["/interface", "/ip/route"],
    }
}

fn invalid(field: &'static str, message: &str) -> Invalid {
    Invalid {
        field,
        message: message.into(),
    }
}

/// The router-edit limits apply to the draft too.
fn validate(d: &DraftConnection, required: &[SourceKind]) -> Result<(), Invalid> {
    if !crate::router_edit::valid_address(&d.address) {
        return Err(invalid("address", "Check the address."));
    }
    if d.port == 0 {
        return Err(invalid("port", "Use an API port from 1 to 65535."));
    }
    if !crate::router_edit::valid_user(&d.username) {
        return Err(invalid("username", "Check the username."));
    }
    let p = &d.password.0;
    if p.is_empty() || p.chars().count() > MAX_PASSWORD || p.chars().any(char::is_control) {
        return Err(invalid(
            "password",
            "Use a credential of 1 to 256 characters, without line breaks.",
        ));
    }
    if let Some(pem) = &d.ca_pem {
        let certs = rustls_pemfile::certs(&mut pem.as_bytes()).collect::<Result<Vec<_>, _>>();
        // An error or no certificate is refused.
        let certs = certs
            .map_err(|_| ())
            .and_then(|c| if c.is_empty() { Err(()) } else { Ok(c) });
        if pem.len() > MAX_CA_PEM || certs.is_err() {
            return Err(invalid(
                "ca_pem",
                "The CA certificate must be PEM text with at least one certificate, up to 16 KiB.",
            ));
        }
    }
    if (1..required.len()).any(|i| required[..i].contains(&required[i])) {
        return Err(invalid("required_sources", "Name each source once."));
    }
    Ok(())
}

/// A router for this test only. The connector uses only its connection fields, so the other
/// fields come from the first configured router.
fn draft_router(cfg: &Config, d: &DraftConnection) -> RouterCfg {
    let mut r = cfg.routers[0].clone();
    r.name = "draft-test".into();
    r.enabled = true;
    r.address = d.address.clone();
    r.port = d.port;
    r.tls = d.tls;
    r.tls_verify = d.tls_verify;
    r.tls_ca_file = String::new();
    r.username = d.username.clone();
    r.password_file = String::new();
    r.writer_username = String::new();
    r.writer_password_file = String::new();
    r.access_profiles = Vec::new();
    r.draft = Some(DraftSecret {
        password: d.password.clone(),
        ca_pem: d.ca_pem.clone(),
    });
    r
}

fn code(e: &ReadError) -> &'static str {
    match e {
        ReadError::Unreachable(_) => "unreachable",
        ReadError::Login(_) => "login",
        ReadError::Tls(_) => "tls",
        ReadError::Timeout(_) => "timeout",
        ReadError::Trap(_) => "trap",
        ReadError::Protocol(_) => "protocol",
    }
}

fn check(stage: TestStage, state: CheckState, code: &str) -> TestCheck {
    TestCheck {
        stage,
        state,
        code: code.into(),
    }
}

/// The checks before an authenticated session: TCP, then TLS, then the login.
fn connected(checks: &mut Vec<TestCheck>, tls: bool, until: TestStage) {
    for stage in [
        TestStage::Transport,
        TestStage::Tls,
        TestStage::Authentication,
    ] {
        if stage == until {
            return;
        }
        checks.push(match (stage, tls) {
            (TestStage::Tls, false) => check(stage, CheckState::NotRun, "tls_off"),
            _ => check(stage, CheckState::Passed, "ok"),
        });
    }
}

/// Names the connection without its secret. A changed draft that keeps all of these values
/// has the same fingerprint; the browser clears the secret after each test.
fn fingerprint(d: &DraftConnection) -> String {
    let fields = serde_json::json!([d.address, d.port, d.tls, d.tls_verify, d.username, d.ca_pem]);
    format!(
        "{:016x}",
        crate::topology::fnv1a64(fields.to_string().as_bytes())
    )
}

/// One bounded test: TCP, TLS, login, identity, then read permission on the source tables.
/// A disabled registered router with the same address and port gets no connection.
pub async fn draft_test<C: Connector>(
    connector: &C,
    cfg: &Config,
    d: &DraftConnection,
    required: &[SourceKind],
) -> Result<DraftTestReport, Invalid> {
    validate(d, required)?;
    let started = Instant::now();
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |t| t.as_secs());
    let mut checks = Vec::new();
    let disabled = cfg
        .routers
        .iter()
        .any(|r| !r.enabled && r.port == d.port && r.address.eq_ignore_ascii_case(&d.address));
    if disabled {
        checks.push(check(TestStage::Transport, CheckState::Failed, "disabled"));
    } else {
        let router = draft_router(cfg, d);
        let run = async {
            let mut session = match connector.connect(&router, &cfg.router_access).await {
                Ok(s) => s,
                Err(e) => {
                    let stage = match e {
                        ReadError::Unreachable(_) => TestStage::Transport,
                        ReadError::Tls(_) => TestStage::Tls,
                        _ => TestStage::Authentication,
                    };
                    connected(&mut checks, d.tls, stage);
                    checks.push(check(stage, CheckState::Failed, code(&e)));
                    return;
                }
            };
            connected(&mut checks, d.tls, TestStage::Identity);
            if let Err(e) = session.print("/system/identity", "name").await {
                checks.push(check(TestStage::Identity, CheckState::Failed, code(&e)));
                return;
            }
            checks.push(check(TestStage::Identity, CheckState::Passed, "ok"));
            if required.is_empty() {
                return;
            }
            // shortcut: reads every row ID of each table; a count-only print would read less.
            for path in required.iter().flat_map(|k| tables(*k)) {
                if let Err(e) = session.print(path, ".id").await {
                    checks.push(check(
                        TestStage::TablePermission,
                        CheckState::Failed,
                        code(&e),
                    ));
                    return;
                }
            }
            checks.push(check(TestStage::TablePermission, CheckState::Passed, "ok"));
        };
        if tokio::time::timeout(Duration::from_secs(JOB_DEADLINE_SECS), run)
            .await
            .is_err()
        {
            let stage = STAGES[checks.len().min(STAGES.len() - 1)];
            checks.push(check(stage, CheckState::Failed, "timeout"));
        }
    }
    for stage in &STAGES[checks.len()..] {
        checks.push(check(*stage, CheckState::NotRun, "not_run"));
    }
    Ok(DraftTestReport {
        connection_fingerprint: fingerprint(d),
        at,
        elapsed_ms: started.elapsed().as_millis() as u64,
        checks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RouterAccess, RouterCfg};
    use crate::model::RosRow;
    use crate::routeros::{ReadError, RouterSession};
    use std::sync::{Arc, Mutex};
    use web_contract::source_studio::{CheckState, SecretInput, TestStage};

    const SECRET: &str = "s3cret-draft-value";

    /// A fake router: it records each connect and command, and fails where the test says.
    #[derive(Clone, Default)]
    struct Fake {
        connects: Arc<Mutex<Vec<RouterCfg>>>,
        calls: Arc<Mutex<Vec<String>>>,
        connect_err: Option<ReadError>,
        fail_path: Option<(&'static str, ReadError)>,
        stall: bool,
    }
    struct Session(Fake);

    impl Connector for Fake {
        type Session = Session;
        fn connect(
            &self,
            r: &RouterCfg,
            _a: &RouterAccess,
        ) -> impl Future<Output = Result<Session, ReadError>> + Send {
            self.connects.lock().unwrap().push(r.clone());
            let me = self.clone();
            async move {
                match me.connect_err.clone() {
                    Some(e) => Err(e),
                    None => Ok(Session(me)),
                }
            }
        }
    }
    impl RouterSession for Session {
        fn print(
            &mut self,
            path: &str,
            proplist: &str,
        ) -> impl Future<Output = Result<Vec<RosRow>, ReadError>> + Send {
            let f = self.0.clone();
            let path = path.to_owned();
            f.calls.lock().unwrap().push(format!("{path} {proplist}"));
            async move {
                if f.stall {
                    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                }
                match &f.fail_path {
                    Some((p, e)) if *p == path => Err(e.clone()),
                    _ => Ok(vec![]),
                }
            }
        }
    }

    fn cfg(extra: &str) -> Config {
        Config::from_toml_str(&format!(
            "[[routers]]\nname='Core'\naddress='192.0.2.1'\nusername='u'\npassword_file='/x'\n\
             [[routers]]\nname='Spare'\nenabled=false\naddress='192.0.2.9'\nport=8729\nusername='u'\npassword_file='/y'\n{extra}"
        ))
        .unwrap()
    }

    fn draft(address: &str) -> DraftConnection {
        DraftConnection {
            address: address.into(),
            port: 8729,
            tls: true,
            tls_verify: true,
            username: "studio".into(),
            password: SecretInput(SECRET.into()),
            ca_pem: None,
        }
    }

    fn states(r: &DraftTestReport) -> Vec<(TestStage, CheckState, String)> {
        r.checks
            .iter()
            .map(|c| (c.stage, c.state, c.code.clone()))
            .collect()
    }

    #[tokio::test]
    async fn draft_test_never_saves_or_enables() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("lqosync3.toml");
        let state = dir.path().join("state");
        std::fs::create_dir(&state).unwrap();
        let text = format!(
            "[runtime]\nstate_dir='{}'\n[[routers]]\nname='Core'\naddress='192.0.2.1'\nusername='u'\npassword_file='/x'\n",
            state.display()
        );
        std::fs::write(&config, &text).unwrap();
        let c = Config::from_toml_str(&text).unwrap();
        let fake = Fake::default();
        let r = draft_test(&fake, &c, &draft("198.51.100.7"), &[SourceKind::Ppp])
            .await
            .unwrap();
        assert!(
            r.checks.iter().all(|c| c.state == CheckState::Passed),
            "{r:?}"
        );
        // Read commands only. No config, registry, or state file changes.
        let calls = fake.calls.lock().unwrap().clone();
        assert_eq!(
            calls,
            [
                "/system/identity name",
                "/ppp/secret .id",
                "/ppp/active .id",
                "/ppp/profile .id"
            ]
        );
        assert_eq!(std::fs::read_to_string(&config).unwrap(), text);
        assert_eq!(std::fs::read_dir(&state).unwrap().count(), 0);
        // The test router exists in memory only, and the config keeps its routers.
        let tested = &fake.connects.lock().unwrap()[0];
        assert_eq!(tested.address, "198.51.100.7");
        assert!(tested.password_file.is_empty() && tested.tls_ca_file.is_empty());
        assert!(tested.draft.is_some());
        assert_eq!(c.routers.len(), 1);
        assert!(c.routers.iter().all(|r| r.draft.is_none()));
    }

    #[tokio::test]
    async fn disabled_registered_router_stays_no_io() {
        let c = cfg("");
        let fake = Fake::default();
        let r = draft_test(&fake, &c, &draft("192.0.2.9"), &[])
            .await
            .unwrap();
        assert!(fake.connects.lock().unwrap().is_empty());
        assert_eq!(
            states(&r)[0],
            (TestStage::Transport, CheckState::Failed, "disabled".into())
        );
        assert!(r.checks[1..].iter().all(|c| c.state == CheckState::NotRun));
        // The same address on another port is another router.
        let mut other = draft("192.0.2.9");
        other.port = 8728;
        draft_test(&fake, &c, &other, &[]).await.unwrap();
        assert_eq!(fake.connects.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn secret_is_absent_from_reports() {
        let c = cfg("");
        let fake = Fake {
            connect_err: Some(ReadError::Login(format!("login failed for {SECRET}"))),
            ..Fake::default()
        };
        let d = draft("198.51.100.7");
        let r = draft_test(&fake, &c, &d, &[SourceKind::Dhcp])
            .await
            .unwrap();
        assert_eq!(
            states(&r)[2],
            (
                TestStage::Authentication,
                CheckState::Failed,
                "login".into()
            )
        );
        let json = serde_json::to_string(&r).unwrap();
        for text in [json, format!("{r:?}"), format!("{d:?}")] {
            assert!(!text.contains(SECRET), "{text}");
        }
        let tested = format!("{:?}", fake.connects.lock().unwrap()[0]);
        assert!(!tested.contains(SECRET), "{tested}");
        // The fingerprint names the connection without the secret.
        let mut changed = d.clone();
        changed.password = SecretInput("another".into());
        let again = draft_test(&fake, &c, &changed, &[SourceKind::Dhcp])
            .await
            .unwrap();
        assert_eq!(again.connection_fingerprint, r.connection_fingerprint);
        changed.username = "other".into();
        let other = draft_test(&fake, &c, &changed, &[SourceKind::Dhcp])
            .await
            .unwrap();
        assert_ne!(other.connection_fingerprint, r.connection_fingerprint);
    }

    #[test]
    fn shared_profile_does_not_enable_writes() {
        let c = cfg(
            "access_profiles=[{id='admin',label='Full account',username='admin',password_file='/a',supported_writes=['wan_address_lists']}]\n",
        );
        let spare = &c.routers[1];
        assert_eq!(spare.access_profiles.len(), 1);
        assert!(spare.writer_username.is_empty() && spare.writer_password_file.is_empty());
        assert!(!c.wan_writer.enabled);
        // The writer session needs its own writer user; a profile does not supply one.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let connector = crate::mikrotik::MikrotikConnector { owner_uid: 0 };
        let mut enabled = spare.clone();
        enabled.enabled = true;
        let writer = rt.block_on(connector.connect_writer(&enabled, &c.router_access));
        assert!(matches!(writer, Err(ReadError::Login(m)) if m.contains("no writer user")));
        assert!(
            !super::super::FEATURES
                .iter()
                .any(|f| matches!(f, web_contract::source_studio::StudioFeature::PolicySave))
        );
    }

    #[tokio::test]
    async fn each_failure_names_its_stage() {
        let c = cfg("");
        let cases = [
            (
                Fake {
                    connect_err: Some(ReadError::Unreachable("x".into())),
                    ..Fake::default()
                },
                TestStage::Transport,
                "unreachable",
            ),
            (
                Fake {
                    connect_err: Some(ReadError::Tls("x".into())),
                    ..Fake::default()
                },
                TestStage::Tls,
                "tls",
            ),
            (
                Fake {
                    fail_path: Some(("/system/identity", ReadError::Trap("x".into()))),
                    ..Fake::default()
                },
                TestStage::Identity,
                "trap",
            ),
            (
                Fake {
                    fail_path: Some(("/ip/dhcp-server/lease", ReadError::Trap("x".into()))),
                    ..Fake::default()
                },
                TestStage::TablePermission,
                "trap",
            ),
        ];
        for (fake, stage, code) in cases {
            let r = draft_test(&fake, &c, &draft("198.51.100.7"), &[SourceKind::Dhcp])
                .await
                .unwrap();
            let s = states(&r);
            assert_eq!(s.len(), 5, "{s:?}");
            let i = s.iter().position(|x| x.0 == stage).unwrap();
            assert_eq!(
                (s[i].1, s[i].2.as_str()),
                (CheckState::Failed, code),
                "{s:?}"
            );
            assert!(s[..i].iter().all(|x| x.1 == CheckState::Passed), "{s:?}");
            assert!(
                s[i + 1..].iter().all(|x| x.1 == CheckState::NotRun),
                "{s:?}"
            );
        }
        // Without TLS the TLS stage does not run, and no source means no table check.
        let mut plain = draft("198.51.100.7");
        plain.tls = false;
        let r = draft_test(&Fake::default(), &c, &plain, &[]).await.unwrap();
        assert_eq!(
            states(&r)[1],
            (TestStage::Tls, CheckState::NotRun, "tls_off".into())
        );
        assert_eq!(r.checks[4].state, CheckState::NotRun);
    }

    #[tokio::test(start_paused = true)]
    async fn a_stalled_router_ends_at_the_deadline() {
        let c = cfg("");
        let fake = Fake {
            stall: true,
            ..Fake::default()
        };
        let r = draft_test(&fake, &c, &draft("198.51.100.7"), &[SourceKind::Ppp])
            .await
            .unwrap();
        assert_eq!(
            states(&r)[3],
            (TestStage::Identity, CheckState::Failed, "timeout".into())
        );
    }

    #[tokio::test]
    async fn an_invalid_draft_is_refused_before_any_connect() {
        let c = cfg("");
        let fake = Fake::default();
        let mut cases: Vec<(DraftConnection, Vec<SourceKind>, &str)> = Vec::new();
        let mut d = draft("bad address");
        cases.push((d.clone(), vec![], "address"));
        d = draft("198.51.100.7");
        d.port = 0;
        cases.push((d.clone(), vec![], "port"));
        d = draft("198.51.100.7");
        d.username = "two words".into();
        cases.push((d.clone(), vec![], "username"));
        d = draft("198.51.100.7");
        d.password = SecretInput(String::new());
        cases.push((d.clone(), vec![], "password"));
        d = draft("198.51.100.7");
        d.ca_pem = Some("not a certificate".into());
        cases.push((d.clone(), vec![], "ca_pem"));
        cases.push((
            draft("198.51.100.7"),
            vec![SourceKind::Ppp, SourceKind::Ppp],
            "required_sources",
        ));
        for (d, required, field) in cases {
            let e = draft_test(&fake, &c, &d, &required).await.unwrap_err();
            assert_eq!(e.field, field);
            assert!(!e.message.contains(SECRET));
        }
        assert!(fake.connects.lock().unwrap().is_empty());
    }
}
