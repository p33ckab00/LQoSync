use crate::{
    normalizer, orchestrator, store,
    web::{App, Shared, check, csrf, error, shell},
};
use axum::{
    Form,
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
};
use maud::html;
use serde::Deserialize;
use std::collections::BTreeMap;

pub(crate) struct Choice {
    pub(crate) value: String,
    pub(crate) label: String,
}
pub(crate) fn choices(a: &App, kind: &str) -> Vec<Choice> {
    let Some(path) = normalizer::source_table(kind) else {
        return vec![];
    };
    let mut choices = vec![];
    for router in &a.settings.routers {
        if router.role == "core" || a.inventory_errors.contains_key(&router.id) {
            continue;
        }
        let Some(inventory) = a.inventories.get(&router.id) else {
            continue;
        };
        let now = orchestrator::now();
        if inventory.router_id != router.id
            || now < inventory.collected_at
            || now - inventory.collected_at > normalizer::MAX_INVENTORY_AGE_SECONDS
            || inventory.errors.contains_key("collection")
            || inventory.errors.contains_key(path)
        {
            continue;
        }
        let Some(rows) = inventory.tables.get(path) else {
            continue;
        };
        let mut names: BTreeMap<&str, (usize, bool)> = BTreeMap::new();
        for row in rows {
            if let Some(name) = row.get("name").filter(|n| !n.is_empty()) {
                let value = names.entry(name).or_default();
                value.0 += 1;
                value.1 |= row.get("disabled").is_some_and(|v| v == "true");
            }
        }
        for (name, (count, disabled)) in names {
            if count == 1 && !disabled {
                choices.push(Choice {
                    value: serde_json::to_string(&[router.id.as_str(), name])
                        .expect("string tuple serializes"),
                    label: format!("{} · {}", router.name, name),
                });
            }
        }
    }
    choices
}
pub async fn sources(State(s): State<Shared>) -> Html<String> {
    let a = s.lock().await;
    let choices: BTreeMap<_, _> = ["pppoe", "dhcp", "hotspot"]
        .into_iter()
        .map(|kind| (kind, self::choices(&a, kind)))
        .collect();
    shell(
        &a,
        "Sources",
        html! {
            section.card {h2{"Subscriber source selection"}p{"Bind each profile or DHCP server to one shaping node. Read router inventory first to discover choices."}
                p.help{"Source selection and healthy scheduled reads publish normalized local snapshots automatically. Automatic publication pause also pauses scheduled collection. Inventories must be no older than five minutes."}
                p{(&a.status)}
                @if !normalizer::has_bindings(&a.settings){p.warning{"No active source bindings. A previous managed generation remains retained until a new complete collection is available."}}
            }
            @for node in a.settings.nodes.iter().filter(|n|normalizer::service(&n.kind).is_some()).take(200) {
                @let selected=if node.router.is_empty()||node.source.is_empty(){String::new()}else{serde_json::to_string(&[&node.router,&node.source]).unwrap_or_default()};
                section.card {h2{(&node.name) " · " (&node.kind)}p.help{"Node ID: " (&node.id)}
                    form method="post" action="/sources/save" {(csrf(&a))input type="hidden" name="node" value=(&node.id);
                        label{"Router and source" select name="selection" {
                            option value="" selected[selected.is_empty()]{"Unassigned"}
                            @if !selected.is_empty()&&!choices[node.kind.as_str()].iter().any(|c|c.value==selected) {
                                option value=(&selected) selected{"Current selection unavailable: " (&node.router) " · " (&node.source)}
                            }
                            @for choice in &choices[node.kind.as_str()] {option value=(&choice.value) selected[choice.value==selected]{(&choice.label)}}
                        }}button{"Save source selection"}
                    }
                }
            }
            section.card {p.help{"The first 200 subscriber nodes are shown. WAN and site nodes remain in Topology. Create subscriber nodes there before assigning sources."}
                p.help{"Exact same-router DHCP/Hotspot IP and MAC overlap uses the Hotspot account. Other duplicate IPs or identities block publication. Multiple IPs on one subscriber identity are not yet supported."}
                a href="/routers"{"Read router inventory"} " · " a href="/topology"{"Topology"} " · " a href="/subscribers"{"Subscribers"}}
        },
    )
}
#[derive(Deserialize)]
pub struct SourceForm {
    token: String,
    node: String,
    selection: String,
}
pub async fn save(State(s): State<Shared>, Form(form): Form<SourceForm>) -> Response {
    let mut a = s.lock().await;
    if let Err(r) = check(&a, &form.token) {
        return r.into_response();
    }
    let Some(node) = a.settings.nodes.iter().find(|n| n.id == form.node) else {
        return error(&a, "Node no longer exists");
    };
    if normalizer::service(&node.kind).is_none() {
        return error(&a, "Choose a subscriber node");
    }
    let selection = if form.selection.is_empty() {
        [String::new(), String::new()]
    } else {
        let Ok(selection) = serde_json::from_str::<[String; 2]>(&form.selection) else {
            return error(&a, "Invalid source selection");
        };
        if !choices(&a, &node.kind)
            .iter()
            .any(|c| c.value == form.selection)
        {
            return error(
                &a,
                "Source metadata changed or is unavailable; read inventory again",
            );
        }
        selection
    };
    let mut next = a.settings.clone();
    let node = next
        .nodes
        .iter_mut()
        .find(|n| n.id == form.node)
        .expect("node found");
    node.router = selection[0].clone();
    node.source = selection[1].clone();
    if let Err(e) = store::save_settings(&a.root, &next) {
        return error(&a, e);
    }
    a.settings = next;
    a.status = "Source selection saved. Waiting for a complete selected-source collection.".into();
    Redirect::to("/sources").into_response()
}
