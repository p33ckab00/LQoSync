use crate::{
    core_recovery::{self, Report},
    web::{Shared, shell},
};
use axum::{
    Router,
    extract::State,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use maud::{DOCTYPE, Markup, html};
use std::{path::PathBuf, sync::Arc};
fn operation(value: Option<&crate::core_writer::OperationReview>) -> String {
    value
        .map(|v| {
            format!(
                "{} · {} · {} · {} · {} → {}",
                v.kind,
                v.router,
                v.row_id.as_deref().unwrap_or("No returned ID"),
                v.address,
                v.from_list.as_deref().unwrap_or("—"),
                v.to_list.as_deref().unwrap_or("—")
            )
        })
        .unwrap_or_else(|| "None recorded".into())
}
fn content(report: Result<Report, anyhow::Error>) -> Markup {
    html! {
        section.card {h2 {"Read-only recovery inspection"}
            p {"Router mutations, recovery approval, resume, rollback, and marker removal are locked."}
            p.help {"Saved records are historical. No live router verification runs here. Known replies do not prove post-write verification or current membership."}
            button type="button" disabled aria-describedby="recovery-lock" {"Approve / resume — locked"}
            p.help id="recovery-lock" {"Isolated RouterOS acceptance and a separately reviewed recovery grant are required."}
        }
        @match report {
            Err(_)=>{section.card {h2 {"Inspection unavailable"}p {"Cannot safely inspect recovery records. No action is authorized. Check local file access and inspection limits."}}}
            Ok(report)=>{
                section.card {h2 {"Recovery marker"}p {(report.marker)}
                    @if let Some(revision)=report.active_revision {p {"Active writer revision: " (revision)}}
                    @if report.limited {p {"Inspection is limited to 16 journals, 128 directory entries, and 64 MiB. The report is incomplete."}}
                    @for issue in report.issues {p {(issue)}}
                }
                @if report.journals.is_empty() {section.card {p {"No validated writer journals are available."}}}
                @for journal in report.journals {section.card {
                    h2 {"Writer journal " (&journal.revision)}
                    p {(&journal.direction) " · " (journal.completed) "/" (journal.operations) " forward results · " (journal.undone) " rollback results"}
                    div.cardgrid {
                        div {h3 {"Pending intent"}p style="overflow-wrap:anywhere" {(operation(journal.pending.as_ref()))}}
                        div {h3 {"Last known result"}p style="overflow-wrap:anywhere" {(operation(journal.last_known.as_ref()))}}
                        div {h3 {"Saved verification"}p {@if journal.saved_verified {"Batch verification was saved. Historical only; freshness must be re-established."}@else {"No completed batch verification is saved."}}}
                    }
                }}
            }
        }
    }
}
pub async fn console(State(shared): State<Shared>) -> Response {
    let root = shared.lock().await.root.clone();
    let report = core_recovery::inspect_async(root).await;
    let a = shared.lock().await;
    shell(
        &a,
        "Recovery",
        html! {p {a href="/core-control" {"Back to Core control"}} (content(report))},
    )
    .into_response()
}
async fn viewer_page(State(root): State<Arc<PathBuf>>) -> Response {
    let report = core_recovery::inspect_async((*root).clone()).await;
    let status = if report.is_ok() {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Html(html! {(DOCTYPE)html lang="en" {head {meta charset="utf-8";meta name="viewport" content="width=device-width,initial-scale=1";title {"LQoSync recovery · read-only"}style {(maud::PreEscaped(include_str!("style.css")))}}
        body {main style="max-width:1200px;margin:auto;padding:20px" {h1 {"Recovery viewer"}p {a href="/" {"Refresh saved records"}}(content(report))}}
    }}.into_string())).into_response()
}
/// This router has GET handlers only and never creates an App or a writer lease.
pub fn viewer(root: PathBuf) -> Router {
    Router::new()
        .route("/", get(viewer_page))
        .route("/core-recovery", get(viewer_page))
        .layer(axum::middleware::from_fn(crate::web::headers))
        .with_state(Arc::new(root))
}
