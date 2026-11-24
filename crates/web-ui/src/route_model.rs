//! Browser routes. Pure parsing, so native tests cover it. The gateway serves the page for each path.

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Page {
    Dashboard,
    Routers,
    NewRouter,
    Workers,
    Journal,
    Router {
        id: String,
        tab: Tab,
    },
    /// `tab`: one of `TOPOLOGY_TABS`. `subscriber`: the selected stable ID on Assignments.
    Topology {
        tab: &'static str,
        subscriber: Option<String>,
    },
    /// The PPPoE (`ppp`) or IPoE / DHCP (`dhcp`) source view. `plans`: the Local plans view.
    Source {
        kind: &'static str,
        plans: bool,
    },
    Unknown,
}

pub(crate) const TOPOLOGY_TABS: [(&str, &str); 4] = [
    ("builder", "Sankey"),
    ("assignments", "Assignments"),
    ("policies", "Policies"),
    ("changes", "Changes"),
];

fn param<'a>(search: &'a str, key: &str) -> Option<&'a str> {
    search
        .trim_start_matches('?')
        .split('&')
        .find_map(|p| p.strip_prefix(key)?.strip_prefix('='))
}

/// A stable subscriber ID from the service (`tm-sub-` and hex digits). Never an IP or a name.
pub(crate) fn subscriber_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

pub(crate) fn assignments_href(subscriber: &str) -> String {
    format!("/topology?tab=assignments&subscriber={subscriber}")
}

/// A section of the router detail page, selected with `?tab=`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tab {
    Overview,
    Connection,
    Sources,
    Pinning,
    Activity,
}
impl Tab {
    pub(crate) const ALL: [Tab; 5] = [
        Tab::Overview,
        Tab::Connection,
        Tab::Sources,
        Tab::Pinning,
        Tab::Activity,
    ];
    pub(crate) fn key(self) -> &'static str {
        match self {
            Tab::Overview => "overview",
            Tab::Connection => "connection",
            Tab::Sources => "sources",
            Tab::Pinning => "wan-pinning",
            Tab::Activity => "activity",
        }
    }
    pub(crate) fn label(self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Connection => "Connection",
            Tab::Sources => "Sources",
            Tab::Pinning => "WAN pinning",
            Tab::Activity => "Activity",
        }
    }
}

/// A router ID from the service: `lq3-` and hex digits. Anything else is not a route.
fn router_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

pub(crate) fn parse(path: &str, search: &str) -> Page {
    let path = path.trim_end_matches('/');
    match path {
        "" => Page::Dashboard,
        "/routers" => Page::Routers,
        "/workers" => Page::Workers,
        "/journal" => Page::Journal,
        "/routers/new" => Page::NewRouter,
        "/topology" => {
            let key = param(search, "tab").unwrap_or("");
            let tab = TOPOLOGY_TABS
                .iter()
                .find(|(k, _)| *k == key)
                .map_or("builder", |(k, _)| k);
            let subscriber = param(search, "subscriber")
                .filter(|s| tab == "assignments" && subscriber_id(s))
                .map(str::to_owned);
            Page::Topology { tab, subscriber }
        }
        "/pppoe" | "/ipoe" => Page::Source {
            kind: if path == "/pppoe" { "ppp" } else { "dhcp" },
            plans: param(search, "view") == Some("plans"),
        },
        _ => match path.strip_prefix("/routers/") {
            Some(id) if router_id(id) => {
                let key = search
                    .trim_start_matches('?')
                    .split('&')
                    .find_map(|p| p.strip_prefix("tab="))
                    .unwrap_or("");
                let tab = Tab::ALL
                    .into_iter()
                    .find(|t| t.key() == key)
                    .unwrap_or(Tab::Overview);
                Page::Router { id: id.into(), tab }
            }
            _ => Page::Unknown,
        },
    }
}

pub(crate) fn router_href(id: &str, tab: Tab) -> String {
    match tab {
        Tab::Overview => format!("/routers/{id}"),
        _ => format!("/routers/{id}?tab={}", tab.key()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes_parse_and_reject_unsafe_ids() {
        assert_eq!(parse("/", ""), Page::Dashboard);
        assert_eq!(parse("/routers/", ""), Page::Routers);
        assert_eq!(parse("/routers/new", ""), Page::NewRouter);
        assert_eq!(parse("/workers/", ""), Page::Workers);
        assert_eq!(parse("/journal", "?unit=x"), Page::Journal);
        let page = parse("/routers/lq3-00ff", "?x=1&tab=connection");
        assert_eq!(
            page,
            Page::Router {
                id: "lq3-00ff".into(),
                tab: Tab::Connection
            }
        );
        // An unknown tab falls back to the overview; an unsafe ID is not a route.
        assert_eq!(
            parse("/routers/lq3-00ff", "?tab=secrets"),
            Page::Router {
                id: "lq3-00ff".into(),
                tab: Tab::Overview
            }
        );
        for bad in ["/routers/a/b", "/routers/%2e%2e", "/routers/a.b", "/users"] {
            assert_eq!(parse(bad, ""), Page::Unknown, "{bad}");
        }
        assert_eq!(router_href("lq3-1", Tab::Overview), "/routers/lq3-1");
        // Topology views. An unknown tab is the Builder; an unsafe subscriber is dropped.
        assert_eq!(
            parse("/topology", ""),
            Page::Topology {
                tab: "builder",
                subscriber: None
            }
        );
        assert_eq!(
            parse("/topology/", "?tab=secrets"),
            Page::Topology {
                tab: "builder",
                subscriber: None
            }
        );
        assert_eq!(
            parse("/topology", "?tab=assignments&subscriber=tm-sub-00ff"),
            Page::Topology {
                tab: "assignments",
                subscriber: Some("tm-sub-00ff".into())
            }
        );
        for bad in ["192.0.2.1", "a/b", "%2e"] {
            assert_eq!(
                parse("/topology", &format!("?tab=assignments&subscriber={bad}")),
                Page::Topology {
                    tab: "assignments",
                    subscriber: None
                },
                "{bad}"
            );
        }
        assert_eq!(
            parse("/topology", "?tab=policies&subscriber=tm-sub-1"),
            Page::Topology {
                tab: "policies",
                subscriber: None
            }
        );
        assert_eq!(
            parse("/pppoe", ""),
            Page::Source {
                kind: "ppp",
                plans: false
            }
        );
        assert_eq!(
            parse("/ipoe", "?view=plans"),
            Page::Source {
                kind: "dhcp",
                plans: true
            }
        );
        assert_eq!(
            assignments_href("tm-sub-1"),
            "/topology?tab=assignments&subscriber=tm-sub-1"
        );
        assert_eq!(
            router_href("lq3-1", Tab::Sources),
            "/routers/lq3-1?tab=sources"
        );
        for t in Tab::ALL {
            assert_eq!(
                parse("/routers/lq3-1", &format!("?tab={}", t.key())),
                Page::Router {
                    id: "lq3-1".into(),
                    tab: t
                }
            );
            assert!(!t.label().is_empty());
        }
    }
}
