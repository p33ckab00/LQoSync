//! The router form: its values, the field checks, and the change to send. Pure, so native tests cover it.
//! The core checks every value again; these checks only give early, field-level help.
use web_contract::routers::{MAX_CA_PEM, MAX_PASSWORD, RouterEdit, RouterSummary};

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Form {
    pub name: String,
    pub address: String,
    pub port: String,
    pub tls: bool,
    pub tls_verify: bool,
    pub username: String,
    /// Only sent for a new router, or when `replace_password` is set.
    pub password: String,
    pub replace_password: bool,
    /// A new CA certificate and its file name.
    pub ca_pem: Option<String>,
    pub ca_name: String,
    pub clear_ca: bool,
    pub pppoe: bool,
    pub dhcp: bool,
    pub hotspot: bool,
    pub ppp_per_plan_node: bool,
    pub hotspot_include_mac: bool,
    pub dhcp_require_bound: bool,
    pub dhcp_discover: bool,
    pub dhcp_shape_unassigned: bool,
    pub latitude: String,
    pub longitude: String,
}

impl Form {
    /// The defaults of a new router: the API over TLS with a checked certificate.
    pub(crate) fn new_router() -> Form {
        Form {
            port: "8729".into(),
            tls: true,
            tls_verify: true,
            hotspot_include_mac: true,
            dhcp_discover: true,
            ..Form::default()
        }
    }
    pub(crate) fn from(r: &RouterSummary) -> Form {
        let source = |kind: &str| r.sources.iter().any(|s| s.kind == kind && s.enabled);
        let o = &r.options;
        Form {
            name: r.name.clone(),
            address: r.address.clone(),
            port: r.port.to_string(),
            tls: r.tls,
            tls_verify: r.tls_verify,
            username: r.username.clone(),
            pppoe: source("ppp"),
            dhcp: source("dhcp"),
            hotspot: source("hotspot"),
            ppp_per_plan_node: o.ppp_per_plan_node,
            hotspot_include_mac: o.hotspot_include_mac,
            dhcp_require_bound: o.dhcp_require_bound,
            dhcp_discover: o.dhcp_discover,
            dhcp_shape_unassigned: o.dhcp_shape_unassigned,
            latitude: r.location.map(|l| l[0].to_string()).unwrap_or_default(),
            longitude: r.location.map(|l| l[1].to_string()).unwrap_or_default(),
            ..Form::default()
        }
    }
}

fn host(a: &str) -> bool {
    (1..=253).contains(&a.len())
        && a.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':'))
}
fn location(f: &Form) -> Result<Option<[f64; 2]>, ()> {
    match (f.latitude.trim(), f.longitude.trim()) {
        ("", "") => Ok(None),
        (a, b) => match (a.parse::<f64>(), b.parse::<f64>()) {
            (Ok(a), Ok(b)) if (-90.0..=90.0).contains(&a) && (-180.0..=180.0).contains(&b) => {
                Ok(Some([a, b]))
            }
            _ => Err(()),
        },
    }
}

/// The fields of one form section, for a step of the Add router form or for an editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Section {
    Connection,
    Sources,
}

/// Field errors as (field, message): a message says what to change. Empty when the section is valid.
pub(crate) fn check(f: &Form, new: bool, section: Section) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    if section == Section::Sources {
        if location(f).is_err() {
            out.push((
                "location",
                "Enter both values: latitude -90 to 90 and longitude -180 to 180.".into(),
            ));
        }
        return out;
    }
    if new
        && !((1..=64).contains(&f.name.len())
            && f.name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-')))
    {
        out.push((
            "name",
            "Use 1 to 64 letters, digits, dots, underscores, or hyphens.".into(),
        ));
    }
    if !host(&f.address) {
        out.push((
            "address",
            "Enter an IP address or a host name, without spaces.".into(),
        ));
    }
    if !f.port.parse::<u16>().is_ok_and(|p| p > 0) {
        out.push(("port", "Enter a port from 1 to 65535.".into()));
    }
    if !(1..=64).contains(&f.username.len()) || f.username.chars().any(char::is_whitespace) {
        out.push((
            "username",
            "Enter the RouterOS user, without spaces.".into(),
        ));
    }
    if new || f.replace_password {
        let n = f.password.chars().count();
        if n == 0 || n > MAX_PASSWORD || f.password.chars().any(char::is_control) {
            out.push((
                "password",
                "Enter a credential of 1 to 256 characters.".into(),
            ));
        }
    }
    if let Some(pem) = &f.ca_pem
        && (pem.len() > MAX_CA_PEM || !pem.contains("-----BEGIN CERTIFICATE-----"))
    {
        out.push((
            "ca",
            "Choose a PEM certificate file of 16 KiB or less.".into(),
        ));
    }
    out
}

/// A new router: every field. The core saves it disabled.
pub(crate) fn create(f: &Form, digest: &str) -> RouterEdit {
    RouterEdit {
        expected_digest: digest.into(),
        name: Some(f.name.trim().into()),
        address: Some(f.address.trim().into()),
        port: f.port.parse().ok(),
        tls: Some(f.tls),
        tls_verify: Some(f.tls_verify),
        username: Some(f.username.trim().into()),
        password: Some(f.password.clone()),
        ca_pem: f.ca_pem.clone().filter(|_| f.tls),
        pppoe: Some(f.pppoe),
        dhcp: Some(f.dhcp),
        hotspot: Some(f.hotspot),
        ppp_per_plan_node: Some(f.ppp_per_plan_node),
        hotspot_include_mac: Some(f.hotspot_include_mac),
        dhcp_require_bound: Some(f.dhcp_require_bound),
        dhcp_discover: Some(f.dhcp_discover),
        dhcp_shape_unassigned: Some(f.dhcp_shape_unassigned),
        location: location(f).ok().flatten(),
        ..RouterEdit::default()
    }
}

/// An edit of one section: only the fields that differ from the router.
pub(crate) fn update(f: &Form, r: &RouterSummary, section: Section, digest: &str) -> RouterEdit {
    let old = Form::from(r);
    let diff = |new: bool, old: bool| (new != old).then_some(new);
    let mut e = RouterEdit {
        expected_digest: digest.into(),
        ..RouterEdit::default()
    };
    match section {
        Section::Connection => {
            e.address = (f.address.trim() != old.address).then(|| f.address.trim().into());
            e.port = f.port.parse().ok().filter(|p| *p != r.port);
            e.tls = diff(f.tls, old.tls);
            e.tls_verify = diff(f.tls_verify, old.tls_verify);
            e.username = (f.username.trim() != old.username).then(|| f.username.trim().into());
            e.password = f.replace_password.then(|| f.password.clone());
            e.ca_pem = f.ca_pem.clone();
            e.clear_ca = f.clear_ca && f.ca_pem.is_none() && r.tls_ca;
        }
        Section::Sources => {
            e.pppoe = diff(f.pppoe, old.pppoe);
            e.dhcp = diff(f.dhcp, old.dhcp);
            e.hotspot = diff(f.hotspot, old.hotspot);
            e.ppp_per_plan_node = diff(f.ppp_per_plan_node, old.ppp_per_plan_node);
            e.hotspot_include_mac = diff(f.hotspot_include_mac, old.hotspot_include_mac);
            e.dhcp_require_bound = diff(f.dhcp_require_bound, old.dhcp_require_bound);
            e.dhcp_discover = diff(f.dhcp_discover, old.dhcp_discover);
            e.dhcp_shape_unassigned = diff(f.dhcp_shape_unassigned, old.dhcp_shape_unassigned);
            let new = location(f).ok().flatten();
            if new != r.location {
                e.location = new;
                e.clear_location = new.is_none();
            }
        }
    }
    e
}

/// True when the edit changes something besides its expected digest.
pub(crate) fn changes(e: &RouterEdit) -> bool {
    *e != RouterEdit {
        expected_digest: e.expected_digest.clone(),
        ..RouterEdit::default()
    }
}

/// An enabled router loses a source: its circuits then leave after the grace time.
pub(crate) fn removes_a_source(f: &Form, r: &RouterSummary) -> bool {
    let old = Form::from(r);
    r.enabled && ((old.pppoe && !f.pppoe) || (old.dhcp && !f.dhcp) || (old.hotspot && !f.hotspot))
}

/// The lines of a list field: trimmed, without empty lines and repeats, in order.
pub(crate) fn lines(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if !out.iter().any(|x| x == l) {
            out.push(l.to_string());
        }
    }
    out
}

/// An IPv4 address or network: `a.b.c.d` or `a.b.c.d/n` with n from 0 to 32.
fn ipv4_net(s: &str) -> bool {
    let (addr, bits) = s.split_once('/').unwrap_or((s, "32"));
    bits.parse::<u8>().is_ok_and(|b| b <= 32) && addr.parse::<std::net::Ipv4Addr>().is_ok()
}
/// The entries that are not IPv4 addresses or networks.
pub(crate) fn bad_ips(list: &[String]) -> Vec<String> {
    list.iter().filter(|x| !ipv4_net(x)).cloned().collect()
}

/// `AA:BB:CC:DD:EE:FF` from any case, with `:` or `-` or no separator.
pub(crate) fn mac(s: &str) -> Option<String> {
    let hex: String = s.chars().filter(|c| !matches!(c, ':' | '-')).collect();
    (hex.len() == 12 && hex.chars().all(|c| c.is_ascii_hexdigit())).then(|| {
        hex.to_ascii_uppercase()
            .as_bytes()
            .chunks(2)
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .collect::<Vec<_>>()
            .join(":")
    })
}

/// One editable client override row: the text of its fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ClientRow {
    pub mac: String,
    pub down: String,
    pub up: String,
    pub name: String,
}
impl ClientRow {
    pub(crate) fn from(c: &web_contract::routers::DhcpClientOverride) -> ClientRow {
        ClientRow {
            mac: c.mac.clone(),
            down: c.download_mbps.to_string(),
            up: c.upload_mbps.to_string(),
            name: c.name.clone().unwrap_or_default(),
        }
    }
}

/// Row errors as (row index, message), and the overrides when there is no error.
pub(crate) fn clients(
    rows: &[ClientRow],
) -> Result<Vec<web_contract::routers::DhcpClientOverride>, Vec<(usize, String)>> {
    let mut errors = Vec::new();
    let mut seen = Vec::new();
    let mut out = Vec::new();
    let rate = |s: &str| {
        s.trim()
            .parse::<f64>()
            .ok()
            .filter(|r| r.is_finite() && *r > 0.0 && *r <= 100_000.0)
    };
    for (i, r) in rows.iter().enumerate() {
        let Some(m) = mac(r.mac.trim()) else {
            errors.push((i, "Enter a MAC address such as 02:00:00:00:0A:01.".into()));
            continue;
        };
        if seen.contains(&m) {
            errors.push((i, "This MAC address is in the list twice.".into()));
            continue;
        }
        seen.push(m.clone());
        let (Some(down), Some(up)) = (rate(&r.down), rate(&r.up)) else {
            errors.push((
                i,
                "Enter both rates in Mbps, above 0 and up to 100000.".into(),
            ));
            continue;
        };
        let name = r.name.trim();
        if name.chars().count() > 64 {
            errors.push((i, "Use a name of up to 64 characters.".into()));
            continue;
        }
        out.push(web_contract::routers::DhcpClientOverride {
            mac: m,
            download_mbps: down,
            upload_mbps: up,
            name: (!name.is_empty()).then(|| name.into()),
        });
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

/// One editable DHCP server row. Empty number fields and an empty source list use the defaults.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ServerRow {
    pub name: String,
    pub mode: String,
    /// Speed sources in order, separated by commas.
    pub sources: String,
    pub down: String,
    pub up: String,
    pub down_factor: String,
    pub up_factor: String,
}
impl ServerRow {
    pub(crate) fn from(s: &web_contract::routers::DhcpServerSetting) -> ServerRow {
        let n = |v: Option<f64>| v.map(|v| v.to_string()).unwrap_or_default();
        ServerRow {
            name: s.name.clone(),
            mode: s.mode.clone(),
            sources: s
                .speed_sources
                .as_ref()
                .map(|v| v.join(", "))
                .unwrap_or_default(),
            down: n(s.default_plan_down_mbps),
            up: n(s.default_plan_up_mbps),
            down_factor: n(s.download_factor),
            up_factor: n(s.upload_factor),
        }
    }
}

pub(crate) const SOURCES: [&str; 3] = ["server_comment", "lease_comment", "settings"];

/// Row errors as (row index, message), and the settings when there is no error.
/// The service checks the same rules again.
pub(crate) fn servers(
    rows: &[ServerRow],
) -> Result<Vec<web_contract::routers::DhcpServerSetting>, Vec<(usize, String)>> {
    let mut errors = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut out = Vec::new();
    let number = |s: &str| -> Result<Option<f64>, ()> {
        match s.trim() {
            "" => Ok(None),
            t => t
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite() && *v > 0.0 && *v <= 100_000.0)
                .map(Some)
                .ok_or(()),
        }
    };
    for (i, r) in rows.iter().enumerate() {
        let name = r.name.trim();
        if name.is_empty() || name.chars().count() > 64 {
            errors.push((i, "Enter the DHCP server name as RouterOS shows it.".into()));
            continue;
        }
        if names.iter().any(|n| n == name) {
            errors.push((i, "This server name is in the list twice.".into()));
            continue;
        }
        names.push(name.into());
        if !matches!(r.mode.as_str(), "per_site" | "per_plan") {
            errors.push((i, "Choose a mode.".into()));
            continue;
        }
        let list: Vec<String> = r
            .sources
            .split(',')
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .map(String::from)
            .collect();
        if list.iter().any(|x| !SOURCES.contains(&x.as_str()))
            || (1..list.len()).any(|j| list[..j].contains(&list[j]))
        {
            errors.push((
                i,
                "Use server_comment, lease_comment, or settings, each once, in order.".into(),
            ));
            continue;
        }
        if r.mode == "per_site" && list.iter().any(|x| x == "lease_comment") {
            errors.push((i, "The per_site mode cannot use lease_comment.".into()));
            continue;
        }
        let (Ok(down), Ok(up), Ok(df), Ok(uf)) = (
            number(&r.down),
            number(&r.up),
            number(&r.down_factor),
            number(&r.up_factor),
        ) else {
            errors.push((
                i,
                "Enter numbers above 0, or leave a field empty for the default.".into(),
            ));
            continue;
        };
        out.push(web_contract::routers::DhcpServerSetting {
            name: name.into(),
            mode: r.mode.clone(),
            speed_sources: (!list.is_empty()).then_some(list),
            default_plan_down_mbps: down,
            default_plan_up_mbps: up,
            download_factor: df,
            upload_factor: uf,
        });
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use web_contract::routers::{RouterOptions, RouterSource};

    fn router() -> RouterSummary {
        RouterSummary {
            id: "lq3-1".into(),
            name: "Access-01".into(),
            enabled: true,
            address: "192.0.2.11".into(),
            port: 8728,
            username: "api".into(),
            tls_ca: true,
            sources: vec![RouterSource {
                kind: "ppp".into(),
                enabled: true,
                ..Default::default()
            }],
            options: RouterOptions {
                dhcp_discover: true,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn list_lines_ips_and_client_rows_are_checked_before_a_save() {
        assert_eq!(lines(" staff \n\nstaff\ntest-*\n"), ["staff", "test-*"]);
        let ips = lines("100.64.9.0/24\n192.0.2.1\n10.0.0.0/33\nexample\n");
        assert_eq!(bad_ips(&ips), ["10.0.0.0/33", "example"]);
        assert_eq!(
            mac("02-00-00-00-0a-01").as_deref(),
            Some("02:00:00:00:0A:01")
        );
        assert_eq!(mac("0200000000aa").as_deref(), Some("02:00:00:00:00:AA"));
        assert_eq!(mac("02:00:00:00:0a"), None);
        let row = |m: &str, d: &str| ClientRow {
            mac: m.into(),
            down: d.into(),
            up: "5".into(),
            name: " Office ".into(),
        };
        let ok = clients(&[row("02:00:00:00:0a:01", "20")]).unwrap();
        assert_eq!(
            (
                ok[0].mac.as_str(),
                ok[0].download_mbps,
                ok[0].name.as_deref()
            ),
            ("02:00:00:00:0A:01", 20.0, Some("Office"))
        );
        let bad = clients(&[
            row("02:00:00:00:0a:01", "20"),
            row("02-00-00-00-0A-01", "30"),
            row("zz", "1"),
            row("02:00:00:00:0a:03", "0"),
        ])
        .unwrap_err();
        let rows: Vec<usize> = bad.iter().map(|(i, _)| *i).collect();
        assert_eq!(rows, [1, 2, 3]);
        assert!(
            clients(&[]).unwrap().is_empty(),
            "an empty list removes every override"
        );
        // A saved override opens in the editor with the same values.
        assert_eq!(clients(&[ClientRow::from(&ok[0])]).unwrap(), ok);
    }

    #[test]
    fn server_rows_keep_the_source_order_and_follow_the_mode_rules() {
        let row = |name: &str, mode: &str, sources: &str| ServerRow {
            name: name.into(),
            mode: mode.into(),
            sources: sources.into(),
            down: "50".into(),
            ..Default::default()
        };
        let ok = servers(&[
            row(" LAN ", "per_plan", "settings, lease_comment"),
            row("GUEST", "per_site", ""),
        ])
        .unwrap();
        assert_eq!(ok[0].name, "LAN");
        assert_eq!(
            ok[0].speed_sources.as_deref(),
            Some(&["settings".to_string(), "lease_comment".to_string()][..]),
            "the order stays"
        );
        assert_eq!(
            (
                ok[1].speed_sources.clone(),
                ok[1].default_plan_down_mbps,
                ok[1].upload_factor
            ),
            (None, Some(50.0), None)
        );
        assert_eq!(
            servers(&[ServerRow::from(&ok[0])]).unwrap()[0],
            ok[0],
            "a saved server opens with the same values"
        );
        let bad = servers(&[
            row("LAN", "per_site", "lease_comment"),
            row("A", "per_plan", "settings, settings"),
            row("B", "", ""),
            row("C", "per_plan", "dns"),
            row("A", "per_plan", ""),
            ServerRow {
                down: "-1".into(),
                ..row("D", "per_plan", "")
            },
        ])
        .unwrap_err();
        let rows: Vec<usize> = bad.iter().map(|(i, _)| *i).collect();
        assert_eq!(rows, [0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn a_new_router_needs_identity_endpoint_and_a_credential() {
        let mut f = Form::new_router();
        let fields: Vec<_> = check(&f, true, Section::Connection)
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(fields, ["name", "address", "username", "password"]);
        f.name = "Core-01".into();
        f.address = "192.0.2.9".into();
        f.username = "api".into();
        f.password = "pw".into();
        assert!(check(&f, true, Section::Connection).is_empty());
        f.name = "Core 01".into();
        f.port = "70000".into();
        f.ca_pem = Some("not pem".into());
        let fields: Vec<_> = check(&f, true, Section::Connection)
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(fields, ["name", "port", "ca"]);
        f.latitude = "14.5".into();
        assert_eq!(check(&f, true, Section::Sources)[0].0, "location");
    }

    #[test]
    fn create_sends_every_field_and_drops_a_ca_without_tls() {
        let mut f = Form::new_router();
        f.name = " Core-01 ".into();
        f.address = "192.0.2.9".into();
        f.username = "api".into();
        f.password = "pw".into();
        f.ca_pem = Some("-----BEGIN CERTIFICATE-----".into());
        f.pppoe = true;
        let e = create(&f, "d1");
        assert_eq!(e.expected_digest, "d1");
        assert_eq!(e.name.as_deref(), Some("Core-01"));
        assert_eq!(
            (e.port, e.tls, e.pppoe),
            (Some(8729), Some(true), Some(true))
        );
        assert!(e.ca_pem.is_some());
        f.tls = false;
        assert!(create(&f, "d1").ca_pem.is_none());
    }

    #[test]
    fn update_sends_only_the_changed_fields_of_its_section() {
        let r = router();
        let mut f = Form::from(&r);
        assert!(!changes(&update(&f, &r, Section::Connection, "d")));
        assert!(!changes(&update(&f, &r, Section::Sources, "d")));
        f.port = "8729".into();
        f.password = "typed but not replacing".into();
        let e = update(&f, &r, Section::Connection, "d");
        assert_eq!(e.port, Some(8729));
        assert_eq!(
            e.password, None,
            "a credential goes only with an explicit replace"
        );
        f.replace_password = true;
        assert!(update(&f, &r, Section::Connection, "d").password.is_some());
        f.clear_ca = true;
        assert!(update(&f, &r, Section::Connection, "d").clear_ca);
        // Sources: a removed source on an enabled router is flagged; location can clear.
        let mut f = Form::from(&r);
        f.pppoe = false;
        f.dhcp = true;
        assert!(removes_a_source(&f, &r));
        let e = update(&f, &r, Section::Sources, "d");
        assert_eq!(
            (e.pppoe, e.dhcp, e.hotspot),
            (Some(false), Some(true), None)
        );
        let mut placed = r.clone();
        placed.location = Some([14.5, 121.0]);
        let mut f = Form::from(&placed);
        assert_eq!(f.latitude, "14.5");
        f.latitude.clear();
        f.longitude.clear();
        let e = update(&f, &placed, Section::Sources, "d");
        assert!(e.clear_location && e.location.is_none());
    }
}
