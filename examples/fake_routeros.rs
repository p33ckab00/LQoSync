//! A fake RouterOS API server for the Studio smoke test. Loopback only, fixed synthetic rows.
//! It answers allowlisted `print` commands, refuses every other command with a trap, and
//! prints one line for each command, so that the test can prove that nothing wrote.
//! Usage: fake_routeros <port> <password>
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

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

type Row = Vec<(&'static str, &'static str)>;

/// The synthetic router: two PPP accounts on one profile, one active session, one WAN.
fn tables() -> HashMap<&'static str, Vec<Row>> {
    HashMap::from([
        ("/system/identity", vec![vec![("name", "smoke-router")]]),
        (
            "/ppp/secret",
            vec![
                vec![
                    (".id", "*1"),
                    ("name", "alice"),
                    ("profile", "basic"),
                    ("disabled", "false"),
                    ("service", "pppoe"),
                ],
                vec![
                    (".id", "*2"),
                    ("name", "carol"),
                    ("profile", "basic"),
                    ("disabled", "false"),
                    ("service", "pppoe"),
                ],
            ],
        ),
        (
            "/ppp/profile",
            vec![vec![
                (".id", "*p1"),
                ("name", "basic"),
                ("rate-limit", "15M/15M"),
            ]],
        ),
        (
            "/ppp/active",
            vec![vec![
                (".id", "*a1"),
                ("name", "alice"),
                ("address", "100.64.0.1"),
                ("caller-id", "02:00:00:00:00:01"),
                ("service", "pppoe"),
            ]],
        ),
        ("/ip/dhcp-server", vec![]),
        ("/ip/dhcp-server/lease", vec![]),
        ("/ip/hotspot", vec![]),
        ("/ip/hotspot/user", vec![]),
        ("/ip/hotspot/user/profile", vec![]),
        ("/ip/hotspot/active", vec![]),
        (
            "/interface",
            vec![
                vec![
                    (".id", "*i1"),
                    ("name", "pppoe-out1"),
                    ("type", "pppoe-out"),
                    ("running", "true"),
                    ("disabled", "false"),
                ],
                vec![
                    (".id", "*i2"),
                    ("name", "ether1"),
                    ("type", "ether"),
                    ("running", "true"),
                    ("disabled", "false"),
                ],
            ],
        ),
        (
            "/ip/route",
            vec![vec![
                ("dst-address", "0.0.0.0/0"),
                ("gateway", "pppoe-out1"),
                ("active", "true"),
                ("routing-table", "main"),
            ]],
        ),
    ])
}

async fn serve(mut s: TcpStream, password: String) {
    let data = tables();
    while let Ok(words) = read_sentence(&mut s).await {
        let tag: Vec<String> = words
            .iter()
            .filter(|w| w.starts_with(".tag="))
            .cloned()
            .collect();
        let done = || sentence(&[vec!["!done".to_string()], tag.clone()].concat());
        let trap = |m: &str| {
            let mut out = sentence(
                &[
                    vec!["!trap".to_string(), format!("=message={m}")],
                    tag.clone(),
                ]
                .concat(),
            );
            out.extend(done());
            out
        };
        let command = words.first().cloned().unwrap_or_default();
        let out = if command == "/login" {
            println!("COMMAND /login");
            if words.iter().any(|w| *w == format!("=password={password}")) {
                done()
            } else {
                trap("invalid user name or password")
            }
        } else if let Some(rows) = command.strip_suffix("/print").and_then(|p| data.get(p)) {
            println!("COMMAND {command}");
            let wanted: Vec<String> = words
                .iter()
                .find_map(|w| w.strip_prefix("=.proplist="))
                .map(|p| p.split(',').map(str::to_owned).collect())
                .unwrap_or_default();
            let mut out = Vec::new();
            for row in rows {
                let mut w = vec!["!re".to_string()];
                w.extend(tag.clone());
                w.extend(
                    row.iter()
                        .filter(|(k, _)| wanted.is_empty() || wanted.iter().any(|x| x == k))
                        .map(|(k, v)| format!("={k}={v}")),
                );
                out.extend(sentence(&w));
            }
            out.extend(done());
            out
        } else {
            // Not allowlisted: a write, a ping, or an unknown path.
            println!("REFUSED {command}");
            trap("not allowed by the smoke router")
        };
        if s.write_all(&out).await.is_err() {
            return;
        }
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(port), Some(password)) = (args.get(1), args.get(2)) else {
        eprintln!("usage: fake_routeros <port> <password>");
        std::process::exit(2);
    };
    let listener = TcpListener::bind(("127.0.0.1", port.parse::<u16>().expect("port"))).await?;
    println!("READY fake RouterOS on 127.0.0.1:{port} (synthetic test rows)");
    loop {
        let (s, _) = listener.accept().await?;
        tokio::spawn(serve(s, password.clone()));
    }
}
