//! Labels and filters for the router inventory. Pure functions, so native tests cover them.
use web_contract::routers::{RouterCollection, RouterSummary};

/// A state label with its tone class. The text carries the meaning; the tone only supports it.
pub(crate) fn read_state(c: Option<&RouterCollection>) -> (&'static str, &'static str) {
    match c.map(|c| c.state.as_str()) {
        None => ("Not read", "neutral"),
        Some("online") => ("Online", "good"),
        Some("offline") => ("Offline", "warn"),
        Some("failed") => ("Read failed", "bad"),
        Some("unreachable") => ("Unreachable", "bad"),
        Some(_) => ("Unknown", "neutral"),
    }
}

/// The row state: Disabled first, else the read state.
pub(crate) fn state(r: &RouterSummary) -> (&'static str, &'static str) {
    if r.enabled {
        read_state(r.collection.as_ref())
    } else {
        ("Disabled", "neutral")
    }
}

pub(crate) fn transport(r: &RouterSummary) -> (&'static str, &'static str) {
    match (r.tls, r.tls_verify, r.tls_ca) {
        (false, _, _) => ("Plain API", "warn"),
        (true, false, _) => ("TLS, certificate not checked", "warn"),
        (true, true, true) => ("TLS, checked with its CA", "good"),
        (true, true, false) => ("TLS, checked with public CAs", "good"),
    }
}

pub(crate) fn role_label(role: &str) -> &'static str {
    match role {
        "wan" => "WAN",
        "ppp" => "PPPoE",
        "dhcp" => "DHCP",
        "hotspot" => "Hotspot",
        _ => "Other",
    }
}

/// The inventory filters. An empty field matches every router.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Filter {
    pub text: String,
    /// A role key (`wan`, `ppp`, `dhcp`, `hotspot`), or empty.
    pub role: String,
    /// `online`, `problem`, `not_read`, `disabled`, or empty.
    pub state: String,
}

pub(crate) fn matches(r: &RouterSummary, f: &Filter) -> bool {
    let text = f.text.trim().to_lowercase();
    let state = r.collection.as_ref().map(|c| c.state.as_str());
    (text.is_empty()
        || r.name.to_lowercase().contains(&text)
        || r.address.to_lowercase().contains(&text))
        && (f.role.is_empty() || r.roles.contains(&f.role))
        && match f.state.as_str() {
            "online" => state == Some("online"),
            "problem" => matches!(state, Some("failed" | "unreachable" | "offline")),
            "not_read" => r.enabled && state.is_none(),
            "disabled" => !r.enabled,
            _ => true,
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn router(name: &str, roles: &[&str], state: Option<&str>) -> RouterSummary {
        RouterSummary {
            name: name.into(),
            enabled: true,
            address: "192.0.2.1".into(),
            roles: roles.iter().map(|r| r.to_string()).collect(),
            collection: state.map(|s| RouterCollection {
                state: s.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
    #[test]
    fn states_and_transports_read_plainly() {
        assert_eq!(read_state(None), ("Not read", "neutral"));
        let c = |s: &str| RouterCollection {
            state: s.into(),
            ..Default::default()
        };
        assert_eq!(read_state(Some(&c("online"))).0, "Online");
        assert_eq!(read_state(Some(&c("unreachable"))), ("Unreachable", "bad"));
        assert_eq!(read_state(Some(&c("new"))).0, "Unknown");
        let mut r = router("Core", &[], None);
        assert_eq!(transport(&r).0, "Plain API");
        r.tls = true;
        assert_eq!(transport(&r), ("TLS, certificate not checked", "warn"));
        r.tls_verify = true;
        assert_eq!(transport(&r).0, "TLS, checked with public CAs");
        r.tls_ca = true;
        assert_eq!(transport(&r).0, "TLS, checked with its CA");
        assert_eq!(role_label("ppp"), "PPPoE");
    }
    #[test]
    fn filters_combine_text_role_and_state() {
        let core = router("FSE-CORE", &["wan"], None);
        let access = router("Access-01", &["ppp", "dhcp"], Some("unreachable"));
        let all = Filter::default();
        assert!(matches(&core, &all) && matches(&access, &all));
        let f = |text: &str, role: &str, state: &str| Filter {
            text: text.into(),
            role: role.into(),
            state: state.into(),
        };
        assert!(matches(&core, &f("core", "", "")), "case-insensitive name");
        assert!(matches(&core, &f("192.0.2", "", "")), "address");
        assert!(!matches(&access, &f("", "wan", "")));
        assert!(matches(&access, &f("", "dhcp", "problem")));
        assert!(matches(&core, &f("", "", "not_read")));
        assert!(!matches(&core, &f("", "", "online")));
        // A disabled router has its own state and is not "not read".
        let mut off = router("Spare", &["ppp"], None);
        off.enabled = false;
        assert_eq!(state(&off), ("Disabled", "neutral"));
        assert_eq!(state(&core).0, "Not read");
        assert!(matches(&off, &f("", "", "disabled")));
        assert!(!matches(&off, &f("", "", "not_read")));
        assert!(!matches(&core, &f("", "", "disabled")));
    }
}
