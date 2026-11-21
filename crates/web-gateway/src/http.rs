//! Same-origin HTTP surface. Accounts never have CRUD routes.
use crate::{
    broker::{Request as BrokerRequest, exchange},
    sessions::{Session, Sessions},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{
        Html, IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tower_http::services::ServeDir;
use web_contract::Principal;
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
async fn principal(s: &WebState, h: &HeaderMap) -> Result<Session, StatusCode> {
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
        .route("/api/v1/routers", get(routers))
        // Browser routes of the single-page UI. The page itself loads data through the API.
        .route("/routers", get(index))
        .route("/routers/{*rest}", get(index))
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
