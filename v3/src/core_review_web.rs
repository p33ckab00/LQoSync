use crate::{
    core_diagnostics::{self, Origin},
    web::{Shared, csrf, error, shell},
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
        _ => return error(&*s.lock().await, "Choose live or saved core inventory"),
    };
    let (settings, root, inventories, errors, epochs) = {
        let a = s.lock().await;
        (
            a.settings.clone(),
            a.root.clone(),
            a.inventories.clone(),
            a.inventory_errors.clone(),
            a.read_epochs.clone(),
        )
    };
    let work_settings = settings.clone();
    let work_inventories = inventories.clone();
    let work_errors = errors.clone();
    let report = tokio::task::spawn_blocking(move || {
        let (inventories, errors) = if origin == Origin::Saved {
            core_diagnostics::saved(&work_settings, &root)
        } else {
            (work_inventories, work_errors)
        };
        core_diagnostics::review(
            &work_settings,
            &inventories,
            &errors,
            crate::orchestrator::now(),
            origin,
        )
    })
    .await;
    let a = s.lock().await;
    if a.settings != settings
        || (origin == Origin::Live
            && (a.read_epochs != epochs
                || a.inventory_errors != errors
                || !crate::orchestrator::same_inventories(&inventories, &a.inventories)))
    {
        return error(&a, "Core review inputs changed; reload the review");
    }
    let report = match report {
        Ok(r) => r,
        Err(_) => return error(&a, "Core diagnostics worker failed"),
    };
    shell(&a,"Core review",html!{
        section.card {h2{"Core policy review"}p.help{"Read-only structural diagnostics. Pass does not prove packet routing or authorize a writer. Subscriber membership checks remain on Core lists."}
            p{@if origin==Origin::Saved{strong{"Saved inventory · historical only"}}@else{strong{"Live process inventory · freshness shown below"}}}
            p{a href="/core-review"{"Live inventory"} " · " a href="/core-review?origin=saved"{"Saved inventory"} " · " a href="/core-lists"{"Core list plan"}}
            p.help{"Saved review never loads historical inventory into the live collector and never publishes a generation."}
        }
        @if report.cores.is_empty(){section.card{p{"No core/both routers are configured. Add a router in Routers first."}}}
        @for core in &report.cores {
            section.card {h2{(&core.router)}p{"Inventory: " (format!("{:?}",core.health)) @if let Some(time)=core.collected_at{" · age " (crate::orchestrator::now().saturating_sub(time)) " seconds"}}
                p{"Members: " (core.counts.address_members) " · Mangle rules: " (core.counts.mangle_rules) " · Routes: " (core.counts.routes)}
                p{"Active PCC rules: " (core.counts.pcc_rules) " · Active FastTrack rules: " (core.counts.fasttrack_rules)}
                form method="post" action=(format!("/routers/{}/inspect",core.router)){(csrf(&a))button{"Read core inventory in Routers"}}
                p.help{"Counts exclude disabled PCC/FastTrack rules. Full rule fields are read so unsupported conditions remain visible."}
                div.tablewrap{table{thead{tr{@for title in ["WAN / observed path","Mapping","Table / list","Expected or observed interface","Structure"]{th{(title)}}}}tbody{
                    @for wan in core.wans.iter().take(200){tr{td{(&wan.wan)}td{(if wan.mapped{"Configured"}else{"Observed only"})}td{(&wan.table)br;(&wan.list)}td{(&wan.interface)}td{(format!("{:?}",wan.status))}}}
                }}}
                @if core.wans.len()>200{p.help{"First 200 paths shown. The report evaluates all configured paths."}}
                @if core.observed_paths_omitted>0{p.help{(core.observed_paths_omitted) " additional observed paths were omitted from analysis (limit 512). Review is incomplete."}}
                @if core.wans.is_empty(){p{"No mapped or observed custom-table WAN paths are available."}}
                h3{"Connection-mark paths"}
                p.help{"Mark-link checks identify subscriber forwarding separately from WAN ingress/router replies. Overall WAN findings and inventory health still apply."}
                div.tablewrap{table{thead{tr{@for title in ["Direction / list","Connection mark","Table / mapped WAN interface","Rule references","Mark-link check"]{th{(title)}}}}tbody{
                    @for path in core.connection_paths.iter().take(200){tr{
                        td{(format!("{:?}",path.direction))br;(&path.source_list)}td{(&path.connection_mark)}td{(&path.table)br;(&path.interface)}
                        td{(&path.classifier_row) " → " (&path.routing_row)}td{(format!("{:?}",path.status))}
                    }}
                }}}
                @if core.connection_paths.len()>200{p.help{"First 200 connection paths shown. Analysis limits appear as global findings."}}
                p.help{"Global findings affect every WAN on this core. A structural pass requires fresh complete live input for any later planning step."}
                div.tablewrap{table{thead{tr{@for title in ["Result / reason","Scope","Source","Detail"]{th{(title)}}}}tbody{
                    @for f in &core.findings{tr{td{(format!("{:?}",f.status))br;(&f.code)}td{(if f.wan.is_empty(){"All WANs"}else{f.wan.as_str()})}td{(&f.table)br;(&f.row)}td{(&f.message)}}}
                }}}
            }
        }
    }).into_response()
}
