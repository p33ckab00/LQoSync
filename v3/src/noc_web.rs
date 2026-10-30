use crate::{
    model::{Node, Plan, Router},
    normalizer, orchestrator,
    web::{App, csrf, node_form, router_form},
};
use axum::response::Html;
use maud::{DOCTYPE, Markup, html};

const CSS: &str = include_str!("style.css");

fn description(title: &str) -> &str {
    match title {
        "Overview" => {
            "Network collection, shaping generations, WAN assignments, and validation at a glance."
        }
        "Routers" => "MikroTik inventory, verified TLS, and source configuration.",
        "Topology" => {
            "Discover inventory, arrange the shaping hierarchy, and inspect selected nodes."
        }
        "WANs" => "Map discovered interfaces to WAN roots and set directional capacity.",
        "Sources" => {
            "Bind discovered PPP profiles, DHCP servers, and Hotspot profiles to shaping nodes."
        }
        "Assignments" => "Review destination pools, subscriber assignments, and manual pins.",
        "Subscribers" => {
            "Source-owned identities and rates from the local snapshot. These are not online session counts."
        }
        "Speed rules" => "Resolve subscriber rates and inspect the source of each speed policy.",
        "Core lists" => {
            "Review intended core membership with protected manual entries and ownership checks."
        }
        "Core review" => {
            "Inspect routing evidence and the limits of the supported policy analysis."
        }
        "Validation" => "Review inventory, identities, rates, capacities, and output consistency.",
        "Settings" => "Configure local collection and publication. Inspect LibreQoS compatibility.",
        _ => "Inspect local configuration and diagnostic results.",
    }
}

pub(crate) fn shell(a: &App, title: &str, content: Markup) -> Html<String> {
    type NavGroup = (
        &'static str,
        &'static [(&'static str, &'static str, &'static str)],
    );
    let groups: &[NavGroup] = &[
        (
            "Operations",
            &[
                ("/", "Overview", "▦"),
                ("/routers", "Routers", "▤"),
                ("/topology", "Topology", "⌘"),
                ("/wans", "WANs", "⌁"),
            ],
        ),
        (
            "Subscriber control",
            &[
                ("/sources", "Sources", "⊞"),
                ("/assignments", "Assignments", "⇆"),
                ("/subscribers", "Subscribers", "≡"),
                ("/speed-rules", "Speed rules", "◫"),
            ],
        ),
        (
            "Routing & verification",
            &[
                ("/core-lists", "Core lists", "☷"),
                ("/core-control", "Core control", "⚙"),
                ("/core-review", "Core review", "◎"),
                ("/validation", "Validation", "✓"),
            ],
        ),
        ("System", &[("/settings", "Settings", "⚙")]),
    ];
    Html(html! {
        (DOCTYPE) html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width,initial-scale=1";
                meta name="color-scheme" content="dark";
                title {(title) " · LQoSync 3"}
                style {(maud::PreEscaped(CSS))}
                script src="/assets/console.js" defer {}
            }
            body {
                a.skiplink href="#page" {"Skip to content"}
                div.app {
                    aside.sidebar #sidebar {
                        a.brand href="/" {
                            span.brandmark {"LQ"}
                            span {span.brandtitle {"LQoSync 3"} span.brandsub {"ISP Operations"}}
                        }
                        nav aria-label="Main navigation" {
                            @for (group, links) in groups {
                                div.navcat {(group)}
                                @for (url, label, icon) in *links {
                                    a class=(if *label == title {"nav active"} else {"nav"}) href=(url) aria-current=[(*label == title).then_some("page")] {
                                        span.navico aria-hidden="true" {(icon)} (label)
                                    }
                                }
                            }
                        }
                        div.sidebarfooter {span.dot {} "Development environment" small {"Core writes and LibreQoS apply disabled"}}
                    }
                    div.main {
                        header.topbar {
                            div.toplocation {
                                button.iconbtn.hamburger #menuBtn type="button" aria-label="Toggle navigation" aria-controls="sidebar" aria-expanded="false" {"☰"}
                                span.breadcrumbs {"LQoSync 3 / " strong {(title)}}
                            }
                            div.toptools {
                                span.tag.amber.desktop-only {"◉ LOCAL STAGING ONLY"}
                                span.tag.muted.desktop-only {(if a.settings.auto_stage {"AUTO COLLECTION"} else {"COLLECTION PAUSED"})}
                                button.iconbtn #densityBtn type="button" title="Toggle spacing" aria-pressed="false" {"▤ Compact"}
                            }
                        }
                        main.content #page {
                            div.pagehead {
                                div {
                                    div.eyebrow {"NETWORK OPERATIONS"}
                                    h1 {(if title == "Overview" {"Operations overview"} else if title == "Topology" {"Topology workspace"} else {title})}
                                    div.subhead {(description(title))}
                                }
                                div.headbuttons {
                                    @match title {
                                        "Overview" => {a.btn.primary href="/topology" {"Open topology"} a.btn href="/validation" {"Validation"}},
                                        "Topology" => {a.btn href="#add-node" {"Add node"} a.btn href="/wans" {"WAN mappings"}},
                                        "Routers" => {a.btn.primary href="#add-router" {"Register router"}},
                                        _ => {},
                                    }
                                }
                            }
                            div.alert {strong {"Local staging only."} span {"Core writes and LibreQoS apply are disabled."}}
                            (content)
                            footer.footer {span {"LQoSync 3 · Rust backend and Rust-rendered UI"} span {"Candidate / staged / applied remain distinct"}}
                        }
                    }
                }
            }
        }
    }.into_string())
}

fn stat(label: &str, value: impl std::fmt::Display, detail: &str) -> Markup {
    html! {section.stat {div.statlabel {(label)} div.statval {(value)} div.statsub {(detail)}}}
}

fn inventory_status(a: &App, router: &Router) -> (&'static str, &'static str) {
    if a.inventory_errors.contains_key(&router.id) {
        return ("Read failed", "pill red");
    }
    let Some(i) = a.inventories.get(&router.id) else {
        return ("Not checked", "pill gray");
    };
    let now = orchestrator::now();
    if i.router_id != router.id || !i.complete() {
        ("Partial / invalid", "pill amber")
    } else if i.collected_at > now || now - i.collected_at > normalizer::MAX_INVENTORY_AGE_SECONDS {
        ("Stale / invalid age", "pill amber")
    } else {
        ("Complete", "pill")
    }
}

pub(crate) fn overview(a: &App, count: usize) -> Html<String> {
    shell(
        a,
        "Overview",
        html! {
            div.grid.stats {
                (stat("Managed routers", a.settings.routers.len(), "Configured sources"))
                (stat("Routing WANs", crate::dynamic::wan_nodes(&a.settings).len(), "Configured WAN mappings"))
                (stat("Subscribers", count, "Snapshot total · not online sessions"))
                (stat("Output state", if a.generation.is_some() {"STAGED"} else {"UNAVAILABLE"}, "Local generation only"))
            }
            div.statusflow {
                span.stage {i {} "Collect"} span.flowarrow {"→"}
                span.stage {i {} "Validate & build"} span.flowarrow {"→"}
                span.stage.staged {i {} "Local staging"} span.flowarrow {"→"}
                span.stage.blocked {i {} "Apply disabled"}
                span.stageend {(if a.settings.auto_stage {"Automatic collection enabled"} else {"Collection paused"})}
            }
            div.grid.twocol.sectiongap {
                section.panel {
                    div.panelhead {strong {"WAN capacity and source mapping"} a.btn href="/wans" {"Manage WANs"}}
                    div.tablewrap {table {
                        thead {tr {th {"WAN"} th {"Configured down / up"} th {"Router / interface"} th {"Mode"}}}
                        tbody {@for n in crate::dynamic::wan_nodes(&a.settings) {
                            tr {td.strong {(&n.name)} td {(n.download) " / " (n.upload) " Mbps"} td {(&n.router) " / " (&n.source)} td {(&n.mode)}}
                        }}
                    }}
                    @if crate::dynamic::wan_nodes(&a.settings).is_empty() {p.emptystate {"No WAN roots configured. Create a root in Topology."}}
                    div.tablefoot {"Configured capacities are not measured traffic utilization."}
                }
                section.panel {
                    div.panelhead {strong {"Collection health"} a.btn href="/routers" {"View routers"}}
                    div.panelbody.checklist {
                        @for router in &a.settings.routers {
                            @let (label, class) = inventory_status(a, router);
                            div.check {div.checkinfo {
                                strong {(&router.name) " " span class=(class) {(label)}}
                                small {(&router.role) " · " @if let Some(i) = a.inventories.get(&router.id) {"Last read Unix time " (i.collected_at)} @else {"No active inventory"}}
                            }}
                        }
                        @if a.settings.routers.is_empty() {p.emptystate {"No routers configured."}}
                    }
                }
            }
            div.grid.twocol {
                section.panel {
                    div.panelhead {strong {"Local orchestration pipeline"}}
                    div.panelbody.activity {
                        @for (name, detail) in [
                            ("01 · Collect selected sources", "Read allowlisted inventory and check source health."),
                            ("02 · Validate and build shaping tree", "Normalize identities, assign parents, and check capacity."),
                            ("03 · Commit one local generation", "Stage the output pair and configured core plan together."),
                            ("04 · Production writer unavailable", "Core writes and production shaping remain disabled."),
                        ] {
                            div.activityrow {span.activitydot {} div {strong {(name)} small {(detail)}}}
                        }
                    }
                }
                section.panel {
                    div.panelhead {strong {"Current publication status"} a.btn href="/validation" {"Open checks"}}
                    div.panelbody {
                        p {(&a.status)}
                        div.kv {span {"Local generation"} strong.mono {(a.generation.as_deref().unwrap_or("Unavailable"))}}
                        div.kv {span {"Production applied"} strong {"Unconfirmed · apply disabled"}}
                        p.note {"Open Validation for current diagnostic results. Local staging does not prove actual routing or shaping."}
                    }
                }
            }
        },
    )
}

pub(crate) fn routers(a: &App, selected: &str) -> Html<String> {
    let selected = a
        .settings
        .routers
        .iter()
        .find(|r| r.id == selected)
        .or_else(|| a.settings.routers.first());
    let complete = a
        .settings
        .routers
        .iter()
        .filter(|r| inventory_status(a, r).0 == "Complete")
        .count();
    shell(
        a,
        "Routers",
        html! {
            div.grid.stats {
                (stat("Configured routers", a.settings.routers.len(), "Core / subscriber / both"))
                (stat("Complete reads", complete, "Fresh process inventory"))
                (stat("Needs review", a.settings.routers.len() - complete, "Failed, missing, partial, or stale"))
                (stat("TLS policy", "VERIFIED", "For connections configured with TLS"))
            }
            div.grid.split.sectiongap {
                section.panel {
                    div.panelhead {strong {"Router sources"} input.search type="search" placeholder="Search routers / roles" aria-label="Search routers" data-filter="[data-router-row]";}
                    div.tablewrap {table {
                        thead {tr {@for label in ["Router", "Role", "API hostname", "Transport", "Last inventory", "Tables"] {th {(label)}}}}
                        tbody {@for router in &a.settings.routers {
                            @let (label, class) = inventory_status(a, router);
                            tr data-router-row class=(if selected.is_some_and(|r| r.id == router.id) {"active"} else {""}) {
                                td {a.strong href=(format!("/routers?selected={}", router.id)) {(&router.name)} div.micro.mono {(&router.id)}}
                                td {(&router.role)} td.mono {(&router.host)} td {(if router.tls {"TLS"} else {"Plain TCP"})}
                                td {span class=(class) {(label)}} td {(a.inventories.get(&router.id).map(|i| i.tables.len()).unwrap_or(0))}
                            }
                        }}
                    }}
                    @if a.settings.routers.is_empty() {p.emptystate {"No routers yet. Register a router below."}}
                    div.tablefoot {"Select a router name to inspect its configuration."}
                }
                section.panel {
                    div.panelhead {strong {"Router inspector"}}
                    div.panelbody {
                        @if let Some(router) = selected {
                            @let (label, class) = inventory_status(a, router);
                            div.eyebrow {"SELECTED ROUTER"} h2 {(&router.name)}
                            div.kv {span {"Stable ID"} strong.mono {(&router.id)}}
                            div.kv {span {"Role"} strong {(&router.role)}}
                            div.kv {span {"Transport"} strong {(if router.tls {"Verified TLS"} else {"Plain TCP"})}}
                            div.kv {span {"Last inventory"} strong {span class=(class) {(label)}}}
                            @if let Some(i) = a.inventories.get(&router.id) {div.kv {span {"Read Unix time"} strong {(i.collected_at)}}}
                            @if let Some(e) = a.inventory_errors.get(&router.id) {p.warning {(e)}}
                            form method="post" action=(format!("/routers/{}/inspect", router.id)) {(csrf(a)) button.btn.primary {"Read inventory"}}
                            a.btn href=(format!("/routers/{}/speeds", router.id)) {"Preview speed rules"}
                            p.note {"Inventory reads use allowlisted print commands. Passwords are never displayed."}
                        } @else {p.emptystate {"Select a configured router."}}
                    }
                }
            }
            @if let Some(router) = selected {
                section.card {
                    details {summary {"Edit selected router · " (&router.name)}
                        (router_form(a, Some(router)))
                        form.inline method="post" action=(format!("/routers/{}/delete", router.id)) {(csrf(a)) button.danger {"Remove router"}}
                    }
                    @if let Some(i) = a.inventories.get(&router.id) {
                        @for (path, rows) in &i.tables {details {summary {(path) " · " (rows.len()) " rows · first 20 shown"} pre {(serde_json::to_string_pretty(&rows[..rows.len().min(20)]).unwrap_or_default())}}}
                        @for (path, e) in &i.errors {p.warning {(path) ": " (e)}}
                    }
                }
            }
        details.panel.sectiongap #add-router {summary.panelhead {strong {"Register router"}} div.panelbody {(router_form(a, None))}}
        },
    )
}

fn hierarchy(a: &App, parent: &str, selected: Option<&Node>, plan: Option<&Plan>) -> Markup {
    html! {
        @for n in a.settings.nodes.iter().filter(|n| n.parent == parent) {
            div.hierarchybranch {
                a class=(if n.kind == "wan" {if selected.is_some_and(|s| s.id == n.id) {"wanbox active"} else {"wanbox"}} else if selected.is_some_and(|s| s.id == n.id) {"wanbranch active"} else {"wanbranch"}) href=(format!("/topology?selected={}", n.id)) {
                    strong {(&n.name)} small {(&n.kind) " · " (&n.mode)}
                    @if let Some(c) = plan.and_then(|p| p.capacities.get(&n.id)) {
                        small {(format!("{:.2}↓ / {:.2}↑ Mbps", c.download, c.upload))}
                        small {(c.subscribers) " snapshot subscribers"}
                    } @else if n.mode == "manual" {small {(n.download) "↓ / " (n.upload) "↑ Mbps"}}
                    @else {small {"Capacity preview unavailable"}}
                }
                div.hierarchychildren {(hierarchy(a, &n.id, selected, plan))}
            }
        }
    }
}

pub(crate) fn topology(
    a: &App,
    selected: &str,
    plan: Option<&Plan>,
    observation: Result<&Option<crate::sessions::Observation>, &String>,
) -> Html<String> {
    let selected = a
        .settings
        .nodes
        .iter()
        .find(|n| n.id == selected)
        .or_else(|| a.settings.nodes.first());
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    if let Ok(Some(observation)) = observation {
        let topology = crate::assignment::Topology::new(&a.settings).ok();
        for id in observation.active.keys() {
            if let Some(route) = observation.assignments.routes.get(id) {
                *counts.entry(route.wan.clone()).or_default() += 1;
            } else if let Some(pin) = observation.assignments.assignments.get(id)
                && let Some(t) = &topology
                && let Ok(wan) = t.root(&pin.node)
            {
                *counts.entry(wan.id.clone()).or_default() += 1;
            }
        }
    }
    shell(
        a,
        "Topology",
        html! {
            @if a.settings.dynamic.is_none(){(crate::dynamic_web::enable(a))}
            div.workbench {
                section.workpane {
                    div.workhead {"Discovered inventory" span.micro {"READ ONLY"}}
                    div.inventorylist {
                        input.search type="search" placeholder="Search inventory" aria-label="Search discovered inventory" data-filter="[data-inventory-row]";
                        @for router in &a.settings.routers {
                            @let (label, class) = inventory_status(a, router);
                            details open {
                                summary {(&router.name) " " span class=(class) {(label)}}
                                @if let Some(i) = a.inventories.get(&router.id) {
                                    @for (table, name, link) in [
                                        ("/interface/print", "Interfaces / WAN candidates", "/wans"),
                                        ("/ppp/profile/print", "PPP profiles", "/sources"),
                                        ("/ppp/secret/print", "PPP secrets", "/sources"),
                                        ("/ppp/active/print", "PPP active sessions", "/sources"),
                                        ("/ip/dhcp-server/print", "DHCP servers", "/sources"),
                                        ("/ip/dhcp-server/lease/print", "DHCP leases", "/sources"),
                                        ("/ip/hotspot/user/profile/print", "Hotspot profiles", "/sources"),
                                        ("/ip/hotspot/active/print", "Hotspot clients", "/sources"),
                                    ] {
                                        @if let Some(rows) = i.tables.get(table) {
                                            details {summary {(name) " · " (rows.len())}
                                                @for row in rows.iter().take(20) {
                                                    a.treeitem data-inventory-row href=(link) {(row.get("name").or_else(|| row.get("user")).or_else(|| row.get("active-address")).or_else(|| row.get("address")).map(String::as_str).unwrap_or("Unnamed record"))}
                                                }

                                                @if rows.len() > 20 {p.note {"First 20 records shown. Inspect the full table on Routers."}}
                                            }
                                        }
                                        @if let Some(error) = i.errors.get(table) {p.warning {(name) ": " (error)}}
                                    }
                                } @else {p.note {"Read router inventory to discover choices."}}
                            }
                        }
                        @if a.settings.routers.is_empty() {p.note {"Register a router and read its inventory."}}
                        p.note {"Discovery does not add shaping nodes. Select sources and WAN mappings explicitly."}
                        a.btn href="/sources" {"Source bindings"}
                    }
                }
                section.workpane {
                    div.workhead {"Network hierarchy" span.micro {"LOCAL PLAN"}}
                    div.workcenter {
                        div.toponote {span {"Exported shaping tree"} span {(a.settings.nodes.len()) " configured nodes"}}
                        div.corenode {(if a.settings.dynamic.is_some(){"Exported shaping roots"}else{"Configured WAN roots"})} div.corestem {}
                        div.wangrid {(hierarchy(a, "", selected, plan))}
                        @if a.settings.nodes.is_empty() {p.emptystate {"Create a WAN root, then add branches."}}
                        @if plan.is_none() && !a.settings.nodes.is_empty() {p.note {"Snapshot capacity preview unavailable. Configured capacities appear where available."}}
                        p.note {"The shaping hierarchy is separate from discovered inventory. Select a node to edit its configuration."}
                    }
                }
                section.workpane {
                    div.workhead {"Selected-item inspector" span.micro {"DETAILS"}}
                    div.inspect {
                        @if let Some(n) = selected {
                            div {
                                div.kind {(&n.kind)} h3 {(&n.name)} div.micro.mono {(&n.id)}
                                div.kv {span {"Parent"} strong {(if n.parent.is_empty() {"Independent root"} else {&n.parent})}}
                                div.kv {span {"Capacity mode"} strong {(&n.mode)}}
                                @if let Some(c) = plan.and_then(|p| p.capacities.get(&n.id)) {
                                    div.kv {span {"Effective down / up"} strong {(format!("{:.2} / {:.2} Mbps", c.download, c.upload))}}
                                    div.kv {span {"Snapshot subscribers"} strong {(c.subscribers)}}
                                }
                                div.kv {span {"Router"} strong {(&n.router)}}
                                div.kv {span {"Source / interface"} strong {(&n.source)}}
                            }
                            div {
                                details {summary {"Edit configuration"} (node_form(a, Some(n)))}
                                @if a.settings.dynamic.is_some(){(crate::dynamic_web::node_tools(a,n))} @else {
                                form method="post" action=(format!("/nodes/{}/delete", n.id)) {(csrf(a)) button.danger {"Remove node"}}}
                                p.note {"Removal keeps router records unchanged. Reassign subscribers and dependent branches before removal."}
                            }
                        } @else {p.note {"Select a node after creating the first WAN root."}}
                    }
                }
            }
            @match observation {
                Ok(Some(_)) => {
                    section.panel.sectiongap {
                        div.panelhead {strong{"Last observed count-mode sessions"} span.pill.blue{"CANDIDATE ONLY"}}
                        div.tablewrap {table {thead {tr {th{"WAN"}th{"Observed clients / bound leases"}}}tbody {
                            @for (wan,count) in counts {tr {td{(wan)}td{(count)}}}
                        }}}
                        div.tablefoot {"Saved source observations, not committed or applied routing. DHCP counts represent bound leases. " a href="/assignments"{"Configure pools and manual pins"}}
                    }
                },
                Err(error) => {p.warning{"Candidate session state unavailable: " (error)}},
                _=>{},
            }
            section.panel.sectiongap {
                div.panelhead {strong {"Configured shaping nodes"} a.btn href="#add-node" {"Add node"}}
                div.tablewrap {table {
                    thead {tr {@for label in ["Node", "Kind", "Parent", "Mode", "Configured down / up", "Source"] {th {(label)}}}}
                    tbody {@for n in &a.settings.nodes {tr {
                        td {a.strong href=(format!("/topology?selected={}", n.id)) {(&n.name)}} td {(&n.kind)} td {(&n.parent)} td {(&n.mode)}
                        td {@if n.mode == "manual" {(n.download) " / " (n.upload) " Mbps"} @else {"Computed"}} td {(&n.source)}
                    }}}
                }}
            }
        details.panel.sectiongap #add-node {summary.panelhead {strong {"Add shaping node"}} div.panelbody {(node_form(a, None))}}
        details.panel.sectiongap {summary.panelhead {strong{"Source selection and automatic grouping"}} div.panelbody{(crate::sources_web::selection_panel(a))}}
            section.card {h2 {"Generated output"}
                @if let Some(p) = plan {
                    details {summary {"network.json candidate preview"} pre {(&p.network)}}
                    details {summary {"ShapedDevices.csv candidate preview · first 20 data lines"} pre {(&p.csv)}}
                } @else {p.note {"A valid complete subscriber snapshot is required for the output preview."}}
            }
        },
    )
}
