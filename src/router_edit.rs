//! Local router registration and edits for the web (`lqosync3 router-edit`). No router I/O.
//! It uses the TUI Settings path: the draft, the validation, and the apply with its backup and
//! secret rollback. Then it signals a reload and appends one audit line without secret values.
use crate::cli::config_digest;
use crate::config::Config;
use crate::topology_manager::policy_draft::router_id;
use crate::tui::settings::apply::{ApplyError, apply};
use crate::tui::settings::draft::{Draft, Seg, valid_router_name};
use crate::tui::settings::secrets::{Pending, Secret};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use toml_edit::Value;
use web_contract::routers::{
    MAX_CA_PEM, MAX_CLIENTS, MAX_LIST, MAX_PASSWORD, RouterEditOutcome, RouterEditReply,
    RouterEditRequest, RouterEvent,
};

pub const AUDIT: &str = "router-audit.jsonl";
const AUDIT_MAX: u64 = 1024 * 1024;

type Fail = (&'static str, String);

fn invalid(m: impl Into<String>) -> Fail {
    ("invalid", m.into())
}

/// A host name or an IP address: no spaces, no control characters, no URL syntax.
fn valid_address(a: &str) -> bool {
    (1..=253).contains(&a.len())
        && a.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':'))
}
fn valid_user(u: &str) -> bool {
    (1..=64).contains(&u.len()) && !u.chars().any(|c| c.is_control() || c.is_whitespace())
}

fn key(i: usize, parts: &[&str]) -> Vec<Seg> {
    let mut p = vec![Seg::Key("routers".into()), Seg::Index(i)];
    p.extend(parts.iter().map(|k| Seg::Key((*k).into())));
    p
}

/// The audit log is append-only. At 1 MiB the file moves to `.1`, so it stays bounded.
pub(crate) fn audit(state_dir: &Path, event: &RouterEvent) -> anyhow::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let path = state_dir.join(AUDIT);
    if std::fs::metadata(&path).is_ok_and(|m| m.len() >= AUDIT_MAX) {
        std::fs::rename(&path, state_dir.join(format!("{AUDIT}.1")))?;
    }
    let mut line = serde_json::to_vec(event)?;
    line.push(b'\n');
    std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .mode(0o600)
        .open(&path)?
        .write_all(&line)?;
    Ok(())
}

/// Send SIGHUP to the service, as the TUI does after an apply. The answer is `sent` or the reason.
pub fn signal_reload(cfg: &Config) -> String {
    signal_service(cfg, libc::SIGHUP)
}

/// `lqosync3 sync-now`: SIGUSR1 starts a cycle now for all routers, as the TUI Sync now does.
/// The service reads only the enabled routers. The answer is JSON: `ok` with `sent`, or the reason.
pub fn sync_now(config_path: &Path) -> RouterEditOutcome {
    let out = |code: &str, message: String| RouterEditOutcome {
        code: code.into(),
        message,
        reply: None,
    };
    match crate::cli::read_config(config_path) {
        Err(e) => out("internal", format!("{e:#}")),
        Ok((cfg, _)) => match signal_service(&cfg, libc::SIGUSR1) {
            m if m == "sent" => out("ok", m),
            m => out("internal", m),
        },
    }
}

fn signal_service(cfg: &Config, signal: libc::c_int) -> String {
    let Some(unit) =
        crate::tui::data::systemctl_show("systemctl", &cfg.tui.service_unit, &["MainPID"])
    else {
        return "cannot read the service process from systemd".into();
    };
    let target = crate::tui::dashboard::signal_target(unit.main_pid, std::process::id(), |p| {
        std::fs::read_to_string(format!("/proc/{p}/comm")).ok()
    });
    match target {
        // SAFETY: kill has no memory preconditions; signal_target checked the PID.
        Ok(pid) if unsafe { libc::kill(pid as libc::pid_t, signal) } == 0 => "sent".into(),
        Ok(pid) => format!("cannot signal PID {pid}"),
        Err(e) => e,
    }
}

/// Set one router key and record its change label.
fn put(
    draft: &mut Draft,
    changes: &mut Vec<String>,
    i: usize,
    parts: &[&str],
    v: Value,
    label: &str,
) -> Result<(), Fail> {
    draft
        .set(&key(i, parts), v)
        .map_err(|err| ("internal", format!("{err:#}")))?;
    changes.push(label.to_string());
    Ok(())
}

pub fn edit(
    config_path: &Path,
    req: &RouterEditRequest,
    now: u64,
    owner_uid: u32,
    reload: &dyn Fn(&Config) -> String,
) -> Result<RouterEditReply, Fail> {
    let internal = |m: String| ("internal", m);
    let e = &req.edit;
    if !valid_user(&req.actor) {
        return Err(invalid("The request has no valid account."));
    }
    let text = std::fs::read_to_string(config_path)
        .map_err(|err| internal(format!("cannot read the config: {err}")))?;
    let digest = config_digest(text.as_bytes());
    if e.expected_digest != digest {
        return Err((
            "conflict",
            "The configuration changed after this page loaded. Load the page again, then repeat the change."
                .into(),
        ));
    }
    let cfg = Config::from_toml_str(&text).map_err(|err| internal(format!("{err:#}")))?;
    let dir = config_path.parent().unwrap_or(Path::new("/"));
    let mut draft = Draft::new(&text).map_err(|err| internal(format!("{err:#}")))?;
    let mut changes: Vec<String> = Vec::new();
    let mut pending: Vec<Pending> = Vec::new();
    let (i, name, action, password_file) = match &req.router_id {
        None => {
            let name = e.name.clone().unwrap_or_default();
            if !valid_router_name(&name) {
                return Err(invalid(
                    "Use 1 to 64 characters for the name: letters, digits, dot, underscore, or hyphen.",
                ));
            }
            if cfg.routers.iter().any(|r| r.name == name) {
                return Err(invalid(format!("A router named {name} exists already.")));
            }
            let (Some(address), Some(username)) = (&e.address, &e.username) else {
                return Err(invalid("A new router needs an address and a username."));
            };
            if e.password.as_deref().is_none_or(str::is_empty) {
                return Err(invalid("A new router needs a credential."));
            }
            if e.enabled.is_some() {
                return Err(invalid(
                    "A new router starts disabled. Enable it after the save.",
                ));
            }
            if !valid_address(address) || !valid_user(username) {
                return Err(invalid("Check the address and the username."));
            }
            let file = dir.join("secrets").join(format!("{name}.password"));
            let i = draft
                .add_router(&name, address, username, &file.to_string_lossy())
                .map_err(|err| invalid(format!("{err:#}")))?;
            // A new router is disabled until an explicit enable. Creation makes no router I/O.
            put(
                &mut draft,
                &mut changes,
                i,
                &["enabled"],
                Value::from(false),
                "created",
            )?;
            (i, name, "create", file)
        }
        Some(id) => {
            let Some(i) = cfg.routers.iter().position(|r| router_id(&r.name) == *id) else {
                return Err((
                    "not_found",
                    "This router is not in the configuration.".into(),
                ));
            };
            let r = &cfg.routers[i];
            if e.name.as_ref().is_some_and(|n| *n != r.name) {
                return Err(invalid(
                    "A router name cannot change: it is the router identity.",
                ));
            }
            if let Some(a) = &e.address {
                if !valid_address(a) {
                    return Err(invalid("Check the address."));
                }
                put(
                    &mut draft,
                    &mut changes,
                    i,
                    &["address"],
                    Value::from(a.as_str()),
                    "address",
                )?;
            }
            if let Some(u) = &e.username {
                if !valid_user(u) {
                    return Err(invalid("Check the username."));
                }
                put(
                    &mut draft,
                    &mut changes,
                    i,
                    &["username"],
                    Value::from(u.as_str()),
                    "username",
                )?;
            }
            let file = if r.password_file.is_empty() {
                dir.join("secrets").join(format!("{}.password", r.name))
            } else {
                PathBuf::from(&r.password_file)
            };
            (i, r.name.clone(), "update", file)
        }
    };
    // Enable removes the key, so an enabled router has no `enabled` line (older binaries load the file).
    let mut action = action;
    match (e.enabled, action) {
        (Some(false), "update") => {
            put(
                &mut draft,
                &mut changes,
                i,
                &["enabled"],
                Value::from(false),
                "disabled",
            )?;
            action = "disable";
        }
        (Some(true), "update") => {
            draft
                .reset(&key(i, &["enabled"]))
                .map_err(|err| internal(format!("{err:#}")))?;
            changes.push("enabled".into());
            action = "enable";
        }
        _ => {}
    }
    if let Some(p) = e.port {
        if p == 0 {
            return Err(invalid("Use an API port from 1 to 65535."));
        }
        put(
            &mut draft,
            &mut changes,
            i,
            &["port"],
            Value::from(i64::from(p)),
            "port",
        )?;
    }
    for (value, parts, label) in [
        (e.tls, &["tls"][..], "tls"),
        (e.tls_verify, &["tls_verify"][..], "tls_verify"),
        (e.pppoe, &["pppoe", "enabled"][..], "pppoe"),
        (e.dhcp, &["dhcp", "enabled"][..], "dhcp"),
        (e.hotspot, &["hotspot", "enabled"][..], "hotspot"),
        (
            e.ppp_per_plan_node,
            &["pppoe", "per_plan_node"][..],
            "ppp_per_plan_node",
        ),
        (
            e.hotspot_include_mac,
            &["hotspot", "include_mac"][..],
            "hotspot_include_mac",
        ),
        (
            e.dhcp_require_bound,
            &["dhcp", "require_bound"][..],
            "dhcp_require_bound",
        ),
        (e.dhcp_discover, &["dhcp", "discover"][..], "dhcp_discover"),
        (
            e.dhcp_shape_unassigned,
            &["dhcp", "shape_unassigned"][..],
            "dhcp_shape_unassigned",
        ),
    ] {
        if let Some(v) = value {
            put(&mut draft, &mut changes, i, parts, Value::from(v), label)?;
        }
    }
    if let Some([lat, lon]) = e.location {
        if !((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)) {
            return Err(invalid(
                "Use a latitude from -90 to 90 and a longitude from -180 to 180.",
            ));
        }
        put(
            &mut draft,
            &mut changes,
            i,
            &["latitude"],
            Value::from(lat),
            "location",
        )?;
        draft
            .set(&key(i, &["longitude"]), Value::from(lon))
            .map_err(|err| internal(format!("{err:#}")))?;
    } else if e.clear_location {
        for k in ["latitude", "longitude"] {
            draft
                .reset(&key(i, &[k]))
                .map_err(|err| internal(format!("{err:#}")))?;
        }
        changes.push("location".into());
    }
    if let Some(p) = &e.password {
        if p.is_empty() || p.chars().count() > MAX_PASSWORD || p.chars().any(char::is_control) {
            return Err(invalid(
                "Use a credential of 1 to 256 characters, without line breaks.",
            ));
        }
        if action == "update" && cfg.routers[i].password_file.is_empty() {
            put(
                &mut draft,
                &mut changes,
                i,
                &["password_file"],
                Value::from(password_file.to_string_lossy().as_ref()),
                "password_file",
            )?;
        }
        pending.push(Pending {
            path: password_file.clone(),
            value: Secret::new(p.clone()),
        });
        changes.push("credential".into());
    }
    if let Some(pem) = &e.ca_pem {
        // The same parse as the TLS client of the service: each certificate must enter a root store.
        let mut roots = rustls::RootCertStore::empty();
        let certs = rustls_pemfile::certs(&mut pem.as_bytes()).collect::<Result<Vec<_>, _>>();
        let usable =
            certs.is_ok_and(|c| !c.is_empty() && c.into_iter().all(|c| roots.add(c).is_ok()));
        if pem.len() > MAX_CA_PEM || !usable {
            return Err(invalid(
                "The CA certificate must be PEM text with at least one certificate, up to 16 KiB.",
            ));
        }
        // The managed store never replaces a CA file that an operator placed somewhere else.
        let ca_dir = dir.join("ca");
        std::os::unix::fs::DirBuilderExt::mode(std::fs::DirBuilder::new().recursive(true), 0o700)
            .create(&ca_dir)
            .map_err(|err| internal(format!("cannot make {}: {err}", ca_dir.display())))?;
        let path = ca_dir.join(format!("{name}.pem"));
        put(
            &mut draft,
            &mut changes,
            i,
            &["tls_ca_file"],
            Value::from(path.to_string_lossy().as_ref()),
            "ca_certificate",
        )?;
        pending.push(Pending {
            path,
            value: Secret::new(pem.clone()),
        });
    } else if e.clear_ca {
        put(
            &mut draft,
            &mut changes,
            i,
            &["tls_ca_file"],
            Value::from(""),
            "ca_certificate",
        )?;
    }
    // Lists replace the whole list. The config rules check the IP networks and the patterns.
    for (list, field, label) in [
        (
            &e.exclude_dhcp_servers,
            "dhcp_servers",
            "exclude_dhcp_servers",
        ),
        (
            &e.exclude_ppp_profiles,
            "ppp_profiles",
            "exclude_ppp_profiles",
        ),
        (&e.exclude_ips, "ips", "exclude_ips"),
    ] {
        let Some(list) = list else { continue };
        if list.len() > MAX_LIST
            || list
                .iter()
                .any(|x| x.trim().is_empty() || x.len() > 128 || x.chars().any(char::is_control))
        {
            return Err(invalid(format!(
                "Use at most {MAX_LIST} entries of 1 to 128 characters, one for each line."
            )));
        }
        let path = key(i, &["exclude", field]);
        if list.is_empty() {
            draft
                .reset(&path)
                .map_err(|err| internal(format!("{err:#}")))?;
            changes.push(label.into());
        } else {
            let mut a = toml_edit::Array::new();
            for x in list {
                a.push(x.trim());
            }
            put(
                &mut draft,
                &mut changes,
                i,
                &["exclude", field],
                Value::Array(a),
                label,
            )?;
        }
    }
    if let Some(clients) = &e.dhcp_clients {
        if clients.len() > MAX_CLIENTS {
            return Err(invalid(format!(
                "Use at most {MAX_CLIENTS} client overrides."
            )));
        }
        let mut seen = HashSet::new();
        for c in clients {
            let Some(mac) = crate::config::normalize_mac(&c.mac) else {
                return Err(invalid(format!("\"{}\" is not a MAC address.", c.mac)));
            };
            if !seen.insert(mac) {
                return Err(invalid(format!(
                    "The MAC address {} is in the list twice.",
                    c.mac
                )));
            }
            let rate = |r: f64| r.is_finite() && r > 0.0 && r <= 100_000.0;
            if !rate(c.download_mbps) || !rate(c.upload_mbps) {
                return Err(invalid("Use rates above 0 and up to 100000 Mbps."));
            }
            if c.name
                .as_ref()
                .is_some_and(|n| n.len() > 64 || n.chars().any(char::is_control))
            {
                return Err(invalid("Use a client name of up to 64 characters."));
            }
        }
        if action == "create" {
            return Err(invalid("Add client overrides after the router is saved."));
        }
        for old in &cfg.routers[i].dhcp.clients {
            draft
                .remove_client(i, &old.mac)
                .map_err(|err| internal(format!("{err:#}")))?;
        }
        for c in clients {
            draft
                .set_client(i, &c.mac, c.download_mbps, c.upload_mbps, c.name.as_deref())
                .map_err(|err| invalid(format!("{err:#}")))?;
        }
        changes.push("dhcp_clients".into());
    }
    if changes.is_empty() {
        return Err(invalid("There is no change to save."));
    }
    let new_text = draft.text();
    let typed: HashSet<PathBuf> = pending.iter().map(|p| p.path.clone()).collect();
    // The config rules, then the `lqosync3 check` rules, as the TUI preview runs them.
    let new_cfg = Config::from_toml_str(&new_text).map_err(|err| invalid(format!("{err:#}")))?;
    let lines = crate::cli::check_with(&new_cfg, owner_uid, &typed);
    let errors: Vec<&str> = lines
        .iter()
        .filter(|l| l.level == crate::cli::Level::Error)
        .map(|l| l.text.as_str())
        .collect();
    if !errors.is_empty() {
        return Err(invalid(errors.join("\n")));
    }
    let warnings: Vec<String> = lines
        .into_iter()
        .filter(|l| l.level == crate::cli::Level::Warn)
        .map(|l| l.text)
        .collect();
    let state_dir = Path::new(&cfg.runtime.state_dir);
    let saved = apply(
        config_path,
        &digest,
        &new_text,
        &pending,
        state_dir,
        cfg.runtime.keep_backups,
        now,
    )
    .map_err(|err| match err {
        ApplyError::Changed { .. } => (
            "conflict",
            "The configuration changed during the save. Load the page again.".to_string(),
        ),
        ApplyError::Io(m) => internal(m),
    })?;
    let id = router_id(&name);
    let mut warnings = warnings;
    let event = RouterEvent {
        at: now,
        actor: req.actor.clone(),
        router_id: id.clone(),
        router: name,
        action: action.into(),
        changes,
        digest: saved.clone(),
    };
    if let Err(err) = audit(state_dir, &event) {
        warnings.push(format!(
            "The change is saved, but the audit log failed: {err:#}"
        ));
    }
    Ok(RouterEditReply {
        router_id: id,
        digest: saved,
        reload: reload(&new_cfg),
        warnings,
    })
}

/// The `router-edit` command: one JSON request in, one JSON outcome out.
pub fn run(config_path: &Path, input: &str, now: u64) -> RouterEditOutcome {
    let outcome = |code: &str, message: String, reply| RouterEditOutcome {
        code: code.into(),
        message,
        reply,
    };
    match serde_json::from_str::<RouterEditRequest>(input) {
        Err(_) => outcome("invalid", "The request is not valid.".into(), None),
        // Secret files belong to root, as for the service.
        Ok(req) => match edit(config_path, &req, now, 0, &signal_reload) {
            Ok(reply) => outcome("ok", String::new(), Some(reply)),
            Err((code, message)) => outcome(code, message, None),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use web_contract::routers::RouterEdit;

    // A self-signed test certificate, not a production CA.
    const PEM: &str = "-----BEGIN CERTIFICATE-----\nMIIBiDCCAS+gAwIBAgIUZf3VepY3yl6fqQAXCeWnrnJdMQAwCgYIKoZIzj0EAwIw\nGjEYMBYGA1UEAwwPbHFvc3luYy10ZXN0LWNhMB4XDTI2MTEyMTAxMTA1MFoXDTM2\nMTExODAxMTA1MFowGjEYMBYGA1UEAwwPbHFvc3luYy10ZXN0LWNhMFkwEwYHKoZI\nzj0CAQYIKoZIzj0DAQcDQgAEopy7BPMP9XGreg2+s3awmsQjv7+yOFyriXzeFREP\nHWWp9fCAugr5nkYVf2mIIzpoTKtRbVwt5TwDPSlYcdqAJaNTMFEwHQYDVR0OBBYE\nFLTiCxN+BFvd4hOLc39oYBeVPD13MB8GA1UdIwQYMBaAFLTiCxN+BFvd4hOLc39o\nYBeVPD13MA8GA1UdEwEB/wQFMAMBAf8wCgYIKoZIzj0EAwIDRwAwRAIgWub+R96Z\nb8bJ98ZcUIjLtVUaF7ubS4pZM1+s2d55bSkCIAGQqvndr+Plh3nV08g1QAIznnuf\npYwrUvqbP7BJuqTZ\n-----END CERTIFICATE-----\n";

    struct Env {
        dir: tempfile::TempDir,
        config: PathBuf,
    }
    fn env() -> Env {
        let dir = tempfile::tempdir().unwrap();
        let lqos = dir.path().join("lqos");
        std::fs::create_dir_all(&lqos).unwrap();
        std::fs::create_dir_all(dir.path().join("secrets")).unwrap();
        let pw = dir.path().join("secrets/R1.password");
        std::fs::write(&pw, "old").unwrap();
        std::fs::set_permissions(&pw, std::fs::Permissions::from_mode(0o600)).unwrap();
        let config = dir.path().join("lqosync3.toml");
        std::fs::write(
            &config,
            format!(
                "[runtime]\nstate_dir = \"{}\"\n[libreqos]\ntarget = \"v2_pair\"\ndirectory = \"{}\"\n\n\
                 [[routers]]\nname = \"R1\"\naddress = \"192.0.2.1\"\nusername = \"u\"\npassword_file = \"{}\"\n[routers.pppoe]\nenabled = true\n",
                dir.path().join("state").display(),
                lqos.display(),
                pw.display()
            ),
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("state")).unwrap();
        Env { dir, config }
    }
    fn digest(e: &Env) -> String {
        config_digest(&std::fs::read(&e.config).unwrap())
    }
    fn request(router_id: Option<&str>, edit: RouterEdit) -> RouterEditRequest {
        RouterEditRequest {
            actor: "operator-a".into(),
            router_id: router_id.map(Into::into),
            edit,
        }
    }
    fn me() -> u32 {
        // SAFETY: geteuid has no preconditions.
        unsafe { libc::geteuid() }
    }
    fn no_reload(_: &Config) -> String {
        "test".into()
    }

    #[test]
    fn a_new_router_is_saved_disabled_with_its_secret_and_an_audit_line() {
        let e = env();
        let edit = RouterEdit {
            expected_digest: digest(&e),
            name: Some("Core-01".into()),
            address: Some("192.0.2.9".into()),
            username: Some("api".into()),
            password: Some("new-secret-value".into()),
            port: Some(8729),
            tls: Some(true),
            ..Default::default()
        };
        let reply = edit_ok(&e, None, edit);
        assert_eq!(reply.router_id, router_id("Core-01"));
        assert_eq!(reply.reload, "test");
        let cfg = Config::from_toml_str(&std::fs::read_to_string(&e.config).unwrap()).unwrap();
        let core = cfg.routers.iter().find(|r| r.name == "Core-01").unwrap();
        assert!(!core.enabled, "a new router waits for an explicit enable");
        assert!(!core.reads_subscribers());
        let secret = e.dir.path().join("secrets/Core-01.password");
        // Secret files end with a newline; the service reads them trimmed.
        assert_eq!(
            std::fs::read_to_string(&secret).unwrap().trim_end(),
            "new-secret-value"
        );
        assert_eq!(
            std::fs::metadata(&secret).unwrap().permissions().mode() & 0o077,
            0
        );
        let log = std::fs::read_to_string(e.dir.path().join("state").join(AUDIT)).unwrap();
        assert!(log.contains("\"action\":\"create\"") && log.contains("operator-a"));
        assert!(!log.contains("new-secret-value"), "{log}");
        assert_eq!(reply.digest, digest(&e));
    }

    fn edit_ok(e: &Env, id: Option<&str>, change: RouterEdit) -> RouterEditReply {
        edit(&e.config, &request(id, change), 1_000, me(), &no_reload).unwrap()
    }

    #[test]
    fn an_update_keeps_the_name_and_a_blank_credential_and_writes_a_managed_ca() {
        let e = env();
        let id = router_id("R1");
        let reply = edit_ok(
            &e,
            Some(&id),
            RouterEdit {
                expected_digest: digest(&e),
                address: Some("r1.example.net".into()),
                tls_verify: Some(true),
                ca_pem: Some(PEM.into()),
                dhcp: Some(true),
                location: Some([14.5, 121.0]),
                ..Default::default()
            },
        );
        assert_eq!(reply.router_id, id);
        let text = std::fs::read_to_string(&e.config).unwrap();
        let cfg = Config::from_toml_str(&text).unwrap();
        let r1 = &cfg.routers[0];
        assert_eq!(r1.address, "r1.example.net");
        assert!(r1.dhcp.enabled && r1.pppoe.enabled);
        assert_eq!(r1.latitude, Some(14.5));
        assert_eq!(
            Path::new(&r1.tls_ca_file),
            e.dir.path().join("ca/R1.pem").as_path()
        );
        assert_eq!(
            std::fs::read_to_string(&r1.tls_ca_file).unwrap().trim_end(),
            PEM.trim_end()
        );
        assert_eq!(
            std::fs::read_to_string(e.dir.path().join("secrets/R1.password"))
                .unwrap()
                .trim_end(),
            "old",
            "an absent credential keeps the current one"
        );
        // A rename is refused: the name is the identity.
        let err = edit(
            &e.config,
            &request(
                Some(&id),
                RouterEdit {
                    expected_digest: digest(&e),
                    name: Some("R2".into()),
                    port: Some(8728),
                    ..Default::default()
                },
            ),
            1_001,
            me(),
            &no_reload,
        )
        .unwrap_err();
        assert_eq!(err.0, "invalid");
    }

    #[test]
    fn a_stale_digest_bad_input_or_unknown_router_changes_nothing() {
        let e = env();
        let before = std::fs::read_to_string(&e.config).unwrap();
        let id = router_id("R1");
        let cases: Vec<(Option<String>, RouterEdit, &str)> = vec![
            (
                Some(id.clone()),
                RouterEdit {
                    expected_digest: "0000000000000000".into(),
                    port: Some(8728),
                    ..Default::default()
                },
                "conflict",
            ),
            (
                Some(id.clone()),
                RouterEdit {
                    expected_digest: digest(&e),
                    address: Some("192.0.2.1; reboot".into()),
                    ..Default::default()
                },
                "invalid",
            ),
            (
                Some(id.clone()),
                RouterEdit {
                    expected_digest: digest(&e),
                    ca_pem: Some("not a certificate".into()),
                    ..Default::default()
                },
                "invalid",
            ),
            (
                Some(id.clone()),
                RouterEdit {
                    expected_digest: digest(&e),
                    ca_pem: Some(
                        "-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n".into(),
                    ),
                    ..Default::default()
                },
                "invalid",
            ),
            (
                Some(id.clone()),
                RouterEdit {
                    expected_digest: digest(&e),
                    ..Default::default()
                },
                "invalid",
            ),
            (
                Some("lq3-ffffffffffffffff".into()),
                RouterEdit {
                    expected_digest: digest(&e),
                    port: Some(8728),
                    ..Default::default()
                },
                "not_found",
            ),
            (
                None,
                RouterEdit {
                    expected_digest: digest(&e),
                    name: Some("R1".into()),
                    address: Some("192.0.2.2".into()),
                    username: Some("u".into()),
                    password: Some("x".into()),
                    ..Default::default()
                },
                "invalid",
            ),
            (
                None,
                RouterEdit {
                    expected_digest: digest(&e),
                    name: Some("R2".into()),
                    address: Some("192.0.2.2".into()),
                    username: Some("u".into()),
                    ..Default::default()
                },
                "invalid",
            ),
        ];
        for (id, edit_, code) in cases {
            let err = edit(
                &e.config,
                &request(id.as_deref(), edit_),
                1_000,
                me(),
                &no_reload,
            )
            .unwrap_err();
            assert_eq!(err.0, code, "{}", err.1);
        }
        assert_eq!(std::fs::read_to_string(&e.config).unwrap(), before);
        assert!(!e.dir.path().join("state").join(AUDIT).exists());
        // The command answers a malformed request without a panic or a write.
        let out = run(
            &e.config,
            "{\"actor\":\"a\",\"edit\":{\"command\":\"x\"}}",
            1_000,
        );
        assert_eq!(out.code, "invalid");
    }

    #[test]
    fn disable_writes_false_and_enable_removes_the_key() {
        let e = env();
        let id = router_id("R1");
        let off = RouterEdit {
            expected_digest: digest(&e),
            enabled: Some(false),
            ..Default::default()
        };
        edit_ok(&e, Some(&id), off);
        let text = std::fs::read_to_string(&e.config).unwrap();
        assert!(text.contains("enabled = false"), "{text}");
        assert!(!Config::from_toml_str(&text).unwrap().routers[0].enabled);
        let on = RouterEdit {
            expected_digest: digest(&e),
            enabled: Some(true),
            ..Default::default()
        };
        edit_ok(&e, Some(&id), on);
        let text = std::fs::read_to_string(&e.config).unwrap();
        assert!(!text.contains("enabled = false"), "{text}");
        let cfg = Config::from_toml_str(&text).unwrap();
        assert!(cfg.routers[0].enabled && cfg.routers[0].pppoe.enabled);
        let log = std::fs::read_to_string(e.dir.path().join("state").join(AUDIT)).unwrap();
        let actions: Vec<String> = log
            .lines()
            .map(|l| serde_json::from_str::<RouterEvent>(l).unwrap().action)
            .collect();
        assert_eq!(actions, ["disable", "enable"]);
        // A new router cannot start enabled.
        let err = edit(
            &e.config,
            &request(
                None,
                RouterEdit {
                    expected_digest: digest(&e),
                    name: Some("R9".into()),
                    address: Some("192.0.2.9".into()),
                    username: Some("u".into()),
                    password: Some("x".into()),
                    enabled: Some(true),
                    ..Default::default()
                },
            ),
            1_000,
            me(),
            &no_reload,
        )
        .unwrap_err();
        assert_eq!(err.0, "invalid");
    }

    #[test]
    fn lists_replace_the_whole_list_and_bad_entries_change_nothing() {
        use web_contract::routers::DhcpClientOverride;
        let e = env();
        let id = router_id("R1");
        let client = |mac: &str, down: f64| DhcpClientOverride {
            mac: mac.into(),
            download_mbps: down,
            upload_mbps: 5.0,
            name: Some("Office".into()),
        };
        edit_ok(
            &e,
            Some(&id),
            RouterEdit {
                expected_digest: digest(&e),
                exclude_ppp_profiles: Some(vec!["staff".into(), "test-*".into()]),
                exclude_ips: Some(vec!["100.64.9.0/24".into()]),
                dhcp_clients: Some(vec![
                    client("02-00-00-00-0a-01", 20.0),
                    client("02:00:00:00:0a:02", 30.0),
                ]),
                ..Default::default()
            },
        );
        let cfg = Config::from_toml_str(&std::fs::read_to_string(&e.config).unwrap()).unwrap();
        let r1 = &cfg.routers[0];
        assert_eq!(r1.exclude.ppp_profiles, ["staff", "test-*"]);
        assert_eq!(r1.exclude.ips, ["100.64.9.0/24"]);
        let macs: Vec<_> = r1.dhcp.clients.iter().map(|c| c.mac.as_str()).collect();
        assert_eq!(
            macs,
            ["02:00:00:00:0A:01", "02:00:00:00:0A:02"],
            "normalized, in order"
        );
        // A new list replaces the old one; an empty list removes the key.
        edit_ok(
            &e,
            Some(&id),
            RouterEdit {
                expected_digest: digest(&e),
                exclude_ppp_profiles: Some(vec![]),
                dhcp_clients: Some(vec![client("02:00:00:00:0a:02", 40.0)]),
                ..Default::default()
            },
        );
        let text = std::fs::read_to_string(&e.config).unwrap();
        let cfg = Config::from_toml_str(&text).unwrap();
        assert!(cfg.routers[0].exclude.ppp_profiles.is_empty());
        assert_eq!(cfg.routers[0].dhcp.clients.len(), 1);
        assert_eq!(cfg.routers[0].dhcp.clients[0].download_mbps, 40.0);
        let before = text;
        for bad in [
            RouterEdit {
                exclude_ips: Some(vec!["100.64.9.0/33".into()]),
                ..Default::default()
            },
            RouterEdit {
                dhcp_clients: Some(vec![client("not-a-mac", 1.0)]),
                ..Default::default()
            },
            RouterEdit {
                dhcp_clients: Some(vec![
                    client("02:00:00:00:0a:01", 1.0),
                    client("02-00-00-00-0A-01", 2.0),
                ]),
                ..Default::default()
            },
            RouterEdit {
                dhcp_clients: Some(vec![client("02:00:00:00:0a:01", 0.0)]),
                ..Default::default()
            },
            RouterEdit {
                exclude_dhcp_servers: Some(vec![" ".into()]),
                ..Default::default()
            },
        ] {
            let change = RouterEdit {
                expected_digest: digest(&e),
                ..bad
            };
            let err = edit(
                &e.config,
                &request(Some(&id), change),
                1_000,
                me(),
                &no_reload,
            )
            .unwrap_err();
            assert_eq!(err.0, "invalid", "{}", err.1);
        }
        assert_eq!(std::fs::read_to_string(&e.config).unwrap(), before);
    }

    #[test]
    fn the_audit_log_rotates_at_its_limit() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(AUDIT), vec![b'x'; AUDIT_MAX as usize]).unwrap();
        audit(dir.path(), &RouterEvent::default()).unwrap();
        assert!(dir.path().join(format!("{AUDIT}.1")).exists());
        assert!(std::fs::metadata(dir.path().join(AUDIT)).unwrap().len() < 1024);
    }
}
