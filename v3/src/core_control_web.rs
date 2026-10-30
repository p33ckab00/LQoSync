use crate::{
    core_console::{self, Config, Mode},
    web::{Shared, check, csrf, error, shell},
};
use axum::{
    Form,
    extract::State,
    response::{IntoResponse, Response},
};
use maud::html;
use serde::Deserialize;
pub async fn page(State(shared): State<Shared>) -> Response {
    let a = shared.lock().await;
    let config = a.settings.core_control.clone().unwrap_or_default();
    let health = a.settings.wan_health.clone().unwrap_or_default();
    let revision = core_console::revision(&a.settings).unwrap_or_default();
    let now = crate::orchestrator::now();
    let current = a.core_runtime.settings_revision == revision
        && now >= a.core_runtime.captured_at
        && now - a.core_runtime.captured_at <= config.options.evidence_seconds;
    let plan_current = current && a.core_runtime.preview.is_some();
    let mut desired = std::collections::BTreeMap::<String, usize>::new();
    if current && let Some(p) = &a.core_runtime.preview {
        for r in p.assignments.routes.values() {
            *desired.entry(r.wan.clone()).or_default() += 1;
        }
    }
    shell(&a,"Core control",html!{
 section.card{h2{"Continuous core control"}p{"Automatic writes blocked"}p.help{"RouterOS acceptance, live verifier and integrated writer coordination are pending. Saving configuration does not enable writes."}p{(&a.core_runtime.status)}@if !plan_current{p{"No preview evidence · waiting for a complete fresh round"}}}
 section.card{h2{"Writer and health policy"}form method="post" action="/core-control/save" data-config="true" data-editor-label="Core control policy"{(csrf(&a))input type="hidden" name="revision" value=(&revision);div.formgrid{
 label{"Mode" select name="mode"{option value="preview" selected[config.mode==Mode::Preview]{"Preview"}option value="paused" selected[config.mode==Mode::Paused]{"Paused"}option value="automatic" disabled{"Automatic — acceptance pending"}}}
 label{"Offline confirmation seconds" input type="number" min="1" max="3600" name="offline_seconds" value=(config.options.offline_seconds);}
 label{"Evidence lifetime seconds" input type="number" min="1" max="300" name="evidence_seconds" value=(config.options.evidence_seconds);}
 label{"Batch mutation limit" input type="number" min="1" max="20" name="batch_size" value=(config.batch_size);}
 label{"WAN failure" select name="evacuate_active"{option value="true" selected[config.options.evacuate_active]{"Evacuate active clients"}option value="false" selected[!config.options.evacuate_active]{"Stop new placements only"}}}
 label{"Pinned client failover" select name="pin_failover"{option value="false" selected[!config.options.pin_failover]{"Keep pin; report blocked"}option value="true" selected[config.options.pin_failover]{"Allow configured eligible failover"}}}
 label{"Health interval seconds" input type="number" min="1" max="300" name="interval_seconds" value=(health.interval_seconds);}
 label{"Failure rounds" input type="number" min="1" max="100" name="failure_rounds" value=(health.failure_rounds);}
 label{"Recovery rounds" input type="number" min="1" max="100" name="success_rounds" value=(health.success_rounds);}
 label{"Recovery delay seconds" input type="number" min="0" max="3600" name="recovery_seconds" value=(health.recovery_seconds);}
 }button{"Save local policy"}}}
 div.cardgrid{@for wan in crate::dynamic::wan_nodes(&a.settings){@let monitor=health.monitors.iter().find(|m|m.wan==wan.id);section.card{h2{(&wan.name)}p{"Observed online: " @if plan_current{(a.core_runtime.observed.get(&wan.id).copied().unwrap_or(0))}@else{"Unknown"} " · Desired: " @if plan_current{(desired.get(&wan.id).copied().unwrap_or(0))}@else{"Unknown"}}
 @if current{ @if let Some(h)=a.core_runtime.health.get(&wan.id){p{(format!("{:?}",h.status_at(now,config.options.evidence_seconds))) " · " (&h.reason)}}}
 form method="post" action="/core-control/probes" data-config="true" data-editor-label="WAN health probes"{(csrf(&a))input type="hidden" name="revision" value=(&revision);input type="hidden" name="wan" value=(&wan.id);div.formgrid{
 @for index in 0..2{label{(format!("Probe {} RouterOS ID",index+1))input name=(format!("probe{}",index+1)) value=(monitor.map(|m|m.probes[index].id.as_str()).unwrap_or("")) maxlength="33" required;}
 label{(format!("Probe {} external IPv4",index+1))input name=(format!("target{}",index+1)) value=(monitor.map(|m|m.probes[index].target.to_string()).unwrap_or_default()) required;}}
 label{"Action" select name="action"{option value="save"{"Save local probe selection"}option value="remove"{"Remove local monitor"}}}}
 p.help{"Select existing ICMP Netwatch entries. This form does not create probes, routes, or scripts."}button{"Save local monitor"}}
 }}}
 @if plan_current{p{"Clients with absent or ambiguous actual WAN coverage: " (a.core_runtime.ambiguous)}
 @if let Some(plan)=&a.core_runtime.preview{section.card{h2{"Reconciliation decisions"}p{(plan.decisions.len()) " decisions · at most 100 displayed · desired state is not applied shaping"}table{thead{tr{th{"Subscriber"}th{"Decision"}th{"Detail"}}}tbody{@for decision in plan.decisions.iter().take(100){@let (id,kind,detail)=describe(decision);tr{td{(id)}td{(kind)}td{(detail)}}}}}}
 }
 }
 section.card{h2{"Shadow worker and recovery"}p{"Shadow computation is serialized per data directory. A shadow ticket cannot authorize router writes."}
 p{@if a.core_runtime.busy {"A shadow computation is active."}@else {"No shadow computation is active in this console."}}
 form method="post" action="/core-control/pause"{(csrf(&a))input type="hidden" name="revision" value=(&revision);button{"Pause shadow preview"}}
 p{a href="/core-recovery" {"Inspect saved writer journals"}}
 p.help{"Approval and resume remain locked. Normal console startup remains blocked by unresolved recovery. Use the separate read-only viewer to inspect that state."}
 }
 }).into_response()
}
fn describe(d: &crate::core_reconcile::Decision) -> (&str, &str, String) {
    use crate::core_reconcile::Decision;
    match d {
        Decision::Blocked { subscriber, reason } => (subscriber, "Blocked", reason.clone()),
        Decision::Keep {
            subscriber, member, ..
        } => (subscriber, "Keep", member.list.clone()),
        Decision::Add {
            subscriber, member, ..
        } => (subscriber, "Add", member.list.clone()),
        Decision::Remove {
            subscriber, member, ..
        } => (subscriber, "Remove", member.list.clone()),
        Decision::Move {
            subscriber,
            before,
            after,
            ..
        } => (
            subscriber,
            "Move",
            format!("{} → {}", before.list, after.list),
        ),
    }
}
#[derive(Deserialize)]
pub struct ControlForm {
    token: String,
    revision: String,
    mode: Mode,
    offline_seconds: u64,
    evidence_seconds: u64,
    batch_size: usize,
    evacuate_active: bool,
    #[serde(default)]
    pin_failover: bool,
    interval_seconds: u64,
    failure_rounds: u32,
    success_rounds: u32,
    recovery_seconds: u64,
}
pub async fn save(State(shared): State<Shared>, Form(f): Form<ControlForm>) -> Response {
    let a = shared.clone().lock_owned().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    if core_console::revision(&a.settings).ok().as_deref() != Some(&f.revision) {
        return error(&a, "Settings changed; reload before saving");
    }
    let mut next = a.settings.clone();
    let control = Config {
        mode: f.mode,
        batch_size: f.batch_size,
        options: crate::core_reconcile::Options {
            evacuate_active: f.evacuate_active,
            pin_failover: f.pin_failover,
            offline_seconds: f.offline_seconds,
            evidence_seconds: f.evidence_seconds,
        },
    };
    if let Err(e) = control.validate() {
        return error(&a, e);
    }
    next.core_control = Some(control);
    let h = next.wan_health.get_or_insert_with(Default::default);
    h.interval_seconds = f.interval_seconds;
    h.evidence_seconds = f.evidence_seconds;
    h.failure_rounds = f.failure_rounds;
    h.success_rounds = f.success_rounds;
    h.recovery_seconds = f.recovery_seconds;
    persist(a, next).await
}
#[derive(Deserialize)]
pub struct ProbeForm {
    token: String,
    revision: String,
    wan: String,
    probe1: String,
    target1: String,
    probe2: String,
    target2: String,
    action: String,
}
pub async fn probes(State(shared): State<Shared>, Form(f): Form<ProbeForm>) -> Response {
    let a = shared.clone().lock_owned().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    if core_console::revision(&a.settings).ok().as_deref() != Some(&f.revision) {
        return error(&a, "Settings changed; reload before saving");
    }
    let mut next = a.settings.clone();
    let h = next.wan_health.get_or_insert_with(Default::default);
    h.monitors.retain(|m| m.wan != f.wan);
    match f.action.as_str() {
        "save" => {
            let targets = match (f.target1.parse(), f.target2.parse()) {
                (Ok(a), Ok(b)) => [a, b],
                _ => return error(&a, "Probe targets must be IPv4 addresses"),
            };
            h.monitors.push(crate::wan_health::Monitor {
                wan: f.wan,
                probes: [
                    crate::wan_health::Probe {
                        id: f.probe1,
                        target: targets[0],
                    },
                    crate::wan_health::Probe {
                        id: f.probe2,
                        target: targets[1],
                    },
                ],
            });
        }
        "remove" => {}
        _ => return error(&a, "Unknown probe action"),
    }
    persist(a, next).await
}
async fn persist(
    mut a: tokio::sync::OwnedMutexGuard<crate::web::App>,
    next: crate::model::Settings,
) -> Response {
    tokio::task::spawn_blocking(move || {
        if let Err(e) = crate::store::save_settings(&a.root, &next) {
            return error(&a, e);
        }
        a.settings = next;
        a.core_coordinator.configure(
            &core_console::revision(&a.settings).unwrap_or_default(),
            core_console::enabled(&a.settings),
        );
        let epoch = a.core_runtime.epoch.wrapping_add(1);
        a.core_runtime = core_console::Runtime {
            epoch,
            status: "Local policy saved; waiting for fresh preview".into(),
            ..Default::default()
        };
        axum::response::Redirect::to("/core-control").into_response()
    })
    .await
    .unwrap_or_else(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[derive(Deserialize)]
pub struct PauseForm {
    token: String,
    revision: String,
}
pub async fn pause(State(shared): State<Shared>, Form(form): Form<PauseForm>) -> Response {
    let a = shared.clone().lock_owned().await;
    if let Err(response) = check(&a, &form.token) {
        return response.into_response();
    }
    if core_console::revision(&a.settings).ok().as_deref() != Some(&form.revision) {
        return error(&a, "Settings changed; reload before pausing");
    }
    let mut next = a.settings.clone();
    next.core_control.get_or_insert_with(Default::default).mode = Mode::Paused;
    a.core_coordinator.configure(&form.revision, false);
    persist(a, next).await
}
