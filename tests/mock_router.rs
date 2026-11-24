//! Tests of the real mikrotik-rs session against a mock RouterOS API server.

use lqosync3::config::{Config, RouterAccess, RouterCfg};
use lqosync3::mikrotik::MikrotikConnector;
use lqosync3::routeros::{ReadError, read_router};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

#[path = "support/routeros_wire.rs"]
mod wire;
use wire::{read_sentence, sentence, word};

/// What the mock does with a print command: rows, an empty answer, a trap, or no answer.
#[derive(Clone)]
enum Answer {
    Rows(Vec<Vec<(String, String)>>),
    Empty,
    Trap(String),
    Silent,
    /// One row with raw value bytes (for example Latin-1 text from Winbox).
    Raw(Vec<(String, Vec<u8>)>),
}

type Log = Arc<Mutex<Vec<Vec<String>>>>;

/// Start a mock router. `answer` gives the reply to each print path. `password` is the accepted one.
async fn mock(
    answer: impl Fn(&str) -> Answer + Send + Sync + 'static,
    password: &'static str,
) -> (u16, Log) {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let log: Log = Arc::default();
    let log2 = log.clone();
    let answer = Arc::new(answer);
    tokio::spawn(async move {
        while let Ok((mut s, _)) = l.accept().await {
            let (log, answer) = (log2.clone(), answer.clone());
            tokio::spawn(async move {
                while let Ok(words) = read_sentence(&mut s).await {
                    log.lock().unwrap().push(words.clone());
                    let tag: Vec<String> = words
                        .iter()
                        .filter(|w| w.starts_with(".tag="))
                        .cloned()
                        .collect();
                    let mut out = Vec::new();
                    if words.first().map(String::as_str) == Some("/login") {
                        // A router whose API hangs at login: no answer at all.
                        if password == "SILENT" {
                            continue;
                        }
                        if words.iter().any(|w| w == &format!("=password={password}")) {
                            out.extend(sentence(&[vec!["!done".to_string()], tag].concat()));
                        } else {
                            out.extend(sentence(
                                &[
                                    vec![
                                        "!trap".to_string(),
                                        "=message=invalid user name or password".into(),
                                    ],
                                    tag.clone(),
                                ]
                                .concat(),
                            ));
                            out.extend(sentence(&[vec!["!done".to_string()], tag].concat()));
                        }
                    } else {
                        let path = words.first().cloned().unwrap_or_default();
                        match answer(path.trim_end_matches("/print")) {
                            Answer::Silent => continue,
                            Answer::Raw(fields) => {
                                let mut raw = word("!re");
                                for t in &tag {
                                    raw.extend(word(t));
                                }
                                for (k, v) in fields {
                                    let mut w = format!("={k}=").into_bytes();
                                    w.extend(v);
                                    let n = w.len();
                                    raw.push(n as u8);
                                    raw.extend(w);
                                }
                                raw.push(0);
                                out.extend(raw);
                                out.extend(sentence(&[vec!["!done".to_string()], tag].concat()));
                            }
                            Answer::Empty => {
                                out.extend(sentence(&[vec!["!empty".to_string()], tag].concat()))
                            }
                            Answer::Trap(m) => {
                                out.extend(sentence(
                                    &[
                                        vec!["!trap".to_string(), format!("=message={m}")],
                                        tag.clone(),
                                    ]
                                    .concat(),
                                ));
                                out.extend(sentence(&[vec!["!done".to_string()], tag].concat()));
                            }
                            Answer::Rows(rows) => {
                                for r in rows {
                                    let mut w = vec!["!re".to_string()];
                                    w.extend(tag.clone());
                                    w.extend(r.into_iter().map(|(k, v)| format!("={k}={v}")));
                                    out.extend(sentence(&w));
                                }
                                out.extend(sentence(&[vec!["!done".to_string()], tag].concat()));
                            }
                        }
                    }
                    // All replies in one write: the burst case.
                    if s.write_all(&out).await.is_err() {
                        return;
                    }
                }
            });
        }
    });
    (port, log)
}

struct Setup {
    _dir: tempfile::TempDir,
    router: RouterCfg,
    access: RouterAccess,
    connector: MikrotikConnector,
}

fn setup(port: u16) -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let pw = dir.path().join("r.password");
    std::fs::write(&pw, "good\n").unwrap();
    std::fs::set_permissions(&pw, std::fs::Permissions::from_mode(0o600)).unwrap();
    let uid = std::fs::metadata(&pw).unwrap().uid();
    let c = Config::from_toml_str(&format!(
        "[router_access]\napi_timeout=1\nconnect_timeout=2\nread_retries=0\nretry_delay=0\n\
         [[routers]]\nname=\"R1\"\naddress=\"127.0.0.1\"\nport={port}\ntls=false\nusername=\"lqosync-read\"\n\
         password_file=\"{}\"\n[routers.pppoe]\nenabled=true\n",
        pw.display()
    ))
    .unwrap();
    Setup {
        _dir: dir,
        router: c.routers[0].clone(),
        access: c.router_access.clone(),
        connector: MikrotikConnector { owner_uid: uid },
    }
}

fn secrets(n: usize) -> Answer {
    Answer::Rows(
        (0..n)
            .map(|i| {
                vec![
                    ("name".to_string(), format!("u{i:03}")),
                    ("profile".into(), "Gold".into()),
                ]
            })
            .collect(),
    )
}

#[tokio::test]
async fn reads_300_rows_sent_in_one_write() {
    let (port, _) = mock(
        |p| {
            if p == "/ppp/secret" {
                secrets(300)
            } else {
                Answer::Rows(vec![])
            }
        },
        "good",
    )
    .await;
    let s = setup(port);
    let snap = read_router(&s.connector, &mut None, &s.router, &s.access)
        .await
        .unwrap();
    assert_eq!(snap.ppp_secret.len(), 300);
    assert_eq!(snap.ppp_secret[299]["name"], "u299");
}

#[tokio::test]
async fn proplist_is_sent() {
    let (port, log) = mock(|_| Answer::Rows(vec![]), "good").await;
    let s = setup(port);
    read_router(&s.connector, &mut None, &s.router, &s.access)
        .await
        .unwrap();
    let log = log.lock().unwrap();
    let print = log
        .iter()
        .find(|w| w[0] == "/ppp/secret/print")
        .expect("secret print sent");
    assert!(
        print
            .iter()
            .any(|w| w == "=.proplist=name,profile,comment,disabled,caller-id"),
        "{print:?}"
    );
    assert!(
        log.iter()
            .all(|w| w[0] == "/login" || w[0].ends_with("/print")),
        "only reads: {log:?}"
    );
}

#[tokio::test]
async fn empty_table_is_ok() {
    let (port, _) = mock(|_| Answer::Empty, "good").await;
    let s = setup(port);
    let snap = read_router(&s.connector, &mut None, &s.router, &s.access)
        .await
        .unwrap();
    assert!(snap.ppp_secret.is_empty());
}

#[tokio::test]
async fn trap_is_a_trap_error() {
    let (port, _) = mock(|_| Answer::Trap("no such command".into()), "good").await;
    let s = setup(port);
    let e = read_router(&s.connector, &mut None, &s.router, &s.access)
        .await
        .unwrap_err();
    assert_eq!(e, ReadError::Trap("no such command".into()));
}

#[tokio::test]
async fn wrong_password_is_login_error() {
    let (port, _) = mock(|_| Answer::Rows(vec![]), "other").await;
    let s = setup(port);
    let e = read_router(&s.connector, &mut None, &s.router, &s.access)
        .await
        .unwrap_err();
    assert!(matches!(e, ReadError::Login(_)), "{e:?}");
    assert!(
        !e.to_string().contains("good"),
        "the password must not be in the error: {e}"
    );
}

#[tokio::test]
async fn closed_port_is_unreachable() {
    let port = {
        TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    };
    let s = setup(port);
    let e = read_router(&s.connector, &mut None, &s.router, &s.access)
        .await
        .unwrap_err();
    assert!(matches!(e, ReadError::Unreachable(_)), "{e:?}");
}

#[tokio::test]
async fn silent_router_times_out_and_does_not_block_another() {
    let (bad, _) = mock(|_| Answer::Silent, "good").await;
    let (good, _) = mock(|_| Answer::Rows(vec![]), "good").await;
    let (sb, sg) = (setup(bad), setup(good));
    let start = Instant::now();
    let mut no_session = None;
    let (rb, rg) = tokio::join!(
        read_router(&sb.connector, &mut no_session, &sb.router, &sb.access),
        async {
            let r = read_router(&sg.connector, &mut None, &sg.router, &sg.access).await;
            (r, start.elapsed())
        }
    );
    assert!(matches!(rb, Err(ReadError::Timeout(_))), "{rb:?}");
    assert!(rg.0.is_ok());
    assert!(
        rg.1 < Duration::from_millis(900),
        "the good router waited {:?}",
        rg.1
    );
}

#[tokio::test]
async fn login_stall_is_failed_not_unreachable() {
    let (port, _) = mock(|_| Answer::Rows(vec![]), "SILENT").await;
    let s = setup(port);
    let e = read_router(&s.connector, &mut None, &s.router, &s.access)
        .await
        .unwrap_err();
    assert!(
        matches!(e, ReadError::Timeout(_)),
        "a hung API is not an unreachable router: {e:?}"
    );
}

#[tokio::test]
async fn latin1_values_are_decoded() {
    let (port, _) = mock(
        |p| {
            if p == "/ppp/secret" {
                Answer::Raw(vec![
                    ("name".into(), b"jos\xe9".to_vec()),
                    ("comment".into(), b"Jos\xe9|25M/5M".to_vec()),
                ])
            } else {
                Answer::Rows(vec![])
            }
        },
        "good",
    )
    .await;
    let s = setup(port);
    let snap = read_router(&s.connector, &mut None, &s.router, &s.access)
        .await
        .unwrap();
    assert_eq!(snap.ppp_secret[0]["name"], "jos\u{e9}");
    assert_eq!(snap.ppp_secret[0]["comment"], "Jos\u{e9}|25M/5M");
}

#[tokio::test]
async fn tls_without_verify_does_not_panic() {
    let (port, _) = mock(|_| Answer::Rows(vec![]), "good").await;
    let mut s = setup(port);
    s.router.tls = true;
    s.router.tls_verify = false;
    // The mock speaks plain TCP, so the TLS handshake fails. It must fail, not panic.
    let e = read_router(&s.connector, &mut None, &s.router, &s.access)
        .await
        .unwrap_err();
    assert!(!matches!(e, ReadError::Login(_)), "{e:?}");
}

fn row(fields: &[(&str, &str)]) -> Vec<(String, String)> {
    fields
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[tokio::test]
async fn ping_reads_the_round_trip_time_and_a_lost_reply() {
    use lqosync3::routeros::{Connector, RouterProbe};
    let (port, log) = mock(
        |p| match p {
            "/ping" => Answer::Rows(vec![row(&[
                ("host", "1.1.1.1"),
                ("time", "12ms345us"),
                ("received", "1"),
            ])]),
            _ => Answer::Empty,
        },
        "good",
    )
    .await;
    let s = setup(port);
    let mut session = s.connector.connect(&s.router, &s.access).await.unwrap();
    assert_eq!(
        session.ping("1.1.1.1", "ether1").await.unwrap(),
        Some(12_345)
    );
    let sent = log
        .lock()
        .unwrap()
        .iter()
        .find(|w| w[0] == "/ping")
        .cloned()
        .unwrap();
    for word in ["=address=1.1.1.1", "=interface=ether1", "=count=1"] {
        assert!(sent.iter().any(|w| w == word), "{sent:?}");
    }
    let (port, _) = mock(
        |p| match p {
            "/ping" => Answer::Rows(vec![row(&[("status", "timeout"), ("received", "0")])]),
            _ => Answer::Empty,
        },
        "good",
    )
    .await;
    let s = setup(port);
    let mut session = s.connector.connect(&s.router, &s.access).await.unwrap();
    assert_eq!(session.ping("1.1.1.1", "ether1").await.unwrap(), None);
}

#[tokio::test]
async fn ping_permission_error_is_a_read_error() {
    use lqosync3::routeros::{Connector, RouterProbe};
    let (port, _) = mock(
        |p| match p {
            "/ping" => Answer::Trap("not enough permissions (9)".into()),
            _ => Answer::Empty,
        },
        "good",
    )
    .await;
    let s = setup(port);
    let mut session = s.connector.connect(&s.router, &s.access).await.unwrap();
    assert!(session.ping("1.1.1.1", "ether1").await.is_err());
}

fn writer_router(s: &Setup) -> lqosync3::config::RouterCfg {
    let mut r = s.router.clone();
    r.writer_username = "lqosync-writer".into();
    r.writer_password_file = r.password_file.clone();
    r
}

#[tokio::test]
async fn the_writer_sends_only_address_list_commands() {
    use lqosync3::routeros::ListWriter;
    use lqosync3::topology_manager::wan_lists::ListEntry;
    let (port, log) = mock(|_| Answer::Rows(vec![]), "good").await;
    let s = setup(port);
    let mut w = s
        .connector
        .connect_writer(&writer_router(&s), &s.access)
        .await
        .unwrap();
    let entry = ListEntry {
        list: "via-WAN1".into(),
        address: "10.0.100.222".parse().unwrap(),
        comment: "lqosync: LitoYambi".into(),
    };
    w.add(&entry).await.unwrap();
    w.set_address("*1A", "10.0.100.223".parse().unwrap())
        .await
        .unwrap();
    w.remove("*1A").await.unwrap();
    let log = log.lock().unwrap();
    let commands: Vec<&str> = log.iter().map(|w| w[0].as_str()).collect();
    assert_eq!(
        commands,
        [
            "/login",
            "/ip/firewall/address-list/add",
            "/ip/firewall/address-list/set",
            "/ip/firewall/address-list/remove"
        ]
    );
    let words = |i: usize| {
        log[i]
            .iter()
            .filter(|w| w.starts_with('='))
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(
        words(1),
        [
            "=list=via-WAN1",
            "=address=10.0.100.222",
            "=comment=lqosync: LitoYambi"
        ]
    );
    assert_eq!(words(2), ["=.id=*1A", "=address=10.0.100.223"]);
    assert_eq!(words(3), ["=.id=*1A"]);
}

#[tokio::test]
async fn the_writer_logs_in_with_the_writer_user() {
    let (port, log) = mock(|_| Answer::Rows(vec![]), "good").await;
    let s = setup(port);
    s.connector
        .connect_writer(&writer_router(&s), &s.access)
        .await
        .unwrap();
    let login = log.lock().unwrap()[0].clone();
    assert!(
        login.iter().any(|w| w == "=name=lqosync-writer"),
        "{login:?}"
    );
    // A router without a writer user gets no write session.
    assert!(
        s.connector
            .connect_writer(&s.router, &s.access)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_refused_write_is_an_error() {
    use lqosync3::routeros::ListWriter;
    let (port, _) = mock(
        |p| match p {
            "/ip/firewall/address-list/remove" => Answer::Trap("no such item".into()),
            _ => Answer::Rows(vec![]),
        },
        "good",
    )
    .await;
    let s = setup(port);
    let mut w = s
        .connector
        .connect_writer(&writer_router(&s), &s.access)
        .await
        .unwrap();
    assert!(w.remove("*9").await.is_err());
}
