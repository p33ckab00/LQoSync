use crate::model::Router;
use anyhow::{Context, Result, bail, ensure};
use mikrotik_rs::{CommandBuilder, Event, MikrotikDevice};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc, time::Duration};

pub type Row = BTreeMap<String, String>;
pub fn read_properties(path: &str) -> Result<&'static str> {
    Ok(match path {
        "/tool/netwatch/print" => {
            ".id,host,type,disabled,status,done-tests,interval,timeout,src-address,up-script,down-script,test-script"
        }
        "/system/identity/print" => "name",
        "/system/resource/print" => "version,uptime,board-name,cpu-load,free-memory,total-memory",
        "/interface/print" => ".id,name,type,running,disabled,actual-mtu,mac-address,comment",
        "/interface/list/member/print" => "list,interface,disabled,dynamic",
        "/interface/list/print" => "name,include,exclude,dynamic,builtin",
        "/ip/route/print"
        | "/ip/firewall/mangle/print"
        | "/ip/firewall/filter/print"
        | "/routing/rule/print" => "*",
        "/routing/table/print" => "name,fib,disabled,dynamic,invalid",
        "/ip/firewall/connection/print" => {
            "src-address,dst-address,reply-src-address,reply-dst-address,protocol,connection-mark"
        }
        "/ip/firewall/address-list/print" => ".id,list,address,comment,dynamic,disabled,timeout",
        "/ip/neighbor/print" => "identity,interface,interface-name,address,mac-address",
        "/ppp/profile/print" => "name,rate-limit,comment",
        "/ppp/active/print" => ".id,name,address,caller-id,service,uptime,comment",
        "/ppp/secret/print" => "name,profile,disabled,comment,caller-id",
        "/ip/dhcp-server/print" => "name,interface,address-pool,disabled,comment",
        "/ip/dhcp-server/lease/print" => {
            "address,active-address,mac-address,active-mac-address,host-name,server,active-server,status,disabled,comment,rate-limit"
        }
        "/ip/hotspot/active/print" => ".id,user,address,mac-address,server,uptime",
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
    pub(crate) async fn migration_command(
        &self,
        action: &str,
        fields: &[(&str, &str)],
    ) -> Result<Option<String>> {
        let keys = fields
            .iter()
            .map(|(k, _)| *k)
            .collect::<std::collections::BTreeSet<_>>();
        let expected = match action {
            "add" => vec!["list", "address", "comment", "disabled"],
            "set" => vec![".id", "address"],
            "remove" => vec![".id"],
            _ => bail!("Unsupported migration command"),
        };
        ensure!(
            keys == expected.into_iter().collect() && keys.len() == fields.len(),
            "Unsupported migration fields"
        );
        self.address_list_command(action, fields).await
    }
    pub(crate) async fn move_owned_host(&self, id: &str, list: &str, comment: &str) -> Result<()> {
        self.address_list_command("set", &[(".id", id), ("list", list), ("comment", comment)])
            .await?;
        Ok(())
    }
    async fn address_list_command(
        &self,
        action: &str,
        fields: &[(&str, &str)],
    ) -> Result<Option<String>> {
        let query = async {
            let path = format!("/ip/firewall/address-list/{action}");
            let mut builder = CommandBuilder::new().command(&path);
            for (key, value) in fields {
                builder = builder.attribute(key, Some(*value));
            }
            let mut receiver = self.device.send_command(builder.build()).await?;
            if let Some(event) = receiver.recv().await {
                match event {
                    Event::Done { attributes, .. } => {
                        if action == "add" {
                            ensure!(
                                attributes.len() == 1,
                                "Unexpected add completion attributes"
                            );
                            let id = attributes
                                .get("ret")
                                .and_then(|v| v.clone())
                                .context("Missing returned add ID")?;
                            ensure!(
                                id.strip_prefix('*').is_some_and(|s| !s.is_empty()
                                    && s.len() <= 32
                                    && s.bytes().all(|b| b.is_ascii_hexdigit())),
                                "Invalid returned row ID"
                            );
                            return Ok(Some(id));
                        }
                        ensure!(attributes.is_empty(), "Unexpected completion attributes");
                        return Ok(None);
                    }
                    _ => bail!("Uncertain RouterOS migration result; inspect the pending journal"),
                }
            }
            bail!("Router migration response ended without completion")
        };
        tokio::time::timeout(self.deadline, query)
            .await
            .context("Router migration timed out; outcome is unknown")?
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
/// Configured console preview only; ordinary inventory reads retain their original table set.
pub async fn inspect_health_from(router: &Router, root: &std::path::Path) -> Result<Inventory> {
    let password = crate::credentials::password(root, router).await?;
    inspect_with_health(router, password, true).await
}
async fn inspect_with_password(router: &Router, password: String) -> Result<Inventory> {
    inspect_with_health(router, password, false).await
}
async fn inspect_with_health(router: &Router, password: String, health: bool) -> Result<Inventory> {
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
        if health {
            paths.push(crate::wan_health::NETWATCH);
        }
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
