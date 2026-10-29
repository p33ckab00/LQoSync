use crate::{
    speeds::SpeedRules,
    store,
    web::{Shared, check, csrf, error, shell},
};
use axum::{
    Form,
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
};
use maud::html;
use serde::Deserialize;
use std::sync::Arc;

pub async fn rules(State(s): State<Shared>) -> Html<String> {
    let a = s.lock().await;
    let rules = &a.settings.speed_rules;
    shell(
        &a,
        "Speed rules",
        html! {
            section.card {h2{"Router-owned maximum speeds"}p{"PPP: secret comment → active comment → profile comment → profile rate → unit-marked profile name."}
                p{"DHCP: lease comment → lease rate. Hotspot: local user comment → profile comment → profile rate."}
                p{"Comments use customer download/upload order. Router rate fields use receive/transmit order and bare bits per second. Missing finite rates stay unresolved."}
                p.help{"Router burst fields are validated but this preview uses only the sustained maximum. It does not export burst policy."}}
            section.card {h2{"Minimum rate policy"}form method="post" action="/speed-rules/save" {(csrf(&a))div.formgrid {
                label {"Minimum rate fraction · 0–1" input type="number" step="any" min="0" max="1" required name="min_rate_fraction" value=(rules.min_rate_fraction);}
                label {"Minimum floor · Mbps" input type="number" step="any" min="0" required name="min_floor_mbps" value=(rules.min_floor_mbps);}
                label {"PPP profile-name fallback" select name="profile_name_fallback" {option value="true" selected[rules.profile_name_fallback]{"Enabled · explicit k/M/G suffix required"}option value="false" selected[!rules.profile_name_fallback]{"Disabled"}}}
            }button{"Save speed rules"}}
                p.help{"Explicit router minima take precedence. Otherwise minimum = max(maximum × fraction, floor), capped at the maximum. The floor never raises a router maximum."}
                p.help{"This save updates local policy. With selected sources, a healthy complete inventory automatically updates the local snapshot and output pair."}}
            section.card {h2{"Inventory previews"}
                @for r in &a.settings.routers {p{a href=(format!("/routers/{}/speeds",r.id)){(&r.name)}}}
                @if a.settings.routers.is_empty(){p.empty{"Configure a subscriber router, then read its inventory."}}
            }
        },
    )
}
#[derive(Deserialize)]
pub struct RulesForm {
    token: String,
    min_rate_fraction: f64,
    min_floor_mbps: f64,
    profile_name_fallback: bool,
}
pub async fn save_rules(State(s): State<Shared>, Form(f): Form<RulesForm>) -> Response {
    let mut a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    let mut next = a.settings.clone();
    next.speed_rules = SpeedRules {
        min_rate_fraction: f.min_rate_fraction,
        min_floor_mbps: f.min_floor_mbps,
        profile_name_fallback: f.profile_name_fallback,
    };
    if let Err(e) = store::save_settings(&a.root, &next) {
        return error(&a, e);
    }
    a.settings = next;
    Redirect::to("/speed-rules").into_response()
}
pub async fn router_speeds(State(s): State<Shared>, Path(id): Path<String>) -> Response {
    let (inventory, rules) = {
        let a = s.lock().await;
        if !a.settings.routers.iter().any(|r| r.id == id) {
            return error(&a, "Router no longer exists");
        }
        let Some(inventory) = a.inventories.get(&id) else {
            return shell(&a,"Speed rules",html!{section.card{h2{"Read inventory first"}p{"No inventory is active in this process. Select Read inventory on the Routers page."}a href="/routers"{"Routers"}}}).into_response();
        };
        (Arc::clone(inventory), a.settings.speed_rules.clone())
    };
    let collected = inventory.collected_at;
    let guard_inventory = Arc::clone(&inventory);
    let guard_rules = rules.clone();
    // CPU work runs outside the application lock and async worker.
    let report =
        tokio::task::spawn_blocking(move || crate::speeds::preview(&inventory, &rules)).await;
    let a = s.lock().await;
    if a.settings.speed_rules != guard_rules
        || !a
            .inventories
            .get(&id)
            .is_some_and(|i| Arc::ptr_eq(i, &guard_inventory))
    {
        return error(&a, "Inventory or speed rules changed; reload the preview");
    }
    let last_error = a.inventory_errors.get(&id);
    let report = match report {
        Ok(Ok(report)) => report,
        Ok(Err(e)) => return error(&a, e),
        Err(_) => return error(&a, "Speed preview could not complete"),
    };
    shell(&a,"Speed rules",html! {
        section.card {h2{"Speed rule preview · " (&id)}span.badge{"PREVIEW ONLY"}
            p.help{"Inventory timestamp: " (collected) ". Current minimum policy is used. No subscriber snapshot or staged generation is changed."}
            @if let Some(e)=last_error {p.warning{"The latest read failed. This preview uses an earlier inventory: " (e)}}
            p{(report.resolved()) " resolved · " (report.unresolved()) " unresolved · " (report.errors.len()) " source errors"}
            @for e in report.errors.iter().take(100){p.warning{(e)}}
            p.help{"Showing the first 200 entries. Counts cover every entry. This preview does not assign shaping nodes or prove an effective RADIUS/session policy."}
            div.tablewrap {table {thead {tr {@for title in ["Source","Identity / group","IPv4","Maximum ↓ / ↑ Mbps","Minimum ↓ / ↑ Mbps","Rule / error"] {th{(title)}}}}
                tbody {@for e in report.entries.iter().take(200){tr {td{(e.service)}td{(&e.identity)br;small{(&e.group)}}td.mono{(&e.ipv4)}
                    @if let Some(speed)=&e.speed {td{(speed.rates.download) " / " (speed.rates.upload)}td{(speed.rates.download_min) " / " (speed.rates.upload_min)}
                        td{(speed.source)details{summary{"Router rule"}pre{(&speed.raw)}}@if speed.burst_ignored {small{"Burst policy is not exported"}}}}
                    @else {td{"—"}td{"—"}td.warning{(e.error.as_deref().unwrap_or("Unresolved"))}}
                }}}
            }}
        }
    }).into_response()
}
