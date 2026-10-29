use crate::model::Router;
use anyhow::{Context, Result, bail, ensure};
use mikrotik_rs::{CommandBuilder, Event, MikrotikDevice};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc, time::Duration};

pub type Row = BTreeMap<String, String>;
pub fn read_properties(path: &str) -> Result<&'static str> {
    Ok(match path {
        "/system/identity/print" => "name",
        "/system/resource/print" => "version,uptime,board-name,cpu-load,free-memory,total-memory",
        "/interface/print" => ".id,name,type,running,disabled,actual-mtu,mac-address,comment",
        "/interface/list/member/print" => "list,interface,disabled,dynamic",
        "/ip/route/print"
        | "/ip/firewall/mangle/print"
        | "/ip/firewall/filter/print"
        | "/routing/rule/print" => "*",
        "/routing/table/print" => "name,fib,disabled,dynamic,invalid",
        "/ip/firewall/address-list/print" => ".id,list,address,comment,dynamic,disabled,timeout",
        "/ip/neighbor/print" => "identity,interface,interface-name,address,mac-address",
        "/ppp/profile/print" => "name,rate-limit,comment",
        "/ppp/active/print" => "name,address,caller-id,service,uptime,comment",
        "/ppp/secret/print" => "name,profile,disabled,comment,caller-id",
        "/ip/dhcp-server/print" => "name,interface,address-pool,disabled",
        "/ip/dhcp-server/lease/print" => {
            "address,active-address,mac-address,active-mac-address,host-name,server,active-server,status,disabled,comment,rate-limit"
        }
        "/ip/hotspot/active/print" => "user,address,mac-address,server",
        "/ip/hotspot/user/profile/print" => "name,rate-limit,comment",
        "/ip/hotspot/user/print" => "name,profile,disabled,comment",
        _ => bail!("RouterOS command is outside the read-only allowlist"),
    })
}
pub struct Client {
    device: MikrotikDevice,
    deadline: Duration,
}
impl Client {
    pub async fn connect_with_password(router: &Router, password: &str) -> Result<Self> {
        ensure!(
            (1..=30).contains(&router.timeout_seconds),
            "Invalid router timeout"
        );
        let deadline = Duration::from_secs(router.timeout_seconds);
        let host = router.host.trim();
        let address = if host.contains(':') && !host.starts_with('[') {
            format!("[{host}]:{}", router.port)
        } else {
            format!("{host}:{}", router.port)
        };
        let connect = async {
            let builder =
                MikrotikDevice::builder(address).credentials(&router.username, Some(password));
            if router.tls {
                let mut roots = rustls::RootCertStore::empty();
                roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
                if !router.tls_ca_file.is_empty() {
                    let pem = tokio::fs::read(&router.tls_ca_file)
                        .await
                        .context("Unable to read configured CA certificate")?;
                    let certs = rustls_pemfile::certs(&mut std::io::Cursor::new(pem))
                        .collect::<std::result::Result<Vec<_>, _>>()?;
                    ensure!(!certs.is_empty(), "CA file contains no certificates");
                    for cert in certs {
                        roots.add(cert)?;
                    }
                }
                let config = rustls::ClientConfig::builder_with_provider(Arc::new(
                    rustls::crypto::ring::default_provider(),
                ))
                .with_safe_default_protocol_versions()?
                .with_root_certificates(roots)
                .with_no_client_auth();
                let server_name = if router.tls_server_name.is_empty() {
                    host.trim_matches(['[', ']']).to_string()
                } else {
                    router.tls_server_name.clone()
                };
                let name = rustls::pki_types::ServerName::try_from(server_name)
                    .context("Invalid TLS server name")?;
                builder.tls_config(Arc::new(config),name).connect().await.map_err(|_|anyhow::anyhow!("Router connection/login failed; check reachability, credentials and TLS certificate"))
            } else {
                builder.connect().await.map_err(|_| {
                    anyhow::anyhow!(
                        "Router connection/login failed; check reachability and credentials"
                    )
                })
            }
        };
        let device = tokio::time::timeout(deadline, connect)
            .await
            .context("Router connection/login timed out")??;
        Ok(Self { device, deadline })
    }
    pub async fn read(&self, path: &str) -> Result<Vec<Row>> {
        let properties = read_properties(path)?;
        let query = async {
            // Full match fields prevent an unseen condition from passing route validation.
            let builder = CommandBuilder::new().command(path);
            let command = if properties == "*" {
                builder.build()
            } else {
                builder.attribute(".proplist", Some(properties)).build()
            };
            let mut receiver = self
                .device
                .send_command(command)
                .await
                .context("Router command could not be sent")?;
            let mut rows = vec![];
            while let Some(event) = receiver.recv().await {
                match event {
                    Event::Reply { response, .. } => {
                        ensure!(rows.len() < 100_000, "Router response exceeds row limit");
                        let row = response
                            .attributes
                            .into_iter()
                            .filter_map(|(k, v)| {
                                (properties == "*" || properties.split(',').any(|p| p == k))
                                    .then_some((k, v.unwrap_or_default()))
                            })
                            .collect();
                        rows.push(row);
                    }
                    Event::Done { .. } | Event::Empty { .. } => return Ok(rows),
                    Event::Trap { .. } => bail!(
                        "Router rejected read request for {path}; no complete table was returned"
                    ),
                    Event::Fatal { .. } => bail!("Router connection closed during read"),
                }
            }
            bail!("Router response ended before completion")
        };
        tokio::time::timeout(self.deadline, query)
            .await
            .context("Router read timed out")?
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub router_id: String,
    pub collected_at: u64,
    pub tables: BTreeMap<String, Vec<Row>>,
    pub errors: BTreeMap<String, String>,
}
impl Inventory {
    pub fn complete(&self) -> bool {
        self.errors.is_empty()
    }
}

pub async fn inspect(router: &Router) -> Result<Inventory> {
    let password = std::env::var(&router.password_env)
        .map_err(|_| anyhow::anyhow!("Configured password environment variable is missing"))?;
    inspect_with_password(router, password).await
}
pub async fn inspect_from(router: &Router, root: &std::path::Path) -> Result<Inventory> {
    let password = crate::credentials::password(root, router).await?;
    inspect_with_password(router, password).await
}
async fn inspect_with_password(router: &Router, password: String) -> Result<Inventory> {
    let client = Client::connect_with_password(router, &password).await?;
    drop(password);
    let mut paths = vec![
        "/system/identity/print",
        "/system/resource/print",
        "/interface/print",
        "/interface/list/member/print",
        "/ip/route/print",
        "/ip/neighbor/print",
    ];
    if router.role != "core" {
        paths.extend([
            "/ppp/profile/print",
            "/ppp/active/print",
            "/ppp/secret/print",
            "/ip/dhcp-server/print",
            "/ip/dhcp-server/lease/print",
            "/ip/hotspot/active/print",
            "/ip/hotspot/user/profile/print",
            "/ip/hotspot/user/print",
        ]);
    }
    if router.role == "core" || router.role == "both" {
        paths.extend(crate::core_plan::EXTRA_TABLES.iter().copied());
    }
    let mut inventory = Inventory {
        router_id: router.id.clone(),
        collected_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
        tables: BTreeMap::new(),
        errors: BTreeMap::new(),
    };
    let collect = async {
        for path in paths {
            match client.read(path).await {
                Ok(rows) => {
                    inventory.tables.insert(path.into(), rows);
                }
                Err(e) => {
                    inventory.errors.insert(path.into(), e.to_string());
                }
            }
        }
    };
    if tokio::time::timeout(Duration::from_secs(45), collect)
        .await
        .is_err()
    {
        inventory.errors.insert(
            "collection".into(),
            "Overall inventory deadline exceeded".into(),
        );
    }
    Ok(inventory)
}
