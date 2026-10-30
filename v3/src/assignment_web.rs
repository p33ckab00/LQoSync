use crate::{
    assignment::{self, AssignmentState, Pin, Pool},
    model::{Settings, Subscriber},
    normalizer, orchestrator,
    routeros::Inventory,
    sessions, store, wan_web,
    web::{App, Shared, check, csrf, error, shell},
};
use axum::{
    Form,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use maud::html;
use serde::Deserialize;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Page {
    q: String,
    offset: usize,
    policy_q: String,
    target_q: String,
}
fn allowed(settings: &Settings, source: &str) -> Vec<String> {
    if let Some(c) = &settings.dynamic {
        return c
            .policies
            .iter()
            .find(|p| p.id == source || p.selections.iter().any(|id| id == source))
            .map(|p| {
                p.targets
                    .iter()
                    .map(|t| serde_json::to_string(t).expect("target serializes"))
                    .collect()
            })
            .unwrap_or_default();
    }
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
    target: String,
}
struct View {
    impacts: crate::dynamic_web::Impacts,
    rows: Vec<Row>,
    total: usize,
    offset: usize,
    stats: BTreeMap<String, (usize, f64, f64)>,
    label: String,
    candidate: bool,
}
#[derive(Clone)]
struct Capture {
    settings: Settings,
    root: PathBuf,
    generation: Option<String>,
    inventories: BTreeMap<String, Arc<Inventory>>,
    errors: BTreeMap<String, String>,
    epochs: BTreeMap<String, u64>,
}
impl Capture {
    fn new(a: &App) -> Self {
        Self {
            settings: a.settings.clone(),
            root: a.root.clone(),
            generation: a.generation.clone(),
            inventories: a.inventories.clone(),
            errors: a.inventory_errors.clone(),
            epochs: a.read_epochs.clone(),
        }
    }
    fn unchanged(&self, a: &App) -> bool {
        a.settings == self.settings
            && a.root == self.root
            && a.generation == self.generation
            && a.inventory_errors == self.errors
            && a.read_epochs == self.epochs
            && orchestrator::same_inventories(&a.inventories, &self.inventories)
    }
    fn fresh(&self) -> anyhow::Result<()> {
        let now = orchestrator::now();
        for router in normalizer::selected_routers(&self.settings) {
            let inv = self
                .inventories
                .get(router)
                .ok_or_else(|| anyhow::anyhow!("Selected inventory is missing"))?;
            anyhow::ensure!(
                now >= inv.collected_at
                    && now - inv.collected_at <= normalizer::MAX_INVENTORY_AGE_SECONDS,
                "Candidate inventory expired; read inventory again"
            );
        }
        assignment::validate_wans(&self.settings, &self.inventories, &self.errors, now)
    }
}
struct Snapshot {
    impacts: crate::dynamic_web::Impacts,
    subscribers: Vec<Subscriber>,
    assignments: AssignmentState,
    label: String,
    candidate: bool,
}
fn assignment_view(input: &Capture) -> anyhow::Result<Snapshot> {
    // Invalid stored state must not be hidden by a live preview or a fallback.
    let (committed, baseline) = store::load_assignment_snapshot(&input.root)?;
    let observation = sessions::load(&input.root)?;
    let impacts = crate::dynamic_web::Impacts::captured(
        &input.settings,
        input.generation.as_ref().map(|_| &baseline),
        observation.as_ref().map(|o| &o.assignments),
    );
    if normalizer::has_bindings(&input.settings) {
        let live = (|| -> anyhow::Result<_> {
            let mut subscribers = normalizer::collect(
                &input.settings,
                &input.inventories,
                &input.errors,
                orchestrator::now(),
            )?;
            let count_mode = crate::dynamic::count_mode(&input.settings);
            let assignments = if count_mode {
                sessions::plan(
                    &input.settings,
                    &mut subscribers,
                    &baseline,
                    observation.as_ref(),
                    &input.inventories,
                    &input.errors,
                    orchestrator::now(),
                )?
                .assignments
            } else {
                assignment::assign(
                    &input.settings,
                    &mut subscribers,
                    &baseline,
                    &input.inventories,
                    &input.errors,
                    orchestrator::now(),
                )?
            };
            Ok((subscribers, assignments))
        })();
        return Ok(match live {
            Ok((subscribers, assignments)) => Snapshot {
                impacts, subscribers, assignments, candidate: true,
                label: "Candidate assignments · fresh selected-source inventory. Local preview only; not committed or applied.".into(),
            },
            Err(error) => Snapshot {
                impacts, subscribers: committed, assignments: baseline, candidate: false,
                label: format!("Committed snapshot · live candidate unavailable: {error}"),
            },
        });
    }
    Ok(Snapshot {
        impacts,
        subscribers: committed,
        assignments: baseline,
        candidate: false,
        label: "Committed snapshot · saved local assignments.".into(),
    })
}
pub async fn assignments(State(s): State<Shared>, Query(page): Query<Page>) -> Response {
    let captured = {
        let a = s.lock().await;
        Capture::new(&a)
    };
    let q = page.q.chars().take(256).collect::<String>();
    let search = q.to_lowercase();
    let input = captured.clone();
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<View> {
        let snapshot = assignment_view(&input)?;
        let topology = if input.settings.dynamic.is_none() {
            Some(assignment::Topology::new(&input.settings)?)
        } else {
            None
        };
        let mut matches = vec![];
        let mut stats: BTreeMap<String, (usize, f64, f64)> = BTreeMap::new();
        for subscriber in snapshot.subscribers {
            let root = if let Some(t) = &topology {
                t.root(&subscriber.node)
                    .map(|n| n.id.as_str())
                    .unwrap_or("Unavailable")
            } else {
                snapshot
                    .assignments
                    .routes
                    .get(&subscriber.id)
                    .map(|r| r.wan.as_str())
                    .unwrap_or("Unavailable")
            };
            let load = stats.entry(root.into()).or_default();
            load.0 += 1;
            load.1 += subscriber.download;
            load.2 += subscriber.upload;
            if search.is_empty()
                || subscriber.id.to_lowercase().contains(&search)
                || subscriber.name.to_lowercase().contains(&search)
                || subscriber.ipv4.contains(&search)
            {
                let source = snapshot
                    .assignments
                    .assignments
                    .get(&subscriber.id)
                    .map(|p| p.source_node.clone())
                    .unwrap_or_else(|| subscriber.node.clone());
                let target = snapshot
                    .assignments
                    .routes
                    .get(&subscriber.id)
                    .map(|r| {
                        serde_json::to_string(&crate::dynamic::Target {
                            node: r.node.clone(),
                            wan: r.wan.clone(),
                        })
                        .expect("target serializes")
                    })
                    .unwrap_or_else(|| {
                        if input.settings.dynamic.is_some() {
                            String::new()
                        } else {
                            subscriber.node.clone()
                        }
                    });
                matches.push(Row {
                    target,
                    record: subscriber,
                    source,
                });
            }
        }
        let total = matches.len();
        let offset = page.offset.min(total.saturating_sub(1) / 100 * 100);
        let rows = matches.into_iter().skip(offset).take(100).collect();
        Ok(View {
            impacts: snapshot.impacts,
            rows,
            total,
            offset,
            stats,
            label: snapshot.label,
            candidate: snapshot.candidate,
        })
    })
    .await;
    let a = s.lock().await;
    if !captured.unchanged(&a) {
        return error(
            &a,
            "Settings, generation or inventory changed; reload assignments",
        );
    }
    let view = match result {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => return error(&a, e),
        Err(_) => return error(&a, "Assignment view worker failed"),
    };
    if view.candidate
        && let Err(e) = captured.fresh()
    {
        return error(&a, e);
    }
    let render = a.clone();
    drop(a);
    tokio::task::spawn_blocking(move || {
    let a = render;
    let revision = crate::topology_edits::revision(&a.settings).unwrap_or_default();
    shell(&a,"Assignments",html!{
        section.card{h2{"Subscriber WAN assignments"}p{(&view.label)}p{(&a.status)}p.help{"These are intended local shaping assignments. Core routing and address-list synchronization are not implemented."}
            div.tablewrap{table{thead{tr{th{"WAN"}th{"Subscribers"}th{"Plan load ↓ / ↑ Mbps"}}}tbody{@for (wan,(count,download,upload)) in &view.stats {tr{td{(wan)}td{(count)}td{(download) " / " (upload)}}}}}}
        }
        @if a.settings.dynamic.is_none(){
        section.card{h2{"Source destination pools"}p.help{"Choose branches or custom groups under mapped WAN roots. Pins override balancing. Online-count mode keeps continuous sessions stable and reassesses new sessions or reconnects."}
            @let source_ids:std::collections::HashSet<_>=a.settings.nodes.iter().filter(|n|normalizer::service(&n.kind).is_some()&&!n.router.is_empty()&&!n.source.is_empty()).map(|n|n.id.as_str()).chain(a.settings.selections.iter().filter(|s|!s.exclude).map(|s|s.node.as_str())).collect();
            @for source in a.settings.nodes.iter().filter(|n|source_ids.contains(n.id.as_str())).take(200){
                @let pool=a.settings.assignment.pools.iter().find(|p|p.source_node==source.id);
                @let targets=allowed(&a.settings,&source.id);
                details{summary{(&source.name) " · " (&source.id)}
                    @for target in &targets {p{(target) @if *target!=source.id{form.inline method="post" action="/assignments/pool"{(csrf(&a))input type="hidden" name="source" value=(&source.id);input type="hidden" name="target" value=(target);input type="hidden" name="action" value="remove";button{"Remove destination"}}}}}
                    form method="post" action="/assignments/pool"{(csrf(&a))input type="hidden" name="source" value=(&source.id);input type="hidden" name="action" value="add";
                        label{"Add compatible destination" select name="target"{@for target in assignment::eligible_targets(&a.settings,&source.id).unwrap_or_default().iter().filter(|t|!targets.contains(t)){option value=(target){(target)}}}}button{"Add destination"}}
                    form method="post" action="/assignments/pool"{(csrf(&a))input type="hidden" name="source" value=(&source.id);input type="hidden" name="action" value="mode";
                        label{"Assignment mode" select name="balance"{option value="false" selected[pool.is_none_or(|p|!p.balance)]{"Original source / explicit pins"}option value="true" selected[pool.is_some_and(|p|p.balance)]{"Automatic balancing"}}}
                        label{"Automatic metric" select name="metric"{option value="online_clients" selected[pool.is_none_or(|p|p.metric=="online_clients")]{"Online clients · reassess on reconnect"}option value="planned_load" selected[pool.is_some_and(|p|p.metric=="planned_load")]{"Planned Mbps / capacity · retain assignments"}}}
                        button{"Save assignment mode"}}
                    @if pool.is_some(){form method="post" action="/assignments/pool"{(csrf(&a))input type="hidden" name="source" value=(&source.id);input type="hidden" name="action" value="delete";button.danger{"Delete pool"}}}
                }
            }
        }
        } @else {(crate::dynamic_web::policies(&a, &page.policy_q.chars().take(256).collect::<String>(), &page.target_q.chars().take(256).collect::<String>(), &view.impacts))}
        section.card{h2{"Subscribers · " (view.total) " match"}form method="get" action="/assignments"{label{"Search ID, name or IPv4" input name="q" value=(&q);}button{"Search"}}
            p.help{"At most 100 records per page. Clearing a pin releases it; an eligible prior assignment stays in place."}
            div.tablewrap{table{thead{tr{@for title in ["Subscriber","Source / current target","Assignment"]{th{(title)}}}}tbody{
                @for row in &view.rows {tr{td{(&row.record.name)br;small{(&row.record.id) " · " (&row.record.ipv4)}}td{(&row.source) " → " (&row.record.node)}
                    td{@if !row.target.is_empty(){form method="post" action="/assignments/pin" data-editor-title=(format!("Pin {}",row.record.name)){(crate::dynamic_web::guards_revision(&a,&revision))input type="hidden" name="id" value=(&row.record.id);
                        select name="target"{@for target in allowed(&a.settings,&row.source){option value=(&target) selected[target==row.target]{(crate::dynamic_web::target_label(&a.settings,&target))}}}button{"Pin subscriber"}}} @else {p.help{"Routing pair unavailable; refresh selected inventory before pinning."}}
                        @if a.settings.assignment.pins.contains_key(&row.record.id){form method="post" action="/assignments/pin" data-editor-title=(format!("Pin {}",row.record.name)){(crate::dynamic_web::guards_revision(&a,&revision))input type="hidden" name="id" value=(&row.record.id);input type="hidden" name="target" value="";button{"Clear pin"}}}
                    }
                }}
            }}}
            @if view.offset>0{a href=(page_link(&q,view.offset.saturating_sub(100))) {"Previous page"}}
            " " @if view.offset+100<view.total{a href=(page_link(&q,view.offset+100)){"Next page"}}
        }
        section.card{h2{"Stored pins · including inactive subscribers"}p.help{"Filter by subscriber ID with the search above. Clear pins before removing their destinations or source bindings."}
            @if let Some(c)=&a.settings.dynamic {
                @for (id,pin) in c.pins.iter().filter(|(id,_)|q.is_empty()||id.contains(&q)).take(100){p{(id) " · " (&pin.policy) " → " (&pin.target.node) " / " (&pin.target.wan)}form method="post" action="/assignments/pin"{(crate::dynamic_web::guards_revision(&a,&revision))input type="hidden" name="id" value=(id);input type="hidden" name="target" value="";button{"Clear stored pin"}}}
            }
            @for (id,pin) in a.settings.assignment.pins.iter().filter(|(id,_)|q.is_empty()||id.contains(&q)).take(100){p{(id) " · " (&pin.source_node) " → " (&pin.node)}form method="post" action="/assignments/pin"{(crate::dynamic_web::guards_revision(&a,&revision))input type="hidden" name="id" value=(id);input type="hidden" name="target" value="";button{"Clear stored pin"}}}
        }
    }).into_response()
    }).await.unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR,"Assignment rendering worker failed").into_response())
}
#[derive(Deserialize)]
pub struct PoolForm {
    token: String,
    source: String,
    #[serde(default)]
    target: String,
    action: String,
    balance: Option<bool>,
    metric: Option<String>,
}
pub async fn save_pool(State(s): State<Shared>, Form(f): Form<PoolForm>) -> Response {
    let a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    let mut next = a.settings.clone();
    let source = next.nodes.iter().find(|n| n.id == f.source);
    if !source.is_some_and(|n| normalizer::is_source_node(&next, &n.id)) {
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
                metric: "online_clients".into(),
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
                if let Some(metric) = f.metric {
                    pool.metric = metric;
                }
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
    #[serde(default)]
    revision: String,
    id: String,
    #[serde(default)]
    target: String,
}
pub async fn save_pin(State(s): State<Shared>, Form(f): Form<PinForm>) -> Response {
    let a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    if a.settings.dynamic.is_some()
        && f.revision != crate::topology_edits::revision(&a.settings).unwrap_or_default()
    {
        return error(&a, "Settings changed; reload before saving");
    }
    let previous = a.settings.clone();
    let mut next = previous.clone();
    let generation = a.generation.clone();
    let captured = Capture::new(&a);
    if f.target.is_empty() {
        let removed = if let Some(c) = &mut next.dynamic {
            c.pins.remove(&f.id).is_some()
        } else {
            next.assignment.pins.remove(&f.id).is_some()
        };
        if !removed {
            return error(&a, "Subscriber pin no longer exists");
        }
        drop(a);
        return wan_web::commit(s, previous, generation, next, "/assignments").await;
    }
    drop(a);
    let mut a = s.clone().lock_owned().await;
    if !captured.unchanged(&a) {
        return error(
            &a,
            "Settings, generation or inventory changed; reload before saving",
        );
    }
    // Keep the captured inventory and its read tickets unchanged through the save.
    tokio::task::spawn_blocking(move || {
        let result = (|| -> anyhow::Result<()> {
            let snapshot = assignment_view(&captured)?;
            anyhow::ensure!(
                snapshot
                    .subscribers
                    .iter()
                    .any(|subscriber| subscriber.id == f.id),
                "Subscriber is absent from the current assignment view"
            );
            let pin = snapshot
                .assignments
                .assignments
                .get(&f.id)
                .ok_or_else(|| anyhow::anyhow!("Subscriber source ownership is missing"))?;
            if let Some(c) = &mut next.dynamic {
                let p = c
                    .policies
                    .iter()
                    .find(|p| p.id == pin.source_node || p.selections.contains(&pin.source_node))
                    .ok_or_else(|| anyhow::anyhow!("Subscriber source policy is unavailable"))?;
                let target: crate::dynamic::Target = serde_json::from_str(&f.target)?;
                anyhow::ensure!(
                    p.targets.contains(&target),
                    "Pin target is outside its selected policy"
                );
                c.pins.insert(
                    f.id,
                    crate::dynamic::DynamicPin {
                        policy: p.id.clone(),
                        target,
                    },
                );
            } else {
                next.assignment.pins.insert(
                    f.id,
                    Pin {
                        source_node: pin.source_node.clone(),
                        node: f.target,
                    },
                );
            }
            crate::web::nodelimits(&next)?;
            assignment::validate(&next)?;
            if snapshot.candidate {
                captured.fresh()?;
            }
            store::save_settings(&a.root, &next)?;
            Ok(())
        })();
        if let Err(e) = result {
            return error(&a, e);
        }
        a.replace_settings(next);
        a.status =
            "Subscriber pin saved. Candidate assignments remain separate from committed output."
                .into();
        Redirect::to("/assignments").into_response()
    })
    .await
    .unwrap_or_else(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, "Pin save worker failed").into_response()
    })
}
