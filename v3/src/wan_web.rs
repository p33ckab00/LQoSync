use crate::{
    assignment,
    model::Settings,
    normalizer, orchestrator, store,
    web::{App, Shared, check, csrf, error, shell},
};
use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use maud::html;
use serde::Deserialize;
use std::collections::BTreeMap;
pub(crate) struct Interface {
    pub(crate) value: String,
    pub(crate) label: String,
}
pub(crate) fn interfaces(a: &App) -> Vec<Interface> {
    let mut choices = vec![];
    let now = orchestrator::now();
    for router in &a.settings.routers {
        let Some(inventory) = a.inventories.get(&router.id) else {
            continue;
        };
        if a.inventory_errors.contains_key(&router.id)
            || inventory.router_id != router.id
            || now < inventory.collected_at
            || now - inventory.collected_at > normalizer::MAX_INVENTORY_AGE_SECONDS
            || inventory.errors.contains_key("/interface/print")
            || inventory.errors.contains_key("collection")
        {
            continue;
        }
        let Some(rows) = inventory.tables.get("/interface/print") else {
            continue;
        };
        let mut names: BTreeMap<&str, Option<&crate::routeros::Row>> = BTreeMap::new();
        for row in rows {
            if let Some(name) = row.get("name").filter(|n| !n.is_empty()) {
                names
                    .entry(name)
                    .and_modify(|r| *r = None)
                    .or_insert(Some(row));
            }
        }
        for (name, row) in names {
            if let Some(row) = row {
                let status = if row.get("disabled").is_some_and(|v| v == "true") {
                    "disabled"
                } else if row.get("running").is_some_and(|v| v == "true") {
                    "running"
                } else {
                    "down/unknown"
                };
                choices.push(Interface {
                    value: serde_json::to_string(&[router.id.as_str(), name])
                        .expect("string tuple serializes"),
                    label: format!("{} · {} · {}", router.name, name, status),
                });
            }
        }
    }
    choices
}
pub async fn wans(State(s): State<Shared>) -> Html<String> {
    let a = s.lock().await;
    if a.settings.dynamic.is_some() {
        return shell(&a, "WANs", crate::dynamic_web::wan_panel(&a));
    }
    let choices = interfaces(&a);
    shell(
        &a,
        "WANs",
        html! {
            section.card {h2{"WAN interface mapping"}p{"Select the router interface for each WAN root. Interface discovery does not identify the real subscriber route automatically."}p.help{"Read router inventory first. Mapping and capacity edits update local staging automatically when selected sources are healthy."}p{(&a.status)}}
            div.cardgrid { @for node in a.settings.nodes.iter().filter(|n|n.kind=="wan") {
                @let selected=if node.router.is_empty()||node.source.is_empty(){String::new()}else{serde_json::to_string(&[&node.router,&node.source]).unwrap_or_default()};
                section.card {h2{(&node.name)}p.help{"WAN root: " (&node.id)}form method="post" action="/wans/save"{(csrf(&a))input type="hidden" name="node" value=(&node.id);div.formgrid{
                    label{"Router / interface" select name="selection" {option value="" selected[selected.is_empty()]{"Unmapped"}
                        @if !selected.is_empty()&&!choices.iter().any(|c|c.value==selected){option value=(&selected) selected{"Current mapping unavailable: " (&node.router) " · " (&node.source)}}
                        @for choice in &choices {option value=(&choice.value) selected[choice.value==selected]{(&choice.label)}}
                    }}
                    label{"WAN download · Mbps" input type="number" min="0" step="any" required name="download" value=(node.download);}
                    label{"WAN upload · Mbps" input type="number" min="0" step="any" required name="upload" value=(node.upload);}
                }button{"Save WAN mapping"}}}
            }
            }
            section.card{p.help{"Use Topology to add WAN roots and real subscriber branches. Interface health gates publication; existing subscribers are not moved automatically during an outage."}a href="/routers"{"Routers"} " · " a href="/topology"{"Topology"} " · " a href="/assignments"{"Subscriber assignments"}}
        },
    )
}
#[derive(Deserialize)]
pub struct WanForm {
    token: String,
    node: String,
    selection: String,
    download: f64,
    upload: f64,
}
pub async fn save_wan(State(s): State<Shared>, Form(f): Form<WanForm>) -> Response {
    let a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    let Some(node) = a
        .settings
        .nodes
        .iter()
        .find(|n| n.id == f.node && n.kind == "wan")
    else {
        return error(&a, "WAN root no longer exists");
    };
    let selection = if f.selection.is_empty() {
        [String::new(), String::new()]
    } else {
        let Ok(selection) = serde_json::from_str::<[String; 2]>(&f.selection) else {
            return error(&a, "Invalid WAN mapping");
        };
        if !interfaces(&a).iter().any(|c| c.value == f.selection) {
            return error(
                &a,
                "WAN interface metadata changed or is stale; read inventory again",
            );
        }
        selection
    };
    let mut next = a.settings.clone();
    let root = next
        .nodes
        .iter_mut()
        .find(|n| n.id == node.id)
        .expect("root exists");
    root.router = selection[0].clone();
    root.source = selection[1].clone();
    root.download = f.download;
    root.upload = f.upload;
    let previous = a.settings.clone();
    let generation = a.generation.clone();
    drop(a);
    commit(s, previous, generation, next, "/wans").await
}
pub(crate) async fn commit(
    s: Shared,
    previous: Settings,
    generation: Option<String>,
    next: Settings,
    location: &'static str,
) -> Response {
    let mut a = s.clone().lock_owned().await;
    if a.settings != previous || a.generation != generation {
        return error(&a, "Settings or generation changed; reload before saving");
    }
    tokio::task::spawn_blocking(move || {
        if let Err(e) = crate::web::nodelimits(&next)
            .and_then(|_| assignment::validate(&next))
            .and_then(|_| crate::dynamic_web::validate_evidence_edit(&a, &next))
            .and_then(|_| store::save_settings(&a.root, &next))
        {
            return error(&a, e);
        }
        a.settings = next;
        a.status =
            "Orchestration settings saved. Waiting for a complete healthy local generation.".into();
        Redirect::to(location).into_response()
    })
    .await
    .unwrap_or_else(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Settings save worker failed",
        )
            .into_response()
    })
}
