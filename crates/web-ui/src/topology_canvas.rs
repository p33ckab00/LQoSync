//! The Topology canvas: cards, wires, a tool dock, and modal editors. Every valid edit saves
//! automatically through the service; card moves save the layout only.
use crate::canvas_layout::{clamp_zoom, fit};
use crate::topology::{Evidence, Reads, now};
use crate::topology_canvas_model::*;
use crate::topology_forms::{blocked, new_key, post, settle, structural_blocked};
use crate::topology_model::*;
use gloo_net::http::Request;
use leptos::prelude::*;
use std::collections::{HashMap, VecDeque};
use wasm_bindgen::JsCast;
use web_contract::Principal;
use web_contract::topology::*;

#[derive(Clone, Copy, PartialEq)]
enum Panel {
    Node,
    Inventory,
    Settings,
    Allocation,
    Preview,
    Excluded,
}

/// The fields of the node modal. An invalid form stays as a draft for its node.
#[derive(Clone, Default, PartialEq)]
struct NodeForm {
    draft: Draft,
    geo: String,
    latitude: String,
    longitude: String,
    error: String,
    /// The access router ID, or empty for none.
    router: String,
    /// The service refusal of the last router change. The draft stays.
    router_error: String,
    /// The `via-` routing list of a WAN node.
    address_list: String,
}

impl NodeForm {
    fn of(n: &Node) -> Self {
        let location = n.geo.as_ref().and_then(|g| g.location.clone());
        Self {
            draft: Draft::of(&Change::Edit(n.clone())),
            geo: match n.geo.as_ref().map(|g| g.geo_type) {
                Some(GeoType::Site) => "site".into(),
                Some(GeoType::Ap) => "ap".into(),
                None => String::new(),
            },
            latitude: location
                .as_ref()
                .map(|l| l.latitude.clone())
                .unwrap_or_default(),
            longitude: location.map(|l| l.longitude).unwrap_or_default(),
            error: String::new(),
            router: n.router_id.clone().unwrap_or_default(),
            router_error: String::new(),
            address_list: n.address_list.clone().unwrap_or_default(),
        }
    }
}

/// The edits of one form that differ from the saved node. An error is a sentence for the form.
fn form_edits(n: &Node, f: &NodeForm) -> Result<Vec<BuilderEdit>, String> {
    let mut edits = Vec::new();
    match build_edit(&Change::Edit(n.clone()), &f.draft).map_err(String::from)? {
        BuilderEdit::NodeUpdate {
            name: None,
            capacity: None,
            ..
        } => {}
        edit => edits.push(edit),
    }
    let location = location_draft(&f.latitude, &f.longitude)?;
    let geo_type = match f.geo.as_str() {
        "site" => Some(GeoType::Site),
        "ap" => Some(GeoType::Ap),
        _ if location.is_some() => return Err("Select Site or AP to save a location.".into()),
        _ => None,
    };
    let geo = geo_type.map(|geo_type| NodeGeo {
        geo_type,
        location: location.clone(),
    });
    if geo != n.geo {
        edits.push(BuilderEdit::NodeLocation {
            node_id: n.id.clone(),
            geo_type,
            location,
        });
    }
    let list = f.address_list.trim();
    if n.kind == "wan" && list != n.address_list.as_deref().unwrap_or_default() {
        edits.push(BuilderEdit::NodeAddressList {
            node_id: n.id.clone(),
            list: (!list.is_empty()).then(|| list.to_string()),
        });
    }
    let router_id = (!f.router.is_empty()).then(|| f.router.clone());
    // A refused router draft stays, but it is not sent again until the operator changes it.
    if router_id != n.router_id && f.router_error.is_empty() {
        edits.push(BuilderEdit::NodeRouter {
            node_id: n.id.clone(),
            router_id,
        });
    }
    Ok(edits)
}

async fn put_layout(
    session: RwSignal<Option<Principal>>,
    body: &LayoutRequest,
) -> Result<u64, &'static str> {
    let csrf = session.get_untracked().map(|p| p.csrf).unwrap_or_default();
    let (signal, _timer) = crate::app::deadline();
    let request = Request::put("/api/v1/topology/layout")
        .abort_signal(Some(&signal))
        .header("x-csrf-token", &csrf)
        .json(body)
        .map_err(|_| "The layout could not be prepared.")?;
    match request.send().await {
        Ok(r) if r.ok() => r
            .json::<LayoutReply>()
            .await
            .map(|l| l.revision)
            .map_err(|_| "The layout answer is not valid."),
        Ok(r) if r.status() == 409 => {
            Err("Another editor moved cards. The canvas shows the saved layout.")
        }
        Ok(r) if r.status() == 401 => {
            session.set(None);
            Err("Your session ended. Sign in again.")
        }
        _ => Err("The layout was not saved. Move a card again to retry."),
    }
}

fn icon(path: &'static str) -> impl IntoView {
    view! {<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d=path/></svg>}
}

const ICON_INVENTORY: &str = "M4 7h16v12H4zM8 7V4h8v3M8 12h8M8 15h5";
const ICON_SETTINGS: &str = "M4 7h10M18 7h2M4 17h4M12 17h8M14 5v4M8 15v4";
const ICON_ALLOCATION: &str = "M12 4v6M6 20v-4h12v4M12 10l-6 6M12 10l6 6";
const ICON_EXCLUDED: &str = "M4 6h16M4 12h10M4 18h7M15 15l5 5M20 15l-5 5";
const ICON_PREVIEW: &str =
    "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z";

#[component]
pub(crate) fn Canvas(
    session: RwSignal<Option<Principal>>,
    data: RwSignal<Option<TopologySummary>>,
    reload: Callback<()>,
) -> impl IntoView {
    let selected = RwSignal::new(None::<String>);
    let panel = RwSignal::new(None::<Panel>);
    let status = RwSignal::new(String::new());
    let zoom = RwSignal::new(1.0_f64);
    let local = RwSignal::new(HashMap::<String, [f64; 2]>::new());
    let layout_revision = RwSignal::new(None::<u64>);
    let layout_generation = StoredValue::new(0u64);
    let forms = RwSignal::new(HashMap::<String, NodeForm>::new());
    let queue = StoredValue::new(VecDeque::<BuilderEdit>::new());
    let draining = RwSignal::new(false);
    let policy_revision = RwSignal::new(None::<u64>);
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let viewport = NodeRef::<leptos::html::Div>::new();
    let list_view = RwSignal::new(false);
    // The selected line: parent card ID and child card ID.
    let selected_link = RwSignal::new(None::<(String, String)>);
    // Cards with their subscriber labels open. Per browser tab; not saved.
    let expanded = RwSignal::new(std::collections::HashSet::<String>::new());
    let direction = RwSignal::new(Direction::TopDown);
    let direction_set = StoredValue::new(false);
    Effect::new(move |_| {
        let saved = data.with(|d| d.as_ref().map(|s| s.layout_direction));
        if let Some(d) = saved
            && !direction_set.get_value()
        {
            direction.set(d);
        }
    });

    // The newest view is the base of the next edit, unless an edit of this page is running.
    Effect::new(move |_| {
        let rev = data.with(|d| d.as_ref().and_then(|s| s.registry_revision.clone()));
        if !draining.get_untracked() {
            policy_revision.set(rev.and_then(|r| r.parse().ok()));
        }
    });
    // An inactive group is hidden; its settings stay.
    let nodes = Memo::new(move |_| {
        data.with(|d| d.as_ref().map(|s| visible(&s.nodes)).unwrap_or_default())
    });
    // One card for each card ID: the canvas never shows the instances of a node.
    let card_list = Memo::new(move |_| nodes.with(|n| cards(n)));
    // Cards that can be a parent get a connector port: WAN, router, and Site/AP cards.
    let parents = Memo::new(move |_| {
        card_list.with(|c| {
            c.iter()
                .filter(|c| can_parent(&c.node))
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    let group_cards = Memo::new(move |_| {
        card_list.with(|c| {
            c.iter()
                .filter(|c| c.node.kind == "group")
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    let wan_names = Memo::new(move |_| {
        nodes.with(|n| {
            n.iter()
                .filter(|w| w.kind == "wan")
                .map(|w| (w.id.clone(), w.name.clone()))
                .collect::<Vec<_>>()
        })
    });
    let balance_of = move |card: &str| {
        data.with(|d| {
            d.as_ref()
                .and_then(|s| s.balance.iter().find(|b| b.card_id == card).cloned())
        })
    };
    let writer_off = move || data.with(|d| d.as_ref().is_some_and(|s| s.wan_lists.is_none()));
    // The height below a group card that its warning and leaf labels take.
    let extra = Memo::new(move |_| {
        let open = expanded.get();
        let off = writer_off();
        card_list.with(|cs| {
            cs.iter()
                .filter_map(|c| {
                    let b = balance_of(&c.id)?;
                    let warn = nodes.with(|ns| routing_warning(ns, &c.node.id, off));
                    let rows = if open.contains(&c.id) {
                        b.leaves.len().div_ceil(2) as f64 * LEAF_ROW
                            + if b.more > 0 { LEAF_ROW } else { 0.0 }
                    } else {
                        0.0
                    };
                    Some((c.id.clone(), leaf_top(warn) + LEAF_LINE + rows))
                })
                .collect::<HashMap<_, _>>()
        })
    });
    let candidates = Memo::new(move |_| {
        data.with(|d| {
            d.as_ref()
                .map(|s| s.wan_candidates.clone())
                .unwrap_or_default()
        })
    });
    let points = Memo::new(move |_| {
        let overrides = local.get();
        let mut p = positions(
            &card_list.get(),
            &candidates.get(),
            direction.get(),
            &extra.get(),
        );
        for (id, point) in &mut p {
            if let Some(o) = overrides.get(id) {
                *point = *o;
            }
        }
        p
    });
    let point_of =
        move |id: &str| points.with(|p| p.iter().find(|(n, _)| n == id).map(|(_, x)| *x));
    let locked = move || structural_blocked(data).is_some();
    let topology_writer = move || {
        data.with_untracked(|d| {
            d.as_ref()
                .is_some_and(|s| s.writer == "topology" && s.mode == "apply")
        })
    };

    // Serialize edits: each one uses the revision that the previous one saved.
    let drain = move || {
        if draining.get_untracked() {
            return;
        }
        draining.set(true);
        leptos::task::spawn_local(async move {
            while let Some(edit) = queue.try_update_value(|q| q.pop_front()).flatten() {
                status.set("Saving…".into());
                let first = matches!(edit, BuilderEdit::Initialize).then_some(0);
                let Some(expected_revision) = policy_revision.get_untracked().or(first) else {
                    status.set("Not saved: the topology view has no policy yet.".into());
                    continue;
                };
                let key = new_key();
                // The card of a router change: its form gets the refusal.
                let router_node = match &edit {
                    BuilderEdit::NodeRouter { node_id, .. } => nodes.with_untracked(|ns| {
                        ns.iter()
                            .find(|n| n.id == *node_id)
                            .map(|n| card_of(n).to_string())
                    }),
                    _ => None,
                };
                let request = EditRequest {
                    key: key.clone(),
                    expected_revision,
                    edit,
                };
                let result = match post(session, "/api/v1/topology/edits", &request).await {
                    Ok(_) => settle(session, &key).await,
                    Err(e) => Err(e),
                };
                match result {
                    Ok(op) if op.state == "saved" => {
                        let revision = op.receipt.and_then(|r| r.revision);
                        if let Some(r) = revision.as_deref().and_then(|r| r.parse().ok()) {
                            policy_revision.set(Some(r));
                        }
                        let revision = revision.unwrap_or_default();
                        status.set(if topology_writer() {
                            format!("Saved revision {revision}. It applies in the next cycle.")
                        } else {
                            format!(
                                "Saved revision {revision}. Publication starts when the topology writer is active."
                            )
                        });
                    }
                    Ok(op) => {
                        let why = op
                            .error
                            .or(op.receipt.and_then(|r| r.error))
                            .unwrap_or_else(|| "the service did not save it.".into());
                        status.set(format!("Not saved: {why}"));
                        // A refused router change: the reason goes next to the select, the draft
                        // stays, and focus moves to the select.
                        if let Some(id) = router_node {
                            forms.update(|f| {
                                if let Some(form) = f.get_mut(&id) {
                                    form.router_error = why.clone();
                                }
                            });
                            if let Some(select) = document().get_element_by_id("canvas-router")
                                && let Ok(select) = select.dyn_into::<web_sys::HtmlElement>()
                            {
                                let _ = select.focus();
                            }
                        }
                        // Later edits were made on top of the refused one; drop them.
                        queue.update_value(VecDeque::clear);
                    }
                    Err(e) => {
                        status.set(format!("Not saved: {e}"));
                        queue.update_value(VecDeque::clear);
                    }
                }
            }
            draining.set(false);
            reload.run(());
        });
    };
    let submit = move |edit: BuilderEdit| {
        queue.update_value(|q| q.push_back(edit));
        drain();
    };
    // One action with its form values, checked by the same rules as the other topology forms.
    let act = move |change: Change, draft: Draft| match build_edit(&change, &draft) {
        Ok(edit) => submit(edit),
        Err(e) => status.set(format!("Not saved: {e}")),
    };
    // The first node of the selected card: new items go below it.
    let selected_first = move || {
        let card = selected.get_untracked()?;
        nodes.with_untracked(|ns| ns.iter().find(|n| card_of(n) == card).map(|n| n.id.clone()))
    };
    let under = move || Draft {
        parent: selected_first().unwrap_or_default(),
        ..Draft::default()
    };
    let remove_link = move |from: String, to: String| match nodes
        .with_untracked(|ns| link_of(ns, &from, &to).filter(|(p, c)| removable(ns, p, c)))
    {
        Some((parent, child)) => {
            selected_link.set(None);
            submit(BuilderEdit::LinkRemove { parent, child });
        }
        None => status.set("Not removed: the router branch keeps its groups.".into()),
    };

    // Save the layout 600 ms after the last card move.
    let save_layout = move || {
        let generation = layout_generation.with_value(|g| g + 1);
        layout_generation.set_value(generation);
        leptos::task::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(600).await;
            if layout_generation.get_value() != generation {
                return;
            }
            let expected_revision = layout_revision.get_untracked().unwrap_or_else(|| {
                data.with_untracked(|d| {
                    d.as_ref()
                        .and_then(|s| s.layout_revision.as_deref())
                        .and_then(|r| r.parse().ok())
                        .unwrap_or(0)
                })
            });
            let body = LayoutRequest {
                expected_revision,
                direction: direction.get_untracked(),
                positions: data.with_untracked(|d| {
                    d.as_ref()
                        .map(|s| saved_positions(&s.nodes, &local.get_untracked()))
                        .unwrap_or_default()
                }),
            };
            match put_layout(session, &body).await {
                Ok(revision) => {
                    layout_revision.set(Some(revision));
                    status.set("Layout saved. Shaping does not change.".into());
                }
                Err(e) => {
                    layout_revision.set(None);
                    local.set(HashMap::new());
                    status.set(e.into());
                    reload.run(());
                }
            }
        });
    };

    let open = move |p: Panel| {
        if p == Panel::Node {
            let Some(id) = selected.get_untracked() else {
                return;
            };
            let node = nodes.with_untracked(|n| n.iter().find(|x| card_of(x) == id).cloned());
            if let Some(node) = node {
                // An invalid draft of this node stays; a clean form starts from the saved node.
                forms.update(|f| {
                    if f.get(&id)
                        .is_none_or(|form| form.error.is_empty() && form.router_error.is_empty())
                    {
                        f.insert(id.clone(), NodeForm::of(&node));
                    }
                });
            }
        }
        panel.set(Some(p));
        if let Some(d) = dialog.get_untracked() {
            let _ = d.show_modal();
        }
    };
    let close = move || {
        if let Some(d) = dialog.get_untracked() {
            d.close();
        }
    };

    // Send the valid changes of the open node form.
    let flush = move || {
        let Some(id) = selected.get_untracked() else {
            return;
        };
        let Some(card) = card_list.with_untracked(|c| c.iter().find(|c| c.id == id).cloned())
        else {
            return;
        };
        let Some(form) = forms.with_untracked(|f| f.get(&id).cloned()) else {
            return;
        };
        let result = form_edits(&card.node, &form).map(|e| card_edits(&card, e));
        forms.update(|f| {
            if let Some(form) = f.get_mut(&id) {
                form.error = result.as_ref().err().cloned().unwrap_or_default();
            }
        });
        if let Ok(edits) = result {
            edits.into_iter().for_each(submit);
        }
    };

    // Card drag: pointer ID, start pointer, start card position, moved.
    let drag = StoredValue::new(None::<(i32, f64, f64, [f64; 2], bool)>);
    let just_dragged = StoredValue::new(false);
    let card = move |c: Card| {
        let n = c.node.clone();
        let id = StoredValue::new(c.id.clone());
        let pos = move || id.with_value(|i| point_of(i)).unwrap_or([0.0, 0.0]);
        let is_selected = move || id.with_value(|i| selected.get().as_deref() == Some(i.as_str()));
        let geo = n.geo.as_ref().map(|g| match g.geo_type {
            GeoType::Site => "Site",
            GeoType::Ap => "AP",
        });
        let kind = match (geo, n.kind.as_str()) {
            (Some(g), "intermediate") => g.to_string(),
            _ => card_kind(&n).to_string(),
        };
        let rate = match n.exported_mbps {
            Some(m) => mbps(Some(m)),
            None => capacity_label(&n.capacity),
        };
        let members = n
            .members
            .map(|c| format!("{c} subscriber{}", if c == 1 { "" } else { "s" }));
        let meta = members
            .as_ref()
            .map_or(rate.clone(), |m| format!("{rate}, {m}"));
        let located = n.geo.as_ref().is_some_and(|g| g.location.is_some());
        let router = n.router_id.as_ref().map(|r| router_name(data, r));
        let auto = n.auto_members.map(|c| format!("+{c} auto"));
        let routed = n.routed.map(|c| format!("{c} routed"));
        let health = n
            .wan_health
            .as_ref()
            .map(|h| (h.state.clone(), health_label(h)));
        let excluded = n.tree_excluded.then_some("Excluded from tree");
        let warning = nodes
            .with_untracked(|ns| routing_warning(ns, &n.id, writer_off()))
            .then_some(ROUTING_WARNING);
        let label = format!(
            "{}, {kind}, {meta}{}{}{}{}{}. Enter opens settings. Shift and arrow keys move the card.",
            n.name,
            if located { ", located" } else { "" },
            routed
                .as_ref()
                .map(|r| format!(", {r}"))
                .unwrap_or_default(),
            router
                .as_ref()
                .map(|r| format!(", router {r}"))
                .unwrap_or_default(),
            auto.as_ref().map(|a| format!(", {a}")).unwrap_or_default(),
            health
                .as_ref()
                .map(|(_, h)| format!(", WAN {h}"))
                .unwrap_or_default(),
        ) + &[excluded, warning]
            .iter()
            .flatten()
            .map(|t| format!(" {t}."))
            .collect::<String>();
        view! {
          <button type="button" class="canvas-node" data-kind=n.kind.clone() data-node-id=c.id.clone()
            aria-pressed=move||is_selected().to_string() aria-label=label
            style:left=move||format!("{}px",pos()[0]) style:top=move||format!("{}px",pos()[1])
            on:click=move|_|{
              if just_dragged.get_value(){ just_dragged.set_value(false); return; }
              selected.set(Some(id.get_value()));
              selected_link.set(None);
            }
            on:dblclick=move|_|{ selected.set(Some(id.get_value())); open(Panel::Node); }
            on:keydown=move|ev: web_sys::KeyboardEvent|{
              let key=ev.key();
              if key=="Enter" && is_selected() { ev.prevent_default(); open(Panel::Node); return; }
              if !ev.shift_key() { return; }
              let (dx,dy)=match key.as_str(){"ArrowLeft"=>(-16.0,0.0),"ArrowRight"=>(16.0,0.0),"ArrowUp"=>(0.0,-16.0),"ArrowDown"=>(0.0,16.0),_=>return};
              ev.prevent_default();
              let next=nudge(pos(),dx,dy);
              local.update(|l|{l.insert(id.get_value(),next);});
              save_layout();
            }
            on:pointerdown=move|ev: web_sys::PointerEvent|{
              if ev.button()!=0 { return; }
              if let Some(t)=ev.current_target().and_then(|t|t.dyn_into::<web_sys::Element>().ok()) {
                let _=t.set_pointer_capture(ev.pointer_id());
              }
              drag.set_value(Some((ev.pointer_id(),ev.client_x() as f64,ev.client_y() as f64,pos(),false)));
            }
            on:pointermove=move|ev: web_sys::PointerEvent|{
              let Some((pid,x0,y0,start,moved))=drag.get_value() else {return};
              if pid!=ev.pointer_id() {return;}
              let z=zoom.get_untracked();
              let (dx,dy)=((ev.client_x() as f64-x0)/z,(ev.client_y() as f64-y0)/z);
              if !moved && dx.abs()+dy.abs()<4.0 {return;}
              drag.set_value(Some((pid,x0,y0,start,true)));
              local.update(|l|{l.insert(id.get_value(),nudge(start,dx,dy));});
            }
            on:pointerup=move|_|{
              if let Some((_,_,_,_,true))=drag.get_value() { just_dragged.set_value(true); save_layout(); }
              drag.set_value(None);
            }
            on:pointercancel=move|_|{
              if let Some((_,_,_,start,true))=drag.get_value() { local.update(|l|{l.insert(id.get_value(),start);}); }
              drag.set_value(None);
            }>
            <span class="canvas-node-top">
              <span class="canvas-node-kind">{kind}</span>
              {router.map(|r|view!{<span class="canvas-node-router">{format!("Router {r}")}</span>})}
              {auto.map(|a|view!{<span class="canvas-node-auto">{a}</span>})}
              {health.map(|(state,h)|view!{<span class="canvas-node-health" data-state=state>{h}</span>})}
              {excluded.map(|t|view!{<span class="canvas-node-router">{t}</span>})}
            </span>
            <strong>{n.name.clone()}</strong>
            <span class="canvas-node-meta"><span>{rate}</span>{members.map(|m|view!{<span>{m}</span>})}</span>
            {warning.map(|t|view!{<span class="canvas-node-warning">{t}</span>})}
          </button>
        }
    };
    // Escape cancels a running card drag before any layout save.
    let cancel_drag = move |ev: web_sys::KeyboardEvent| {
        if ev.key() != "Escape" {
            return;
        }
        if let Some((_, _, _, start, true)) = drag.get_value() {
            if let Some(id) = selected.get_untracked() {
                local.update(|l| {
                    l.insert(id, start);
                });
            }
            drag.set_value(None);
            just_dragged.set_value(true);
        }
    };

    // Inventory actions from the router reads.
    let read_action = RwSignal::new(None::<Change>);
    let wan_form = RwSignal::new(None::<(Change, Draft, String)>);
    // The one-time routing adoption is a reviewed change in the house change dialog.
    let adopt = RwSignal::new(None::<Change>);
    let confirm_rebalance = RwSignal::new(false);
    // Connector drag: parent node ID, start pointer, start point, end point (canvas pixels).
    let connect = RwSignal::new(None::<(String, f64, f64, [f64; 2], [f64; 2])>);
    let handle = move |c: Card| {
        let n = c.node.clone();
        let id = StoredValue::new(c.id.clone());
        let pos = move || id.with_value(|i| point_of(i)).unwrap_or([0.0, 0.0]);
        let out = move || port(pos(), direction.get(), true);
        // The pointer drag is a shortcut. Keyboard users set a parent in the settings of a card.
        view! {
          <button type="button" class="canvas-connector" tabindex="-1" aria-hidden="true"
            title=format!("Drag from {} to the card that goes below it",n.name) disabled=locked
            style:left=move||format!("{}px",out()[0]-9.0) style:top=move||format!("{}px",out()[1]-9.0)
            on:pointerdown=move|ev: web_sys::PointerEvent|{
              if ev.button()!=0 { return; }
              ev.stop_propagation();
              if let Some(t)=ev.current_target().and_then(|t|t.dyn_into::<web_sys::Element>().ok()) {
                let _=t.set_pointer_capture(ev.pointer_id());
              }
              let start=out();
              connect.set(Some((id.get_value(),ev.client_x() as f64,ev.client_y() as f64,start,start)));
            }
            on:pointermove=move|ev: web_sys::PointerEvent|{
              let z=zoom.get_untracked();
              connect.update(|c|if let Some((_,x0,y0,start,end))=c{
                *end=[start[0]+(ev.client_x() as f64-*x0)/z,start[1]+(ev.client_y() as f64-*y0)/z];
              });
            }
            on:pointerup=move|ev: web_sys::PointerEvent|{
              let Some((from,..))=connect.get_untracked() else {return};
              connect.set(None);
              let target=document().element_from_point(ev.client_x() as f32,ev.client_y() as f32)
                .and_then(|e|e.closest("[data-node-id]").ok().flatten())
                .and_then(|e|e.get_attribute("data-node-id"));
              let Some(target)=target else {return};
              match nodes.with_untracked(|ns|drop_edit(ns,&from,&target)) {
                Ok(edit)=>submit(edit),
                Err(why)=>status.set(format!("Not connected: {why}")),
              }
            }
            on:pointercancel=move|_|connect.set(None)>"+"</button>
        }
    };
    let candidate = move |(id, router_id, router, iface): (String, String, String, String)| {
        let card_id = StoredValue::new(id);
        let pos = move || card_id.with_value(|i| point_of(i)).unwrap_or([0.0, 0.0]);
        let label = format!("{iface} on {router}, WAN, not configured. Enter adds it as a WAN.");
        let name = iface.clone();
        let (r2, rn2, if2) = (router_id.clone(), router.clone(), iface.clone());
        let place = format!("{router}, default route");
        view! {
          // A WAN needs a saved capacity before it can be a parent: its port opens the WAN form.
          <button type="button" class="canvas-connector" tabindex="-1" aria-hidden="true" disabled=locked
            title=format!("Save a capacity for {iface} first, then connect it")
            style:left=move||format!("{}px",pos()[0]+CARD_W/2.0-9.0) style:top=move||format!("{}px",pos()[1]+CARD_H-9.0)
            on:pointerdown=move|ev: web_sys::PointerEvent|{
              ev.stop_propagation();
              status.set(format!("Save a capacity for {} first, then connect it.",if2));
              read_action.set(Some(Change::CreateWan(r2.clone(),rn2.clone(),if2.clone())));
              open(Panel::Inventory);
            }>"+"</button>
          <button type="button" class="canvas-candidate" data-kind="wan" aria-label=label disabled=locked
            style:left=move||format!("{}px",pos()[0]) style:top=move||format!("{}px",pos()[1])
            on:click=move|_|{
              read_action.set(Some(Change::CreateWan(router_id.clone(),router.clone(),iface.clone())));
              open(Panel::Inventory);
            }>
            <span class="canvas-node-top"><span class="canvas-node-kind">"WAN"</span><span class="canvas-chip" data-tone="muted">"Not configured"</span></span>
            <strong>{name}</strong>
            <span class="canvas-node-meta"><span>{place}</span></span>
          </button>
        }
    };
    // One line for each flow between two shown cards; the width is the subscriber count.
    let wires = move || {
        let sel = selected.get();
        let link = selected_link.get();
        let dir = direction.get();
        let flows = data.with(|d| {
            d.as_ref()
                .map(|s| lines(&nodes.get(), &s.flows))
                .unwrap_or_default()
        });
        let max = flows.iter().map(|f| f.subscribers).max().unwrap_or(0);
        flows
            .into_iter()
            .filter_map(|f| {
                let d = wire_in(point_of(&f.from)?, point_of(&f.to)?, dir);
                let active = sel.as_deref().is_some_and(|s| s == f.to || s == f.from);
                let picked = link.as_ref().is_some_and(|(a, b)| *a == f.from && *b == f.to);
                let width = format!("{}", flow_width(f.subscribers, max));
                let (from, to) = (f.from.clone(), f.to.clone());
                Some(view! {
                  <g class="canvas-flow" data-from=f.from.clone() data-to=f.to.clone()>
                    <path d=d.clone() class:active=active class:picked=picked style:stroke-width=width/>
                    <path d=d class="hit" on:click=move|ev: web_sys::MouseEvent|{
                      ev.stop_propagation();
                      selected.set(None);
                      selected_link.set(Some((from.clone(),to.clone())));
                    }/>
                  </g>
                })
            })
            .collect_view()
    };
    // The leaf line below a group card: its subscribers on each WAN, and up to 50 labels.
    let leaves = move |c: Card| {
        let id = StoredValue::new(c.id.clone());
        let source = c.node.source_id.clone().unwrap_or_default();
        let warn = nodes.with_untracked(|ns| routing_warning(ns, &c.node.id, writer_off()));
        let pos = move || id.with_value(|i| point_of(i)).unwrap_or([0.0, 0.0]);
        let wan = move |w: &str| {
            wan_names.with(|n| {
                let i = n.iter().position(|(id, _)| id == w);
                (
                    i.map_or(0, |i| i % 6),
                    i.map_or_else(|| w.to_string(), |i| n[i].1.clone()),
                )
            })
        };
        let open = move || id.with_value(|i| expanded.with(|e| e.contains(i)));
        move || {
            let b = id.with_value(|i| balance_of(i))?;
            let line = b
                .by_wan
                .iter()
                .map(|(w, n)| {
                    let (mark, name) = wan(w);
                    view! {<span class="canvas-leaf-wan" data-wan=mark>{format!("{name} {n}")}</span>}
                })
                .collect_view();
            let toggle_label = if open() {
                "Hide subscribers"
            } else {
                "Show subscribers"
            };
            let labels = open().then(|| {
                let more = b.more;
                let href = format!("/topology?tab=assignments&group={}", source);
                view! {
                  <ul class="canvas-leaf-list">
                    {b.leaves.iter().map(|l|{
                      let (mark, name) = wan(&l.wan);
                      view!{<li><span class="canvas-leaf-wan" data-wan=mark title=name.clone()>{l.name.clone()}</span></li>}
                    }).collect_view()}
                  </ul>
                  {(more > 0).then(||view!{<a class="canvas-link" href=href>{format!("+{more} more")}</a>})}
                }
            });
            Some(view! {
              <div class="canvas-leaves" data-card=id.get_value()
                style:left=move||format!("{}px",pos()[0])
                style:top=move||format!("{}px",pos()[1]+CARD_H+leaf_top(warn))>
                <p class="canvas-leaf-line">{line}
                  <button type="button" class="quiet canvas-leaf-toggle" aria-expanded=move||open().to_string()
                    on:click=move|_|expanded.update(|e|{let i=id.get_value(); if !e.remove(&i){e.insert(i);}})>{toggle_label}</button>
                </p>
                {labels}
              </div>
            })
        }
    };
    // The connector that the operator drags from a parent card.
    let pending = move || {
        connect.get().and_then(|(from, _, _, _, end)| {
            let a = port(point_of(&from)?, direction.get(), true);
            let d = format!("M{} {} L{} {}", a[0], a[1], end[0], end[1]);
            Some(view! {<path d=d class="pending"/>})
        })
    };
    let size = move || points.with(|p| surface(p));
    let selected_node = move || {
        let id = selected.get()?;
        nodes.with(|n| n.iter().find(|x| card_of(x) == id).cloned())
    };

    Effect::new(move |_| match read_action.get() {
        Some(Change::AddSource(key, _)) => {
            submit(BuilderEdit::SourceAdd {
                key,
                parent: selected_first(),
            });
            read_action.set(None);
        }
        Some(Change::CreateWan(router_id, router, iface)) => {
            let draft = Draft {
                name: iface.clone(),
                mode: "fixed".into(),
                ..Draft::default()
            };
            wan_form.set(Some((
                Change::CreateWan(router_id, router, iface),
                draft,
                String::new(),
            )));
            read_action.set(None);
        }
        _ => {}
    });

    let tool = move |p: Panel, label: &'static str, path: &'static str| {
        view! {
          <button type="button" class="canvas-tool" aria-label=label title=label
            aria-haspopup="dialog" on:click=move|_|open(p)>{icon(path)}</button>
        }
    };
    let wan_field = move |f: fn(&mut Draft) -> &mut String, v: String| {
        wan_form.update(|w| {
            if let Some((_, d, _)) = w {
                *f(d) = v;
            }
        })
    };

    view! {
      <section class="canvas-shell" aria-label="Topology canvas workspace">
        <div class="canvas-bar">
          <p class="canvas-facts">{move||data.with(|d|d.as_ref().map(|s|{
            let wans=s.nodes.iter().filter(|n|n.kind=="wan").count();
            format!("Policy revision {} · {wans} WANs · {} nodes · {} active subscribers",
              s.registry_revision.clone().unwrap_or("none".into()),s.nodes.len(),s.subscribers_active)
          }))}</p>
          <p class="canvas-status" role="status" aria-live="polite">{move||status.get()}</p>
        </div>
        {move||structural_blocked(data).filter(|_|data.with(Option::is_some)).map(|why|view!{<p class="notice" role="note">{why}</p>})}
        <div class="canvas-frame">
          <Show when=move||list_view.get()>
            <ul class="canvas-list" aria-label="Topology cards">
              {move||card_list.get().into_iter().map(|c|{
                let (id,n)=(c.id.clone(),c.node);
                let chips=[n.tree_excluded.then_some("Excluded from tree"),
                  data.with_untracked(|d|d.as_ref().is_some_and(|s|routing_warning(&visible(&s.nodes),&n.id,s.wan_lists.is_none()))).then_some(ROUTING_WARNING)]
                  .into_iter().flatten().collect::<Vec<_>>().join(" · ");
                view!{<li><button type="button" class="quiet" on:click=move|_|{selected.set(Some(id.clone()));open(Panel::Node);}>
                  <strong>{n.name.clone()}</strong><small>{kind_label(&n.kind)}</small>{(!chips.is_empty()).then(||view!{<small>{chips}</small>})}
                </button></li>}
              }).collect_view()}
              {move||candidate_cards(&candidates.get()).into_iter().map(|(_,rid,router,iface)|{
                let label=format!("{iface} on {router}");
                view!{<li><button type="button" class="quiet" disabled=locked on:click=move|_|{
                    read_action.set(Some(Change::CreateWan(rid.clone(),router.clone(),iface.clone())));
                    open(Panel::Inventory);
                  }><strong>{label}</strong><small>"WAN · not configured"</small></button></li>}
              }).collect_view()}
            </ul>
          </Show>
          <div class="canvas-viewport" node_ref=viewport hidden=move||list_view.get() tabindex="0" role="region"
            aria-label="Topology canvas. Arrow keys scroll. Tab moves between cards."
            on:keydown=move|ev: web_sys::KeyboardEvent|{
              if matches!(ev.key().as_str(),"Delete"|"Backspace") && !locked()
                && let Some((from,to))=selected_link.get_untracked() {
                ev.prevent_default();
                remove_link(from,to);
                return;
              }
              cancel_drag(ev);
            }
            on:click=move|ev: web_sys::MouseEvent|{
              // A click on empty canvas clears the selection.
              let empty=ev.target().and_then(|t|t.dyn_into::<web_sys::Element>().ok())
                .is_some_and(|e|e.class_list().contains("canvas-layer")||e.class_list().contains("canvas-surface"));
              if empty { selected.set(None); selected_link.set(None); }
            }>
            <div class="canvas-surface" style:width=move||format!("{}px",size().0*zoom.get()) style:height=move||format!("{}px",size().1*zoom.get())>
              <div class="canvas-layer" data-direction=move||direction_name(direction.get()) style:width=move||format!("{}px",size().0) style:height=move||format!("{}px",size().1)
                style:transform=move||format!("scale({})",zoom.get())>
                <svg class="canvas-wires" aria-hidden="true" width=move||size().0 height=move||size().1>{wires}{pending}</svg>
                <For each=move||card_list.get() key=|c|{let n=&c.node; format!("{}|{}|{}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",c.id,n.name,n.kind,n.geo,n.capacity,n.exported_mbps,c.members,n.router_id,(n.auto_members,&n.wan_health,n.routed,n.tree_excluded))} children=card/>
                <For each=move||group_cards.get() key=|c|c.id.clone() children=leaves/>
                <For each=move||parents.get() key=|c|c.id.clone() children=handle/>
                <For each=move||candidate_cards(&candidates.get()) key=|c|c.0.clone() children=candidate/>
              </div>
            </div>
            <Show when=move||data.with(|d|d.as_ref().is_some_and(|s|s.nodes.is_empty()))>
              <div class="canvas-empty">
                <p>"The canvas is empty. Open Router inventory to add WANs and subscriber sources."</p>
                <Show when=move||data.with(|d|d.as_ref().is_some_and(|s|!s.frontend_initialized && !s.enrolled && s.writer=="legacy" && s.sources.is_empty()))>
                  <button type="button" class="primary" on:click=move|_|act(Change::Initialize,Draft::default())>"Start empty topology"</button>
                </Show>
              </div>
            </Show>
          </div>
          <div class="canvas-dock" role="toolbar" aria-label="Topology tools" aria-orientation="vertical">
            {tool(Panel::Inventory,"Router inventory",ICON_INVENTORY)}
            {tool(Panel::Settings,"Topology settings",ICON_SETTINGS)}
            {tool(Panel::Allocation,"Subscriber allocation",ICON_ALLOCATION)}
            {tool(Panel::Preview,"Preview and evidence",ICON_PREVIEW)}
            {tool(Panel::Excluded,"Excluded sources",ICON_EXCLUDED)}
          </div>
          <div class="canvas-zoom" role="group" aria-label="Zoom">
            <button type="button" class="quiet" aria-label="Zoom out" on:click=move|_|zoom.update(|z|*z=clamp_zoom(((*z-0.1)*10.0).round()/10.0))
              disabled={move||zoom.get()<=0.26}>"−"</button>
            <button type="button" class="quiet canvas-zoom-level" aria-label="Zoom to 100%" title="Zoom to 100%"
              on:click=move|_|zoom.set(1.0)>{move||format!("{:.0}%",zoom.get()*100.0)}</button>
            <button type="button" class="quiet" aria-label="Zoom in" on:click=move|_|zoom.update(|z|*z=clamp_zoom(((*z+0.1)*10.0).round()/10.0))
              disabled={move||zoom.get()>=1.99}>"+"</button>
            <span class="canvas-zoom-sep" aria-hidden="true"></span>
            <button type="button" class="quiet" on:click=move|_|{
              let Some(v)=viewport.get_untracked() else {return};
              let b=points.with_untracked(|p|p.iter().fold([f64::MAX,f64::MAX,f64::MIN,f64::MIN],|b,(_,q)|
                [b[0].min(q[0]),b[1].min(q[1]),b[2].max(q[0]+CARD_W),b[3].max(q[1]+CARD_H)]));
              let (z,_,_)=fit(b,[v.client_width() as f64,v.client_height() as f64]);
              zoom.set(z);
              v.set_scroll_left((b[0]*z) as i32);
              v.set_scroll_top((b[1]*z) as i32);
            }>"Fit"</button>
            <button type="button" class="quiet" disabled=locked on:click=move|_|{
              // Clear the saved positions: every card returns to the automatic layout.
              local.set(HashMap::new());
              let expected_revision=layout_revision.get_untracked().unwrap_or_else(||data.with_untracked(|d|
                d.as_ref().and_then(|s|s.layout_revision.as_deref()).and_then(|r|r.parse().ok()).unwrap_or(0)));
              leptos::task::spawn_local(async move{
                match put_layout(session,&LayoutRequest{expected_revision,positions:Default::default(),direction:direction.get_untracked()}).await{
                  Ok(r)=>{layout_revision.set(Some(r));status.set("Layout reset. Shaping does not change.".into());}
                  Err(e)=>{layout_revision.set(None);status.set(e.into());}
                }
                reload.run(());
              });
            }>"Reset layout"</button>
            <label class="canvas-direction"><span class="sr-only">"Layout direction"</span>
              <select aria-label="Layout direction" disabled=locked prop:value=move||direction_name(direction.get())
                on:change=move|ev|{
                  let Some(next)=direction_of(&event_target_value(&ev)) else {return};
                  direction_set.set_value(true);
                  direction.set(next);
                  // The new direction restores its saved positions; the old ones stay saved.
                  local.set(HashMap::new());
                  let expected_revision=layout_revision.get_untracked().unwrap_or_else(||data.with_untracked(|d|
                    d.as_ref().and_then(|s|s.layout_revision.as_deref()).and_then(|r|r.parse().ok()).unwrap_or(0)));
                  leptos::task::spawn_local(async move{
                    match put_layout(session,&LayoutRequest{expected_revision,positions:Default::default(),direction:next}).await{
                      Ok(r)=>{layout_revision.set(Some(r));status.set("Layout direction saved. Shaping does not change.".into());}
                      Err(e)=>{layout_revision.set(None);status.set(e.into());}
                    }
                    reload.run(());
                  });
                }>
                <option value="top_down">"Top-down"</option>
                <option value="bottom_up">"Bottom-up"</option>
                <option value="left_right">"Left to right"</option>
                <option value="right_left">"Right to left"</option>
              </select></label>
            <button type="button" class="quiet" aria-pressed=move||list_view.get().to_string() on:click=move|_|list_view.set(!list_view.get_untracked())>"List view"</button>
            <Show when=move||selected_link.get().is_some()>
              <span class="canvas-zoom-sep" aria-hidden="true"></span>
              <button type="button" class="quiet danger" disabled=locked
                on:click=move|_|if let Some((from,to))=selected_link.get_untracked(){remove_link(from,to);}>"Remove link"</button>
            </Show>
            <Show when=move||selected.get().is_some()>
              <span class="canvas-zoom-sep" aria-hidden="true"></span>
              <button type="button" class="quiet" on:click=move|_|open(Panel::Node)>"Card settings"</button>
            </Show>
          </div>
        </div>
        <dialog node_ref=dialog class="topology-dialog canvas-dialog" aria-labelledby="canvas-dialog-title"
          on:close=move|_|{
            // A late close event of an earlier modal must not clear a modal that is open again.
            if dialog.get_untracked().is_some_and(|d|d.open()) { return; }
            if panel.get_untracked()==Some(Panel::Node){ flush(); }
            panel.set(None); wan_form.set(None);
          }>
          <header class="add-router-heading">
            <div><h2 id="canvas-dialog-title">{move||match panel.get(){
              Some(Panel::Node)=>selected_node().map(|n|n.name).unwrap_or("Node".into()),
              Some(Panel::Inventory)=>"Router inventory".into(),
              Some(Panel::Settings)=>"Topology settings".into(),
              Some(Panel::Allocation)=>"Subscriber allocation".into(),
              Some(Panel::Preview)=>"Preview and evidence".into(),
              Some(Panel::Excluded)=>"Excluded sources".into(),
              None=>String::new(),
            }}</h2></div>
            <button type="button" class="router-icon quiet" aria-label="Close" title="Close" on:click=move|_|close()>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18"/></svg>
            </button>
          </header>
          <div class="canvas-dialog-body">
            {move||match panel.get(){
              Some(Panel::Node)=>view!{<NodeEditor data forms selected locked flush act=Callback::new(move|(c,d)|act(c,d)) send=Callback::new(submit) close=Callback::new(move|()|close())
                unlink=Callback::new(move|(from,to)|remove_link(from,to)) status/>}.into_any(),
              Some(Panel::Inventory)=>view!{
                <p class="field-help">{move||match selected_node(){Some(n)=>format!("New items go under {}.",n.name),None=>"New items go at the top level. Select a card first to place them under it.".into()}}</p>
                <div class="form-actions">
                  <button type="button" class="quiet" disabled=locked on:click=move|_|act(Change::Create,Draft{
                    name:"New group".into(),kind:"group".into(),mode:"automatic".into(),..under()})>"Add empty group"</button>
                </div>
                <h3>"Saved sources"</h3>
                <ul class="plain-list">
                  {move||data.with(|d|d.as_ref().map(|s|s.sources.iter().filter(|x|x.kind=="subscribers" && !x.excluded).map(|src|{
                    let id=src.id.clone();
                    // An unplaced source shows where automatic placement puts its subscribers.
                    let auto=(!s.nodes.iter().any(|n|n.source_id.as_deref()==Some(src.id.as_str()))).then(||{
                      let names:Vec<_>=src.auto_parents.iter().map(|p|s.nodes.iter().find(|n|&n.id==p).map_or(p.clone(),|n|n.name.clone())).collect();
                      if names.is_empty(){"Auto: no parent".to_string()}else{format!("Auto: {}",names.join(", "))}
                    });
                    view!{<li><strong>{src.name.clone()}</strong><small>"Subscriber source"</small>{auto.map(|a|view!{<small class="canvas-auto-parent">{a}</small>})}
                      <button type="button" class="quiet" disabled=locked aria-label=format!("Add {} to canvas",src.name)
                        on:click=move|_|act(Change::AttachSource(id.clone(),String::new(),None),under())>"Add to canvas"</button></li>}
                  }).collect_view()))}
                </ul>
                {move||wan_form.get().map(|(change,draft,error)|{
                  let label=match &change{Change::CreateWan(_,router,iface)=>format!("{iface} on {router}"),_=>String::new()};
                  view!{<form class="router-form canvas-wan-form" on:submit=move|ev|{
                      ev.prevent_default();
                      let Some((c,d,_))=wan_form.get_untracked() else {return};
                      match build_edit(&c,&d){
                        Ok(edit)=>{submit(edit);wan_form.set(None);}
                        Err(e)=>wan_form.update(|w|if let Some((_,_,err))=w{*err=e.into();}),
                      }
                    }>
                    <h3>{format!("New WAN · {label}")}</h3>
                    <label>"Name"<input prop:value=draft.name.clone() on:input=move|ev|wan_field(|d|&mut d.name,event_target_value(&ev))/></label>
                    <label>"Download Mbps"<input inputmode="numeric" prop:value=draft.down.clone() on:input=move|ev|wan_field(|d|&mut d.down,event_target_value(&ev))/></label>
                    <label>"Upload Mbps"<input inputmode="numeric" prop:value=draft.up.clone() on:input=move|ev|wan_field(|d|&mut d.up,event_target_value(&ev))/></label>
                    {(!error.is_empty()).then(||view!{<p class="field-error" role="alert">{error.clone()}</p>})}
                    <div class="form-actions"><button type="submit" class="primary">"Add WAN to canvas"</button></div>
                  </form>}
                })}
                <Reads session action=read_action reload locked/>
              }.into_any(),
              Some(Panel::Settings)=>view!{
                <p class="field-help">"Plans, speed rules, group factors, and fallback keep their reviewed editors in Policies for now."</p>
                <dl class="router-facts">
                  {move||data.with(|d|d.as_ref().and_then(|s|s.speed.clone()).map(|sp|{
                    let rule=|r:&SpeedRule|format!("{} {}",r.value,if r.unit=="mbps"{"Mbps"}else{"%"});
                    view!{<div><dt>"Minimum down / up"</dt><dd>{format!("{} / {}",rule(&sp.down_min),rule(&sp.up_min))}</dd></div>
                      <div><dt>"Maximum down / up"</dt><dd>{format!("{} / {}",rule(&sp.down_max),rule(&sp.up_max))}</dd></div>}
                  }))}
                  <div><dt>"Plans"</dt><dd>{move||data.with(|d|d.as_ref().map(|s|s.plans.len()).unwrap_or(0)).to_string()}</dd></div>
                </dl>
                <a class="button quiet" href="/topology?tab=policies">"Open Policies"</a>
                <fieldset class="canvas-modes"><legend>"WAN failover"</legend>
                  <div class="check">
                    <input type="checkbox" id="canvas-failback" disabled=locked
                      prop:checked=move||data.with(|d|d.as_ref().is_none_or(|s|s.auto_failback))
                      on:change=move|ev|submit(BuilderEdit::AutoFailback{enabled:event_target_checked(&ev)})/>
                    <label for="canvas-failback">"Automatic failback"
                      <small>"On: subscribers return to a WAN after it is stable for 10 minutes. Off: they stay on the other WAN until you return them."</small></label>
                  </div>
                  {move||data.with(|d|d.as_ref().map(|s|s.nodes.iter().filter_map(|n|{
                    let h=n.wan_health.as_ref()?;
                    h.error.as_ref().map(|e|view!{<p class="field-help">{format!("{}: {e}",n.name)}</p>})
                  }).collect_view()))}
                  <Show when=move||data.with(|d|d.as_ref().is_some_and(|s|s.nodes.iter().any(|n|n.wan_health.as_ref().is_some_and(|h|h.state=="waiting_for_rebalance"))))>
                    <div class="form-actions">
                      <button type="button" class="quiet" disabled=locked on:click=move|_|submit(BuilderEdit::FailbackRelease)>"Return subscribers to recovered WANs"</button>
                    </div>
                  </Show>
                </fieldset>
                <fieldset class="canvas-modes"><legend>"Routing lists"</legend>
                  <p role="status">{move||data.with(|d|match d.as_ref().and_then(|s|s.wan_lists.as_ref()){
                    None=>"Off. The service writes no routing list.".to_string(),
                    Some(w) if w.state=="written"=>format!("Written · {} entries · {} added, {} changed, {} removed",w.owned,w.added,w.changed,w.removed),
                    Some(w) if w.state=="held"=>format!("Held: {}",w.error.clone().unwrap_or_default()),
                    Some(w)=>format!("Not written: {}",w.error.clone().unwrap_or_default()),
                  })}</p>
                  <p class="field-help">"The service writes one via- list for each WAN on the core router. It publishes shaping only after the router has the lists."</p>
                  <div class="form-actions">
                    <button type="button" class="quiet" disabled=locked on:click=move|_|{close();adopt.set(Some(Change::AdoptRouting));}>"Adopt current routing"</button>
                  </div>
                  {move||{
                    let moves=data.with(|d|d.as_ref().map(|s|s.routing_moves).unwrap_or(0));
                    view!{
                      <p class="field-help">{if moves==0 {"Routing is balanced. Rebalance routing moves nobody now.".to_string()}
                        else {format!("Rebalance routing moves {moves} subscriber{} to another WAN. Their open connections break.",if moves==1{""}else{"s"})}}</p>
                      <div class="form-actions">
                        <button type="button" class="quiet" disabled=move||locked()||moves==0 on:click=move|_|{
                          if confirm_rebalance.get_untracked(){
                            submit(BuilderEdit::RoutingRebalance{apply:true});
                            confirm_rebalance.set(false);
                          } else { confirm_rebalance.set(true); }
                        }>{move||if confirm_rebalance.get(){format!("Confirm: move {moves}")}else{"Rebalance routing".to_string()}}</button>
                      </div>
                    }
                  }}
                </fieldset>
              }.into_any(),
              Some(Panel::Allocation)=>view!{
                {move||{
                  let current=data.with(|d|d.as_ref().map(|s|s.allocation.clone())).filter(|m|!m.is_empty()).unwrap_or("count".into());
                  let pick=move|m:&'static str|move|_|submit(BuilderEdit::AllocationDefault{mode:m.into()});
                  view!{
                  <fieldset class="canvas-modes"><legend>"Topology default"</legend>
                    <label><input type="radio" name="allocation-default" prop:checked=current=="count" disabled=locked on:change=pick("count")/>"Equal subscriber count, pins included"</label>
                    <label><input type="radio" name="allocation-default" prop:checked=current=="capacity" disabled=locked on:change=pick("capacity")/>"Capacity: plan demand against the capacity above each node"</label>
                    <label><input type="radio" name="allocation-default" prop:checked=current=="quality" disabled=locked on:change=pick("quality")/>"Quality: capacity multiplied by the WAN quality score"</label>
                  </fieldset>}
                }}
                {move||selected_node().filter(|n|n.kind=="group").and_then(|n|n.source_id.clone().map(|s|(s,n.name))).map(|(source,name)|{
                  let mode=data.with(|d|d.as_ref().and_then(|s|s.factors.iter().find(|f|f.source_id==source).map(|f|f.allocation.clone()))).unwrap_or_default();
                  view!{
                  <label>{format!("Mode of {name}")}<select prop:value=mode disabled=locked on:change=move|ev|{
                      let m=event_target_value(&ev);
                      submit(BuilderEdit::GroupAllocation{source_id:source.clone(),mode:(!m.is_empty()).then_some(m)});
                    }>
                    <option value="">"Use the topology default"</option>
                    <option value="count">"Equal subscriber count"</option>
                    <option value="capacity">"Capacity"</option>
                    <option value="quality">"Quality"</option>
                  </select></label>}
                })}
                <p class="field-help">"A mode applies to new subscribers and to the next rebalance. It moves nobody now. Stale WAN quality data uses capacity."</p>
                <p class="field-help">"A shaping placement does not change MikroTik routing."</p>
                <div class="form-actions">
                  {move||selected_node().filter(|n|n.kind=="group").and_then(|n|n.source_id.clone().map(|s|(s,n.name))).map(|(source,name)|view!{
                    <button type="button" class="quiet" disabled=move||blocked(data).is_some()
                      on:click=move|_|act(Change::Rebalance(Some(source.clone()),String::new()),Draft::default())>{format!("Rebalance {name}")}</button>})}
                  <button type="button" class="quiet" disabled=move||blocked(data).is_some()
                    on:click=move|_|act(Change::Rebalance(None,String::new()),Draft::default())>"Rebalance all sources"</button>
                </div>
                {move||blocked(data).map(|why|view!{<p class="field-help">{why}</p>})}
              }.into_any(),
              Some(Panel::Preview)=>view!{
                <p class="field-help">"Read-only. Saving and applying never wait for this preview."</p>
                <Evidence data/>
                {move||data.with(|d|d.as_ref().map(|s|view!{
                  <p class="field-help">{format!("{} conflicts · observed {}",s.conflicts_total,s.observed.as_ref().map(|o|age(now(),o.collected_at)).unwrap_or("never".into()))}</p>
                  <ul class="plain-list">{s.conflicts.iter().take(20).map(|c|view!{<li><strong>{c.subject.clone()}</strong><small>{c.detail.clone()}</small></li>}).collect_view()}</ul>
                }))}
              }.into_any(),
              Some(Panel::Excluded)=>view!{
                <p class="field-help">"LQoSync does not shape the subscribers of these sources, and does not add them to the canvas again."</p>
                <ul class="plain-list">
                  {move||data.with(|d|d.as_ref().map(|s|s.sources.iter().filter(|x|x.excluded).map(|src|{
                    let id=src.id.clone();
                    view!{<li><strong>{src.name.clone()}</strong><small>"Not read"</small>
                      <button type="button" class="quiet" disabled=locked aria-label=format!("Read {} again",src.name)
                        on:click=move|_|submit(BuilderEdit::SourceExclude{source_id:id.clone(),excluded:false})>"Read again"</button></li>}
                  }).collect_view()))}
                </ul>
                <Show when=move||data.with(|d|d.as_ref().is_none_or(|s|!s.sources.iter().any(|x|x.excluded)))>
                  <p class="field-help">"No source is excluded."</p>
                </Show>
              }.into_any(),
              None=>().into_any(),
            }}
          </div>
          <footer class="canvas-dialog-footer">
            <p role="status">{move||status.get()}</p>
            <button type="button" class="quiet" on:click=move|_|close()>"Close"</button>
          </footer>
        </dialog>
        <crate::topology_forms::ChangeDialog session data action=adopt reload/>
      </section>
    }
}

/// The text of a WAN probe state. Text, not only color, carries the state.
fn health_label(h: &WanHealthView) -> String {
    match h.state.as_str() {
        "up" => match h.quality_percent {
            Some(q) => format!("Up, {q}%"),
            None => "Up".into(),
        },
        "degraded" => "Degraded".into(),
        "unavailable" => "Unavailable".into(),
        "recovering" => "Recovering".into(),
        "waiting_for_rebalance" => "Waiting for Rebalance".into(),
        _ => "No probe data".into(),
    }
}

/// The display name of a router ID from the collection, or the ID itself.
fn router_name(data: RwSignal<Option<TopologySummary>>, id: &str) -> String {
    data.with_untracked(|d| {
        d.as_ref()
            .and_then(|s| s.collection.iter().find(|r| r.router_id == id))
            .map_or_else(|| id.to_string(), |r| r.name.clone())
    })
}

/// The node modal: General, Capacity, Location, and Structure. Valid changes save on change.
#[component]
fn NodeEditor(
    data: RwSignal<Option<TopologySummary>>,
    forms: RwSignal<HashMap<String, NodeForm>>,
    selected: RwSignal<Option<String>>,
    locked: impl Fn() -> bool + Copy + Send + Sync + 'static,
    flush: impl Fn() + Copy + Send + Sync + 'static,
    act: Callback<(Change, Draft)>,
    send: Callback<BuilderEdit>,
    close: Callback<()>,
    unlink: Callback<(String, String)>,
    status: RwSignal<String>,
) -> impl IntoView {
    let node = move || {
        let id = selected.get()?;
        data.with(|d| d.as_ref()?.nodes.iter().find(|n| card_of(n) == id).cloned())
    };
    let form = move || {
        selected
            .get()
            .and_then(|id| forms.with(|f| f.get(&id).cloned()))
            .unwrap_or_default()
    };
    let set = move |f: fn(&mut NodeForm) -> &mut String, value: String| {
        if let Some(id) = selected.get_untracked() {
            forms.update(|all| {
                if let Some(form) = all.get_mut(&id) {
                    *f(form) = value;
                }
            });
        }
    };
    let confirm_remove = RwSignal::new(false);
    let confirm_exclude = RwSignal::new(false);
    let paste = move |ev: web_sys::ClipboardEvent| {
        let text = ev
            .clipboard_data()
            .and_then(|d| d.get_data("text").ok())
            .unwrap_or_default();
        match split_paste(&text) {
            Some(Ok(l)) => {
                ev.prevent_default();
                set(|f| &mut f.latitude, l.latitude);
                set(|f| &mut f.longitude, l.longitude);
                flush();
            }
            Some(Err(e)) => {
                ev.prevent_default();
                set(|f| &mut f.error, e);
            }
            None => {}
        }
    };
    view! {
      {move||node().map(|n|{
        let wan=n.kind=="wan";
        let parents:Vec<(String,String)>=data.with(|d|d.as_ref().map(|s|{
          s.nodes.iter().filter(|p|p.id!=n.id && p.kind!="group" && !is_below(&s.nodes,&p.id,&n.id)).map(|p|(p.id.clone(),p.name.clone())).collect()
        }).unwrap_or_default());
        let current_parent=n.parent.clone().unwrap_or_default();
        // Routers without a router node, and the router of this node.
        let routers:Vec<(String,String)>=data.with(|d|d.as_ref().map(|s|s.collection.iter()
          .filter(|r|n.router_id.as_deref()==Some(r.router_id.as_str())||!s.nodes.iter().any(|m|m.router_id.as_deref()==Some(r.router_id.as_str())))
          .map(|r|(r.router_id.clone(),r.name.clone())).collect()).unwrap_or_default());
        let (move_node,remove_node,child_id)=(n.clone(),n.clone(),n.id.clone());
        let card=card_of(&n).to_string();
        let shown=data.with(|d|d.as_ref().map(|s|visible(&s.nodes)).unwrap_or_default());
        let count=shown.iter().filter(|m|card_of(m)==card).count();
        let names=|id:&str|shown.iter().find(|m|card_of(m)==id).map_or(id.to_string(),|m|m.name.clone());
        // The lines into this card, as text: the parent card, its subscribers, and removal.
        let links:Vec<(String,String,u64,bool)>=data.with(|d|d.as_ref().map(|s|lines(&shown,&s.flows).into_iter().filter(|f|f.to==card).map(|f|{
          let ok=link_of(&shown,&f.from,&f.to).is_some_and(|(p,c)|removable(&shown,&p,&c));
          (f.from.clone(),names(&f.from),f.subscribers,ok)
        }).collect()).unwrap_or_default());
        // Cards that can take a new line to this card: the same rules as a connector drag.
        let linkable:Vec<(String,String)>=cards(&shown).into_iter()
          .filter(|c|c.id!=card && can_parent(&c.node) && !links.iter().any(|l|l.0==c.id) && drop_edit(&shown,&c.id,&card).is_ok())
          .map(|c|(c.id,c.name)).collect();
        let (card2,shown2)=(card.clone(),shown.clone());
        let whole=cards(&shown).into_iter().find(|c|c.id==card);
        // A group is removed through its links or not read. A merged card is removed through
        // its links. A WAN, router, or Site card can hold child cards.
        let group_source=(n.kind=="group").then(||n.source_id.clone()).flatten();
        let removable_node=group_source.is_none()&&count==1;
        let can_add_child=n.kind!="group";
        view!{
        <form class="canvas-node-form" on:submit=move|ev|{ev.prevent_default();flush();}>
          {data.with(|d|d.as_ref().is_some_and(|s|routing_warning(&visible(&s.nodes),&n.id,s.wan_lists.is_none()))).then(||view!{
            <p class="canvas-note" data-tone="warn" role="note">{ROUTING_WARNING}". Each subscriber keeps its rate."</p>})}
          <div class="canvas-row" data-cols=if wan {"1"} else {"2"}>
            <label>"Name"<input prop:value=move||form().draft.name disabled=locked
              on:input=move|ev|set(|f|&mut f.draft.name,event_target_value(&ev)) on:change=move|_|flush()/></label>
            {(!wan).then(||view!{
              <label>"Access router"<select id="canvas-router" prop:value=move||form().router disabled=locked
                title="Subscribers of this router stay in this branch."
                aria-invalid=move||(!form().router_error.is_empty()).to_string()
                aria-describedby=move||(!form().router_error.is_empty()).then_some("canvas-router-error")
                on:change=move|ev|{set(|f|&mut f.router,event_target_value(&ev));set(|f|&mut f.router_error,String::new());flush();}>
                <option value="">"None"</option>
                {routers.into_iter().map(|(rid,name)|view!{<option value=rid>{name}</option>}).collect_view()}
              </select></label>
            })}
          </div>
          {move||(!form().router_error.is_empty()).then(||view!{<p id="canvas-router-error" class="field-error">{form().router_error}</p>})}
          <fieldset><legend>"Capacity"</legend>
            <div class="canvas-row" data-cols="3">
              <label>"Mode"<select prop:value=move||form().draft.mode disabled=move||locked()||wan
                on:change=move|ev|{set(|f|&mut f.draft.mode,event_target_value(&ev));flush();}>
                <option value="inherit">"Inherit from parent"</option>
                <option value="automatic">"Sum of plans"</option>
                <option value="fixed">"Fixed"</option>
                <option value="wan_percent">"Percent of WAN"</option>
              </select></label>
              <Show when=move||matches!(form().draft.mode.as_str(),"fixed"|"wan_percent")>
                <label>"Download"<span class="canvas-unit" data-unit=move||if form().draft.mode=="fixed"{"Mbps"}else{"%"}>
                  <input inputmode="numeric" prop:value=move||form().draft.down disabled=locked
                    on:input=move|ev|set(|f|&mut f.draft.down,event_target_value(&ev)) on:change=move|_|flush()/></span></label>
                <label>"Upload"<span class="canvas-unit" data-unit=move||if form().draft.mode=="fixed"{"Mbps"}else{"%"}>
                  <input inputmode="numeric" prop:value=move||form().draft.up disabled=locked
                    on:input=move|ev|set(|f|&mut f.draft.up,event_target_value(&ev)) on:change=move|_|flush()/></span></label>
              </Show>
            </div>
          </fieldset>
          {(!wan).then(||view!{<fieldset><legend>"Links"</legend>
            <ul class="canvas-links">
              {links.into_iter().map(|(from,name,subscribers,ok)|{
                let (from2,to)=(from.clone(),card2.clone());
                view!{<li><span>{format!("{name} · {subscribers} subscriber{}",if subscribers==1{""}else{"s"})}</span>
                  {ok.then(||view!{<button type="button" class="quiet danger" disabled=locked aria-label=format!("Remove link from {name}")
                    on:click=move|_|unlink.run((from2.clone(),to.clone()))>"Remove link"</button>})}</li>}
              }).collect_view()}
            </ul>
            <div class="canvas-row" data-cols="2">
              {(count==1).then(||view!{
                <label>"Parent"<select prop:value=current_parent.clone() disabled=locked
                  on:change=move|ev|{
                    act.run((Change::Move(move_node.clone()),Draft{parent:event_target_value(&ev),..Draft::default()}));
                  }>
                  <option value="">"Top level"</option>
                  {parents.into_iter().map(|(pid,name)|view!{<option value=pid>{name}</option>}).collect_view()}
                </select></label>
              })}
              {(!linkable.is_empty()).then(||view!{
                <label>"Add link from"<select disabled=locked on:change=move|ev|{
                    let v=event_target_value(&ev);
                    if v.is_empty(){ return; }
                    match drop_edit(&shown2,&v,&card){
                      Ok(edit)=>send.run(edit),
                      Err(why)=>status.set(format!("Not connected: {why}")),
                    }
                  }>
                  <option value="">"Select a card"</option>
                  {linkable.into_iter().map(|(pid,name)|view!{<option value=pid>{name}</option>}).collect_view()}
                </select></label>
              })}
            </div>
          </fieldset>})}
          {n.router_id.is_some().then(||{
            let id=n.id.clone();
            view!{
            <fieldset><legend>"Router"</legend>
              <div class="check">
                <input type="checkbox" id="canvas-tree-exclude" disabled=locked prop:checked=n.tree_excluded
                  on:change=move|ev|{
                    let edit=BuilderEdit::NodeTreeExclude{node_id:id.clone(),excluded:event_target_checked(&ev)};
                    // Every copy of the router card leaves or joins the tree together.
                    match &whole{Some(c)=>card_edits(c,vec![edit]).into_iter().for_each(|e|send.run(e)),None=>send.run(edit)}
                  }/>
                <label for="canvas-tree-exclude">"Exclude from network tree"
                  <small>"Its groups go to the parent of this card in network.json."</small></label>
              </div>
              <a class="canvas-link" href="/routers">"Router connection settings"</a>
            </fieldset>}
          })}
          {wan.then(||view!{
            <fieldset><legend>"Routing"</legend>
              <label>"Address list"<input autocomplete="off" spellcheck="false" prop:value=move||form().address_list disabled=locked
                placeholder=format!("via-{}",n.name)
                on:input=move|ev|set(|f|&mut f.address_list,event_target_value(&ev)) on:change=move|_|flush()/></label>
              {n.routed.map(|c|view!{<p class="field-help">{format!("{c} clients at the last write.")}</p>})}
            </fieldset>
          })}
          <fieldset><legend>"Location"</legend>
            <div class="canvas-row" data-cols="geo">
              <label>"Map type"<select prop:value=move||form().geo disabled=locked
                on:change=move|ev|{set(|f|&mut f.geo,event_target_value(&ev));flush();}>
                <option value="">"None"</option><option value="site">"Site"</option><option value="ap">"AP"</option>
              </select></label>
              <label>"Latitude"<input inputmode="decimal" autocomplete="off" prop:value=move||form().latitude disabled=locked
                on:paste=paste on:input=move|ev|set(|f|&mut f.latitude,event_target_value(&ev)) on:change=move|_|flush()/></label>
              <label>"Longitude"<input inputmode="decimal" autocomplete="off" prop:value=move||form().longitude disabled=locked
                on:paste=paste on:input=move|ev|set(|f|&mut f.longitude,event_target_value(&ev)) on:change=move|_|flush()/></label>
            </div>
            <p class="field-help">"Paste a Google Maps pair into Latitude to fill both."</p>
          </fieldset>
          {move||(!form().error.is_empty()).then(||view!{<p class="field-error" role="alert">{form().error}</p>})}
          <div class="canvas-actions">
            {can_add_child.then(||view!{
              <button type="button" class="quiet" disabled=locked on:click=move|_|act.run((Change::CreateChild(child_id.clone()),Draft{
                name:"New node".into(),parent:child_id.clone(),kind:"intermediate".into(),mode:"inherit".into(),..Draft::default()}))>"Add child node"</button>
            })}
            <span class="canvas-actions-gap"></span>
            {group_source.map(|source|view!{
              <button type="button" class="quiet danger" disabled=locked title="LQoSync stops reading and shaping this source. Excluded sources can read it again."
                on:click=move|_|{
                  if confirm_exclude.get_untracked(){
                    send.run(BuilderEdit::SourceExclude{source_id:source.clone(),excluded:true});
                    confirm_exclude.set(false);
                    close.run(());
                  } else { confirm_exclude.set(true); }
                }>{move||if confirm_exclude.get(){"Confirm do not read"}else{"Do not read"}}</button>
            })}
            {removable_node.then(||view!{
              <button type="button" class="quiet danger" disabled=locked on:click=move|_|{
                if confirm_remove.get_untracked(){
                  act.run((Change::Remove(remove_node.clone()),Draft::default()));
                  confirm_remove.set(false);
                  close.run(());
                } else { confirm_remove.set(true); }
              }>{move||if confirm_remove.get(){"Confirm remove"}else{"Remove node"}}</button>
            })}
          </div>
          <details class="canvas-ids"><summary>"Details"</summary>
            <dl>
              <dt>"Kind"</dt><dd>{kind_label(&n.kind)}</dd>
              <dt>"Card ID"</dt><dd>{card_of(&n).to_string()}</dd>
              {n.source_id.clone().map(|s|view!{<dt>"Source ID"</dt><dd>{s}</dd>})}
            </dl>
          </details>
        </form>}
      })}
    }
}
