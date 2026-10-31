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
    let cfg = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| ReadError::Tls(e.to_string()))?
        .with_root_certificates(roots)
        .with_no_client_auth();
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
                ReadError::Unreachable(io.to_string())
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
        _access: &RouterAccess,
    ) -> impl Future<Output = Result<MikrotikSession, ReadError>> + Send {
        let router = router.clone();
        let owner_uid = self.owner_uid;
        async move {
            let password = read_secret(Path::new(&router.password_file), owner_uid)
                .map_err(|e| ReadError::Login(format!("{e:#}")))?;
            let addr = (router.address.clone(), router.port);
            let device = if !router.tls {
                MikrotikDevice::connect(addr, &router.username, Some(&password)).await
            } else if !router.tls_verify {
                eprintln!(
                    "warning: router {}: tls_verify = false, the certificate is not checked",
                    router.name
                );
                MikrotikDevice::builder(addr)
                    .credentials(&router.username, Some(&password))
                    .tls_insecure()
                    .connect()
                    .await
            } else {
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
                    Event::Reply { response, .. } => rows.push(
                        response
                            .attributes
                            .into_iter()
                            .map(|(k, v)| (k, v.unwrap_or_default()))
                            .collect(),
                    ),
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
