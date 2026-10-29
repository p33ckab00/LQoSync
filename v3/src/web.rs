use crate::{model::*, store};
use axum::{
    Form, Router as HttpRouter,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use maud::{DOCTYPE, Markup, html};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

pub struct App {
    pub root: PathBuf,
    pub settings: Settings,
    pub token: String,
    pub status: String,
    pub generation: Option<String>,
}
pub type Shared = Arc<Mutex<App>>;
impl App {
    pub fn open(root: PathBuf) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&root)?;
        let root = root.canonicalize()?;
        anyhow::ensure!(
            !root.starts_with("/opt/libreqos") && !root.starts_with("/etc/libreqos"),
            "Application state must be separate from LibreQoS"
        );
        let settings = store::load_settings(&root)?;
        let generation = store::current_generation(&root);
        Ok(Self {
            root,
            settings,
            token: uuid::Uuid::new_v4().to_string(),
            status: "Waiting for a subscriber snapshot. Production adapters are not enabled."
                .into(),
            generation,
        })
    }
    pub fn reconcile(&mut self) {
        if !self.settings.auto_stage {
            self.status = "Local auto-stage is paused. Production adapters are not enabled.".into();
            return;
        }
        let result = (|| -> anyhow::Result<String> {
            let subscribers = store::load_snapshot(&self.root)?;
            anyhow::ensure!(
                !subscribers.is_empty(),
                "Waiting for a complete subscriber snapshot"
            );
            let p = compile(&self.settings, &subscribers)?;
            store::publish(&self.root, &p)
        })();
        match result {
            Ok(g) => {
                self.generation = Some(g);
                self.status =
                    "Local generation staged. No router or LibreQoS changes were sent.".into();
            }
            Err(e) => self.status = format!("Not staged: {e}. Last valid generation retained."),
        }
    }
}
pub fn routes(state: Shared) -> HttpRouter {
    HttpRouter::new()
        .route("/", get(overview))
        .route("/routers", get(routers))
        .route("/topology", get(topology))
        .route("/subscribers", get(subscribers))
        .route("/settings", get(settings))
        .route("/routers/save", post(save_router))
        .route("/routers/{id}/delete", post(delete_router))
        .route("/nodes/save", post(save_node))
        .route("/nodes/{id}/delete", post(delete_node))
        .route("/settings/save", post(save_options))
        .route("/health", get(|| async { "ok" }))
        .layer(axum::middleware::from_fn(headers))
        .with_state(state)
}
async fn headers(req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let trusted_host = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<axum::http::uri::Authority>().ok())
        .is_some_and(|a| matches!(a.host(), "localhost" | "127.0.0.1" | "[::1]"));
    if !trusted_host {
        return (
            StatusCode::FORBIDDEN,
            "Use the loopback address or an SSH tunnel.",
        )
            .into_response();
    }
    let mut r = next.run(req).await;
    r.headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    r.headers_mut().insert(header::CONTENT_SECURITY_POLICY,"default-src 'self'; style-src 'unsafe-inline'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'".parse().unwrap());
    r.headers_mut()
        .insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    r
}
const CSS: &str = include_str!("style.css");
fn shell(a: &App, title: &str, content: Markup) -> Html<String> {
    Html(html!{ (DOCTYPE) html lang="en" {head {meta charset="utf-8";meta name="viewport" content="width=device-width, initial-scale=1";title{(title) " · LQoSync 3"}style{(maud::PreEscaped(CSS))}}
        body {aside {a.brand href="/" {span.mark{"LQ"} div{"LQoSync" small{"VERSION 3 · RUST"}}}
            nav {@for (url,label) in [("/","Overview"),("/routers","Routers"),("/topology","Topology"),("/subscribers","Subscribers"),("/settings","Settings")] {a href=(url) class=(if label==title {"active"} else {""}) {(label)}}}
            div.railfoot {span.dot{} " Local development" p{"Production adapters disabled"}}}
        main {header {div {p.eyebrow{"NETWORK OPERATIONS"} h1{(title)}} span.badge{"LOCAL STAGING"}}
            div.notice {strong{"Production is untouched. "} "This console writes only inside its own data directory. Core writes and LibreQoS apply are not implemented yet."}
            (content) footer {"LQoSync 3 · Rust backend and Rust-rendered UI · " (a.root.display())}}
    }}}.into_string())
}
fn csrf(a: &App) -> Markup {
    html! {input type="hidden" name="token" value=(&a.token);}
}
fn field(label: &str, name: &str, value: &str, kind: &str) -> Markup {
    html! {label {(label) input name=(name) value=(value) type=(kind) required;}}
}
fn select(label: &str, name: &str, options: &[(&str, &str)], selected: &str) -> Markup {
    html! {label {(label) select name=(name) {@for (v,t) in options {option value=(v) selected[*v==selected] {(t)}}}}}
}
fn check(a: &App, token: &str) -> Result<(), (StatusCode, &'static str)> {
    if token == a.token {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            "Invalid form token. Reload the page.",
        ))
    }
}
fn error(a: &App, e: impl std::fmt::Display) -> Response {
    (StatusCode::UNPROCESSABLE_ENTITY,shell(a,"Validation error",html!{section.card {h2{"Settings were not changed"}p{(e)}a href="/topology"{"Return to topology"}}})).into_response()
}
fn commit(a: &mut App, next: Settings) -> anyhow::Result<()> {
    store::save_settings(&a.root, &next)?;
    a.settings = next;
    a.reconcile();
    Ok(())
}
async fn overview(State(s): State<Shared>) -> Html<String> {
    let a = s.lock().await;
    let count = store::load_snapshot(&a.root).map(|s| s.len()).unwrap_or(0);
    shell(
        &a,
        "Overview",
        html! {div.metrics {@for (label,value) in [("Managed routers",a.settings.routers.len()),("WAN roots",a.settings.nodes.iter().filter(|n|n.kind=="wan").count()),("Subscriber snapshot",count)] {section.card {p.eyebrow{(label)}div.number{(value)}}}}
            section.card {div.sectionhead{h2{"Automation pipeline"}span.badge{(if a.settings.auto_stage {"AUTO-STAGE ON"} else {"PAUSED"})}}
                div.pipeline {span{"Collect"}span{"→"}span{"Build topology"}span{"→"}span{"Stage pair"}span{"→"}span.muted{"Production apply · pending"}}
                p{(&a.status)}@if let Some(g)=&a.generation {p.mono{"Local generation: " (g)}}}
            div.grid {section.card {h2{"Start with your routers"}p{"Register core and subscriber routers. Credentials use environment references."}a.button href="/routers"{"Configure routers"}}
                section.card {h2{"Define the shaping hierarchy"}p{"Create WAN roots, then PPPoE, DHCP, Hotspot or site branches. Each branch has real capacity."}a.button href="/topology"{"Build topology"}}}
        },
    )
}
async fn routers(State(s): State<Shared>) -> Html<String> {
    let a = s.lock().await;
    shell(
        &a,
        "Routers",
        html! {section.card {div.sectionhead{h2{"Router inventory"}span.badge{(a.settings.routers.len()) " configured"}}
        @if a.settings.routers.is_empty(){p.empty{"No routers yet. Add a router below. No connection is made in this delivery."}}
        @for r in &a.settings.routers {details {summary {strong{(&r.name)} " · " (&r.host) " · " (&r.role) span.badge{"NOT CONNECTED"}}
            (router_form(&a,Some(r))) form.inline method="post" action=(format!("/routers/{}/delete",r.id)){(csrf(&a)) button.danger{"Remove router"}}}}
        }section.card {h2{"Add router"}(router_form(&a,None))}},
    )
}
fn router_form(a: &App, r: Option<&Router>) -> Markup {
    let value = |f: fn(&Router) -> &str| r.map(f).unwrap_or("");
    html! {form method="post" action="/routers/save" {(csrf(a)) input type="hidden" name="original_id" value=(value(|r|&r.id));div.formgrid{
        (field("Stable router ID","id",value(|r|&r.id),"text")) (field("Display name","name",value(|r|&r.name),"text"))
        (field("Host / IP","host",value(|r|&r.host),"text")) (field("API port","port",&r.map(|r|r.port).unwrap_or(8729).to_string(),"number"))
        (field("API username","username",value(|r|&r.username),"text")) (field("Password environment variable","password_env",value(|r|&r.password_env),"text"))
        (select("Router role","role",&[("core","Core / edge"),("distro","Subscriber router"),("both","Core and subscribers")],r.map(|r|r.role.as_str()).unwrap_or("distro")))
        (select("API transport","tls",&[("true","TLS"),("false","Plain TCP")],if r.is_none_or(|r|r.tls){"true"}else{"false"}))
    }p.help{"No router password is stored in TOML. Connection and discovery adapters are pending."}button{"Save router"}}}
}
#[derive(Deserialize)]
struct RouterForm {
    token: String,
    original_id: String,
    id: String,
    name: String,
    host: String,
    port: u16,
    username: String,
    password_env: String,
    role: String,
    tls: bool,
}
async fn save_router(State(s): State<Shared>, Form(f): Form<RouterForm>) -> Response {
    let mut a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    let mut next = a.settings.clone();
    if !f.original_id.is_empty() && f.id != f.original_id {
        return error(&a, "Stable IDs cannot be renamed");
    }
    let r = Router {
        id: f.id,
        name: f.name,
        host: f.host,
        port: f.port,
        username: f.username,
        password_env: f.password_env,
        role: f.role,
        tls: f.tls,
    };
    if f.original_id.is_empty() {
        next.routers.push(r);
    } else if let Some(old) = next.routers.iter_mut().find(|r| r.id == f.original_id) {
        *old = r;
    } else {
        return error(&a, "Router no longer exists");
    }
    match commit(&mut a, next) {
        Ok(()) => Redirect::to("/routers").into_response(),
        Err(e) => error(&a, e),
    }
}
#[derive(Deserialize)]
struct Token {
    token: String,
}
async fn delete_router(
    State(s): State<Shared>,
    Path(id): Path<String>,
    Form(f): Form<Token>,
) -> Response {
    let mut a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    let mut next = a.settings.clone();
    next.routers.retain(|r| r.id != id);
    match commit(&mut a, next) {
        Ok(()) => Redirect::to("/routers").into_response(),
        Err(e) => error(&a, e),
    }
}
async fn topology(State(s): State<Shared>) -> Html<String> {
    let a = s.lock().await;
    let snapshot = store::load_snapshot(&a.root).unwrap_or_default();
    let plan = compile(&a.settings, &snapshot).ok();
    shell(
        &a,
        "Topology",
        html! {section.card {div.sectionhead{h2{"Shaping tree"}span.badge{"REAL QUEUES ONLY"}}
            p.help{"The discovered network map is separate. Every exported node is a real shaping node."}
            @if a.settings.nodes.is_empty(){p.empty{"Create an explicit WAN root, then add subscriber branches."}}
            (node_tree(&a,"",plan.as_ref()))
            @if plan.is_none() && !a.settings.nodes.is_empty(){p.warning{(&a.status)}}
        }section.card {h2{"Add shaping node"}(node_form(&a,None))}
        section.card {h2{"Generated output"}@if let Some(p)=&plan {details{summary{"network.json preview"}pre{(&p.network)}}details{summary{"ShapedDevices.csv preview"}pre{(&p.csv)}}} @else {p{"A valid complete subscriber snapshot is required before computed branches can be published."}}}},
    )
}
fn node_tree(a: &App, parent: &str, plan: Option<&Plan>) -> Markup {
    html! {ul.tree {@for n in a.settings.nodes.iter().filter(|n|n.parent==parent) {li {details {summary {strong{(&n.name)} " · " (&n.kind) span.badge{(&n.mode)}
        @if let Some(c)=plan.and_then(|p|p.capacities.get(&n.id)) {span.rate{(format!("↓ {:.2}  ↑ {:.2} Mbps · {} subscribers",c.download,c.upload,c.subscribers))}}
        @else if n.mode=="manual" {span.rate{(format!("↓ {}  ↑ {} Mbps",n.download,n.upload))}}
        }
        @if let Some(c)=plan.and_then(|p|p.capacities.get(&n.id)) {p.help{(format!("Computed / explicit: {:.2}/{:.2} Mbps. Effective: {:.2}/{:.2} Mbps. Parent caps apply independently.",c.raw_download,c.raw_upload,c.download,c.upload))}}
        (node_form(a,Some(n))) form.inline method="post" action=(format!("/nodes/{}/delete",n.id)){(csrf(a))button.danger{"Remove node"}}}
        (node_tree(a,&n.id,plan))}}
    }}
}
fn node_form(a: &App, n: Option<&Node>) -> Markup {
    let value = |f: fn(&Node) -> &str| n.map(f).unwrap_or("");
    html! {form method="post" action="/nodes/save" {(csrf(a))input type="hidden" name="original_id" value=(value(|n|&n.id));div.formgrid{
        (field("Stable node ID","id",value(|n|&n.id),"text")) (field("Display name","name",value(|n|&n.name),"text"))
        label{"Parent" select name="parent" {option value="" selected[n.is_none_or(|n|n.parent.is_empty())]{"No parent · WAN root"}@for p in &a.settings.nodes { @if n.is_none_or(|n|n.id!=p.id) {option value=(&p.id) selected[n.is_some_and(|n|n.parent==p.id)]{(&p.name)}}}}}
        (select("Node type","kind",&[("wan","WAN root"),("pppoe","PPPoE profile"),("dhcp","DHCP server"),("hotspot","Hotspot"),("site","Site / backhaul")],n.map(|n|n.kind.as_str()).unwrap_or("wan")))
        label{"Source router" select name="router" {option value=""{"Unassigned"}@for r in &a.settings.routers {option value=(&r.id) selected[n.is_some_and(|n|n.router==r.id)]{(&r.name)}}}}
        label{"Source profile / server / interface" input name="source" value=(value(|n|&n.source));}
        (select("Capacity mode","mode",&[("manual","Manual / explicit"),("computed","Subscriber aggregate × factor")],n.map(|n|n.mode.as_str()).unwrap_or("manual")))
        @for (label,key,v) in [("Explicit download Mbps","download",n.map(|n|n.download).unwrap_or(0.0)),("Explicit upload Mbps","upload",n.map(|n|n.upload).unwrap_or(0.0)),("Download factor","download_factor",n.map(|n|n.download_factor).unwrap_or(1.0)),("Upload factor","upload_factor",n.map(|n|n.upload_factor).unwrap_or(1.0))] {label{(label)input type="number" step="any" min="0" name=(key) value=(v) required;}}
    }p.help{"Subscriber rates remain source-owned. Computed group capacities are capped at the immediate parent. Stable IDs are the exported JSON keys and CSV parents."}button{"Save node"}}}
}
#[derive(Deserialize)]
struct NodeForm {
    token: String,
    original_id: String,
    id: String,
    name: String,
    parent: String,
    kind: String,
    router: String,
    source: String,
    mode: String,
    download: f64,
    upload: f64,
    download_factor: f64,
    upload_factor: f64,
}
async fn save_node(State(s): State<Shared>, Form(f): Form<NodeForm>) -> Response {
    let mut a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    if !f.original_id.is_empty() && f.id != f.original_id {
        return error(&a, "Stable IDs cannot be renamed");
    }
    let mut next = a.settings.clone();
    let n = Node {
        id: f.id,
        name: f.name,
        parent: f.parent,
        kind: f.kind,
        router: f.router,
        source: f.source,
        mode: f.mode,
        download: f.download,
        upload: f.upload,
        download_factor: f.download_factor,
        upload_factor: f.upload_factor,
    };
    if f.original_id.is_empty() {
        next.nodes.push(n);
    } else if let Some(old) = next.nodes.iter_mut().find(|n| n.id == f.original_id) {
        *old = n;
    } else {
        return error(&a, "Node no longer exists");
    }
    // Validate explicit limits even before a subscriber snapshot exists.
    if let Err(e) = nodelimits(&next) {
        return error(&a, e);
    }
    match store::load_snapshot(&a.root) {
        Ok(rows) if !rows.is_empty() => {
            if let Err(e) = compile(&next, &rows) {
                return error(&a, e);
            }
        }
        Err(e) => return error(&a, e),
        _ => {}
    }
    match commit(&mut a, next) {
        Ok(()) => Redirect::to("/topology").into_response(),
        Err(e) => error(&a, e),
    }
}
fn nodelimits(s: &Settings) -> anyhow::Result<()> {
    validate(s)?;
    for n in &s.nodes {
        if n.mode == "manual" {
            let mut parent = n.parent.as_str();
            while !parent.is_empty() {
                let p = s.nodes.iter().find(|p| p.id == parent).unwrap();
                if p.mode == "manual" {
                    anyhow::ensure!(
                        n.download <= p.download && n.upload <= p.upload,
                        "Explicit capacity exceeds ancestor"
                    );
                }
                parent = &p.parent;
            }
        }
    }
    Ok(())
}
async fn delete_node(
    State(s): State<Shared>,
    Path(id): Path<String>,
    Form(f): Form<Token>,
) -> Response {
    let mut a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    let mut next = a.settings.clone();
    next.nodes.retain(|n| n.id != id);
    if store::load_snapshot(&a.root)
        .unwrap_or_default()
        .iter()
        .any(|s| s.node == id)
    {
        return error(&a, "Reassign subscribers before removing this node");
    }
    match commit(&mut a, next) {
        Ok(()) => Redirect::to("/topology").into_response(),
        Err(e) => error(&a, e),
    }
}
async fn subscribers(State(s): State<Shared>) -> Html<String> {
    let a = s.lock().await;
    let rows = store::load_snapshot(&a.root);
    shell(
        &a,
        "Subscribers",
        html! {
            section.card {
                h2 {"Source-owned subscriber snapshot"}
                p.help {"RouterOS collection is pending. This page reads the application-local state/subscribers.toml file. No sample subscribers are presented as live data."}
                @match rows {
                    Ok(rows) => {
                        @if rows.is_empty() {p.empty {"No subscriber snapshot yet."}}
                        @else {
                            div.scroll {
                                table {
                                    thead {tr {@for h in ["Circuit","Name","IPv4","Parent","Download","Upload"] {th {(h)}}}}
                                    tbody {
                                        @for s in rows {
                                            tr {
                                                td.mono {(&s.id)} td {(&s.name)} td.mono {(&s.ipv4)}
                                                td {(&s.node)} td {(s.download) " Mbps"} td {(s.upload) " Mbps"}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Err(e) => {p.warning {(e)}}
                }
            }
        },
    )
}
async fn settings(State(s): State<Shared>) -> Html<String> {
    let a = s.lock().await;
    shell(
        &a,
        "Settings",
        html! {section.card{h2{"Automation settings"}form method="post" action="/settings/save"{(csrf(&a))div.formgrid{(field("Snapshot check interval · seconds","interval_seconds",&a.settings.interval_seconds.to_string(),"number"))
            (select("Local automatic publication","auto_stage",&[("true","Enabled"),("false","Paused")],if a.settings.auto_stage{"true"}else{"false"}))}button{"Save settings"}}
            p.help{"Valid edits trigger local publication automatically. This delivery does not activate production auto-apply."}}
            section.card{h2{"Storage"}p.mono{(a.root.join("settings.toml").display())}p{"Settings and runtime state remain inside this application. No database is required."}}
        },
    )
}
#[derive(Deserialize)]
struct OptionsForm {
    token: String,
    interval_seconds: u64,
    auto_stage: bool,
}
async fn save_options(State(s): State<Shared>, Form(f): Form<OptionsForm>) -> Response {
    let mut a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    let mut next = a.settings.clone();
    next.interval_seconds = f.interval_seconds;
    next.auto_stage = f.auto_stage;
    match commit(&mut a, next) {
        Ok(()) => Redirect::to("/settings").into_response(),
        Err(e) => error(&a, e),
    }
}
