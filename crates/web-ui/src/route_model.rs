//! Browser routes. Pure parsing, so native tests cover it. The gateway serves the page for each path.

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Page {
    Dashboard,
    Routers,
    NewRouter,
    Workers,
    Journal,
    Router { id: String, tab: Tab },
    Unknown,
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
