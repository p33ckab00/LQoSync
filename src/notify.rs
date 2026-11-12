//! Telegram notifier (Plan 2b): events from the cycle status, the message text, and the send.

pub mod delivery;
pub mod outbox;
use crate::config::Telegram;
use crate::secrets::read_secret;
use crate::status::{CircuitInfo, Status, UpdatedInfo};
use rustls::pki_types::ServerName;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Escape a value for Telegram HTML.
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Cut `text` into parts of at most `max` characters, at line ends where possible.
pub fn split(text: &str, max: usize) -> Vec<String> {
    let max = max.max(1);
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut cur_len = 0;
    for line in text.split('\n') {
        let mut chars: Vec<char> = line.chars().collect();
        // A line longer than one part is cut into full parts first.
        while chars.len() > max {
            if !cur.is_empty() {
                parts.push(std::mem::take(&mut cur));
                cur_len = 0;
            }
            parts.push(chars.drain(..max).collect());
        }
        let n = chars.len();
        let need = if cur.is_empty() { n } else { cur_len + 1 + n };
        if need > max && !cur.is_empty() {
            parts.push(std::mem::take(&mut cur));
            cur_len = 0;
        }
        if !cur.is_empty() {
            cur.push('\n');
            cur_len += 1;
        }
        cur.extend(chars);
        cur_len += n;
    }
    if !cur.is_empty() || parts.is_empty() {
        parts.push(cur);
    }
    parts
}

/// The events of the last cycles, so that the same event is sent one time only.
#[derive(Debug, Default, Clone)]
pub struct Notifier {
    routers: BTreeMap<String, &'static str>,
    last_error: Option<String>,
    last_alerts: Vec<String>,
    last_bridge: Option<crate::bridge::BridgeStats>,
}

/// At most `max` characters of `s`, with "..." at the end if it was cut.
fn cap(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((i, _)) => format!("{}...", &s[..i]),
        None => s.to_string(),
    }
}

/// A multi-line text as one `<code>` element on each line, so a split at a line end keeps valid HTML.
fn code_block(out: &mut Vec<String>, label: &str, text: &str) {
    out.push(format!("<b>{label}:</b>"));
    out.extend(
        cap(text, 1000)
            .lines()
            .take(20)
            .map(|l| format!("<code>{}</code>", esc(&cap(l, 300)))),
    );
}

/// "name  ip  down/up Mbps<sep>parent", without the empty parts.
fn circuit_line(c: &CircuitInfo, sep: &str) -> String {
    let mut t = c.name.clone();
    if !c.ipv4.is_empty() {
        t += &format!("  {}", c.ipv4);
    }
    if !c.download_max.is_empty() {
        t += &format!("  {}/{} Mbps", c.download_max, c.upload_max);
    }
    if !c.parent.is_empty() {
        t += &format!("{sep}{}", c.parent);
    }
    t
}

/// "name  ip: column old -> new, ...". An empty value shows as "(empty)".
fn updated_line(u: &UpdatedInfo) -> String {
    let v = |x: &str| {
        if x.is_empty() {
            "(empty)".to_string()
        } else {
            x.to_string()
        }
    };
    let changes: Vec<String> = u
        .changes
        .iter()
        .map(|(col, old, new)| format!("{col} {} -> {}", v(old), v(new)))
        .collect();
    format!("{}  {}: {}", u.name, u.ipv4, changes.join(", "))
}

fn list(out: &mut Vec<String>, title: &str, items: &[String], max: usize) {
    if items.is_empty() {
        return;
    }
    out.push(format!("<b>{title}</b>"));
    out.extend(items.iter().take(max).map(|i| format!("- {}", esc(i))));
    if items.len() > max {
        out.push(format!("...and {} more", items.len() - max));
    }
    out.push(String::new());
}

impl Notifier {
    /// The message for this cycle, or `None` if the cycle has no event (spec "Events").
    pub fn message(&mut self, tg: &Telegram, st: &Status) -> Option<String> {
        // The memory key: the first line without digits. A time or a PID in the text of the
        // same error does not make a new event.
        let key = |t: &Option<String>| {
            t.as_deref().map(|t| {
                t.lines()
                    .next()
                    .unwrap_or_default()
                    .chars()
                    .filter(|c| !c.is_ascii_digit())
                    .collect::<String>()
            })
        };
        let error = match (&st.error, &st.config_error) {
            (None, None) => None,
            (e, c) => Some(format!("{:?}|{:?}", key(e), key(c))),
        };
        let alerts = st.report.alerts.clone();
        let mut router_changes = Vec::new();
        for r in &st.routers {
            let prev = self.routers.get(&r.name).copied();
            // A router with no earlier state is an event only if it is not online.
            let changed = prev.map_or(r.state != "online", |p| p != r.state);
            if changed {
                let why = r
                    .error
                    .as_deref()
                    .map_or(String::new(), |e| format!(" ({e})"));
                router_changes.push((
                    r.state,
                    format!(
                        "{}: {} -> {}{why}",
                        r.name,
                        prev.unwrap_or("(start)"),
                        r.state
                    ),
                ));
            }
        }
        let error_new = error.is_some() && error != self.last_error;
        let cleared = error.is_none() && self.last_error.is_some();
        let alerts_new = !alerts.is_empty() && alerts != self.last_alerts;
        // The bridge counters only go up. A lower value is a bridge restart: a new baseline.
        let mut bridge_up = Vec::new();
        if let (Some(p), Some(c)) = (self.last_bridge, st.bridge)
            && c.budget_defers >= p.budget_defers
            && c.iter_gt100ms >= p.iter_gt100ms
        {
            for (name, a, b) in [
                ("budget_defers", p.budget_defers, c.budget_defers),
                ("iter_gt100ms", p.iter_gt100ms, c.iter_gt100ms),
            ] {
                if b > a {
                    bridge_up.push(format!("{name} {a} to {b}"));
                }
            }
        }
        if st.bridge.is_some() {
            self.last_bridge = st.bridge;
        }
        self.routers = st
            .routers
            .iter()
            .map(|r| (r.name.clone(), r.state))
            .collect();
        self.last_error = error;
        self.last_alerts = alerts;
        if !tg.enabled {
            return None;
        }

        let on_err = tg.notify_on_error;
        let router_bad = router_changes.iter().any(|(s, _)| *s != "online");
        let changes = tg.notify_on_changes && st.files_changed;
        let title = if on_err && error_new {
            "ALERT: <b>lqosync3 cycle error</b>"
        } else if on_err && router_bad {
            "ALERT: <b>router state changed</b>"
        } else if on_err && alerts_new {
            "ALERT: <b>removal guard held a write</b>"
        } else if on_err && !bridge_up.is_empty() {
            "ALERT: <b>LibreQoS bridge worker is behind</b>"
        } else if st.native_ready_new {
            "OK: <b>LibreQoS native schema 2 is ready</b>"
        } else if changes && st.mode == "shadow" {
            "OK: <b>lqosync3 candidate files changed</b>"
        } else if changes {
            "OK: <b>lqosync3 update applied</b>"
        } else if on_err && !router_changes.is_empty() {
            "OK: <b>router online again</b>"
        } else if on_err && cleared {
            "OK: <b>lqosync3 error cleared</b>"
        } else {
            return None;
        };

        let prefix = if st.mode == "shadow" { "[shadow] " } else { "" };
        let yes = |b: bool| if b { "Yes" } else { "No" };
        let mut out = vec![
            format!("{prefix}{title}"),
            String::new(),
            format!(
                "<b>Mode:</b> {} | <b>Target:</b> {}",
                esc(&st.mode),
                esc(&st.target)
            ),
            format!("<b>Files changed:</b> {}", yes(st.files_changed)),
            // Only the first line: the full reload error is in the error block.
            format!(
                "<b>Reload:</b> {}",
                esc(&cap(st.reload.lines().next().unwrap_or_default(), 300))
            ),
            format!("<b>Next scan:</b> {}s", st.next_scan_seconds),
        ];
        if !bridge_up.is_empty() {
            let b = st.bridge.unwrap_or(crate::bridge::BridgeStats {
                budget_defers: 0,
                iter_gt100ms: 0,
                iter_gt1s: 0,
                iter_max_ms: 0,
            });
            out.push(format!(
                "<b>Bridge:</b> {} (iter_gt1s {}, iter_max_ms {}). If it goes up again, add a worker: [queues] workers in lqos.conf.",
                esc(&bridge_up.join(", ")),
                b.iter_gt1s,
                b.iter_max_ms
            ));
        }
        if st.native_ready_new {
            out.push(
                "<b>Native schema 2:</b> the probe passed. The operator can change [topology] source from shaped_devices to libreqos_network.".into(),
            );
        }
        if let Some(e) = &st.error {
            code_block(&mut out, "Error", e);
        }
        if let Some(e) = &st.config_error {
            code_block(&mut out, "Config error", e);
        }
        out.push(String::new());
        let max = tg.max_list_items;
        let changes_text: Vec<String> = router_changes.into_iter().map(|(_, t)| t).collect();
        list(&mut out, "Router changes", &changes_text, max);
        out.extend(cycle_telemetry(tg, st));
        out.push(change_details(tg, &st.report));
        Some(out.join("\n").trim_end().to_string())
    }
}

pub(crate) fn cycle_telemetry(tg: &Telegram, st: &Status) -> Vec<String> {
    let mut out = Vec::new();
    if tg.include_telemetry {
        out.push(if st.cycle_ms > 0 {
            format!("<b>Cycle time:</b> {} ms", st.cycle_ms)
        } else {
            "<b>Cycle time:</b> pending at write snapshot".into()
        });
        let routers: Vec<String> = st
            .routers
            .iter()
            .map(|r| {
                let rows: usize = r.rows.values().sum();
                let why = r
                    .error
                    .as_deref()
                    .map_or(String::new(), |e| format!(" ({e})"));
                format!(
                    "{}: {}, {rows} rows, {} ms{why}",
                    r.name, r.state, r.read_ms
                )
            })
            .collect();
        list(&mut out, "Routers", &routers, tg.max_list_items);
    }
    out
}

/// Freeze subscriber details with the options selected for this write.
pub fn change_details(tg: &Telegram, rep: &crate::status::ReportSummary) -> String {
    let mut out = Vec::new();
    let max = tg.max_list_items;

    let n = |k: &str| rep.counts.get(k).copied().unwrap_or(0);
    out.push("<b>Summary</b>".into());
    for (label, k) in [
        ("Added", "added"),
        ("Updated", "updated"),
        ("Removed", "removed"),
        ("Held", "held"),
        ("Duplicates", "duplicates"),
        ("Warnings", "warnings"),
    ] {
        out.push(format!("- {label}: {}", n(k)));
    }
    out.push(String::new());
    if tg.include_added {
        let added: Vec<String> = rep.added.iter().map(|c| circuit_line(c, " -> ")).collect();
        list(&mut out, "Added", &added, max);
    }
    if tg.include_updated {
        let updated: Vec<String> = rep.updated.iter().map(updated_line).collect();
        list(&mut out, "Updated", &updated, max);
    }
    if tg.include_removed {
        let removed: Vec<String> = rep
            .removed
            .iter()
            .map(|c| circuit_line(c, ", from "))
            .collect();
        list(&mut out, "Removed", &removed, max);
    }
    list(&mut out, "Held by the removal guard", &rep.held, max);
    let dups: Vec<String> = rep
        .duplicates
        .iter()
        .map(|(ip, kept, dropped)| format!("{ip}: kept {kept}, skipped {dropped}"))
        .collect();
    list(&mut out, "Duplicate IPs", &dups, max);
    list(&mut out, "Alerts", &rep.alerts, max);
    list(&mut out, "Warnings", &rep.warnings, max);
    out.join("\n").trim_end().to_string()
}

/// The bot token: the variable in `bot_token_env` if it is set and not empty, else `bot_token_file`.
/// An error never contains the token.
pub fn token(tg: &Telegram, owner_uid: u32) -> Result<String, String> {
    if !tg.bot_token_env.is_empty()
        && let Ok(v) = std::env::var(&tg.bot_token_env)
        && !v.trim().is_empty()
    {
        return Ok(v.trim().to_string());
    }
    let t = read_secret(Path::new(&tg.bot_token_file), owner_uid).map_err(|e| format!("{e:#}"))?;
    match t.trim() {
        "" => Err(format!("the bot token in {} is empty", tg.bot_token_file)),
        t => Ok(t.to_string()),
    }
}

/// The chat ID: the variable in `chat_id_env` if it is set and not empty, else `chat_id`.
pub fn chat_id(tg: &Telegram) -> String {
    if !tg.chat_id_env.is_empty()
        && let Ok(v) = std::env::var(&tg.chat_id_env)
        && !v.trim().is_empty()
    {
        return v.trim().to_string();
    }
    tg.chat_id.trim().to_string()
}

/// Send one HTTP/1.1 POST with a JSON body and read the status. The error never contains `path`.
pub async fn post_json<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    host: &str,
    path: &str,
    body: &str,
) -> Result<(), String> {
    let (code, value) = response(stream, host, path, body).await?;
    if (200..300).contains(&code) {
        Ok(())
    } else {
        Err(format!(
            "HTTP {code}: {}",
            value["description"].as_str().unwrap_or_default()
        ))
    }
}
async fn response<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    host: &str,
    path: &str,
    body: &str,
) -> Result<(u16, serde_json::Value), String> {
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(req.as_bytes())
        .await
        .map_err(|e| format!("cannot send the request: {e}"))?;
    stream
        .flush()
        .await
        .map_err(|e| format!("cannot send the request: {e}"))?;
    // Read to the end of the body (Content-Length), or to the close. 64 KiB at most.
    let mut resp = Vec::new();
    let mut chunk = [0u8; 4096];
    while resp.len() < 64 * 1024 {
        match stream.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => resp.extend_from_slice(&chunk[..n]),
            // A server can close TLS without close_notify after the full answer. Use what came.
            Err(_) if !resp.is_empty() => break,
            Err(e) => return Err(format!("no answer: {e}")),
        }
        if body_complete(&resp) {
            break;
        }
    }
    let text = String::from_utf8_lossy(&resp);
    let code: u16 = text
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .ok_or("the answer has no HTTP status")?;
    if resp.len() > 64 * 1024 {
        return Err("response exceeds bound".into());
    }
    let end = resp
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("missing HTTP header")?;
    let head = String::from_utf8_lossy(&resp[..end]);
    let raw = &resp[end + 4..];
    let body = if is_chunked(&head) {
        if !body_complete(&resp) {
            return Err("incomplete chunked body".into());
        }
        dechunk(raw)
    } else {
        if let Some(len) = head.lines().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.eq_ignore_ascii_case("content-length")
                .then(|| v.trim().parse::<usize>().ok())?
        }) && raw.len() != len
        {
            return Err("incomplete HTTP body".into());
        }
        String::from_utf8(raw.to_vec()).map_err(|_| "invalid response encoding")?
    };
    let value = serde_json::from_str(&body).map_err(|_| "invalid API response")?;
    Ok((code, value))
}

fn is_chunked(head: &str) -> bool {
    head.lines().any(|l| {
        l.split_once(':').is_some_and(|(k, v)| {
            k.trim().eq_ignore_ascii_case("transfer-encoding")
                && v.trim().eq_ignore_ascii_case("chunked")
        })
    })
}

/// The data of a chunked body (`<hex size>\r\n<data>\r\n` ... `0\r\n\r\n`), from the raw bytes.
/// A bad size or a short chunk ends the parse; it never panics.
fn dechunk(body: &[u8]) -> String {
    decode_chunks(body)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default()
}
fn decode_chunks(mut body: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let eol = body
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or("missing chunk size")?;
        let line = std::str::from_utf8(&body[..eol]).map_err(|_| "invalid chunk size")?;
        let n = usize::from_str_radix(line.split(';').next().unwrap_or("").trim(), 16)
            .map_err(|_| "invalid chunk size")?;
        let rest = &body[eol + 2..];
        if n == 0 {
            if rest == b"\r\n" || rest.ends_with(b"\r\n\r\n") {
                return Ok(out);
            }
            return Err("incomplete final chunk".into());
        }
        let end = n
            .checked_add(2)
            .filter(|end| *end <= rest.len())
            .ok_or("incomplete chunk")?;
        if &rest[n..end] != b"\r\n" {
            return Err("invalid chunk separator".into());
        }
        out.extend_from_slice(&rest[..n]);
        body = &rest[end..];
    }
}

/// The answer has its full head and a body of `Content-Length` bytes, or the last chunk.
fn body_complete(resp: &[u8]) -> bool {
    let Some(end) = resp.windows(4).position(|w| w == b"\r\n\r\n") else {
        return false;
    };
    let head = String::from_utf8_lossy(&resp[..end]);
    if is_chunked(&head) {
        let body = &resp[end + 4..];
        return decode_chunks(body).is_ok();
    }
    head.lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.trim()
                .eq_ignore_ascii_case("content-length")
                .then(|| v.trim().parse::<usize>().ok())?
        })
        .is_some_and(|len| resp.len() - end - 4 >= len)
}

/// rustls client config with the public CA roots. The certificate is always checked.
fn tls_config() -> Arc<rustls::ClientConfig> {
    static CFG: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    CFG.get_or_init(|| {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let cfg = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .expect("ring supports the default TLS versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
        Arc::new(cfg)
    })
    .clone()
}

/// Send one message part. `timeout` covers the connect, TLS, request, and answer.
pub async fn send(tg: &Telegram, text: &str, owner_uid: u32) -> Result<(), String> {
    send_receipt(tg, text, owner_uid)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}
pub async fn send_receipt(
    tg: &Telegram,
    text: &str,
    owner_uid: u32,
) -> Result<delivery::ApiReceipt, delivery::DeliveryError> {
    use delivery::DeliveryError;
    let token = token(tg, owner_uid).map_err(|_| DeliveryError {
        reason: "telegram_credentials_unavailable".into(),
        retry_after: None,
        permanent: true,
    })?;
    let chat = chat_id(tg);
    if chat.is_empty() {
        return Err(DeliveryError {
            reason: "telegram_chat_missing".into(),
            retry_after: None,
            permanent: true,
        });
    }
    let body=serde_json::json!({"chat_id":chat,"text":text,"parse_mode":"HTML","disable_web_page_preview":true}).to_string();
    let path = format!("/bot{token}/sendMessage");
    let host = tg.api_host.as_str();
    let work = async {
        let tcp = tokio::net::TcpStream::connect((host, tg.api_port))
            .await
            .map_err(|_| DeliveryError::transient("telegram_connect_failed"))?;
        let name = ServerName::try_from(host.to_string())
            .map_err(|_| DeliveryError::transient("telegram_host_invalid"))?;
        let tls = tokio_rustls::TlsConnector::from(tls_config())
            .connect(name, tcp)
            .await
            .map_err(|_| DeliveryError::transient("telegram_tls_failed"))?;
        delivery::post_receipt(tls, host, &path, &body).await
    };
    tokio::time::timeout(Duration::from_secs(tg.timeout), work)
        .await
        .map_err(|_| DeliveryError::transient("telegram_timeout"))?
}

/// Send a message in parts of `max_message_chars`. Stop at the first failed part.
pub async fn send_all(tg: &Telegram, text: &str, owner_uid: u32) -> Result<(), String> {
    for part in delivery::html_parts(text, tg.max_message_chars)? {
        send(tg, &part, owner_uid).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Telegram;
    use crate::status::{ReportSummary, RouterStatus};

    fn tg() -> Telegram {
        Telegram {
            enabled: true,
            chat_id: "1".into(),
            ..Telegram::default()
        }
    }

    fn st(mode: &str, state: &'static str) -> Status {
        Status {
            version: 2,
            updated: 0,
            mode: mode.into(),
            target: "v2_pair".into(),
            cycle_ms: 12,
            next_scan_seconds: 120,
            routers: vec![RouterStatus {
                name: "R1".into(),
                state,
                error: None,
                rows: [("/ppp/active".to_string(), 3)].into(),
                read_ms: 7,
            }],
            report: ReportSummary::default(),
            files_changed: false,
            reload: "skipped".into(),
            error: None,
            config_error: None,
            ..Status::default()
        }
    }

    #[test]
    fn no_event_gives_no_message() {
        let mut n = Notifier::default();
        assert_eq!(n.message(&tg(), &st("apply", "online")), None);
        assert_eq!(n.message(&tg(), &st("apply", "online")), None);
    }

    #[test]
    fn a_native_probe_that_becomes_ready_gives_one_notice() {
        let mut n = Notifier::default();
        let mut s = st("apply", "online");
        s.target = "v3_pair".into();
        s.native = "ready".into();
        s.native_ready_new = true;
        let m = n.message(&tg(), &s).expect("a notice");
        assert!(m.contains("native schema 2"), "{m}");
        assert!(
            m.contains("shaped_devices") && m.contains("libreqos_network"),
            "{m}"
        );
        s.native_ready_new = false;
        assert_eq!(n.message(&tg(), &s), None, "one time only");
    }

    #[test]
    fn a_bridge_counter_that_goes_up_gives_one_alert() {
        let b = |d: u64, g: u64| crate::bridge::BridgeStats {
            budget_defers: d,
            iter_gt100ms: g,
            iter_gt1s: 0,
            iter_max_ms: 94,
        };
        let mut n = Notifier::default();
        let mut s = st("apply", "online");
        s.bridge = Some(b(1, 0));
        assert_eq!(
            n.message(&tg(), &s),
            None,
            "the first value is the baseline"
        );
        assert_eq!(n.message(&tg(), &s), None, "no change");
        s.bridge = Some(b(4, 0));
        let m = n.message(&tg(), &s).expect("an alert");
        assert!(
            m.contains("bridge") && m.contains("budget_defers 1 to 4"),
            "{m}"
        );
        assert_eq!(n.message(&tg(), &s), None, "one time only");
        s.bridge = Some(b(4, 2));
        let m = n.message(&tg(), &s).expect("an alert");
        assert!(m.contains("iter_gt100ms 0 to 2"), "{m}");
        s.bridge = None;
        assert_eq!(n.message(&tg(), &s), None, "no data");
        s.bridge = Some(b(0, 0));
        assert_eq!(
            n.message(&tg(), &s),
            None,
            "a bridge restart starts the counters again"
        );
        s.bridge = Some(b(1, 0));
        assert!(n.message(&tg(), &s).is_some());
    }

    #[test]
    fn disabled_gives_no_message() {
        let mut n = Notifier::default();
        let mut s = st("apply", "failed");
        s.files_changed = true;
        let off = Telegram {
            enabled: false,
            ..tg()
        };
        assert_eq!(n.message(&off, &s), None);
    }

    #[test]
    fn files_changed_lists_the_circuits() {
        let mut n = Notifier::default();
        let mut s = st("apply", "online");
        s.files_changed = true;
        s.reload = "ok".into();
        let info = |n: &str, ip: &str, d: &str, u: &str, p: &str| CircuitInfo {
            name: n.into(),
            ipv4: ip.into(),
            download_max: d.into(),
            upload_max: u.into(),
            parent: p.into(),
        };
        s.report.added = vec![info("alice", "100.64.0.2", "25", "5", "PPP-R1")];
        s.report.updated = vec![UpdatedInfo {
            name: "bob".into(),
            ipv4: "100.64.0.3".into(),
            changes: vec![
                ("IPv4".into(), "100.64.0.9".into(), "100.64.0.3".into()),
                ("Parent Node ID".into(), "".into(), "lq3-1".into()),
            ],
        }];
        s.report.removed = vec![info("carol", "100.64.0.4", "10", "10", "PPP-R1")];
        s.report.counts = [("added", 1), ("updated", 1), ("removed", 1)].into();
        let m = n.message(&tg(), &s).unwrap();
        assert!(m.starts_with("OK: <b>lqosync3 update applied</b>"), "{m}");
        for t in [
            "- alice  100.64.0.2  25/5 Mbps -&gt; PPP-R1",
            "- bob  100.64.0.3: IPv4 100.64.0.9 -&gt; 100.64.0.3, Parent Node ID (empty) -&gt; lq3-1",
            "- carol  100.64.0.4  10/10 Mbps, from PPP-R1",
            "<b>Reload:</b> ok",
            "- Added: 1",
        ] {
            assert!(m.contains(t), "missing {t}: {m}");
        }
        assert!(m.contains("R1: online, 3 rows, 7 ms"), "{m}");
    }

    #[test]
    fn shadow_title_has_the_prefix() {
        let mut n = Notifier::default();
        let mut s = st("shadow", "online");
        s.files_changed = true;
        let m = n.message(&tg(), &s).unwrap();
        assert!(
            m.starts_with("[shadow] OK: <b>lqosync3 candidate files changed</b>"),
            "{m}"
        );
    }

    #[test]
    fn notify_on_changes_off_skips_a_change() {
        let mut n = Notifier::default();
        let mut s = st("apply", "online");
        s.files_changed = true;
        let t = Telegram {
            notify_on_changes: false,
            ..tg()
        };
        assert_eq!(n.message(&t, &s), None);
    }

    #[test]
    fn an_error_is_sent_once_then_cleared() {
        let mut n = Notifier::default();
        let mut s = st("apply", "online");
        s.error = Some("LibreQoS reload failed: exit 1".into());
        let m = n.message(&tg(), &s).unwrap();
        assert!(m.starts_with("ALERT: <b>lqosync3 cycle error</b>"), "{m}");
        assert!(
            m.contains("<code>LibreQoS reload failed: exit 1</code>"),
            "{m}"
        );
        assert_eq!(
            n.message(&tg(), &s),
            None,
            "the same error is not sent again"
        );
        s.error = None;
        let m = n.message(&tg(), &s).unwrap();
        assert!(m.starts_with("OK: <b>lqosync3 error cleared</b>"), "{m}");
        assert_eq!(n.message(&tg(), &s), None);
    }

    #[test]
    fn an_error_with_other_numbers_is_not_a_new_event() {
        let mut n = Notifier::default();
        let mut s = st("apply", "online");
        s.error = Some("LibreQoS reload failed: pid 1234 at 10:00:01\nTraceback line 5".into());
        assert!(n.message(&tg(), &s).is_some());
        s.error = Some("LibreQoS reload failed: pid 5678 at 10:00:31\nTraceback line 9".into());
        assert_eq!(n.message(&tg(), &s), None, "only numbers changed");
        s.error = Some("cannot read network.json".into());
        assert!(
            n.message(&tg(), &s).is_some(),
            "another error is a new event"
        );
    }

    #[test]
    fn a_config_error_is_an_error_event() {
        let mut n = Notifier::default();
        let mut s = st("apply", "online");
        s.config_error = Some("bad key".into());
        let m = n.message(&tg(), &s).unwrap();
        assert!(
            m.contains("<b>Config error:</b>\n<code>bad key</code>"),
            "{m}"
        );
    }

    #[test]
    fn router_state_changes_are_sent() {
        let mut n = Notifier::default();
        assert_eq!(n.message(&tg(), &st("apply", "online")), None);
        let mut s = st("apply", "failed");
        s.routers[0].error = Some("timeout: x".into());
        let m = n.message(&tg(), &s).unwrap();
        assert!(m.starts_with("ALERT: <b>router state changed</b>"), "{m}");
        assert!(m.contains("R1: online -&gt; failed (timeout: x)"), "{m}");
        assert_eq!(n.message(&tg(), &s), None);
        let m = n.message(&tg(), &st("apply", "online")).unwrap();
        assert!(m.starts_with("OK: <b>router online again</b>"), "{m}");
    }

    #[test]
    fn first_cycle_sends_only_a_router_that_is_not_online() {
        let mut n = Notifier::default();
        let m = n.message(&tg(), &st("apply", "unreachable")).unwrap();
        assert!(m.contains("R1: (start) -&gt; unreachable"), "{m}");
    }

    #[test]
    fn guard_alerts_are_sent_when_they_change() {
        let mut n = Notifier::default();
        let mut s = st("apply", "online");
        s.report.alerts = vec!["removal guard: 60 rows".into()];
        let m = n.message(&tg(), &s).unwrap();
        assert!(
            m.starts_with("ALERT: <b>removal guard held a write</b>"),
            "{m}"
        );
        assert_eq!(n.message(&tg(), &s), None);
    }

    #[test]
    fn lists_are_cut_and_names_are_escaped() {
        let mut n = Notifier::default();
        let mut s = st("apply", "online");
        s.files_changed = true;
        s.report.added = (0..5)
            .map(|i| CircuitInfo {
                name: format!("u{i}"),
                ..CircuitInfo::default()
            })
            .collect();
        s.report.added.push(CircuitInfo {
            name: "<b>&x".into(),
            ..CircuitInfo::default()
        });
        let t = Telegram {
            max_list_items: 2,
            ..tg()
        };
        let m = n.message(&t, &s).unwrap();
        assert!(m.contains("- u1\n...and 4 more"), "{m}");
        assert!(!m.contains("u2"), "{m}");
        assert_eq!(esc("<b>&x"), "&lt;b&gt;&amp;x");
    }

    #[test]
    fn split_cuts_at_line_ends() {
        let text = "aaaa\nbbbb\ncccc";
        assert_eq!(split(text, 9), ["aaaa\nbbbb", "cccc"]);
        assert_eq!(split("short", 100), ["short"]);
        let long = "x".repeat(25);
        let parts = split(&long, 10);
        assert_eq!(parts, ["x".repeat(10), "x".repeat(10), "x".repeat(5)]);
        assert!(
            split("ü".repeat(30).as_str(), 10)
                .iter()
                .all(|p| p.chars().count() <= 10)
        );
    }

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A one-request HTTP server. It gives the request text to the test and sends `answer`.
    async fn server(answer: &'static str) -> (u16, tokio::task::JoinHandle<String>) {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        let h = tokio::spawn(async move {
            let (mut c, _) = l.accept().await.unwrap();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 1024];
            loop {
                let n = c.read(&mut chunk).await.unwrap();
                buf.extend_from_slice(&chunk[..n]);
                let t = String::from_utf8_lossy(&buf).to_string();
                if let Some((head, body)) = t.split_once("\r\n\r\n") {
                    let len: usize = head
                        .lines()
                        .find_map(|l| l.strip_prefix("Content-Length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    if body.len() >= len {
                        break;
                    }
                }
            }
            c.write_all(answer.as_bytes()).await.unwrap();
            c.shutdown().await.unwrap();
            String::from_utf8(buf).unwrap()
        });
        (port, h)
    }

    #[tokio::test]
    async fn post_json_sends_request_and_reads_status() {
        let (port, h) = server("HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n{\"ok\":true}").await;
        let s = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        post_json(s, "api.telegram.org", "/botT/sendMessage", r#"{"a":1}"#)
            .await
            .unwrap();
        let req = h.await.unwrap();
        assert!(
            req.starts_with("POST /botT/sendMessage HTTP/1.1\r\n"),
            "{req}"
        );
        for t in [
            "Host: api.telegram.org\r\n",
            "Content-Type: application/json\r\n",
            "Content-Length: 7\r\n",
            "Connection: close\r\n",
        ] {
            assert!(req.contains(t), "missing {t:?}: {req}");
        }
        assert!(req.ends_with(r#"{"a":1}"#), "{req}");
    }

    #[tokio::test]
    async fn post_json_error_has_the_description_and_not_the_path() {
        let (port, _h) = server(
            "HTTP/1.1 400 Bad Request\r\nContent-Length: 56\r\n\r\n{\"ok\":false,\"description\":\"Bad Request: chat not found\"}",
        )
        .await;
        let s = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let e = post_json(s, "h", "/botSECRET/sendMessage", "{}")
            .await
            .unwrap_err();
        assert!(e.contains("400") && e.contains("chat not found"), "{e}");
        assert!(!e.contains("SECRET"), "{e}");
    }

    #[test]
    fn token_env_wins_and_empty_env_uses_file() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("telegram.token");
        std::fs::write(&f, "FILETOKEN\n").unwrap();
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o600)).unwrap();
        let uid = std::fs::metadata(&f).unwrap().uid();
        // SAFETY: the test owns these variable names; no other test reads them.
        unsafe {
            std::env::set_var("LQ3_TEST_TOKEN_SET", "ENVTOKEN");
            std::env::set_var("LQ3_TEST_TOKEN_EMPTY", "");
        }
        let mut t = Telegram {
            bot_token_file: f.display().to_string(),
            bot_token_env: "LQ3_TEST_TOKEN_SET".into(),
            ..tg()
        };
        assert_eq!(token(&t, uid).unwrap(), "ENVTOKEN");
        t.bot_token_env = "LQ3_TEST_TOKEN_EMPTY".into();
        assert_eq!(token(&t, uid).unwrap(), "FILETOKEN");
        let e = token(&t, uid + 1).unwrap_err();
        assert!(!e.contains("FILETOKEN"), "{e}");
    }

    #[tokio::test]
    async fn send_times_out_on_a_silent_server() {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        let _keep = tokio::spawn(async move {
            let (_c, _) = l.accept().await.unwrap();
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        });
        // SAFETY: the test owns this variable name.
        unsafe { std::env::set_var("LQ3_TEST_TOKEN_SEND", "T") };
        let t = Telegram {
            bot_token_env: "LQ3_TEST_TOKEN_SEND".into(),
            api_host: "127.0.0.1".into(),
            api_port: port,
            timeout: 1,
            ..tg()
        };
        let start = std::time::Instant::now();
        let e = send(&t, "hi", 0).await.unwrap_err();
        assert!(e.contains("time"), "{e}");
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
    }

    /// A server that sends `answer` and keeps the connection open.
    async fn open_server(answer: &'static [u8]) -> u16 {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut c, _) = l.accept().await.unwrap();
            let mut b = [0u8; 4096];
            let _ = c.read(&mut b).await.unwrap();
            c.write_all(answer).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        });
        port
    }

    #[tokio::test]
    async fn post_json_reads_a_chunked_answer_without_a_close() {
        let port = open_server(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nb\r\n{\"ok\":true}\r\n0\r\n\r\n",
        )
        .await;
        let s = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let r = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            post_json(s, "h", "/botT/x", "{}"),
        )
        .await;
        assert_eq!(r.expect("no wait for the close"), Ok(()));
        let port = open_server(b"HTTP/1.1 400 Bad Request\r\ntransfer-encoding: chunked\r\n\r\n1a\r\n{\"description\":\"bad chat\"}\r\n0\r\n\r\n").await;
        let s = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let r = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            post_json(s, "h", "/botT/x", "{}"),
        )
        .await;
        assert_eq!(
            r.expect("no wait for the close"),
            Err("HTTP 400: bad chat".to_string())
        );
    }

    #[test]
    fn dechunk_never_panics_on_bad_bytes() {
        // Chunk sizes count bytes; an invalid byte must not break the parse.
        assert_eq!(
            dechunk(b"2\r\n\xffa\r\n3\r\nbcd\r\n0\r\n\r\n"),
            "\u{fffd}abcd"
        );
        assert_eq!(dechunk(b"9\r\nshort"), "");
        assert_eq!(dechunk(b"zz\r\nx"), "");
        assert_eq!(dechunk(b"5;ext=1\r\nhello\r\n0\r\n\r\n"), "hello");
    }

    #[tokio::test]
    async fn post_json_stops_at_content_length_without_a_close() {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        let _keep = tokio::spawn(async move {
            let (mut c, _) = l.accept().await.unwrap();
            let mut b = [0u8; 4096];
            let _ = c.read(&mut b).await.unwrap();
            c.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n{\"ok\":true}")
                .await
                .unwrap();
            // The server keeps the connection open.
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        });
        let s = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let r = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            post_json(s, "h", "/p", "{}"),
        )
        .await;
        assert_eq!(r, Ok(Ok(())));
    }

    #[test]
    fn a_long_multi_line_error_splits_into_valid_html_parts() {
        let mut n = Notifier::default();
        let mut s = st("apply", "online");
        let tail: Vec<String> = (0..30)
            .map(|i| format!("Traceback line {i} <x> & y"))
            .collect();
        let e = format!("LibreQoS reload failed: exit 1: {}", tail.join("\n"));
        s.reload = e.clone();
        s.error = Some(e);
        let m = n.message(&tg(), &s).unwrap();
        assert!(
            m.contains("<b>Reload:</b> LibreQoS reload failed: exit 1: Traceback line 0"),
            "{m}"
        );
        assert!(!m.contains("<b>Reload:</b> LibreQoS reload failed: exit 1: Traceback line 0 &lt;x&gt; &amp; y\nTraceback"), "{m}");
        for part in split(&m, 200) {
            assert_eq!(
                part.matches("<code>").count(),
                part.matches("</code>").count(),
                "{part}"
            );
        }
    }

    #[test]
    fn a_very_long_error_is_cut() {
        let mut n = Notifier::default();
        let mut s = st("apply", "online");
        s.error = Some("e".repeat(10_000));
        let m = n.message(&tg(), &s).unwrap();
        assert!(m.chars().count() < 3_000, "{}", m.chars().count());
    }

    #[test]
    fn an_empty_token_is_an_error() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("telegram.token");
        std::fs::write(&f, "\n").unwrap();
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o600)).unwrap();
        let uid = std::fs::metadata(&f).unwrap().uid();
        let t = Telegram {
            bot_token_file: f.display().to_string(),
            ..tg()
        };
        assert!(token(&t, uid).unwrap_err().contains("empty"));
    }
}
