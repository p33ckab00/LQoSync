use crate::{
    core_diagnostics::Origin,
    web::{Shared, error, shell},
};
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use maud::html;
use serde::Deserialize;
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Page {
    origin: String,
}
pub async fn page(State(s): State<Shared>, Query(page): Query<Page>) -> Response {
    let origin = match page.origin.as_str() {
        "" | "live" => Origin::Live,
        "saved" => Origin::Saved,
        _ => return error(&*s.lock().await, "Choose live or saved validation inputs"),
    };
    let (settings, root, inventories, errors, epochs, generation) = {
        let a = s.lock().await;
        (
            a.settings.clone(),
            a.root.clone(),
            a.inventories.clone(),
            a.inventory_errors.clone(),
            a.read_epochs.clone(),
            a.generation.clone(),
        )
    };
    let work_settings = settings.clone();
    let work_inv = inventories.clone();
    let work_errors = errors.clone();
    let work_root = root.clone();
    let result = tokio::task::spawn_blocking(move || {
        let (inventories, errors) = if origin == Origin::Saved {
            let selected = crate::validation::required_routers(&work_settings);
            crate::core_diagnostics::saved_routers(
                work_settings
                    .routers
                    .iter()
                    .filter(|r| selected.contains(r.id.as_str())),
                &work_root,
            )
        } else {
            (work_inv, work_errors)
        };
        crate::validation::review(
            &work_settings,
            &inventories,
            &errors,
            &work_root,
            crate::orchestrator::now(),
            origin,
        )
    })
    .await;
    let a = s.lock().await;
    if a.settings != settings
        || a.generation != generation
        || crate::store::current_generation(&root) != generation
        || (origin == Origin::Live
            && (a.read_epochs != epochs
                || a.inventory_errors != errors
                || !crate::orchestrator::same_inventories(&inventories, &a.inventories)))
    {
        return error(&a, "Validation inputs changed; reload the review");
    }
    let report = match result {
        Ok(r) => r,
        Err(_) => return error(&a, "Validation worker failed"),
    };
    shell(&a,"Validation",html!{
        section.card{h2{"Subscriber and output validation"}p{(format!("{:?}",report.status())) " · " (report.subscribers) " subscribers"}
            p.help{"Read-only consistency review. This page does not publish output, modify routers, or apply LibreQoS shaping."}
            @if origin==Origin::Saved{p.warning{"Historical review · saved snapshots cannot establish current live validation."}}
            @else{p{"Live process inventory · checked at " (report.reviewed_at)}}
            @if let Some(g)=&report.generation{p.mono{"Generation: " (g)}}
            p{a href="/validation"{"Live inputs"} " · " a href="/validation?origin=saved"{"Saved inputs"} " · " a href="/core-review"{"Core policy paths"}}
        }
        section.card{h2{"Checks"}div.tablewrap{table{thead{tr{th{"Check"}th{"Result"}th{"Detail"}}}tbody{
            @for c in &report.checks{tr{td{(&c.code)}td{(format!("{:?}",c.status))}td{(&c.detail)}}}
        }}}}
        section.card{h2{"Subscriber WAN coverage"}div.tablewrap{table{thead{tr{th{"WAN"}th{"Subscribers"}}}tbody{
            @for row in report.coverage.iter().take(200){tr{td{(&row.wan)}td{(row.subscribers)}}}
        }}}@if report.coverage.len()>200{p.help{"First 200 WANs shown. All assigned subscribers were checked."}}
            p{a href="/topology"{"Topology"} " · " a href="/sources"{"Sources"} " · " a href="/wans"{"WANs"} " · " a href="/core-lists"{"Core list mappings"}}
            p.help{"A consistency pass does not prove packet routing, hardware behavior, or production readiness. Core policy limits remain in force."}
        }
    }).into_response()
}
