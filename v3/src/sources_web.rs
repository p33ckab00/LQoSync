use crate::{
    normalizer, orchestrator, store,
    web::{App, Shared, check, csrf, error, shell},
};
use axum::{
    Form,
    extract::{Path, Query, State},
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
#[derive(Default, Deserialize)]
pub struct Search {
    #[serde(default)]
    q: String,
}
pub async fn sources(State(s): State<Shared>, Query(search): Query<Search>) -> Html<String> {
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
            (selection_panel_query(&a, &search.q.chars().take(256).collect::<String>()))
            @for node in a.settings.nodes.iter().filter(|n|a.settings.dynamic.is_none()&&normalizer::service(&n.kind).is_some()).take(200) {
                @let selected=if node.router.is_empty()||node.source.is_empty(){String::new()}else{serde_json::to_string(&[&node.router,&node.source]).unwrap_or_default()};
                section.card {h2{(&node.name) " · " (&node.kind)}p.help{"Node ID: " (&node.id)}
                    form method="post" action="/sources/save" {(csrf(&a))input type="hidden" name="node" value=(&node.id);
                        label{"Router and source" select name="selection" {
                            option value="" selected[selected.is_empty()]{"Unassigned"}
                            @if !selected.is_empty()&&!choices[node.kind.as_str()].iter().take(200).any(|c|c.value==selected) {
                                option value=(&selected) selected{"Current selection unavailable: " (&node.router) " · " (&node.source)}
                            }
                            @for choice in choices[node.kind.as_str()].iter().take(200) {option value=(&choice.value) selected[choice.value==selected]{(&choice.label)}}
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
    if let Err(e) =
        validate_selection_settings(&a, &next).and_then(|_| store::save_settings(&a.root, &next))
    {
        return error(&a, e);
    }
    a.replace_settings(next);
    a.status = "Source selection saved. Waiting for a complete selected-source collection.".into();
    Redirect::to("/sources").into_response()
}

fn subscriber_table(kind: &str) -> Option<&'static str> {
    match kind {
        "pppoe" => Some("/ppp/secret/print"),
        "dhcp" => Some("/ip/dhcp-server/lease/print"),
        "hotspot" => Some("/ip/hotspot/user/print"),
        _ => None,
    }
}
fn identity(row: &crate::routeros::Row, kind: &str, selector: &str) -> Option<String> {
    if kind == "dhcp" && selector == "subscriber" {
        let raw = row
            .get("active-mac-address")
            .filter(|s| !s.is_empty())
            .or_else(|| row.get("mac-address"))?;
        normalizer::canonical_mac(raw).ok()
    } else {
        row.get("name").filter(|s| !s.is_empty()).cloned()
    }
}
fn fresh_inventory<'a>(a: &'a App, router: &str) -> Option<&'a crate::routeros::Inventory> {
    if !a
        .settings
        .routers
        .iter()
        .any(|r| r.id == router && r.role != "core")
        || a.inventory_errors.contains_key(router)
    {
        return None;
    }
    let inv = a.inventories.get(router)?;
    let now = orchestrator::now();
    (inv.router_id == router
        && now >= inv.collected_at
        && now - inv.collected_at <= normalizer::MAX_INVENTORY_AGE_SECONDS
        && !inv.errors.contains_key("collection"))
    .then_some(inv)
}
fn selection_choices(a: &App, query: &str) -> Vec<Choice> {
    let query = query.to_lowercase();
    let mut result = vec![];
    for router in &a.settings.routers {
        let Some(inv) = fresh_inventory(a, &router.id) else {
            continue;
        };
        for kind in ["pppoe", "dhcp", "hotspot"] {
            for selector in ["group", "subscriber"] {
                let path = if selector == "group" {
                    normalizer::source_table(kind)
                } else {
                    subscriber_table(kind)
                }
                .expect("known kind");
                if inv.errors.contains_key(path) {
                    continue;
                }
                let Some(rows) = inv.tables.get(path) else {
                    continue;
                };
                let mut names: BTreeMap<String, (usize, bool)> = BTreeMap::new();
                for row in rows {
                    if let Some(name) = identity(row, kind, selector) {
                        let v = names.entry(name).or_default();
                        v.0 += 1;
                        v.1 |= row.get("disabled").is_some_and(|v| v == "true");
                    }
                }
                for (name, (count, disabled)) in names {
                    let label = format!("{} · {} · {} · {}", router.name, kind, selector, name);
                    if count == 1 && !disabled && label.to_lowercase().contains(&query) {
                        result.push(Choice {
                            value: serde_json::to_string(&[
                                router.id.as_str(),
                                kind,
                                selector,
                                name.as_str(),
                            ])
                            .expect("string tuple"),
                            label,
                        });
                        if result.len() == 200 {
                            return result;
                        }
                    }
                }
            }
        }
    }
    result
}
pub(crate) fn selection_panel(a: &App) -> maud::Markup {
    selection_panel_query(a, "")
}
fn selection_panel_query(a: &App, query: &str) -> maud::Markup {
    let choices = selection_choices(a, query);
    let revision = crate::topology_edits::revision(&a.settings).unwrap_or_default();
    html! {
        section.card { h2 {"Independent sources and exclusions"}
            p.help {"Select a profile, server, account, or lease. Individual choices override group choices. An exclusion skips matching records. Removing a selection keeps router data and topology."}
            form method="get" action="/sources" {label {"Search inventory" input name="q" value=(query) maxlength="256";} button {"Search"}}
            p.help {"First 200 matching choices and selections are shown. Read fresh router inventory to update the choices."}
            form method="post" action="/sources/selection" {(crate::dynamic_web::guards_revision(a,&revision))
                @if let Some(c)=&a.settings.dynamic {
                    label{"Assignment policy" select name="policy"{option value=""{"Create a policy for this selection"}@for p in &c.policies{option value=(&p.id){(if p.name.is_empty(){&p.id}else{&p.name}) " · " (&p.id)}}}}
                    label{"Routing WAN for a new policy" select name="wan"{option value=""{"Choose a WAN for a new policy"}@for w in &c.wans{option value=(&w.id){(&w.name) " · " (&w.id)}}}}
                }
                label {"Inventory source" select name="selection" required {option value="" {"Choose a source"} @for c in choices {option value=(c.value) {(c.label)}}}}
                label {"Destination node ID" input name="node" list="selection-nodes" maxlength="80" placeholder="Choose or enter an existing node ID";}
                label {"Create a branch under node ID" input name="parent" list="selection-nodes" maxlength="80" placeholder="Optional parent node ID";}
                datalist id="selection-nodes" {@for node in a.settings.nodes.iter().take(200) {option value=(&node.id) {(&node.name) " · " (&node.kind)}}}
                label {input type="checkbox" name="exclude" value="true"; "Exclude matching records"}
                button {"Save independent selection"}
            }
            div.tablewrap {table {thead {tr {th {"Router / service"} th {"Selector"} th {"Destination"} th {"Action"}}} tbody {
                @for selection in a.settings.selections.iter().filter(|s|query.is_empty() || format!("{} {} {} {}",s.router,s.kind,s.value,s.node).to_lowercase().contains(&query.to_lowercase())).take(200) {
                    tr {td {(&selection.router) " / " (&selection.kind)} td {(&selection.selector) ": " (&selection.value)} td {@if selection.exclude {"Excluded"} @else if let Some(c)=&a.settings.dynamic {@if let Some(p)=c.policies.iter().find(|p|p.selections.contains(&selection.id)){(&p.id)}} @else {(&selection.node)}} td {
                        (crate::dynamic_web::source_editor(a,selection,&revision))
                        form method="post" action=(format!("/sources/selection/{}/delete",selection.id)) {(crate::dynamic_web::guards_revision(a,&revision)) button.danger {"Remove"}}
                    }}
                }
            }}}
        }
    }
}
#[derive(Deserialize)]
pub struct SelectionForm {
    token: String,
    #[serde(default)]
    revision: String,
    #[serde(default)]
    original_id: String,
    #[serde(default)]
    policy: String,
    #[serde(default)]
    wan: String,
    selection: String,
    #[serde(default)]
    node: String,
    #[serde(default)]
    parent: String,
    #[serde(default)]
    exclude: bool,
}
pub async fn save_selection(State(s): State<Shared>, Form(form): Form<SelectionForm>) -> Response {
    let mut a = s.lock().await;
    if let Err(r) = check(&a, &form.token) {
        return r.into_response();
    }
    if a.settings.dynamic.is_some()
        && form.revision != crate::topology_edits::revision(&a.settings).unwrap_or_default()
    {
        return error(&a, "Settings changed; reload before saving");
    }
    let Ok([router, kind, selector, value]) = serde_json::from_str::<[String; 4]>(&form.selection)
    else {
        return error(&a, "Invalid source selection");
    };
    let Some(inv) = fresh_inventory(&a, &router) else {
        return error(
            &a,
            "Source inventory is stale or unavailable; read inventory again",
        );
    };
    let path = if selector == "group" {
        normalizer::source_table(&kind)
    } else if selector == "subscriber" {
        subscriber_table(&kind)
    } else {
        None
    };
    let Some(path) = path else {
        return error(&a, "Invalid source selector type");
    };
    if inv.errors.contains_key(path) {
        return error(&a, "Source metadata read failed; read inventory again");
    }
    let canonical = if kind == "dhcp" && selector == "subscriber" {
        match normalizer::canonical_mac(&value) {
            Ok(v) => v,
            Err(e) => return error(&a, e),
        }
    } else {
        value.clone()
    };
    let found: Vec<_> = inv
        .tables
        .get(path)
        .into_iter()
        .flatten()
        .filter(|r| identity(r, &kind, &selector).as_deref() == Some(&canonical))
        .collect();
    if found.len() != 1 || found[0].get("disabled").is_some_and(|v| v == "true") {
        return error(
            &a,
            "Source metadata changed or is unavailable; read inventory again",
        );
    }
    let mut next = a.settings.clone();
    if a.settings.dynamic.is_some() && !form.policy.is_empty() && !form.parent.is_empty() {
        return error(
            &a,
            "Choose an existing policy or create a branch for a new policy",
        );
    }
    let mut node = form.node;
    if !form.parent.is_empty() && !form.exclude {
        if !next.nodes.iter().any(|n| n.id == form.parent) {
            return error(&a, "Parent node no longer exists");
        }
        node = uuid::Uuid::new_v4().to_string();
        next.nodes.push(crate::model::Node {
            id: node.clone(),
            name: value.clone(),
            parent: form.parent,
            kind: if next.dynamic.is_some() {
                "group".into()
            } else {
                kind.clone()
            },
            router: String::new(),
            source: String::new(),
            mode: "computed".into(),
            download: 0.,
            upload: 0.,
            download_factor: 1.,
            upload_factor: 1.,
        });
    }
    let selection_id = if form.original_id.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        if next.dynamic.is_none() || !next.selections.iter().any(|s| s.id == form.original_id) {
            return error(&a, "Source selection no longer exists");
        }
        form.original_id.clone()
    };
    if let Some(c) = &mut next.dynamic {
        if !form.original_id.is_empty() {
            if let Some(old) = c
                .policies
                .iter()
                .find(|p| p.selections.contains(&selection_id))
                && (form.exclude || old.id != form.policy)
            {
                if c.pins.values().any(|pin| pin.policy == old.id) {
                    return error(
                        &a,
                        "Clear stored pins in the old policy before excluding or regrouping this selection",
                    );
                }
                if !form.exclude {
                    let preserves =
                        if let Some(new) = c.policies.iter().find(|p| p.id == form.policy) {
                            old.targets.iter().all(|t| new.targets.contains(t))
                        } else {
                            old.targets
                                .iter()
                                .all(|t| t.node == node && t.wan == form.wan)
                        };
                    if !preserves {
                        return error(
                            &a,
                            "Use explicit destination replacement before moving this selection to a policy with different destinations",
                        );
                    }
                }
            }
            for p in &mut c.policies {
                p.selections.retain(|id| id != &selection_id);
            }
        }
        if !form.exclude {
            if !form.policy.is_empty() {
                let Some(p) = c.policies.iter_mut().find(|p| p.id == form.policy) else {
                    return error(&a, "Assignment policy no longer exists");
                };
                p.selections.push(selection_id.clone());
            } else {
                let target = crate::dynamic::Target {
                    node: node.clone(),
                    wan: form.wan,
                };
                c.policies.push(crate::dynamic::Policy {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: value.clone(),
                    selections: vec![selection_id.clone()],
                    targets: vec![target.clone()],
                    default: target,
                    balance: false,
                    metric: "online_clients".into(),
                });
            }
        }
        c.policies.retain(|p| !p.selections.is_empty());
        let policies: std::collections::HashSet<_> =
            c.policies.iter().map(|p| p.id.as_str()).collect();
        c.replacements
            .retain(|r| policies.contains(r.policy.as_str()));
        node.clear();
    }
    next.selections.retain(|s| s.id != selection_id);
    next.selections.push(crate::selection::Selection {
        id: selection_id,
        router,
        kind,
        selector,
        value: canonical,
        node,
        exclude: form.exclude,
    });
    if let Err(e) =
        validate_selection_settings(&a, &next).and_then(|_| store::save_settings(&a.root, &next))
    {
        return error(&a, e);
    }
    a.replace_settings(next);
    a.status = "Independent source selection saved. Waiting for a complete collection.".into();
    Redirect::to("/sources").into_response()
}
#[derive(Deserialize)]
pub struct DeleteSelectionForm {
    token: String,
    #[serde(default)]
    revision: String,
}
pub async fn delete_selection(
    State(s): State<Shared>,
    Path(id): Path<String>,
    Form(form): Form<DeleteSelectionForm>,
) -> Response {
    let mut a = s.lock().await;
    if let Err(r) = check(&a, &form.token) {
        return r.into_response();
    }
    if a.settings.dynamic.is_some()
        && form.revision != crate::topology_edits::revision(&a.settings).unwrap_or_default()
    {
        return error(&a, "Settings changed; reload before saving");
    }
    if !a.settings.selections.iter().any(|s| s.id == id) {
        return error(&a, "Selection no longer exists");
    }
    let mut next = a.settings.clone();
    next.selections.retain(|s| s.id != id);
    if let Some(c) = &mut next.dynamic {
        for p in &mut c.policies {
            p.selections.retain(|s| s != &id);
        }
        c.policies.retain(|p| !p.selections.is_empty());
        let policies: std::collections::HashSet<_> =
            c.policies.iter().map(|p| p.id.as_str()).collect();
        c.replacements
            .retain(|r| policies.contains(r.policy.as_str()));
    }
    if let Err(e) =
        validate_selection_settings(&a, &next).and_then(|_| store::save_settings(&a.root, &next))
    {
        return error(&a, e);
    }
    a.replace_settings(next);
    a.status = "Selection removed. Router data and topology are unchanged.".into();
    Redirect::to("/sources").into_response()
}

fn validate_selection_settings(a: &App, next: &crate::model::Settings) -> anyhow::Result<()> {
    if next.dynamic.is_some() {
        crate::web::nodelimits(next)?;
        return crate::dynamic_web::validate_evidence_edit(a, next);
    }
    crate::web::nodelimits(next)?;
    let mut rows = store::load_snapshot(&a.root)?;
    if !rows.is_empty() {
        crate::minimums::allocate(next, &mut rows)?;
        crate::model::compile(next, &rows)?;
    }
    Ok(())
}
