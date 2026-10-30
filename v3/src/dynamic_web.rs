use crate::{
    dynamic::{self, Replacement, Target, Wan},
    model::{Node, Settings},
    topology_edits::{self, Operation},
    wan_web,
    web::{App, Shared, check, csrf, error},
};
use axum::{
    Form,
    extract::State,
    response::{IntoResponse, Response},
};
use maud::{Markup, html};
use serde::Deserialize;

pub(crate) fn guards(a: &App) -> Markup {
    guards_revision(
        a,
        &topology_edits::revision(&a.settings).unwrap_or_default(),
    )
}
pub(crate) fn guards_revision(a: &App, revision: &str) -> Markup {
    html! {(csrf(a))input type="hidden" name="revision" value=(revision);}
}
/// Reject edits that would make persisted pre-conversion evidence impossible to adopt.
pub(crate) fn validate_evidence_edit(a: &App, next: &Settings) -> anyhow::Result<()> {
    if a.settings.dynamic.is_none() || next.dynamic.is_none() {
        return Ok(());
    }
    let committed = crate::store::load_assignments(&a.root)?;
    let observation = crate::sessions::load(&a.root)?;
    let evidence = std::iter::once(&committed).chain(observation.as_ref().map(|o| &o.assignments));
    let legacy: Vec<_> = evidence
        .filter(|s| s.assignments.keys().any(|id| !s.routes.contains_key(id)))
        .collect();
    if legacy.is_empty() {
        return Ok(());
    }
    let rows = if crate::normalizer::has_bindings(next) {
        crate::normalizer::collect(
            next,
            &a.inventories,
            &a.inventory_errors,
            crate::orchestrator::now(),
        )?
    } else {
        Vec::new()
    };
    for state in legacy {
        dynamic::adopt(next, &rows, state).map_err(|e| {
            anyhow::anyhow!("Publish a valid dynamic generation before this source edit: {e}")
        })?;
    }
    Ok(())
}
fn nodes(s: &Settings, current: &str) -> Markup {
    html! {@for n in &s.nodes{option value=(&n.id) selected[n.id==current]{(&n.name) " · " (&n.id)}}}
}
fn wans(s: &Settings, current: &str) -> Markup {
    html! {@for w in dynamic::wan_nodes(s){option value=(&w.id) selected[w.id==current]{(&w.name) " · " (&w.id)}}}
}
fn target_key(t: &Target) -> String {
    serde_json::to_string(t).expect("target serializes")
}
pub(crate) fn target_label(s: &Settings, key: &str) -> String {
    let Ok(t) = serde_json::from_str::<Target>(key) else {
        return key.into();
    };
    let node = s
        .nodes
        .iter()
        .find(|n| n.id == t.node)
        .map(|n| n.name.as_str())
        .unwrap_or(&t.node);
    let wan = s
        .dynamic
        .as_ref()
        .and_then(|c| c.wans.iter().find(|w| w.id == t.wan))
        .map(|w| w.name.as_str())
        .unwrap_or(&t.wan);
    format!("{node} · {wan}")
}
#[derive(Default)]
struct Counts {
    nodes: std::collections::HashMap<String, usize>,
    targets: Option<std::collections::HashMap<(String, Target), usize>>,
}
impl Counts {
    fn new(s: &Settings, state: &crate::assignment::AssignmentState) -> Self {
        let parents: std::collections::HashMap<_, _> = s
            .nodes
            .iter()
            .map(|n| (n.id.as_str(), n.parent.as_str()))
            .collect();
        let mut result = Self {
            targets: (state.routes.len() == state.assignments.len()).then(Default::default),
            ..Default::default()
        };
        for pin in state.assignments.values() {
            let mut node = pin.node.as_str();
            let mut depth = 0;
            while !node.is_empty() && depth <= s.nodes.len() {
                *result.nodes.entry(node.into()).or_default() += 1;
                node = parents.get(node).copied().unwrap_or("");
                depth += 1;
            }
        }
        if let Some(targets) = &mut result.targets {
            for route in state.routes.values() {
                *targets
                    .entry((
                        route.policy.clone(),
                        Target {
                            node: route.node.clone(),
                            wan: route.wan.clone(),
                        },
                    ))
                    .or_default() += 1;
            }
        }
        result
    }
}
pub(crate) struct Impacts {
    committed: Option<Counts>,
    candidate: Option<Counts>,
}
impl Impacts {
    pub(crate) fn captured(
        s: &Settings,
        committed: Option<&crate::assignment::AssignmentState>,
        candidate: Option<&crate::assignment::AssignmentState>,
    ) -> Self {
        Self {
            committed: committed.map(|state| Counts::new(s, state)),
            candidate: candidate.map(|state| Counts::new(s, state)),
        }
    }
    fn load(a: &App) -> Self {
        Self {
            committed: a
                .generation
                .as_ref()
                .and_then(|_| crate::store::load_assignments(&a.root).ok())
                .map(|s| Counts::new(&a.settings, &s)),
            candidate: crate::sessions::load(&a.root)
                .ok()
                .flatten()
                .map(|o| Counts::new(&a.settings, &o.assignments)),
        }
    }
    fn show(&self, node: &str, target: Option<(&str, &Target)>) -> Markup {
        let count = |c: &Option<Counts>| -> String {
            let Some(c) = c else {
                return "unavailable".into();
            };
            match target {
                Some((policy, t)) => c
                    .targets
                    .as_ref()
                    .map(|m| {
                        m.get(&(policy.into(), t.clone()))
                            .copied()
                            .unwrap_or(0)
                            .to_string()
                    })
                    .unwrap_or_else(|| "unavailable (legacy WAN evidence)".into()),
                None => c.nodes.get(node).copied().unwrap_or(0).to_string(),
            }
        };
        html! {p.note{"Affected subscribers · committed: " (count(&self.committed)) " · stored candidate: " (count(&self.candidate)) ". Captured local evidence; inactive assignment records are included."}}
    }
}
pub(crate) fn enable(a: &App) -> Markup {
    html! {section.card{h2{"Independent topology"}p.help{"Separate inventory sources, exported nodes, and routing WANs. Existing IDs, policies, pins, and limits are preserved after validation."}form method="post" action="/dynamic/enable" data-editor-title="Enable independent topology"{(guards(a))button{"Enable independent topology"}}}}
}
pub(crate) fn node_tools(a: &App, n: &Node) -> Markup {
    let revision = topology_edits::revision(&a.settings).unwrap_or_default();
    let c = a.settings.dynamic.as_ref().unwrap();
    let impacts = Impacts::load(a);
    let references = c
        .policies
        .iter()
        .flat_map(|p| &p.targets)
        .filter(|t| t.node == n.id)
        .count();
    let pins = c.pins.values().filter(|p| p.target.node == n.id).count();
    let children = a.settings.nodes.iter().filter(|p| p.parent == n.id).count();
    html! {(impacts.show(&n.id,None))p.note{(children) " dependent children · " (references) " destinations · " (pins) " pins"}
    form method="post" action="/dynamic/node" data-editor-title=(format!("Promote {}",n.name)){(guards_revision(a,&revision))input type="hidden" name="node" value=(&n.id);input type="hidden" name="action" value="promote";p{"Move this branch one level upward. A direct child becomes an independent root. Routing WAN assignments stay unchanged."}button{"Promote node"}}
    form method="post" action="/dynamic/node" data-editor-title=(format!("Move / demote {}",n.name)){(guards_revision(a,&revision))input type="hidden" name="node" value=(&n.id);input type="hidden" name="action" value="move";
    label{"New parent" select name="parent"{option value="" selected[n.parent.is_empty()]{"Independent root"}@for parent in a.settings.nodes.iter().filter(|p|p.id!=n.id){option value=(&parent.id) selected[parent.id==n.parent]{(&parent.name) " · " (&parent.id)}}}}button{"Move / demote node"}}
    form method="post" action="/dynamic/node" data-editor-title=(format!("Remove {}",n.name)){(guards_revision(a,&revision))input type="hidden" name="node" value=(&n.id);input type="hidden" name="action" value="remove";
    (impacts.show(&n.id,None))p{"Choose replacements for dependent destinations and children. This removes only the selected exported node. Router records and routing WANs remain unchanged."}
    label{"Replacement destination" select name="replacement"{option value=""{"No destination replacement"}@for target in a.settings.nodes.iter().filter(|p|p.id!=n.id){option value=(&target.id){(&target.name) " · " (&target.id)}}}}
    label{"Move direct children to" select name="child_parent"{option value="__unset"{"Choose if children exist"}option value=""{"Independent roots"}@for parent in a.settings.nodes.iter().filter(|p|p.id!=n.id){option value=(&parent.id){(&parent.name) " · " (&parent.id)}}}}
    button.danger{"Remove node with selected replacements"}}
    }
}
pub(crate) fn policies(
    a: &App,
    policy_query: &str,
    target_query: &str,
    impacts: &Impacts,
) -> Markup {
    let revision = topology_edits::revision(&a.settings).unwrap_or_default();
    let c = a.settings.dynamic.as_ref().unwrap();
    let selections: std::collections::HashMap<_, _> = a
        .settings
        .selections
        .iter()
        .map(|s| (s.id.as_str(), s))
        .collect();
    let policy_query = policy_query.to_lowercase();
    let target_query = target_query.to_lowercase();
    html! {section.card{h2{"Selected source policies"}p.help{"Select eligible node/WAN pairs. Mixed services can share destinations. Continuous sessions keep compatible pairs; new sessions and reconnects can balance."}
       form method="get" action="/assignments"{label{"Find policy name or ID" input name="policy_q" value=(&policy_query);}label{"Find destination node or WAN ID" input name="target_q" value=(&target_query);}button{"Filter policies"}}
       p.help{"First 200 matching policies and destinations per policy are shown. Source summaries show the first 20 members. Use filters to find other entries."}
       div.cardgrid { @for p in c.policies.iter().filter(|p| policy_query.is_empty() || format!("{} {}",p.id,p.name).to_lowercase().contains(&policy_query)).take(200) {
       section.card{h3{(if p.name.is_empty(){&p.id}else{&p.name})}p.help{(&p.id)}p.help{@for id in p.selections.iter().take(20){@if let Some(x)=selections.get(id.as_str()){(&x.kind) " / " (&x.value) " · "}} (p.selections.len()) " selected sources"}
       p{(if p.balance{"Automatic"}else{"Manual default / pins"}) " · " (&p.metric)}
       @for target in p.targets.iter().filter(|t| target_query.is_empty() || format!("{} {}",t.node,t.wan).to_lowercase().contains(&target_query)).take(200) {
       p{(target_label(&a.settings,&target_key(target))) @if target==&p.default{" · default"}}
       form method="post" action="/dynamic/policy" data-editor-title="Remove destination"{(guards_revision(a,&revision))input type="hidden" name="policy" value=(&p.id);input type="hidden" name="action" value="remove";input type="hidden" name="target" value=(target_key(target));
       (impacts.show(&target.node,Some((&p.id,target))))
       label{"Explicit replacement for active assignments and pins" select name="replacement"{option value=""{"Choose another eligible pair"}@for replacement in p.targets.iter().filter(|t|*t!=target){option value=(target_key(replacement)){(target_label(&a.settings,&target_key(replacement)))}}}}button.danger{"Remove destination"}}
       }
       form method="post" action="/dynamic/policy" data-editor-title="Add destination"{(guards_revision(a,&revision))input type="hidden" name="policy" value=(&p.id);input type="hidden" name="action" value="add";
       label{"Exported node" select name="node"{(nodes(&a.settings,""))}}label{"Routing WAN" select name="wan"{(wans(&a.settings,""))}}button{"Add destination"}}
       form method="post" action="/dynamic/policy" data-editor-title="Assignment mode and default"{(guards_revision(a,&revision))input type="hidden" name="policy" value=(&p.id);input type="hidden" name="action" value="mode";
       label{"Policy name" input name="name" value=(&p.name) maxlength="256";}
    label{"Assignment mode" select name="balance"{option value="false" selected[!p.balance]{"Manual default / pins"}option value="true" selected[p.balance]{"Automatic balancing"}}}
       label{"Metric" select name="metric"{option value="online_clients" selected[p.metric=="online_clients"]{"Online clients · reconnect balancing"}option value="planned_load" selected[p.metric=="planned_load"]{"Planned Mbps / capacity"}}}
       label{"Default destination" select name="target"{@for target in &p.targets{option value=(target_key(target)) selected[target==&p.default]{(target_label(&a.settings,&target_key(target)))}}}}button{"Save assignment policy"}}
       }
       }
       }
       @if c.policies.is_empty(){p{"Select sources on Sources to create the first policy."}}
       }}
}
pub(crate) fn wan_panel(a: &App) -> Markup {
    let revision = topology_edits::revision(&a.settings).unwrap_or_default();
    let choices = wan_web::interfaces(a);
    let c = a.settings.dynamic.as_ref().unwrap();
    html! {section.card{h2{"Routing WANs"}p.help{"These mappings are independent of exported roots. One WAN can serve several roots, and one root can serve clients on several WANs."}}
    div.cardgrid { @for current in c.wans.iter().map(Some).chain(std::iter::once(None)) {
    @let id=current.map(|w|w.id.as_str()).unwrap_or("");
    @let selected=current.map(|w|serde_json::to_string(&[&w.router,&w.interface]).unwrap()).unwrap_or_default();
    section.card{h2{(current.map(|w|w.name.as_str()).unwrap_or("Add routing WAN"))}
    form method="post" action="/dynamic/wan" data-editor-title=(current.map(|w|format!("Configure {}",w.name)).unwrap_or_else(||"Add routing WAN".into())){(guards_revision(a,&revision))input type="hidden" name="action" value="save";input type="hidden" name="original_id" value=(id);
    label{"Stable WAN ID" input required name="id" value=(id) readonly[current.is_some()];}
    label{"Display name" input required name="name" value=(current.map(|w|w.name.as_str()).unwrap_or(""));}
    label{"Router / interface" select name="selection" required{option value=""{"Read fresh router inventory and select an interface"}@if !selected.is_empty()&&!choices.iter().any(|x|x.value==selected){option value=(&selected) selected{"Current mapping · refresh inventory before save"}}@for choice in &choices{option value=(&choice.value) selected[choice.value==selected]{(&choice.label)}}}}
    @for (label,key,value) in [("Download Mbps","download",current.map(|w|w.download).unwrap_or(0.)),("Upload Mbps","upload",current.map(|w|w.upload).unwrap_or(0.))]{label{(label)input required type="number" min="0.000001" step="any" name=(key) value=(value);}}
    button{"Save routing WAN"}}
    @if current.is_some(){form method="post" action="/dynamic/wan" data-editor-title="Remove routing WAN"{(guards_revision(a,&revision))input type="hidden" name="id" value=(id);input type="hidden" name="action" value="remove";p{"Remove dependent policy and core mappings before removing this WAN."}button.danger{"Remove routing WAN"}}}
    }
    }
    }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub(crate) struct EditForm {
    token: String,
    revision: String,
    action: String,
    node: String,
    parent: String,
    replacement: String,
    child_parent: String,
    policy: String,
    target: String,
    wan: String,
    balance: bool,
    metric: String,
    id: String,
    original_id: String,
    name: String,
    selection: String,
    download: f64,
    upload: f64,
}
impl Default for EditForm {
    fn default() -> Self {
        Self {
            token: String::new(),
            revision: String::new(),
            action: String::new(),
            node: String::new(),
            parent: String::new(),
            replacement: String::new(),
            child_parent: "__unset".into(),
            policy: String::new(),
            target: String::new(),
            wan: String::new(),
            balance: false,
            metric: "online_clients".into(),
            id: String::new(),
            original_id: String::new(),
            name: String::new(),
            selection: String::new(),
            download: 0.,
            upload: 0.,
        }
    }
}
fn authorized(a: &App, f: &EditForm) -> anyhow::Result<()> {
    anyhow::ensure!(
        check(a, &f.token).is_ok(),
        "Invalid form token. Reload the page."
    );
    anyhow::ensure!(
        f.revision == topology_edits::revision(&a.settings)?,
        "Settings changed since this form was loaded; reload before saving"
    );
    Ok(())
}
pub(crate) async fn enable_dynamic(State(s): State<Shared>, Form(f): Form<EditForm>) -> Response {
    let a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    if let Err(e) = authorized(&a, &f) {
        return error(&a, e);
    }
    let next = match (|| -> anyhow::Result<Settings> {
        let next = dynamic::migrate(&a.settings)?;
        let committed = crate::store::load_assignments(&a.root)?;
        let observation = crate::sessions::load(&a.root)?;
        if crate::normalizer::has_bindings(&next)
            && (!committed.assignments.is_empty() || observation.is_some())
        {
            let rows = crate::normalizer::collect(
                &next,
                &a.inventories,
                &a.inventory_errors,
                crate::orchestrator::now(),
            )?;
            dynamic::adopt(&next, &rows, &committed)?;
            if let Some(o) = observation {
                dynamic::adopt(&next, &rows, &o.assignments)?;
            }
        }
        Ok(next)
    })() {
        Ok(next) => next,
        Err(e) => return error(&a, e),
    };
    let previous = a.settings.clone();
    let generation = a.generation.clone();
    drop(a);
    wan_web::commit(s, previous, generation, next, "/topology").await
}
pub(crate) async fn node_edit(State(s): State<Shared>, Form(f): Form<EditForm>) -> Response {
    let a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    if let Err(e) = authorized(&a, &f) {
        return error(&a, e);
    }
    let operation = match f.action.as_str() {
        "move" => Operation::Reparent {
            id: f.node,
            parent: f.parent,
        },
        "promote" => {
            let Some(n) = a.settings.nodes.iter().find(|n| n.id == f.node) else {
                return error(&a, "Node no longer exists");
            };
            if n.parent.is_empty() {
                return error(&a, "Node is already an independent root");
            }
            let parent = a
                .settings
                .nodes
                .iter()
                .find(|p| p.id == n.parent)
                .map(|p| p.parent.clone())
                .unwrap_or_default();
            Operation::Reparent { id: f.node, parent }
        }
        "remove" => Operation::Remove {
            id: f.node,
            replacement: (!f.replacement.is_empty()).then_some(f.replacement),
            child_parent: (f.child_parent != "__unset").then_some(f.child_parent),
        },
        _ => return error(&a, "Unknown topology operation"),
    };
    let next = match topology_edits::preview(&a.settings, operation) {
        Ok(p) => p.settings,
        Err(e) => return error(&a, e),
    };
    let previous = a.settings.clone();
    let generation = a.generation.clone();
    drop(a);
    wan_web::commit(s, previous, generation, next, "/topology").await
}
pub(crate) async fn policy_edit(State(s): State<Shared>, Form(f): Form<EditForm>) -> Response {
    let a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    if let Err(e) = authorized(&a, &f) {
        return error(&a, e);
    }
    let mut next = a.settings.clone();
    let Some(c) = next.dynamic.as_mut() else {
        return error(&a, "Enable independent topology first");
    };
    let Some(p) = c.policies.iter_mut().find(|p| p.id == f.policy) else {
        return error(&a, "Policy no longer exists");
    };
    let result = (|| -> anyhow::Result<()> {
        match f.action.as_str() {
            "add" => {
                let t = Target {
                    node: f.node,
                    wan: f.wan,
                };
                anyhow::ensure!(!p.targets.contains(&t), "Destination already exists");
                p.targets.push(t);
            }
            "mode" => {
                p.name = f.name;
                p.default = serde_json::from_str(&f.target)?;
                p.balance = f.balance;
                p.metric = f.metric;
            }
            "remove" => {
                let from: Target = serde_json::from_str(&f.target)?;
                let to: Target = serde_json::from_str(&f.replacement)
                    .map_err(|_| anyhow::anyhow!("Choose an explicit replacement destination"))?;
                anyhow::ensure!(
                    from != to && p.targets.contains(&from) && p.targets.contains(&to),
                    "Choose a different eligible replacement"
                );
                p.targets.retain(|t| t != &from);
                if p.default == from {
                    p.default = to.clone();
                }
                for pin in c.pins.values_mut() {
                    if pin.policy == p.id && pin.target == from {
                        pin.target = to.clone();
                    }
                }
                c.replacements
                    .retain(|r| !(r.policy == p.id && r.from == from));
                c.replacements.push(Replacement {
                    policy: p.id.clone(),
                    from,
                    to,
                });
            }
            _ => anyhow::bail!("Unknown policy operation"),
        }
        Ok(())
    })();
    if let Err(e) = result {
        return error(&a, e);
    }
    let previous = a.settings.clone();
    let generation = a.generation.clone();
    drop(a);
    wan_web::commit(s, previous, generation, next, "/assignments").await
}
pub(crate) async fn wan_edit(State(s): State<Shared>, Form(f): Form<EditForm>) -> Response {
    let a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    if let Err(e) = authorized(&a, &f) {
        return error(&a, e);
    }
    let mut next = a.settings.clone();
    let Some(c) = next.dynamic.as_mut() else {
        return error(&a, "Enable independent topology first");
    };
    match f.action.as_str() {
        "save" => {
            if !f.original_id.is_empty() && f.id != f.original_id {
                return error(&a, "Stable WAN IDs cannot be renamed");
            }
            if f.original_id.is_empty() && c.wans.iter().any(|w| w.id == f.id) {
                return error(&a, "WAN ID already exists");
            }
            if !f.original_id.is_empty() && !c.wans.iter().any(|w| w.id == f.original_id) {
                return error(&a, "WAN no longer exists");
            }
            if !wan_web::interfaces(&a)
                .iter()
                .any(|i| i.value == f.selection)
            {
                return error(
                    &a,
                    "WAN interface metadata changed or is stale; read inventory again",
                );
            }
            let Ok([router, interface]) = serde_json::from_str::<[String; 2]>(&f.selection) else {
                return error(&a, "Invalid WAN interface selection");
            };
            c.wans.retain(|w| w.id != f.id);
            c.wans.push(Wan {
                id: f.id,
                name: f.name,
                router,
                interface,
                download: f.download,
                upload: f.upload,
            });
        }
        "remove" => {
            let before = c.wans.len();
            c.wans.retain(|w| w.id != f.id);
            if c.wans.len() == before {
                return error(&a, "WAN no longer exists");
            }
        }
        _ => return error(&a, "Unknown WAN operation"),
    }
    let previous = a.settings.clone();
    let generation = a.generation.clone();
    drop(a);
    wan_web::commit(s, previous, generation, next, "/wans").await
}

pub(crate) fn source_editor(
    a: &App,
    selection: &crate::selection::Selection,
    revision: &str,
) -> Markup {
    let Some(c) = &a.settings.dynamic else {
        return html! {};
    };
    let policy = c
        .policies
        .iter()
        .find(|p| p.selections.contains(&selection.id));
    let current = policy.map(|p| p.id.as_str()).unwrap_or("");
    let node = policy.map(|p| p.default.node.as_str()).unwrap_or("");
    let wan = policy.map(|p| p.default.wan.as_str()).unwrap_or("");
    let value = serde_json::to_string(&[
        &selection.router,
        &selection.kind,
        &selection.selector,
        &selection.value,
    ])
    .expect("source serializes");
    html! {form method="post" action="/sources/selection" data-editor-title=(format!("Source {}",selection.value)){
        (guards_revision(a,revision))input type="hidden" name="original_id" value=(&selection.id);input type="hidden" name="selection" value=(value);
        p{(&selection.router) " / " (&selection.kind) " / " (&selection.value)}
        label{"Assignment policy" select name="policy"{option value="" selected[current.is_empty()]{"Create a new policy"}@for p in &c.policies{option value=(&p.id) selected[p.id==current]{(if p.name.is_empty(){&p.id}else{&p.name})}}}}
        label{"Exported node for a new policy" select name="node"{(nodes(&a.settings,node))}}
        label{"Routing WAN for a new policy" select name="wan"{(wans(&a.settings,wan))}}
        label{input type="checkbox" name="exclude" value="true" checked[selection.exclude];"Exclude matching records"}
        p.help{"Regrouping preserves selected destinations. Replace destinations explicitly on Assignments before changing them. Clear old policy pins before excluding or regrouping a selection."}
        button{"Save source selection"}
    }}
}
