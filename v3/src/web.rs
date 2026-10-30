use crate::{model::*, store};
use axum::{
    Form, Router as HttpRouter,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use maud::{Markup, html};
use serde::Deserialize;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct App {
    pub root: PathBuf,
    pub settings: Settings,
    pub token: String,
    pub status: String,
    pub generation: Option<String>,
    pub inventories: BTreeMap<String, Arc<crate::routeros::Inventory>>,
    pub inventory_errors: BTreeMap<String, String>,
    pub read_epochs: BTreeMap<String, u64>,
    next_read: u64,
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
            inventories: BTreeMap::new(),
            inventory_errors: BTreeMap::new(),
            read_epochs: BTreeMap::new(),
            next_read: 0,
        })
    }
    pub fn begin_read(&mut self, id: &str) -> u64 {
        self.next_read += 1;
        self.read_epochs.insert(id.into(), self.next_read);
        self.next_read
    }
    pub fn record_inventory(
        &mut self,
        id: String,
        result: anyhow::Result<crate::routeros::Inventory>,
    ) {
        match result {
            Ok(inventory) => {
                let saved = toml::to_string_pretty(&inventory)
                    .map_err(anyhow::Error::from)
                    .and_then(|text| {
                        store::atomic_write(
                            &self
                                .root
                                .join("state/router-inventory")
                                .join(format!("{id}.toml")),
                            &text,
                        )
                    });
                if let Err(e) = saved {
                    self.inventory_errors
                        .insert(id, format!("Inventory could not be saved: {e}"));
                } else {
                    self.inventory_errors.remove(&id);
                    self.inventories.insert(id, Arc::new(inventory));
                }
            }
            Err(e) => {
                self.inventory_errors.insert(id, e.to_string());
            }
        }
    }
    pub fn reconcile(&mut self) {
        if !self.settings.auto_stage {
            self.status = "Local auto-stage is paused. Production adapters are not enabled.".into();
            return;
        }
        if crate::normalizer::has_bindings(&self.settings)
            || store::is_managed(&self.root).unwrap_or(true)
        {
            self.status =
                "Waiting for a fresh complete source collection. Last valid generation retained."
                    .into();
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
        .route("/wans", get(crate::wan_web::wans))
        .route("/wans/save", post(crate::wan_web::save_wan))
        .route("/assignments", get(crate::assignment_web::assignments))
        .route("/assignments/pool", post(crate::assignment_web::save_pool))
        .route("/assignments/pin", post(crate::assignment_web::save_pin))
        .route("/core-lists", get(crate::core_web::page))
        .route("/core-lists/save", post(crate::core_web::save))
        .route("/core-review", get(crate::core_review_web::page))
        .route("/validation", get(crate::validation_web::page))
        .route("/sources", get(crate::sources_web::sources))
        .route("/sources/save", post(crate::sources_web::save))
        .route(
            "/sources/selection",
            post(crate::sources_web::save_selection),
        )
        .route(
            "/sources/selection/{id}/delete",
            post(crate::sources_web::delete_selection),
        )
        .route("/routers/save", post(save_router))
        .route("/routers/{id}/delete", post(delete_router))
        .route("/routers/{id}/inspect", post(inspect_router))
        .route("/routers/{id}/speeds", get(crate::speed_web::router_speeds))
        .route("/speed-rules", get(crate::speed_web::rules))
        .route("/speed-rules/save", post(crate::speed_web::save_rules))
        .route("/speed-rules/dhcp", post(crate::dhcp_speed_web::save))
        .route("/nodes/save", post(save_node))
        .route("/nodes/{id}/delete", post(delete_node))
        .route("/settings/save", post(save_options))
        .route("/dynamic/enable", post(crate::dynamic_web::enable_dynamic))
        .route("/dynamic/node", post(crate::dynamic_web::node_edit))
        .route("/dynamic/policy", post(crate::dynamic_web::policy_edit))
        .route("/dynamic/wan", post(crate::dynamic_web::wan_edit))
        .route("/health", get(|| async { "ok" }))
        .route(
            "/assets/console.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("console.js"),
                )
            }),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::orchestrator::after_mutation,
        ))
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
pub(crate) fn shell(a: &App, title: &str, content: Markup) -> Html<String> {
    crate::noc_web::shell(a, title, content)
}
pub(crate) fn csrf(a: &App) -> Markup {
    html! {input type="hidden" name="token" value=(&a.token);}
}
fn field(label: &str, name: &str, value: &str, kind: &str) -> Markup {
    html! {label {(label) input name=(name) value=(value) type=(kind) required;}}
}
fn select(label: &str, name: &str, options: &[(&str, &str)], selected: &str) -> Markup {
    html! {label {(label) select name=(name) {@for (v,t) in options {option value=(v) selected[*v==selected] {(t)}}}}}
}
pub(crate) fn check(a: &App, token: &str) -> Result<(), (StatusCode, &'static str)> {
    if token == a.token {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            "Invalid form token. Reload the page.",
        ))
    }
}
pub(crate) fn error(a: &App, e: impl std::fmt::Display) -> Response {
    (StatusCode::UNPROCESSABLE_ENTITY,shell(a,"Validation error",html!{section.card {h2{"Settings were not changed"}p{(e)}a href="/topology"{"Return to topology"}}})).into_response()
}
fn commit(a: &mut App, next: Settings) -> anyhow::Result<()> {
    crate::dynamic_web::validate_evidence_edit(a, &next)?;
    store::save_settings(&a.root, &next)?;
    a.settings = next;
    a.reconcile();
    Ok(())
}
async fn overview(State(s): State<Shared>) -> Html<String> {
    let root = s.lock().await.root.clone();
    let count = tokio::task::spawn_blocking(move || {
        store::load_snapshot(&root).map(|s| s.len()).unwrap_or(0)
    })
    .await
    .unwrap_or(0);
    let a = s.lock().await;
    crate::noc_web::overview(&a, count)
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct Selection {
    selected: String,
}
async fn routers(State(s): State<Shared>, Query(selection): Query<Selection>) -> Html<String> {
    let a = s.lock().await;
    crate::noc_web::routers(&a, &selection.selected)
}

pub(crate) fn router_form(a: &App, r: Option<&Router>) -> Markup {
    let value = |f: fn(&Router) -> &str| r.map(f).unwrap_or("");
    html! {form method="post" action="/routers/save" {(csrf(a)) input type="hidden" name="original_id" value=(value(|r|&r.id));div.formgrid{
        (field("Stable router ID","id",value(|r|&r.id),"text")) (field("Display name","name",value(|r|&r.name),"text"))
        (field("Host / IP","host",value(|r|&r.host),"text")) (field("API port","port",&r.map(|r|r.port).unwrap_or(8729).to_string(),"number"))
        (field("API username","username",value(|r|&r.username),"text")) label{"Password environment reference · optional with credentials.toml" input name="password_env" value=(value(|r|&r.password_env));}
        (select("Router role","role",&[("core","Core / edge"),("distro","Subscriber router"),("both","Core and subscribers")],r.map(|r|r.role.as_str()).unwrap_or("distro")))
        (select("API transport","tls",&[("true","TLS"),("false","Plain TCP")],if r.is_none_or(|r|r.tls){"true"}else{"false"}))
        label{"TLS certificate name · optional" input name="tls_server_name" value=(value(|r|&r.tls_server_name));}
        label{"Custom CA PEM path · optional" input name="tls_ca_file" value=(value(|r|&r.tls_ca_file));}
        (field("Operation timeout · seconds","timeout_seconds",&r.map(|r|r.timeout_seconds).unwrap_or(10).to_string(),"number"))
    }p.help{"Read-only passwords can use this application's private credentials.toml, keyed by router ID, or an environment reference. Passwords are never displayed here. TLS validates certificates. Saving does not connect; Read inventory uses only allowlisted print commands."}button{"Save router"}}}
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
    #[serde(default)]
    tls_server_name: String,
    #[serde(default)]
    tls_ca_file: String,
    #[serde(default = "router_timeout")]
    timeout_seconds: u64,
}
fn router_timeout() -> u64 {
    10
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
    let router_id = f.id.clone();
    let r = Router {
        id: f.id,
        name: f.name,
        host: f.host,
        port: f.port,
        username: f.username,
        password_env: f.password_env,
        role: f.role,
        tls: f.tls,
        tls_server_name: f.tls_server_name,
        tls_ca_file: f.tls_ca_file,
        timeout_seconds: f.timeout_seconds,
    };
    if f.original_id.is_empty() {
        next.routers.push(r);
    } else if let Some(old) = next.routers.iter_mut().find(|r| r.id == f.original_id) {
        *old = r;
    } else {
        return error(&a, "Router no longer exists");
    }
    match commit(&mut a, next) {
        Ok(()) => {
            a.begin_read(&router_id);
            a.inventories.remove(&router_id);
            a.inventory_errors.remove(&router_id);
            Redirect::to("/routers").into_response()
        }
        Err(e) => error(&a, e),
    }
}
#[derive(Deserialize)]
struct Token {
    token: String,
}
async fn inspect_router(
    State(s): State<Shared>,
    Path(id): Path<String>,
    Form(f): Form<Token>,
) -> Response {
    let (router, ticket, root) = {
        let mut a = s.lock().await;
        if let Err(r) = check(&a, &f.token) {
            return r.into_response();
        }
        let Some(router) = a.settings.routers.iter().find(|r| r.id == id).cloned() else {
            return error(&a, "Router no longer exists");
        };
        let ticket = a.begin_read(&id);
        (router, ticket, a.root.clone())
    };
    let result = crate::routeros::inspect_from(&router, &root).await;
    let a = s.clone().lock_owned().await;
    if !a.settings.routers.contains(&router) || a.read_epochs.get(&id) != Some(&ticket) {
        return error(&a, "Router read was superseded; result discarded");
    }
    let _ = tokio::task::spawn_blocking(move || {
        let mut a = a;
        a.record_inventory(id, result);
    })
    .await;
    Redirect::to("/routers").into_response()
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
        Ok(()) => {
            a.begin_read(&id);
            a.inventories.remove(&id);
            a.inventory_errors.remove(&id);
            Redirect::to("/routers").into_response()
        }
        Err(e) => error(&a, e),
    }
}
async fn topology(State(s): State<Shared>, Query(selection): Query<Selection>) -> Html<String> {
    let (settings, root, generation) = {
        let a = s.lock().await;
        (a.settings.clone(), a.root.clone(), a.generation.clone())
    };
    let work_settings = settings.clone();
    let view = tokio::task::spawn_blocking(move || {
        let plan = store::load_snapshot(&root)
            .ok()
            .and_then(|snapshot| compile(&work_settings, &snapshot).ok());
        (
            plan,
            crate::sessions::load(&root).map_err(|e| e.to_string()),
        )
    })
    .await
    .unwrap_or_else(|e| (None, Err(e.to_string())));
    let (mut plan, mut observation) = view;
    if let Some(plan) = &mut plan {
        plan.csv = plan.csv.lines().take(21).collect::<Vec<_>>().join("\n");
    }
    let a = s.lock().await;
    if a.settings != settings || a.generation != generation {
        plan = None;
        observation = Ok(None);
    }
    crate::noc_web::topology(&a, &selection.selected, plan.as_ref(), observation.as_ref())
}

pub(crate) fn node_form(a: &App, n: Option<&Node>) -> Markup {
    let value = |f: fn(&Node) -> &str| n.map(f).unwrap_or("");
    html! {form method="post" action="/nodes/save" {(csrf(a))input type="hidden" name="original_id" value=(value(|n|&n.id)); input type="hidden" name="revision" value=(crate::topology_edits::revision(&a.settings).unwrap_or_default());div.formgrid{
        (field("Stable node ID","id",value(|n|&n.id),"text")) (field("Display name","name",value(|n|&n.name),"text"))
        label{"Parent" select name="parent" {option value="" selected[n.is_none_or(|n|n.parent.is_empty())]{(if a.settings.dynamic.is_some(){"Independent root"}else{"No parent · WAN root"})}@for p in &a.settings.nodes { @if n.is_none_or(|n|n.id!=p.id) {option value=(&p.id) selected[n.is_some_and(|n|n.parent==p.id)]{(&p.name)}}}}}
        (select("Node type","kind",&[("wan","WAN root"),("group","Custom group"),("pppoe","PPPoE profile"),("dhcp","DHCP server"),("hotspot","Hotspot"),("site","Site / backhaul")],n.map(|n|n.kind.as_str()).unwrap_or(if a.settings.dynamic.is_some(){"group"}else{"wan"})))
        @if a.settings.dynamic.is_none() {label{"Source router" select name="router" {option value=""{"Unassigned"}@for r in &a.settings.routers {option value=(&r.id) selected[n.is_some_and(|n|n.router==r.id)]{(&r.name)}}}}
        label{"Source profile / server / interface" input name="source" value=(value(|n|&n.source));}} @else {input type="hidden" name="router" value="";input type="hidden" name="source" value="";}
        (select("Capacity mode","mode",&[("manual","Manual / explicit"),("computed","Subscriber aggregate × factor")],n.map(|n|n.mode.as_str()).unwrap_or("manual")))
        @for (label,key,v) in [("Explicit download Mbps","download",n.map(|n|n.download).unwrap_or(0.0)),("Explicit upload Mbps","upload",n.map(|n|n.upload).unwrap_or(0.0)),("Download factor","download_factor",n.map(|n|n.download_factor).unwrap_or(1.0)),("Upload factor","upload_factor",n.map(|n|n.upload_factor).unwrap_or(1.0))] {label{(label)input type="number" step="any" min="0" name=(key) value=(v) required;}}
    }p.help{"Subscriber rates remain source-owned. Computed group capacities are capped at the immediate parent. Stable IDs are the exported JSON keys and CSV parents."}button{"Save node"}}}
}
#[derive(Deserialize)]
struct NodeForm {
    token: String,
    #[serde(default)]
    revision: String,
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
    if a.settings.dynamic.is_some()
        && f.revision != crate::topology_edits::revision(&a.settings).unwrap_or_default()
    {
        return error(&a, "Settings changed; reload before saving");
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
        Ok(mut rows) if !rows.is_empty() && next.dynamic.is_none() => {
            if crate::normalizer::has_bindings(&next)
                && let Err(e) = crate::minimums::allocate(&next, &mut rows)
            {
                return error(&a, e);
            }
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
pub(crate) fn nodelimits(s: &Settings) -> anyhow::Result<()> {
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
    if next.dynamic.is_some() {
        return error(
            &a,
            "Use Remove with explicit replacements in the node inspector",
        );
    }
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
    let root = s.lock().await.root.clone();
    let rows = tokio::task::spawn_blocking(move || {
        store::load_snapshot(&root).map(|rows| {
            let count = rows.len();
            (count, rows.into_iter().take(200).collect::<Vec<_>>())
        })
    })
    .await
    .unwrap_or_else(|e| Err(e.into()));
    let a = s.lock().await;
    shell(
        &a,
        "Subscribers",
        html! {
            section.card {
                h2 {"Source-owned subscriber snapshot"}
                p{(&a.status)}
                p.help{"Showing at most 200 records from the last committed generation."}
                p.help {"This page reads the last committed local subscriber snapshot. Selected router sources update it automatically after a complete valid collection. Partial or invalid scans retain the last valid snapshot."}
                @match rows {
                    Ok((count,rows)) => {
                        p{(count) " subscribers in the committed snapshot"}
                        @if rows.is_empty() {p.empty {"No subscriber snapshot yet."}}
                        @else {
                            div.scroll {
                                table {
                                    thead {tr {@for h in ["Circuit","Name","IPv4","Parent","Download","Upload"] {th {(h)}}}}
                                    tbody {
                                        @for s in rows.iter().take(200) {
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
    let detection = crate::libreqos::detect().await;
    let a = s.lock().await;
    shell(
        &a,
        "Settings",
        html! {div.cardgrid {section.card{h2{"LibreQoS compatibility"}
            p{"Detected main package: " strong{(detection.version.as_deref().unwrap_or("Unavailable"))}}
            @if let Some(adapter)=&detection.adapter {p{strong{(adapter.label())}}p.mono{(adapter.request_description())}}
            p.help{(&detection.detail)}p.help{"Detection is read-only. The preflight package version does not determine the running LibreQoS version. Older releases outside 2.0–2.2 and future major releases require compatibility validation."}
        }
        section.card{h2{"Automation settings"}form method="post" action="/settings/save"{(csrf(&a))div.formgrid{(field("Snapshot check interval · seconds","interval_seconds",&a.settings.interval_seconds.to_string(),"number"))
            (select("Local automatic publication","auto_stage",&[("true","Enabled"),("false","Paused")],if a.settings.auto_stage{"true"}else{"false"}))}button{"Save settings"}}
            p.help{"Valid edits trigger local publication automatically. This delivery does not activate production auto-apply."}}
            section.card{h2{"Storage"}p.mono{(a.root.join("settings.toml").display())}p{"Settings and runtime state remain inside this application. No database is required."}}
        }},
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
