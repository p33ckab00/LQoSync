//! The real router session on `mikrotik-rs` 0.9 (`mikrotik-tokio` 0.3). It only sends `print` commands.

use crate::config::{RouterAccess, RouterCfg};
use crate::model::RosRow;
use crate::routeros::{Connector, ReadError, RouterSession};
use crate::secrets::read_secret;
use mikrotik_rs::DeviceError;
use mikrotik_rs::{CommandBuilder, Event, MikrotikDevice};
use rustls::pki_types::ServerName;
use std::path::Path;
use std::sync::Arc;

/// Opens API sessions. `owner_uid` is the required owner of the password files (`0` = root).
pub struct MikrotikConnector {
    pub owner_uid: u32,
}

pub struct MikrotikSession {
    device: MikrotikDevice,
}

/// A certificate verifier that accepts any certificate (`tls_verify = false`).
#[derive(Debug)]
struct AnyCertificate(Arc<rustls::crypto::CryptoProvider>);

impl rustls::client::danger::ServerCertVerifier for AnyCertificate {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// The rustls client config for a router: its CA file, or the public CAs.
pub fn tls_client_config(router: &RouterCfg) -> Result<Arc<rustls::ClientConfig>, ReadError> {
    let mut roots = rustls::RootCertStore::empty();
    if router.tls_ca_file.is_empty() {
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    } else {
        let pem = std::fs::read(&router.tls_ca_file)
            .map_err(|e| ReadError::Tls(format!("tls_ca_file {}: {e}", router.tls_ca_file)))?;
        for cert in rustls_pemfile::certs(&mut pem.as_slice()) {
            let cert = cert
                .map_err(|e| ReadError::Tls(format!("tls_ca_file {}: {e}", router.tls_ca_file)))?;
            roots
                .add(cert)
                .map_err(|e| ReadError::Tls(format!("tls_ca_file {}: {e}", router.tls_ca_file)))?;
        }
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|e| ReadError::Tls(e.to_string()))?;
    let cfg = if router.tls_verify {
        builder.with_root_certificates(roots).with_no_client_auth()
    } else {
        // tls_verify = false: encrypt, but accept any certificate. The handshake signatures are still checked.
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AnyCertificate(provider)))
            .with_no_client_auth()
    };
    Ok(Arc::new(cfg))
}

/// Map a `mikrotik-rs` error to a `ReadError`. A TLS error inside an I/O error is a TLS error.
fn map_err(e: DeviceError) -> ReadError {
    match e {
        DeviceError::Connection(io) => {
            let tls = io
                .get_ref()
                .is_some_and(|inner| inner.downcast_ref::<rustls::Error>().is_some());
            if tls {
                ReadError::Tls(io.to_string())
            } else {
                use std::io::ErrorKind::*;
                match io.kind() {
                    ConnectionRefused | TimedOut | HostUnreachable | NetworkUnreachable => {
                        ReadError::Unreachable(io.to_string())
                    }
                    // The TCP connect passed: a reset or an early close is a protocol problem.
                    _ => ReadError::Protocol(io.to_string()),
                }
            }
        }
        DeviceError::Authentication { response } => ReadError::Login(response.message),
        DeviceError::Login(e) => ReadError::Login(e.to_string()),
        DeviceError::Tls(e) => ReadError::Tls(e.to_string()),
        other => ReadError::Protocol(other.to_string()),
    }
}

impl Connector for MikrotikConnector {
    type Session = MikrotikSession;

    fn connect(
        &self,
        router: &RouterCfg,
        access: &RouterAccess,
    ) -> impl Future<Output = Result<MikrotikSession, ReadError>> + Send {
        let router = router.clone();
        let connect_timeout = std::time::Duration::from_secs(access.connect_timeout);
        let owner_uid = self.owner_uid;
        async move {
            let password = read_secret(Path::new(&router.password_file), owner_uid)
                .map_err(|e| ReadError::Login(format!("{e:#}")))?;
            let addr = (router.address.clone(), router.port);
            // Only a failed TCP connect means "unreachable". A hung login or TLS handshake later is a failure.
            match tokio::time::timeout(
                connect_timeout,
                tokio::net::TcpStream::connect(addr.clone()),
            )
            .await
            {
                Ok(Ok(_probe)) => {}
                Ok(Err(e)) => {
                    return Err(ReadError::Unreachable(format!(
                        "{}:{}: {e}",
                        router.address, router.port
                    )));
                }
                Err(_) => {
                    return Err(ReadError::Unreachable(format!(
                        "{}:{}: TCP connect took more than {} s",
                        router.address,
                        router.port,
                        connect_timeout.as_secs()
                    )));
                }
            }
            let device = if !router.tls {
                MikrotikDevice::connect(addr, &router.username, Some(&password)).await
            } else {
                if !router.tls_verify {
                    eprintln!(
                        "warning: router {}: tls_verify = false, the certificate is not checked",
                        router.name
                    );
                }
                let cfg = tls_client_config(&router)?;
                let name = ServerName::try_from(router.address.clone())
                    .map_err(|e| ReadError::Tls(format!("address {}: {e}", router.address)))?;
                MikrotikDevice::builder(addr)
                    .credentials(&router.username, Some(&password))
                    .tls_config(cfg, name)
                    .connect()
                    .await
            };
            device
                .map(|device| MikrotikSession { device })
                .map_err(map_err)
        }
    }
}

impl RouterSession for MikrotikSession {
    fn print(
        &mut self,
        path: &str,
        proplist: &str,
    ) -> impl Future<Output = Result<Vec<RosRow>, ReadError>> + Send {
        let device = self.device.clone();
        let command = CommandBuilder::new()
            .command(&format!("{path}/print"))
            .attribute(".proplist", Some(proplist))
            .build();
        async move {
            let mut rx = device.send_command(command).await.map_err(map_err)?;
            let mut rows = Vec::new();
            while let Some(event) = rx.recv().await {
                match event {
                    Event::Reply { response, .. } => {
                        let raw = response.attributes_raw;
                        rows.push(
                            response
                                .attributes
                                .into_iter()
                                .map(|(k, v)| {
                                    let v = v.unwrap_or_else(|| {
                                        // shortcut: a value that is not UTF-8 (Winbox writes Windows text) is read as
                                        // Latin-1; CP1252 differs only in 0x80-0x9F. Use a codepage crate if that matters.
                                        raw.get(&k)
                                            .and_then(|b| b.as_ref())
                                            .map_or(String::new(), |b| {
                                                b.iter().map(|&c| c as char).collect()
                                            })
                                    });
                                    (k, v)
                                })
                                .collect(),
                        )
                    }
                    Event::Done { .. } | Event::Empty { .. } => return Ok(rows),
                    Event::Trap { response, .. } => return Err(ReadError::Trap(response.message)),
                    Event::Fatal { reason } => {
                        return Err(ReadError::Protocol(format!("{reason:?}")));
                    }
                }
            }
            Err(ReadError::Protocol(
                "the connection closed before the command completed".into(),
            ))
        }
    }
}
