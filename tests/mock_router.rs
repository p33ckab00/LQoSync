//! Tests of the real mikrotik-rs session against a mock RouterOS API server.

use lqosync3::config::{Config, RouterAccess, RouterCfg};
use lqosync3::mikrotik::MikrotikConnector;
use lqosync3::routeros::{ReadError, read_router};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Encode one API word with the RouterOS length prefix.
fn word(w: &str) -> Vec<u8> {
    let b = w.as_bytes();
    let n = b.len();
    let mut out = match n {
        0..=0x7f => vec![n as u8],
        0x80..=0x3fff => ((n as u16) | 0x8000).to_be_bytes().to_vec(),
        0x4000..=0x1f_ffff => ((n as u32) | 0xc0_0000).to_be_bytes()[1..].to_vec(),
        _ => ((n as u32) | 0xe000_0000).to_be_bytes().to_vec(),
    };
    out.extend_from_slice(b);
    out
}

fn sentence(words: &[String]) -> Vec<u8> {
    let mut out: Vec<u8> = words.iter().flat_map(|w| word(w)).collect();
    out.push(0);
    out
}

async fn read_len(s: &mut TcpStream) -> std::io::Result<usize> {
    let b0 = s.read_u8().await? as usize;
    Ok(match b0 {
        0..=0x7f => b0,
        0x80..=0xbf => ((b0 & 0x3f) << 8) | s.read_u8().await? as usize,
        0xc0..=0xdf => ((b0 & 0x1f) << 16) | (s.read_u16().await? as usize),
        _ => {
            let mut b = [0u8; 3];
            s.read_exact(&mut b).await?;
            ((b0 & 0x0f) << 24) | ((b[0] as usize) << 16) | ((b[1] as usize) << 8) | b[2] as usize
        }
    })
}

async fn read_sentence(s: &mut TcpStream) -> std::io::Result<Vec<String>> {
    let mut words = Vec::new();
    loop {
        let n = read_len(s).await?;
        if n == 0 {
            return Ok(words);
        }
        let mut buf = vec![0u8; n];
        s.read_exact(&mut buf).await?;
        words.push(String::from_utf8_lossy(&buf).into_owned());
    }
}

/// What the mock does with a print command: rows, an empty answer, a trap, or no answer.
#[derive(Clone)]
enum Answer {
    Rows(Vec<Vec<(String, String)>>),
    Empty,
    Trap(String),
    Silent,
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
