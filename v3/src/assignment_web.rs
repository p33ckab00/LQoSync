use crate::{
    assignment::{self, Pin, Pool},
    model::{Settings, Subscriber},
    normalizer, store, wan_web,
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
fn allowed(settings: &Settings, source: &str) -> Vec<String> {
    let mut targets = settings
        .assignment
        .pools
        .iter()
        .find(|p| p.source_node == source)
        .map(|p| p.targets.clone())
        .unwrap_or_else(|| vec![source.into()]);
    targets.sort();
    targets
}
fn page_link(query: &str, offset: usize) -> String {
    let query: String = query
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    format!("/assignments?q={query}&offset={offset}")
}
struct Row {
    record: Subscriber,
    source: String,
}
struct View {
    rows: Vec<Row>,
    total: usize,
    offset: usize,
    stats: BTreeMap<String, (usize, f64, f64)>,
}
pub async fn assignments(State(s): State<Shared>, Query(page): Query<Page>) -> Response {
    let (settings, root, generation) = {
        let a = s.lock().await;
        (a.settings.clone(), a.root.clone(), a.generation.clone())
    };
    let q = page.q.chars().take(256).collect::<String>();
    let search = q.to_lowercase();
    let work_settings = settings.clone();
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<View> {
        let (subscribers, state) = store::load_assignment_snapshot(&root)?;
        let topology = assignment::Topology::new(&work_settings)?;
        let mut matches = vec![];
        let mut stats: BTreeMap<String, (usize, f64, f64)> = BTreeMap::new();
        for subscriber in subscribers {
            let root = topology
                .root(&subscriber.node)
                .map(|n| n.id.as_str())
                .unwrap_or("Unavailable");
            let load = stats.entry(root.into()).or_default();
            load.0 += 1;
            load.1 += subscriber.download;
            load.2 += subscriber.upload;
            if search.is_empty()
                || subscriber.id.to_lowercase().contains(&search)
                || subscriber.name.to_lowercase().contains(&search)
                || subscriber.ipv4.contains(&search)
            {
                let source = state
                    .assignments
                    .get(&subscriber.id)
                    .map(|p| p.source_node.clone())
                    .unwrap_or_else(|| subscriber.node.clone());
                matches.push(Row {
                    record: subscriber,
                    source,
                });
            }
        }
        let total = matches.len();
        let offset = page.offset.min(total.saturating_sub(1) / 100 * 100);
        let rows = matches.into_iter().skip(offset).take(100).collect();
        Ok(View {
            rows,
            total,
            offset,
            stats,
        })
    })
    .await;
    let a = s.lock().await;
    if a.settings != settings || a.generation != generation {
        return error(&a, "Settings or generation changed; reload assignments");
    }
    let view = match result {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => return error(&a, e),
        Err(_) => return error(&a, "Assignment view worker failed"),
    };
    shell(&a,"Assignments",html!{
        section.card{h2{"Subscriber WAN assignments"}p{(&a.status)}p.help{"These are intended local shaping assignments. Core routing and address-list synchronization are not implemented."}
            div.tablewrap{table{thead{tr{th{"WAN"}th{"Subscribers"}th{"Plan load ↓ / ↑ Mbps"}}}tbody{@for (wan,(count,download,upload)) in &view.stats {tr{td{(wan)}td{(count)}td{(download) " / " (upload)}}}}}}
        }
        section.card{h2{"Source destination pools"}p.help{"Create matching subscriber branches under mapped WAN roots in Topology. Pins override balancing. Valid previous assignments remain stable; enabling balance does not move existing subscribers."}
            @for source in a.settings.nodes.iter().filter(|n|normalizer::service(&n.kind).is_some()&&!n.router.is_empty()&&!n.source.is_empty()).take(200){
                @let pool=a.settings.assignment.pools.iter().find(|p|p.source_node==source.id);
                @let targets=allowed(&a.settings,&source.id);
                details{summary{(&source.name) " · " (&source.id)}
                    @for target in &targets {p{(target) @if *target!=source.id{form.inline method="post" action="/assignments/pool"{(csrf(&a))input type="hidden" name="source" value=(&source.id);input type="hidden" name="target" value=(target);input type="hidden" name="action" value="remove";button{"Remove destination"}}}}}
                    form method="post" action="/assignments/pool"{(csrf(&a))input type="hidden" name="source" value=(&source.id);input type="hidden" name="action" value="add";
                        label{"Add compatible destination" select name="target"{@for target in assignment::eligible_targets(&a.settings,&source.id).unwrap_or_default().iter().filter(|t|!targets.contains(t)){option value=(target){(target)}}}}button{"Add destination"}}
                    form method="post" action="/assignments/pool"{(csrf(&a))input type="hidden" name="source" value=(&source.id);input type="hidden" name="action" value="mode";
                        label{"New subscriber assignment" select name="balance"{option value="false" selected[pool.is_none_or(|p|!p.balance)]{"Original source / explicit pins"}option value="true" selected[pool.is_some_and(|p|p.balance)]{"Balance new subscribers by planned load / capacity"}}}button{"Save assignment mode"}}
                    @if pool.is_some(){form method="post" action="/assignments/pool"{(csrf(&a))input type="hidden" name="source" value=(&source.id);input type="hidden" name="action" value="delete";button.danger{"Delete pool"}}}
                }
            }
        }
        section.card{h2{"Subscribers · " (view.total) " match"}form method="get" action="/assignments"{label{"Search ID, name or IPv4" input name="q" value=(&q);}button{"Search"}}
            p.help{"At most 100 records per page. Clearing a pin releases it; an eligible prior assignment stays in place."}
            div.tablewrap{table{thead{tr{@for title in ["Subscriber","Source / current target","Assignment"]{th{(title)}}}}tbody{
                @for row in &view.rows {tr{td{(&row.record.name)br;small{(&row.record.id) " · " (&row.record.ipv4)}}td{(&row.source) " → " (&row.record.node)}
                    td{form method="post" action="/assignments/pin"{(csrf(&a))input type="hidden" name="id" value=(&row.record.id);
                        select name="target"{@for target in allowed(&a.settings,&row.source){option value=(&target) selected[target==row.record.node]{(&target)}}}button{"Pin subscriber"}}
                        @if a.settings.assignment.pins.contains_key(&row.record.id){form method="post" action="/assignments/pin"{(csrf(&a))input type="hidden" name="id" value=(&row.record.id);input type="hidden" name="target" value="";button{"Clear pin"}}}
                    }
                }}
            }}}
            @if view.offset>0{a href=(page_link(&q,view.offset.saturating_sub(100))) {"Previous page"}}
            " " @if view.offset+100<view.total{a href=(page_link(&q,view.offset+100)){"Next page"}}
        }
        section.card{h2{"Stored pins · including inactive subscribers"}p.help{"Filter by subscriber ID with the search above. Clear pins before removing their destinations or source bindings."}
            @for (id,pin) in a.settings.assignment.pins.iter().filter(|(id,_)|q.is_empty()||id.contains(&q)).take(100){p{(id) " · " (&pin.source_node) " → " (&pin.node)}form method="post" action="/assignments/pin"{(csrf(&a))input type="hidden" name="id" value=(id);input type="hidden" name="target" value="";button{"Clear stored pin"}}}
        }
    }).into_response()
}
#[derive(Deserialize)]
pub struct PoolForm {
    token: String,
    source: String,
    #[serde(default)]
    target: String,
    action: String,
    balance: Option<bool>,
}
pub async fn save_pool(State(s): State<Shared>, Form(f): Form<PoolForm>) -> Response {
    let a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    let mut next = a.settings.clone();
    let source = next.nodes.iter().find(|n| n.id == f.source);
    if !source.is_some_and(|n| {
        normalizer::service(&n.kind).is_some() && !n.router.is_empty() && !n.source.is_empty()
    }) {
        return error(&a, "Choose a bound subscriber source");
    }
    if f.action == "delete" {
        next.assignment.pools.retain(|p| p.source_node != f.source);
    } else {
        if !next
            .assignment
            .pools
            .iter()
            .any(|p| p.source_node == f.source)
        {
            next.assignment.pools.push(Pool {
                source_node: f.source.clone(),
                targets: vec![f.source.clone()],
                balance: false,
            });
        }
        let pool = next
            .assignment
            .pools
            .iter_mut()
            .find(|p| p.source_node == f.source)
            .expect("pool exists");
        match f.action.as_str() {
            "add" => {
                if !pool.targets.contains(&f.target) {
                    pool.targets.push(f.target);
                }
            }
            "remove" => pool.targets.retain(|t| t != &f.target),
            "mode" => {
                let Some(balance) = f.balance else {
                    return error(&a, "Select an assignment mode");
                };
                pool.balance = balance;
            }
            _ => return error(&a, "Invalid pool action"),
        }
        pool.targets.sort();
    }
    let previous = a.settings.clone();
    let generation = a.generation.clone();
    drop(a);
    wan_web::commit(s, previous, generation, next, "/assignments").await
}
#[derive(Deserialize)]
pub struct PinForm {
    token: String,
    id: String,
    #[serde(default)]
    target: String,
}
pub async fn save_pin(State(s): State<Shared>, Form(f): Form<PinForm>) -> Response {
    let a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    let previous = a.settings.clone();
    let mut next = previous.clone();
    let generation = a.generation.clone();
    let root = a.root.clone();
    if f.target.is_empty() {
        if next.assignment.pins.remove(&f.id).is_none() {
            return error(&a, "Subscriber pin no longer exists");
        }
        drop(a);
        return wan_web::commit(s, previous, generation, next, "/assignments").await;
    }
    drop(a);
    let id = f.id.clone();
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<Pin> {
        let (subscribers, state) = store::load_assignment_snapshot(&root)?;
        anyhow::ensure!(
            subscribers.iter().any(|s| s.id == id),
            "Subscriber is absent from the committed snapshot"
        );
        state
            .assignments
            .get(&id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Subscriber source ownership is missing"))
    })
    .await;
    let pin = match result {
        Ok(Ok(pin)) => pin,
        Ok(Err(e)) => return error(&*s.lock().await, e),
        Err(_) => return error(&*s.lock().await, "Pin lookup worker failed"),
    };
    next.assignment.pins.insert(
        f.id,
        Pin {
            source_node: pin.source_node,
            node: f.target,
        },
    );
    wan_web::commit(s, previous, generation, next, "/assignments").await
}
