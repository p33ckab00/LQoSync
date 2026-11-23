//! Topology Studio HTTP routes. Typed requests only: the browser never names a service operation.
//! Errors carry `StudioError` with the HTTP status.
use crate::broker::{Request as BrokerRequest, exchange};
use crate::http::{WebState, principal, queued, writer};
use axum::{
    Json,
    body::Bytes,
    extract::{State, rejection::BytesRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use web_contract::source_studio::{StudioError, StudioJob, StudioJobRequest};

fn error(status: StatusCode, code: &str, message: &str) -> Response {
    let body = StudioError {
        code: code.into(),
        message: message.into(),
        field: None,
    };
    (status, Json(body)).into_response()
}

fn status_error(status: StatusCode) -> Response {
    let (code, message) = match status {
        StatusCode::BAD_REQUEST => ("invalid", "Check the request."),
        StatusCode::UNAUTHORIZED => ("unauthorized", "Sign in again."),
        StatusCode::FORBIDDEN => (
            "forbidden",
            "The page has no valid session token. Load it again.",
        ),
        StatusCode::CONFLICT => ("key_reused", "Use a new key for a changed request."),
        StatusCode::GONE => ("expired", "The result expired. Start it again."),
        StatusCode::TOO_MANY_REQUESTS => ("busy", "Too many jobs are open. Try again soon."),
        _ => (
            "unavailable",
            "The service did not answer. Check the operation before you send it again.",
        ),
    };
    error(status, code, message)
}

/// `GET /api/v1/topology/source-capabilities`. It starts no router read.
pub(crate) async fn capabilities(State(s): State<WebState>, h: HeaderMap) -> Response {
    if let Err(status) = principal(&s, &h).await {
        return status_error(status);
    }
    match exchange(&s.broker, &BrokerRequest::StudioCapabilities).await {
        Ok(r) if r.ok => match r.capabilities {
            Some(c) => Json(c).into_response(),
            None => error(
                StatusCode::NOT_FOUND,
                "unavailable",
                "This service has no Topology Studio operations.",
            ),
        },
        _ => status_error(StatusCode::SERVICE_UNAVAILABLE),
    }
}

/// `POST /api/v1/topology/source-jobs`: `202 Operation` with its `Location`.
pub(crate) async fn jobs(
    State(s): State<WebState>,
    h: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let p = match writer(&s, &h).await {
        Ok(p) => p,
        Err(status) => return status_error(status),
    };
    let body = match body {
        Ok(b) => b,
        Err(e) if e.status() == StatusCode::PAYLOAD_TOO_LARGE => {
            return error(
                e.status(),
                "too_large",
                "The request is larger than 32 KiB.",
            );
        }
        Err(e) => return error(e.status(), "invalid", "Check the source job."),
    };
    let Ok(request) = serde_json::from_slice::<StudioJobRequest>(&body) else {
        return error(StatusCode::BAD_REQUEST, "invalid", "Check the source job.");
    };
    // Collection and evaluation arrive with plan tasks B2 and B3.
    if !matches!(request.job, StudioJob::DraftTest { .. }) {
        return error(
            StatusCode::NOT_FOUND,
            "unavailable",
            "This service cannot run this job yet.",
        );
    }
    let request = BrokerRequest::StudioJob {
        actor: p.username,
        request,
    };
    queued(&s, &request).await.unwrap_or_else(status_error)
}
