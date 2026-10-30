use crate::{
    core_plan::{self, Action, Binding, Entry},
    orchestrator, store, wan_web,
    web::{Shared, check, csrf, error, shell},
};
use axum::{
    Form,
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use maud::html;
use serde::Deserialize;
use std::collections::BTreeMap;
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Page {
    q: String,
    offset: usize,
}
pub fn revision(settings: &crate::model::Settings, generation: &Option<String>) -> String {
    let text = serde_json::to_vec(&(settings, generation)).expect("validated settings serialize");
    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &text).to_string()
}
fn page_link(q: &str, offset: usize) -> String {
    let q: String = q
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    format!("/core-lists?q={q}&offset={offset}")
}
fn tables(a: &crate::web::App, router: &str) -> Vec<String> {
    let Some(inv) = a.inventories.get(router) else {
        return vec![];
    };
    let now = orchestrator::now();
    if inv.router_id != router
        || a.inventory_errors.contains_key(router)
        || inv.errors.contains_key("collection")
        || inv.errors.contains_key("/routing/table/print")
        || now < inv.collected_at
        || now - inv.collected_at > crate::normalizer::MAX_INVENTORY_AGE_SECONDS
    {
        return vec![];
    }
    let mut names = BTreeMap::<String, usize>::new();
    for row in inv.tables.get("/routing/table/print").into_iter().flatten() {
        if let Some(name) = row
            .get("name")
            .filter(|n| core_plan::valid_name(n) && n.as_str() != "main")
            && crate::core_routes::fib(row).unwrap_or(false)
            && !crate::core_routes::flag(row, "disabled", false).unwrap_or(true)
            && !crate::core_routes::flag(row, "invalid", false).unwrap_or(true)
        {
            *names.entry(name.clone()).or_default() += 1;
        }
    }
    names
        .into_iter()
        .filter(|(_, count)| *count == 1)
        .map(|(name, _)| name)
        .collect()
}
struct View {
    generation: String,
    owner: String,
    bindings: Vec<Binding>,
    inventory_at: BTreeMap<String, u64>,
    counts: [usize; 3],
    protected: usize,
    rows: Vec<Entry>,
    total: usize,
    offset: usize,
}
pub async fn page(State(s): State<Shared>, Query(page): Query<Page>) -> Response {
    let (settings, root, generation) = {
        let a = s.lock().await;
        (a.settings.clone(), a.root.clone(), a.generation.clone())
    };
    let q: String = page.q.chars().take(256).collect();
    let search = q.to_lowercase();
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<Option<View>> {
        let Some(committed) = store::load_core_plan(&root)? else {
            return Ok(None);
        };
        let plan = committed.plan;
        let mut counts = [0; 3];
        for entry in &plan.entries {
            counts[match entry.action {
                Action::Add => 0,
                Action::Keep => 1,
                Action::Remove => 2,
            }] += 1;
        }
        let matches = |entry: &Entry| {
            search.is_empty()
                || [
                    entry.router.as_str(),
                    entry.wan.as_str(),
                    entry.list.as_str(),
                    entry.address.as_str(),
                    entry.subscriber.as_str(),
                    match entry.action {
                        Action::Add => "add",
                        Action::Keep => "keep",
                        Action::Remove => "remove",
                    },
                ]
                .iter()
                .any(|v| v.to_lowercase().contains(&search))
        };
        let total = plan.entries.iter().filter(|e| matches(e)).count();
        let offset = page.offset.min(total.saturating_sub(1) / 100 * 100);
        let rows = plan
            .entries
            .into_iter()
            .filter(matches)
            .skip(offset)
            .take(100)
            .collect();
        Ok(Some(View {
            generation: committed.generation,
            owner: plan.owner,
            bindings: plan.bindings,
            inventory_at: plan.inventory_at,
            counts,
            protected: plan.protected,
            rows,
            total,
            offset,
        }))
    })
    .await;
    let a = s.lock().await;
    if a.settings != settings || a.generation != generation {
        return error(
            &a,
            "Settings or generation changed; reload the core planner",
        );
    }
    let (view, load_error) = match result {
        Ok(Ok(view)) => (view, None),
        Ok(Err(e)) => (
            None,
            Some(format!("Committed core plan is unavailable: {e:#}")),
        ),
        Err(_) => (None, Some("Core preview worker failed".into())),
    };
    let rev = revision(&a.settings, &a.generation);
    shell(&a,"Core lists",html!{
        section.card {h2{"Core address-list planner"}p{(&a.status)}p.help{"Automatic local plans only. Router writes and production apply are disabled. Structural route checks do not prove the actual packet path."}
            p.help{"Map the WAN to a core/both router first. Read fresh inventory. Each list needs a direct prerouting mark to its custom table and an active default through that WAN. Complex policy requires review."}
            @if let Some(message)=&load_error{p{(message)}}
        }
        @for wan in crate::dynamic::wan_nodes(&a.settings) {
            @let binding=a.settings.core_lists.bindings.iter().find(|b|b.wan==wan.id);
            @let choices=tables(&a,&wan.router);
            section.card {h2{(&wan.name) " · " (&wan.id)}
                form method="post" action="/core-lists/save" {(csrf(&a))input type="hidden" name="revision" value=(&rev);input type="hidden" name="action" value="save";input type="hidden" name="wan" value=(&wan.id);
                    div.formgrid {
                        label{"Core router" select name="router" {option value=""{"Select mapped core router"}@for r in a.settings.routers.iter().filter(|r|r.id==wan.router&&(r.role=="core"||r.role=="both")){option value=(&r.id) selected[binding.is_some_and(|b|b.router==r.id)]{(&r.name)}}}}
                        label{"Address-list name" input required maxlength="64" name="list" value=(binding.map(|b|b.list.clone()).unwrap_or_else(||format!("lqosync3_{}",wan.id)));}
                        label{"Routing table" select name="table" {option value=""{"Select discovered custom FIB table"}@if let Some(b)=binding { @if !choices.contains(&b.table){option value=(&b.table) selected{"Current table unavailable: " (&b.table)}}}@for name in &choices{option value=(name) selected[binding.is_some_and(|b|b.table==*name)]{(name)}}}}
                    }button{"Save core mapping"}
                }
                @if binding.is_some(){form method="post" action="/core-lists/save"{(csrf(&a))input type="hidden" name="revision" value=(&rev);input type="hidden" name="action" value="remove";input type="hidden" name="wan" value=(&wan.id);button.danger{"Remove local mapping"}}}
            }
        }
        @if let Some(view)=&view {
            section.card {h2{"Last committed local plan"}p.mono{"Generation: " (&view.generation)}
                p{"Add: " (view.counts[0]) " · Keep: " (view.counts[1]) " · Remove: " (view.counts[2]) " · Protected members preserved: " (view.protected)}
                p.help{"Historical plan from a complete valid collection. Failed or stale collections retain this plan. Changes below are a preview, not executable commands."}
                @for b in &view.bindings {p{(&b.wan) " → " (&b.router) " · " (&b.list) " → " (&b.table)}}
                @for (router,time) in &view.inventory_at{p.help{(router) " inventory age: " (orchestrator::now().saturating_sub(*time)) " seconds"}}
                details{summary{"Ownership scope"}p.mono{(&view.owner)}p.help{"Exact application/subscriber/WAN markers protect manual and other-owner members. Lists outside active bindings remain untouched."}}
            }
            section.card {h2{(view.total) " match"}form method="get" action="/core-lists"{label{"Search action, subscriber ID, IPv4, WAN, core or list" input name="q" value=(&q);}button{"Search"}}
                div.tablewrap{table{thead{tr{@for title in ["Plan","Core / list","Subscriber / IPv4","WAN / node","Router row"]{th{(title)}}}}tbody{@for e in &view.rows{tr{
                    td{(match e.action{Action::Add=>"add",Action::Keep=>"keep",Action::Remove=>"remove"})}td{(&e.router)br;(&e.list)}td{(&e.address)br;small{(&e.subscriber)}}td{(&e.wan)br;(&e.node)}td{(&e.entry_id)}
                }}}}}
                p.help{"At most 100 entries per page. Only exact owned static host rows can appear as removals."}
                @if view.offset>0{a href=(page_link(&q,view.offset.saturating_sub(100))){"Previous page"}} " " @if view.offset+100<view.total{a href=(page_link(&q,view.offset+100)){"Next page"}}
            }
        } @else {section.card{p{"No committed core plan is available. Configure the mappings and collect a complete healthy snapshot."}}}
    }).into_response()
}
#[derive(Deserialize)]
pub struct MappingForm {
    token: String,
    revision: String,
    action: String,
    wan: String,
    #[serde(default)]
    router: String,
    #[serde(default)]
    list: String,
    #[serde(default)]
    table: String,
}
pub async fn save(State(s): State<Shared>, Form(f): Form<MappingForm>) -> Response {
    let a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    if f.revision != revision(&a.settings, &a.generation) {
        return error(
            &a,
            "Settings or generation changed since this form was loaded; reload before saving",
        );
    }
    let previous = a.settings.clone();
    let generation = a.generation.clone();
    let mut next = previous.clone();
    match f.action.as_str() {
        "save" => {
            if !tables(&a, &f.router).contains(&f.table) {
                return error(
                    &a,
                    "Routing table metadata changed or is stale; read core inventory again",
                );
            }
            next.core_lists.bindings.retain(|b| b.wan != f.wan);
            next.core_lists.bindings.push(Binding {
                wan: f.wan,
                router: f.router,
                list: f.list,
                table: f.table,
            });
            next.core_lists.bindings.sort_by(|a, b| a.wan.cmp(&b.wan));
            if next.core_lists.owner.is_empty() {
                next.core_lists.owner = uuid::Uuid::new_v4().to_string();
            }
        }
        "remove" => {
            let before = next.core_lists.bindings.len();
            next.core_lists.bindings.retain(|b| b.wan != f.wan);
            if before == next.core_lists.bindings.len() {
                return error(&a, "Core mapping no longer exists");
            }
        }
        _ => return error(&a, "Invalid core mapping action"),
    }
    drop(a);
    wan_web::commit(s, previous, generation, next, "/core-lists").await
}
