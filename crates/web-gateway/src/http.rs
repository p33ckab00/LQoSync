//! Same-origin HTTP surface. Accounts never have CRUD routes.
use crate::{
    broker::{Request as BrokerRequest, exchange, exchange_within},
    sessions::{Session, Sessions},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{
        Html, IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post, put},
};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tower_http::services::ServeDir;
use web_contract::Principal;
use web_contract::routers::{RouterEditForm, RouterEditOutcome};
#[derive(Clone)]
pub struct WebState {
    pub broker: PathBuf,
    pub sessions: Sessions,
    pub origin: String,
    pub secure: bool,
    pub index: Arc<String>,
}
type ApiResult = Result<Response, StatusCode>;
fn token(h: &HeaderMap) -> &str {
    h.get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .split(';')
        .map(str::trim)
        .find_map(|p| p.strip_prefix("lqosync_session="))
        .unwrap_or("")
}
fn same_origin(s: &WebState, h: &HeaderMap) -> bool {
    h.get(header::ORIGIN).and_then(|v| v.to_str().ok()) == Some(&s.origin)
}
pub(crate) async fn principal(s: &WebState, h: &HeaderMap) -> Result<Session, StatusCode> {
    let sessions = s.sessions.clone();
    let key = token(h).to_owned();
    let record = tokio::task::spawn_blocking(move || sessions.get(&key))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let active = exchange(
        &s.broker,
        &BrokerRequest::Active {
            id: record.id,
            epoch: record.epoch,
        },
    )
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if !active.ok {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(record)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Login {
    username: String,
    password: String,
}
async fn login(State(s): State<WebState>, h: HeaderMap, Json(body): Json<Login>) -> ApiResult {
    if !same_origin(&s, &h) {
        return Err(StatusCode::FORBIDDEN);
    }
    if body.username.len() > 64 || body.password.len() > 256 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let reply = exchange(
        &s.broker,
        &BrokerRequest::Login {
            username: body.username,
            password: body.password,
        },
    )
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let a = reply.account.ok_or(StatusCode::UNAUTHORIZED)?;
    let username = a.username.clone();
    let db = s.sessions.clone();
    let (session, csrf) = tokio::task::spawn_blocking(move || db.create(&a))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let cookie = format!(
        "lqosync_session={session}; Path=/; HttpOnly; SameSite=Strict; Max-Age=28800{}",
        if s.secure { "; Secure" } else { "" }
    );
    Ok((
        [(header::SET_COOKIE, cookie)],
        Json(Principal { username, csrf }),
    )
        .into_response())
}
async fn current(State(s): State<WebState>, h: HeaderMap) -> ApiResult {
    let p = principal(&s, &h).await?;
    Ok(Json(Principal {
        username: p.username,
        csrf: p.csrf,
    })
    .into_response())
}
async fn logout(State(s): State<WebState>, h: HeaderMap) -> ApiResult {
    if !same_origin(&s, &h) {
        return Err(StatusCode::FORBIDDEN);
    }
    let p = principal(&s, &h).await?;
    if h.get("x-csrf-token").and_then(|v| v.to_str().ok()) != Some(&p.csrf) {
        return Err(StatusCode::FORBIDDEN);
    }
    let key = token(&h).to_owned();
    let db = s.sessions.clone();
    tokio::task::spawn_blocking(move || db.remove(&key))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok((
        [(
            header::SET_COOKIE,
            "lqosync_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
        )],
        StatusCode::NO_CONTENT,
    )
        .into_response())
}
async fn overview(State(s): State<WebState>, h: HeaderMap) -> ApiResult {
    principal(&s, &h).await?;
    let reply = exchange(&s.broker, &BrokerRequest::Overview)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let view = reply.overview.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(view).into_response())
}
#[derive(serde::Deserialize)]
struct WindowQuery {
    window: String,
}
/// One bucketed history window for the traffic chart. Only `1h`, `6h`, and `24h` exist.
async fn traffic(
    State(s): State<WebState>,
    h: HeaderMap,
    Query(q): Query<WindowQuery>,
) -> ApiResult {
    principal(&s, &h).await?;
    if !crate::history::WINDOWS
        .iter()
        .any(|(name, _)| *name == q.window)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let reply = exchange(&s.broker, &BrokerRequest::Traffic { window: q.window })
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let window = reply.traffic.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(window).into_response())
}
/// The cached router inventory. The service writes it each cycle; this read starts no router I/O.
async fn routers(State(s): State<WebState>, h: HeaderMap) -> ApiResult {
    principal(&s, &h).await?;
    let reply = exchange(&s.broker, &BrokerRequest::Routers)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let inventory = reply.routers.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(inventory).into_response())
}
/// A state-changing request: same origin, a current session, and its CSRF token.
pub(crate) async fn writer(s: &WebState, h: &HeaderMap) -> Result<Session, StatusCode> {
    if !same_origin(s, h) {
        return Err(StatusCode::FORBIDDEN);
    }
    let p = principal(s, h).await?;
    if h.get("x-csrf-token").and_then(|v| v.to_str().ok()) != Some(&p.csrf) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(p)
}
/// A router ID from the inventory: `lq3-` and hex digits.
pub(crate) fn router_id(id: &str) -> bool {
    id.len() <= 64 && id.starts_with("lq3-") && id[4..].bytes().all(|b| b.is_ascii_hexdigit())
}
/// Save a local router change. The core command checks and writes it; no router I/O.
async fn edit_router(
    s: &WebState,
    h: &HeaderMap,
    router_id: Option<String>,
    form: RouterEditForm,
) -> ApiResult {
    let p = writer(s, h).await?;
    let request = BrokerRequest::RouterEdit {
        actor: p.username,
        router_id,
        form: Box::new(form),
    };
    let reply = exchange_within(&s.broker, &request, std::time::Duration::from_secs(28))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let outcome: RouterEditOutcome = reply.edit.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let status = match outcome.code.as_str() {
        "ok" => StatusCode::OK,
        "conflict" => StatusCode::CONFLICT,
        "invalid" => StatusCode::UNPROCESSABLE_ENTITY,
        "not_found" => StatusCode::NOT_FOUND,
        // The body code tells the page to ask for the password; 401 would end the session.
        "reauth" => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    Ok((status, Json(outcome)).into_response())
}
async fn create_router(
    State(s): State<WebState>,
    h: HeaderMap,
    Json(form): Json<RouterEditForm>,
) -> ApiResult {
    edit_router(&s, &h, None, form).await
}
async fn update_router(
    State(s): State<WebState>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(form): Json<RouterEditForm>,
) -> ApiResult {
    if !router_id(&id) {
        return Err(StatusCode::NOT_FOUND);
    }
    edit_router(&s, &h, Some(id), form).await
}
/// One connection test. The service runs it; a disabled router gets no connection.
async fn test_router(State(s): State<WebState>, h: HeaderMap, Path(id): Path<String>) -> ApiResult {
    let p = writer(&s, &h).await?;
    if !router_id(&id) {
        return Err(StatusCode::NOT_FOUND);
    }
    let request = BrokerRequest::RouterTest {
        actor: p.username,
        router_id: id,
    };
    let reply = exchange_within(&s.broker, &request, std::time::Duration::from_secs(70))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(reply.test.ok_or(StatusCode::SERVICE_UNAVAILABLE)?).into_response())
}
/// Start a cycle now for all enabled routers (SIGUSR1 to the service).
async fn sync_routers(State(s): State<WebState>, h: HeaderMap) -> ApiResult {
    writer(&s, &h).await?;
    let reply = exchange_within(
        &s.broker,
        &BrokerRequest::SyncNow,
        std::time::Duration::from_secs(25),
    )
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let outcome = reply.edit.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let status = if outcome.code == "ok" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    Ok((status, Json(outcome)).into_response())
}
#[derive(serde::Deserialize)]
struct JournalQuery {
    unit: String,
    priority: Option<u8>,
    before: Option<u64>,
}
/// One page of a service log, read-only. Only allowlisted units.
async fn journal(
    State(s): State<WebState>,
    h: HeaderMap,
    Query(q): Query<JournalQuery>,
) -> ApiResult {
    principal(&s, &h).await?;
    let priority = q.priority.unwrap_or(6);
    if !crate::journal::allowed(&q.unit) || priority > 7 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let request = BrokerRequest::Journal {
        unit: q.unit,
        priority,
        before_usec: q.before,
    };
    let reply = exchange_within(&s.broker, &request, std::time::Duration::from_secs(15))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(reply.journal.ok_or(StatusCode::SERVICE_UNAVAILABLE)?).into_response())
}
/// A topology read from the service cache. It starts no router I/O and no mutation.
async fn topology_read(s: &WebState, h: &HeaderMap, read: crate::topology::Read) -> ApiResult {
    use web_contract::topology::TopologyReply;
    principal(s, h).await?;
    if !read.valid() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let reply = exchange(&s.broker, &BrokerRequest::Topology { read })
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(
        match reply.topology.ok_or(StatusCode::SERVICE_UNAVAILABLE)? {
            TopologyReply::Summary(v) => Json(v).into_response(),
            TopologyReply::Assignments(v) => Json(v).into_response(),
            TopologyReply::Observations(v) => Json(v).into_response(),
            TopologyReply::Subscriber(v) => Json(v).into_response(),
            TopologyReply::Expired => return Err(StatusCode::CONFLICT),
            TopologyReply::NotFound => return Err(StatusCode::NOT_FOUND),
        },
    )
}
/// Send one queue request to the broker: `202` with the operation and its URL, or the refusal.
pub(crate) async fn queued(s: &WebState, request: &BrokerRequest) -> ApiResult {
    let reply = exchange(&s.broker, request)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    match (reply.operation, reply.refused.as_deref()) {
        (Some(op), _) => {
            let location = format!("/api/v1/topology/operations/{}", op.key);
            let mut r = (StatusCode::ACCEPTED, Json(op)).into_response();
            if let Ok(v) = axum::http::HeaderValue::from_str(&location) {
                r.headers_mut().insert(header::LOCATION, v);
            }
            Ok(r)
        }
        (None, Some("key_reused")) => Err(StatusCode::CONFLICT),
        (None, Some("busy")) => Err(StatusCode::TOO_MANY_REQUESTS),
        (None, Some("invalid")) => Err(StatusCode::BAD_REQUEST),
        (None, Some("expired")) => Err(StatusCode::GONE),
        _ => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}
/// Ask for the impact review of one Builder edit. The review arrives on the operation.
async fn topology_propose(
    State(s): State<WebState>,
    h: HeaderMap,
    Json(request): Json<web_contract::topology::ProposalRequest>,
) -> ApiResult {
    let p = writer(&s, &h).await?;
    queued(
        &s,
        &BrokerRequest::TopologyPropose {
            actor: p.username,
            request,
        },
    )
    .await
}
/// Save one canvas edit automatically. The service validates it; there is no review step.
async fn topology_edit(
    State(s): State<WebState>,
    h: HeaderMap,
    Json(request): Json<web_contract::topology::EditRequest>,
) -> ApiResult {
    let p = writer(&s, &h).await?;
    queued(
        &s,
        &BrokerRequest::TopologyEdit {
            actor: p.username,
            request,
        },
    )
    .await
}
/// Save all canvas card positions: `409` when the layout changed. No shaping change.
async fn topology_layout(
    State(s): State<WebState>,
    h: HeaderMap,
    Json(request): Json<web_contract::topology::LayoutRequest>,
) -> ApiResult {
    writer(&s, &h).await?;
    let reply = exchange(&s.broker, &BrokerRequest::TopologyLayout { request })
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    match (reply.layout, reply.refused.as_deref()) {
        (Some(layout), _) => Ok(Json(layout).into_response()),
        (None, Some("conflict")) => Err(StatusCode::CONFLICT),
        (None, Some("invalid")) => Err(StatusCode::BAD_REQUEST),
        _ => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}
/// Start one explicit router read (refresh or discovery). Only this POST reads routers.
async fn topology_reads(
    State(s): State<WebState>,
    h: HeaderMap,
    Json(request): Json<web_contract::topology::ReadRequest>,
) -> ApiResult {
    let p = writer(&s, &h).await?;
    queued(
        &s,
        &BrokerRequest::TopologyRead {
            actor: p.username,
            request,
        },
    )
    .await
}
/// Save a reviewed proposal of this account. `410` when it expired or was replaced.
async fn topology_confirm(
    State(s): State<WebState>,
    h: HeaderMap,
    Path(proposal): Path<String>,
    Json(body): Json<web_contract::topology::ConfirmRequest>,
) -> ApiResult {
    let p = writer(&s, &h).await?;
    if proposal.len() > 64 {
        return Err(StatusCode::BAD_REQUEST);
    }
    queued(
        &s,
        &BrokerRequest::TopologyConfirm {
            actor: p.username,
            proposal,
            key: body.key,
        },
    )
    .await
}
async fn topology_cancel(
    State(s): State<WebState>,
    h: HeaderMap,
    Path(proposal): Path<String>,
) -> ApiResult {
    let p = writer(&s, &h).await?;
    if proposal.len() > 64 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let request = BrokerRequest::TopologyCancel {
        actor: p.username,
        proposal,
    };
    let reply = exchange(&s.broker, &request)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(if reply.ok {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::GONE
    }
    .into_response())
}
/// The state of one topology change of this account.
async fn topology_operation(
    State(s): State<WebState>,
    h: HeaderMap,
    Path(key): Path<String>,
) -> ApiResult {
    let p = principal(&s, &h).await?;
    if key.len() > 64 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let request = BrokerRequest::TopologyOperation {
        actor: p.username,
        key,
    };
    let reply = exchange(&s.broker, &request)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let op = reply.operation.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if op.state == "not_found" {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(op).into_response())
}
async fn topology(State(s): State<WebState>, h: HeaderMap) -> ApiResult {
    topology_read(&s, &h, crate::topology::Read::Summary).await
}
async fn topology_assignments(
    State(s): State<WebState>,
    h: HeaderMap,
    Query(q): Query<web_contract::topology::PageQuery>,
) -> ApiResult {
    topology_read(&s, &h, crate::topology::Read::Assignments(q)).await
}
async fn topology_observations(
    State(s): State<WebState>,
    h: HeaderMap,
    Query(q): Query<web_contract::topology::PageQuery>,
) -> ApiResult {
    topology_read(&s, &h, crate::topology::Read::Observations(q)).await
}
async fn topology_subscriber(
    State(s): State<WebState>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult {
    topology_read(&s, &h, crate::topology::Read::Subscriber(id)).await
}
/// One router with its lists, from the service cache.
async fn router_detail(
    State(s): State<WebState>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult {
    principal(&s, &h).await?;
    if !router_id(&id) {
        return Err(StatusCode::NOT_FOUND);
    }
    let reply = exchange(&s.broker, &BrokerRequest::RouterDetail { router_id: id })
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(reply.router.ok_or(StatusCode::NOT_FOUND)?).into_response())
}
async fn router_activity(
    State(s): State<WebState>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult {
    principal(&s, &h).await?;
    if !router_id(&id) {
        return Err(StatusCode::NOT_FOUND);
    }
    let reply = exchange(&s.broker, &BrokerRequest::RouterActivity { router_id: id })
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let activity = reply.activity.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(activity).into_response())
}
async fn live(State(s): State<WebState>, h: HeaderMap) -> ApiResult {
    principal(&s, &h).await?;
    // Slow clients own at most one snapshot; there is no per-client event queue.
    static STREAMS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(16);
    let permit = STREAMS
        .try_acquire()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(1));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let stream = futures_util::stream::unfold(
        (s, h, tick, permit, false),
        |(s, h, mut tick, permit, ended)| async move {
            if ended {
                return None;
            }
            tick.tick().await;
            let payload = match principal(&s, &h).await {
                Err(StatusCode::UNAUTHORIZED) => "unauthorized".to_owned(),
                Err(_) => "unavailable".to_owned(),
                Ok(_) => match exchange(&s.broker, &BrokerRequest::Overview).await {
                    Ok(reply) => reply
                        .overview
                        .and_then(|v| serde_json::to_string(&v).ok())
                        .unwrap_or_else(|| "unavailable".into()),
                    Err(_) => "unavailable".into(),
                },
            };
            let ended = matches!(payload.as_str(), "unauthorized" | "unavailable");
            Some((
                Ok::<_, std::convert::Infallible>(Event::default().data(payload)),
                (s, h, tick, permit, ended),
            ))
        },
    );
    Ok((
        [("x-accel-buffering", "no")],
        Sse::new(stream).keep_alive(KeepAlive::default()),
    )
        .into_response())
}
async fn index(State(s): State<WebState>, h: HeaderMap) -> Html<String> {
    let theme = h
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .map(str::trim)
        .find_map(|v| v.strip_prefix("lqosync_theme="))
        .filter(|v| matches!(*v, "light" | "dark"))
        .unwrap_or("system");
    Html(
        s.index
            .replace("data-theme=\"system\"", &format!("data-theme=\"{theme}\"")),
    )
}
async fn headers(request: Request, next: Next) -> Response {
    let cache_asset = request.uri().path().starts_with("/assets/")
        && !request.uri().path().ends_with("bootstrap.js")
        && !request.uri().path().ends_with("index.html");
    let mut r = next.run(request).await;
    let success = r.status().is_success();
    let h = r.headers_mut();
    for (k, v) in [
        ("cache-control", "no-store"),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "same-origin"),
        (
            "content-security-policy",
            "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
    ] {
        h.insert(
            axum::http::HeaderName::from_static(k),
            axum::http::HeaderValue::from_static(v),
        );
    }
    if cache_asset && success {
        h.insert(
            header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("public, max-age=3600"),
        );
    }
    r
}
pub fn router(state: WebState, dist: PathBuf) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/v1/session", get(current).post(login).delete(logout))
        .route("/api/v1/overview", get(overview))
        .route("/api/v1/overview/live", get(live))
        .route("/api/v1/traffic", get(traffic))
        .route("/api/v1/journal", get(journal))
        // A router edit can carry a CA certificate, so its body limit is larger.
        .route(
            "/api/v1/routers",
            get(routers)
                .post(create_router)
                .layer(DefaultBodyLimit::max(32 * 1024)),
        )
        .route(
            "/api/v1/routers/{id}",
            get(router_detail)
                .patch(update_router)
                .layer(DefaultBodyLimit::max(64 * 1024)),
        )
        .route("/api/v1/routers/{id}/activity", get(router_activity))
        .route(
            "/api/v1/routers/{id}/source-config",
            get(crate::source_studio::config),
        )
        .route("/api/v1/routers/{id}/test", post(test_router))
        .route("/api/v1/routers/sync", post(sync_routers))
        .route("/api/v1/topology", get(topology))
        .route("/api/v1/topology/proposals", post(topology_propose))
        .route("/api/v1/topology/reads", post(topology_reads))
        // A source policy set is larger than other edits; other routes keep 4 KiB.
        .route(
            "/api/v1/topology/edits",
            post(topology_edit).layer(DefaultBodyLimit::max(32 * 1024)),
        )
        .route("/api/v1/topology/layout", put(topology_layout))
        .route(
            "/api/v1/topology/proposals/{key}/confirm",
            post(topology_confirm),
        )
        .route(
            "/api/v1/topology/proposals/{key}/cancel",
            post(topology_cancel),
        )
        .route("/api/v1/topology/operations/{key}", get(topology_operation))
        .route(
            "/api/v1/topology/source-candidates/{id}",
            get(crate::source_studio::candidate),
        )
        .route(
            "/api/v1/topology/source-candidates/{id}/records",
            get(crate::source_studio::candidate_records),
        )
        .route(
            "/api/v1/topology/source-candidates/{id}/issues",
            get(crate::source_studio::candidate_issues),
        )
        .route(
            "/api/v1/topology/source-capabilities",
            get(crate::source_studio::capabilities),
        )
        .route(
            "/api/v1/topology/source-jobs",
            post(crate::source_studio::jobs).layer(DefaultBodyLimit::max(
                web_contract::source_studio::MAX_JOB_BODY,
            )),
        )
        .route("/api/v1/topology/assignments", get(topology_assignments))
        .route("/api/v1/topology/observations", get(topology_observations))
        .route(
            "/api/v1/topology/subscribers/{id}",
            get(topology_subscriber),
        )
        // Browser routes of the single-page UI. The page itself loads data through the API.
        .route("/routers", get(index))
        .route("/routers/{*rest}", get(index))
        .route("/workers", get(index))
        .route("/journal", get(index))
        .route("/topology", get(index))
        .route("/pppoe", get(index))
        .route("/ipoe", get(index))
        .route(
            "/api/{*path}",
            get(|| async { StatusCode::NOT_FOUND })
                .post(|| async { StatusCode::NOT_FOUND })
                .put(|| async { StatusCode::NOT_FOUND })
                .patch(|| async { StatusCode::NOT_FOUND })
                .delete(|| async { StatusCode::NOT_FOUND }),
        )
        .nest_service("/assets", ServeDir::new(dist).precompressed_gzip())
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::from_fn(headers))
        .with_state(state)
}
