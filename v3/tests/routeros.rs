use lqosync_v3::{model::Router, routeros::*};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn router(port: u16) -> Router {
    Router {
        id: "test".into(),
        name: "Test".into(),
        host: "127.0.0.1".into(),
        port,
        username: "test".into(),
        password_env: "TEST_PASSWORD".into(),
        role: "distro".into(),
        tls: false,
        tls_server_name: String::new(),
        tls_ca_file: String::new(),
        timeout_seconds: 2,
    }
}
async fn sentence(stream: &mut (impl tokio::io::AsyncRead + Unpin)) -> Vec<String> {
    let mut words = vec![];
    loop {
        let first = stream.read_u8().await.unwrap();
        let len = if first < 128 {
            usize::from(first)
        } else {
            (usize::from(first & 0x7f) << 8) | usize::from(stream.read_u8().await.unwrap())
        };
        if len == 0 {
            return words;
        }
        let mut b = vec![0; len];
        stream.read_exact(&mut b).await.unwrap();
        words.push(String::from_utf8(b).unwrap());
    }
}
async fn reply(stream: &mut (impl tokio::io::AsyncWrite + Unpin), words: &[String]) {
    let mut out = vec![];
    for word in words {
        assert!(word.len() < 128);
        out.push(word.len() as u8);
        out.extend(word.as_bytes());
    }
    out.push(0);
    stream.write_all(&out).await.unwrap();
}
#[test]
fn no_mutations_or_secret_properties_are_exposed() {
    assert!(read_properties("/interface/print").is_ok());
    for path in [
        "/ip/firewall/address-list/print",
        "/ip/firewall/mangle/print",
        "/routing/table/print",
        "/routing/rule/print",
        "/ip/firewall/filter/print",
    ] {
        assert!(read_properties(path).is_ok(), "Missing core read: {path}");
    }
    for path in [
        "/ip/firewall/address-list/add",
        "/interface/set",
        "/system/reboot",
        "/ppp/secret/export",
    ] {
        assert!(read_properties(path).is_err());
    }
    assert!(
        !read_properties("/ppp/secret/print")
            .unwrap()
            .split(',')
            .any(|p| p == "password")
    );
}
#[tokio::test]
async fn external_client_logs_in_and_reads_tagged_rows() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (mut s, _) = listener.accept().await.unwrap();
        let login = sentence(&mut s).await;
        assert_eq!(login[0], "/login");
        assert!(login.iter().any(|w| w == "=name=test"));
        let login_tag = login.iter().find(|w| w.starts_with(".tag=")).cloned();
        let mut done = vec!["!done".into()];
        if let Some(t) = login_tag {
            done.push(t);
        }
        reply(&mut s, &done).await;
        let cmd = sentence(&mut s).await;
        assert_eq!(cmd[0], "/system/identity/print");
        let tag = cmd.iter().find(|w| w.starts_with(".tag=")).unwrap().clone();
        assert!(cmd.iter().any(|w| w == "=.proplist=name"));
        reply(
            &mut s,
            &[
                "!re".into(),
                "=name=Mock router".into(),
                "=password=must-not-escape".into(),
                tag.clone(),
            ],
        )
        .await;
        reply(&mut s, &["!done".into(), tag]).await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    });
    let client = Client::connect_with_password(&router(port), "fake-password")
        .await
        .unwrap();
    let rows = client.read("/system/identity/print").await.unwrap();
    assert_eq!(rows[0]["name"], "Mock router");
    assert!(!rows[0].contains_key("password"));
    assert!(client.read("/system/reboot").await.is_err());
    server.await.unwrap();
}

#[tokio::test]
async fn full_mangle_reads_preserve_unknown_matchers_for_fail_closed_route_checks() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let login = sentence(&mut stream).await;
        let mut done = vec!["!done".into()];
        done.extend(login.into_iter().filter(|w| w.starts_with(".tag=")));
        reply(&mut stream, &done).await;
        let command = sentence(&mut stream).await;
        assert_eq!(command[0], "/ip/firewall/mangle/print");
        assert!(!command.iter().any(|w| w.starts_with("=.proplist=")));
        let tag = command
            .iter()
            .find(|w| w.starts_with(".tag="))
            .unwrap()
            .clone();
        reply(
            &mut stream,
            &[
                "!re".into(),
                "=chain=prerouting".into(),
                "=future-routing-matcher=needs-review".into(),
                tag.clone(),
            ],
        )
        .await;
        reply(&mut stream, &["!done".into(), tag]).await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    });
    let client = Client::connect_with_password(&router(port), "fake-password")
        .await
        .unwrap();
    let rows = client.read("/ip/firewall/mangle/print").await.unwrap();
    assert_eq!(rows[0]["future-routing-matcher"], "needs-review");
    server.await.unwrap();
}

#[tokio::test]
async fn incomplete_and_rejected_reads_never_become_successful_tables() {
    for mode in ["trap", "close", "timeout"] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let login = sentence(&mut s).await;
            let mut done = vec!["!done".into()];
            done.extend(login.into_iter().filter(|w| w.starts_with(".tag=")));
            reply(&mut s, &done).await;
            let cmd = sentence(&mut s).await;
            let tag = cmd.iter().find(|w| w.starts_with(".tag=")).unwrap().clone();
            reply(&mut s, &["!re".into(), "=name=Partial".into(), tag.clone()]).await;
            if mode == "trap" {
                reply(&mut s, &["!trap".into(), "=message=Denied".into(), tag]).await;
            }
            if mode != "close" {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        });
        let mut config = router(port);
        config.timeout_seconds = 1;
        let client = Client::connect_with_password(&config, "fake")
            .await
            .unwrap();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.read("/system/identity/print"),
        )
        .await
        .unwrap();
        assert!(result.is_err(), "{mode}");
        server.abort();
    }
}
#[tokio::test]
async fn tls_rejects_empty_ca_before_connecting() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ca.pem");
    std::fs::write(&path, "not a certificate").unwrap();
    let mut config = router(1);
    config.tls = true;
    config.tls_ca_file = path.to_str().unwrap().into();
    assert!(
        Client::connect_with_password(&config, "fake")
            .await
            .err()
            .unwrap()
            .to_string()
            .contains("no certificates")
    );
}

#[tokio::test]
async fn burst_response_preserves_every_row_and_completion() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (mut s, _) = listener.accept().await.unwrap();
        let login = sentence(&mut s).await;
        let mut done = vec!["!done".into()];
        done.extend(login.into_iter().filter(|w| w.starts_with(".tag=")));
        reply(&mut s, &done).await;
        let cmd = sentence(&mut s).await;
        let tag = cmd.iter().find(|w| w.starts_with(".tag=")).unwrap().clone();
        let mut burst = vec![];
        for i in 0..200 {
            for word in ["!re".to_string(), format!("=name=row-{i}"), tag.clone()] {
                burst.push(word.len() as u8);
                burst.extend(word.as_bytes());
            }
            burst.push(0);
        }
        for word in ["!done".to_string(), tag] {
            burst.push(word.len() as u8);
            burst.extend(word.as_bytes());
        }
        burst.push(0);
        s.write_all(&burst).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    });
    let client = Client::connect_with_password(&router(port), "fake")
        .await
        .unwrap();
    let rows = client.read("/system/identity/print").await.unwrap();
    assert_eq!(rows.len(), 200);
    assert_eq!(rows[199]["name"], "row-199");
    server.await.unwrap();
}

#[tokio::test]
async fn tls_checks_trust_and_certificate_name_before_login() {
    use std::{io::Cursor, sync::Arc};
    let certs = rustls_pemfile::certs(&mut Cursor::new(include_bytes!("fixtures/router-test.pem")))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = rustls_pemfile::private_key(&mut Cursor::new(include_bytes!(
        "fixtures/router-test-key.pem"
    )))
    .unwrap()
    .unwrap();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(certs, key)
    .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    for (trusted, name, expected) in [
        (true, "router.test", true),
        (true, "wrong.test", false),
        (false, "router.test", false),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let acceptor = acceptor.clone();
        let server = tokio::spawn(async move {
            let (s, _) = listener.accept().await.unwrap();
            if let Ok(mut tls) = acceptor.accept(s).await {
                let login = sentence(&mut tls).await;
                let mut done = vec!["!done".into()];
                done.extend(login.into_iter().filter(|w| w.starts_with(".tag=")));
                reply(&mut tls, &done).await;
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        });
        let mut config = router(port);
        config.tls = true;
        config.tls_server_name = name.into();
        if trusted {
            config.tls_ca_file = format!(
                "{}/tests/fixtures/router-ca.pem",
                env!("CARGO_MANIFEST_DIR")
            );
        }
        let connected = Client::connect_with_password(&config, "fake").await;
        assert_eq!(connected.is_ok(), expected, "trusted={trusted},name={name}");
        server.await.unwrap();
    }
}
