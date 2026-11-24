//! The Topology canvas: a Sankey of the network tree, a tool dock, and modal editors. Columns,
//! card heights, and one live stream for each WAN come from `crate::sankey`. Every valid edit
//! saves automatically through the service.
use crate::canvas_layout::{clamp_zoom, fit};
use crate::topology::{Evidence, now};
use crate::topology_canvas_model::*;
use crate::topology_forms::{blocked, new_key, post, settle, structural_blocked};
use crate::topology_model::*;
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

pub(crate) fn icon(path: &'static str) -> impl IntoView {
    view! {<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d=path/></svg>}
}

const ICON_INVENTORY: &str = "M4 7h16v12H4zM8 7V4h8v3M8 12h8M8 15h5";
const ICON_SETTINGS: &str = "M4 7h10M18 7h2M4 17h4M12 17h8M14 5v4M8 15v4";
const ICON_ALLOCATION: &str = "M12 4v6M6 20v-4h12v4M12 10l-6 6M12 10l6 6";
const ICON_EXCLUDED: &str = "M4 6h16M4 12h10M4 18h7M15 15l5 5M20 15l-5 5";
const ICON_PREVIEW: &str =
    "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z";

const ICON_HELP: &str = "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM12 11v6M12 7v1";
const ICON_LIST: &str = "M8 6h12M8 12h12M8 18h12M3 6h1M3 12h1M3 18h1";
const ICON_MINUS: &str = "M4 12h16";
const ICON_PLUS: &str = "M4 12h16M12 4v16";
const ICON_FIT: &str = "M3 9V3h6M15 3h6v6M21 15v6h-6M9 21H3v-6";
const ICON_ACTUAL: &str = "M5 7v10M3 9l2-2 2 2M12 7h7v10h-7z";

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
    // The camera of the infinite canvas: screen point = world point × zoom + pan.
    let pan = RwSignal::new([0.0_f64; 2]);
    let forms = RwSignal::new(HashMap::<String, NodeForm>::new());
    let queue = StoredValue::new(VecDeque::<BuilderEdit>::new());
    let draining = RwSignal::new(false);
    let policy_revision = RwSignal::new(None::<u64>);
    let projection = RwSignal::new(None::<web_contract::source_studio::CandidateSummary>);
    let studio_open = RwSignal::new(false);
    let studio_router = RwSignal::new(None::<String>);
    let help_dialog = NodeRef::<leptos::html::Dialog>::new();
    let help_open = RwSignal::new(false);
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let viewport = NodeRef::<leptos::html::Div>::new();
    let frame = NodeRef::<leptos::html::Div>::new();
    let frame_bounds = RwSignal::new([0.0_f64; 4]);
    let measure = move || {
        if let Some(f) = frame.get_untracked() {
            let r = f.get_bounding_client_rect();
            frame_bounds.set([r.x(), r.y(), r.width(), r.height()]);
        }
    };
    request_animation_frame(measure);
    let resize = window_event_listener(leptos::ev::resize, move |_| measure());
    on_cleanup(move || resize.remove());

    let list_view = RwSignal::new(false);
    // The selected line: parent card ID and child card ID.
    let selected_link = RwSignal::new(None::<(String, String, Option<String>)>);
    // The Sankey always runs from left to right. The Studio preview overlay reads this signal.
    let direction = RwSignal::new(Direction::LeftRight);
    // The popover of the selected card or stream. Non-modal: focus stays on the canvas.
    let popover = RwSignal::new(false);
    // Escape closes the popover wherever the focus is. The focus does not move.
    // Escape inside the popover returns the focus to the selected card.
    let escape = window_event_listener(leptos::ev::keydown, move |ev| {
        if ev.key() != "Escape" || !popover.get_untracked() {
            return;
        }
        popover.set(false);
        let inside = document()
            .active_element()
            .is_some_and(|e| e.closest(".sankey-popover").ok().flatten().is_some());
        if inside
            && let Some(id) = selected.get_untracked()
            && let Ok(Some(card)) = document().query_selector(&format!("[data-node-id=\"{id}\"]"))
            && let Ok(card) = card.dyn_into::<web_sys::HtmlElement>()
        {
            let _ = card.focus();
        }
    });
    on_cleanup(move || escape.remove());
    // The live throughput, read every 5 s while the canvas is open. The last good reply stays.
    let live = RwSignal::new(None::<TopologyThroughput>);
    // The loop ends when the canvas is gone: its signals are then disposed.
    leptos::task::spawn_local(async move {
        while live.try_with_untracked(|_| ()).is_some() {
            let Ok(t) =
                crate::topology::get::<TopologyThroughput>(session, "/api/v1/topology/throughput")
                    .await
            else {
                // A gateway that does not answer: the kept sample is old, never live.
                let gone = live
                    .try_update(|t| {
                        if let Some(t) = t {
                            t.stale = true;
                            t.complete = false;
                            t.reason =
                                Some("The gateway did not answer. The last sample is old.".into());
                        }
                    })
                    .is_none();
                if gone {
                    break;
                }
                gloo_timers::future::TimeoutFuture::new(5000).await;
                continue;
            };
            if live.try_set(Some(t)).is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(5000).await;
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
    let wan_names = Memo::new(move |_| {
        nodes.with(|n| {
            n.iter()
                .filter(|w| w.kind == "wan")
                .map(|w| (w.id.clone(), w.name.clone()))
                .collect::<Vec<_>>()
        })
    });
    let writer_off = move || data.with(|d| d.as_ref().is_some_and(|s| s.wan_lists.is_none()));
    let candidates = Memo::new(move |_| {
        data.with(|d| {
            d.as_ref()
                .map(|s| s.wan_candidates.clone())
                .unwrap_or_default()
        })
    });
    // The order of the last layout keeps cards in place between refreshes.
    let order = StoredValue::new(Vec::<String>::new());
    let sk = Memo::new(move |_| {
        let input = data.with(|d| {
            d.as_ref().map(|s| {
                let shown = TopologySummary {
                    nodes: nodes.get(),
                    ..s.clone()
                };
                live.with(|t| crate::sankey::input(&shown, t.as_ref()))
            })
        })?;
        let out = crate::sankey::layout(&input, &order.get_value());
        let mut next: Vec<&crate::sankey::CardBox> = out.cards.iter().collect();
        next.sort_by(|a, b| a.column.cmp(&b.column).then(a.y.total_cmp(&b.y)));
        order.set_value(next.into_iter().map(|c| c.id.clone()).collect());
        Some((input, out))
    });
    let metric = move || {
        sk.with(|k| {
            k.as_ref()
                .map_or(crate::sankey::Metric::Subscribers, |(i, _)| i.metric)
        })
    };
    let box_of = move |id: &str| {
        sk.with(|k| {
            k.as_ref()
                .and_then(|(_, o)| o.cards.iter().find(|c| c.id == id).cloned())
        })
    };
    // Card corners in canvas pixels. WAN candidates stack below the first column.
    let points = Memo::new(move |_| {
        let mut p: Vec<(String, [f64; 2])> = sk.with(|k| {
            k.as_ref()
                .map(|(_, o)| {
                    o.cards
                        .iter()
                        .map(|c| (c.id.clone(), [c.x + MARGIN, c.y + SANKEY_TOP]))
                        .collect()
                })
                .unwrap_or_default()
        });
        let mut below = p
            .iter()
            .filter_map(|(id, at)| box_of(id).filter(|b| b.column == 0).map(|b| at[1] + b.h))
            .fold(SANKEY_TOP, f64::max)
            + crate::sankey::GAP;
        for (id, ..) in candidate_cards(&candidates.get()) {
            p.push((id, [MARGIN, below]));
            below += CARD_H + crate::sankey::GAP;
        }
        p
    });
    let point_of =
        move |id: &str| points.with(|p| p.iter().find(|(n, _)| n == id).map(|(_, x)| *x));
    let locked = move || structural_blocked(data).is_some() || projection.with(Option::is_some);
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
        measure();
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
        let excluded = n.tree_excluded.then_some("Display only · not shaped");
        let warning = nodes
            .with_untracked(|ns| routing_warning(ns, &n.id, writer_off()))
            .then_some(ROUTING_WARNING);
        let label = format!(
            "{}, {kind}, {meta}{}{}{}{}{}. Enter shows details. Enter again opens settings.",
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
        // The label starts with the name and the live value; the value changes each refresh.
        let label_rest = label
            .strip_prefix(&format!("{}, ", n.name))
            .unwrap_or(&label)
            .to_string();
        let card_name = n.name.clone();
        let height = move || id.with_value(|i| box_of(i)).map_or(CARD_H, |b| b.h);
        let column = move || id.with_value(|i| box_of(i)).map_or(0, |b| b.column);
        let value = move || id.with_value(|i| box_of(i)).map(|b| b.value);
        // A WAN card wears the color of its stream.
        let wan_id = (n.kind == "wan").then(|| n.id.clone());
        let tone = {
            let wan_id = wan_id.clone();
            move || {
                sk.with(|k| {
                    k.as_ref()
                        .map(|(i, _)| wan_tone(&i.wans, wan_id.as_deref()))
                        .unwrap_or_default()
                })
            }
        };
        view! {
          <button type="button" class="canvas-node sankey-card" data-kind=n.kind.clone() data-node-id=c.id.clone()
            data-column=move||column().to_string() data-tone=move||wan_id.is_some().then(&tone)
            aria-pressed=move||is_selected().to_string()
            aria-describedby=move||(is_selected()&&popover.get()).then_some("sankey-details")
            aria-label=move||format!("{}, {}, {}",card_name,value_text(value(),metric()),label_rest)
            style:left=move||format!("{}px",pos()[0]) style:top=move||format!("{}px",pos()[1])
            style:height=move||format!("{}px",height())
            on:click=move|_|{
              selected.set(Some(id.get_value()));
              selected_link.set(None);
              popover.set(true);
            }
            on:dblclick=move|_|{ selected.set(Some(id.get_value())); open(Panel::Node); }
            on:keydown=move|ev: web_sys::KeyboardEvent|{
              if ev.key()=="Enter" && is_selected() { ev.prevent_default(); popover.set(false); open(Panel::Node); }
            }>
            <span class="canvas-node-top">
              <span class="canvas-node-kind">{kind}</span>
              {router.map(|r|view!{<span class="canvas-node-router">{format!("Router {r}")}</span>})}
              {auto.map(|a|view!{<span class="canvas-node-auto">{a}</span>})}
              {health.map(|(state,h)|view!{<span class="canvas-node-health" data-state=state>{h}</span>})}
              {excluded.map(|t|view!{<span class="canvas-node-router">{t}</span>})}
            </span>
            <strong>{n.name.clone()}</strong>
            <span class="sankey-value">{move||value_text(value(),metric())}</span>
            <span class="canvas-node-meta"><span>{rate}</span>{members.map(|m|view!{<span>{m}</span>})}</span>
            {warning.map(|t|view!{<span class="canvas-node-warning">{t}</span>})}
          </button>
        }
    };
    // Inventory actions from the router reads.
    let read_action = RwSignal::new(None::<Change>);
    let wan_form = RwSignal::new(None::<(Change, Draft, String)>);
    // The one-time routing adoption is a reviewed change in the house change dialog.
    let adopt = RwSignal::new(None::<Change>);
    let confirm_rebalance = RwSignal::new(false);
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
    // One filled band for each WAN stream between two cards. The selection dims the others.
    let ribbons = move || {
        let sel = selected.get();
        let link = selected_link.get();
        let (wans, list) = sk.with(|k| {
            k.as_ref()
                .map(|(i, o)| (i.wans.clone(), o.ribbons.clone()))
                .unwrap_or_default()
        });
        list.into_iter()
            .map(|r| {
                let active = sel.as_deref().is_some_and(|s| s == r.from || s == r.to)
                    || link
                        .as_ref()
                        .is_some_and(|(a, b, w)| *a == r.from && *b == r.to && *w == r.wan);
                let dim = (sel.is_some() || link.is_some()) && !active;
                let tone = wan_tone(&wans, r.wan.as_deref());
                let (from, to, wan) = (r.from.clone(), r.to.clone(), r.wan.clone());
                view! {
                  <path class="sankey-ribbon" class:active=active class:dim=dim d=r.d.clone()
                    data-from=r.from.clone() data-to=r.to.clone() data-tone=tone.clone()
                    data-wan=r.wan.clone().unwrap_or_else(||"unknown".into())
                    on:click=move|ev: web_sys::MouseEvent|{
                      ev.stop_propagation();
                      selected.set(None);
                      selected_link.set(Some((from.clone(),to.clone(),wan.clone())));
                      popover.set(true);
                    }/>
                }
            })
            .collect_view()
    };
    // The name of a card, a service tier, or a WAN stream.
    let name_of = move |id: &str| -> String {
        tier_name(id).map(str::to_string).unwrap_or_else(|| {
            card_list
                .with(|c| c.iter().find(|c| c.id == id).map(|c| c.node.name.clone()))
                .unwrap_or_else(|| id.to_string())
        })
    };
    let wan_name = move |w: Option<&str>| match w {
        Some(w) => wan_names
            .with(|n| n.iter().find(|(id, _)| id == w).map(|(_, n)| n.clone()))
            .unwrap_or_else(|| w.to_string()),
        None => unknown_stream(metric()).to_string(),
    };
    // The details of the selected card or stream, next to it on the canvas.
    let details = move || {
        if !popover.get() {
            return None;
        }
        let (input, out) = sk.get()?;
        let at = |id: &str| out.cards.iter().find(|c| c.id == id).cloned();
        if let Some((from, to, wan)) = selected_link.get() {
            let a = at(&from)?;
            let title = format!(
                "{} · {} → {}",
                wan_name(wan.as_deref()),
                name_of(&from),
                name_of(&to)
            );
            let r = out
                .ribbons
                .iter()
                .find(|r| r.from == from && r.to == to && r.wan == wan)?;
            let rate = match input.metric {
                crate::sankey::Metric::LiveMbps => {
                    let up = live.with(|t| {
                        t.as_ref().and_then(|t| {
                            t.flows
                                .iter()
                                .find(|f| f.from == from && f.to == to && f.wan == wan)
                                .map(|f| f.up_bps)
                        })
                    });
                    format!(
                        "{} down · {} up",
                        value_text(Some(r.value), input.metric),
                        value_text(up.map(|u| u as f64 / 1_000_000.0), input.metric)
                    )
                }
                crate::sankey::Metric::Subscribers => value_text(Some(r.value), input.metric),
            };
            return Some(view! {
              <div class="sankey-popover" id="sankey-details" role="dialog" aria-label="Stream details"
                style:left=format!("{}px",a.x+a.w+MARGIN+12.0) style:top=format!("{}px",a.y+SANKEY_TOP)>
                <strong>{title}</strong><p>{rate}</p>
              </div>
            }.into_any());
        }
        let id = selected.get()?;
        let c = at(&id)?;
        // The streams into the card, or out of it for a first-column card, summed for each WAN.
        let mut lines: Vec<(Option<String>, f64)> = Vec::new();
        let into = out.ribbons.iter().any(|r| r.to == id);
        for r in out
            .ribbons
            .iter()
            .filter(|r| if into { r.to == id } else { r.from == id })
        {
            match lines.iter_mut().find(|(w, _)| *w == r.wan) {
                Some((_, v)) => *v += r.value,
                None => lines.push((r.wan.clone(), r.value)),
            }
        }
        let name = name_of(&id);
        let tier = tier_name(&id).is_some();
        Some(view! {
          <div class="sankey-popover" id="sankey-details" role="dialog" aria-label=format!("{name} details")
            style:left=format!("{}px",c.x+c.w+MARGIN+12.0) style:top=format!("{}px",c.y+SANKEY_TOP)>
            <strong>{name.clone()}</strong>
            <p>{value_text(Some(c.value),input.metric)}</p>
            <ul>{lines.into_iter().map(|(w,v)|{
              let tone=wan_tone(&input.wans,w.as_deref());
              view!{<li><span class="sankey-swatch" data-tone=tone></span>{format!("{}: {}",wan_name(w.as_deref()),value_text(Some(v),input.metric))}</li>}
            }).collect_view()}</ul>
            {(!tier).then(||view!{<button type="button" class="quiet" on:click=move|_|{popover.set(false);open(Panel::Node);}>"Open full settings"</button>})}
          </div>
        }.into_any())
    };
    // The service tier cards: the last column. They have no settings.
    let tiers = move || {
        sk.with(|k| {
            k.as_ref()
                .map(|(i, o)| (i.metric, o.cards.clone()))
                .unwrap_or((crate::sankey::Metric::Subscribers, Vec::new()))
        })
    };
    let tier_card = move |c: crate::sankey::CardBox| {
        let id = c.id.clone();
        let name = tier_name(&c.id).unwrap_or("Service tier");
        let is_selected = {
            let id = id.clone();
            move || selected.get().as_deref() == Some(id.as_str())
        };
        view! {
          <button type="button" class="sankey-card sankey-tier" data-node-id=c.id.clone() data-column=c.column.to_string()
            aria-pressed=move||is_selected().to_string()
            aria-label=format!("{name}, service tier, {}. Enter shows details.",value_text(Some(c.value),tiers().0))
            style:left=format!("{}px",c.x+MARGIN) style:top=format!("{}px",c.y+SANKEY_TOP) style:height=format!("{}px",c.h)
            on:click=move|_|{selected.set(Some(id.clone()));selected_link.set(None);popover.set(true);}>
            <span class="canvas-node-top"><span class="canvas-node-kind">"Service tier"</span></span>
            <strong>{name}</strong>
            <span class="sankey-value">{value_text(Some(c.value),tiers().0)}</span>
          </button>
        }
    };
    // Column headers: the card kinds in each column.
    let headers = move || {
        sk.with(|k| {
            let Some((_, o)) = k.as_ref() else {
                return Vec::new();
            };
            (0..o.columns)
                .map(|col| {
                    let mut kinds: Vec<&'static str> = Vec::new();
                    for c in o.cards.iter().filter(|c| c.column == col) {
                        let kind = if tier_name(&c.id).is_some() {
                            "Service tier"
                        } else {
                            card_list
                                .with(|l| {
                                    l.iter().find(|x| x.id == c.id).map(|x| card_kind(&x.node))
                                })
                                .unwrap_or("Node")
                        };
                        if !kinds.contains(&kind) {
                            kinds.push(kind);
                        }
                    }
                    (col, kinds.join(" · "))
                })
                .collect::<Vec<_>>()
        })
    };
    let world_points = Memo::new(move |_| {
        let mut at = points.get();
        if let Some(p) = projection.get() {
            let (_, extra) = crate::topology_studio_preview::candidate_geometry(
                &p,
                data.get().as_ref(),
                &at,
                direction.get(),
            );
            at.extend(extra);
        }
        at
    });
    // The canvas size: the cards with their Sankey heights, and room around them.
    let size = move || {
        let (w, h) = world_points.with(|p| surface(p));
        let bottom = sk.with(|k| k.as_ref().map_or(0.0, |(_, o)| o.bounds[3]));
        (w, h.max(bottom + SANKEY_TOP + 4.0 * MARGIN))
    };
    let view_size = move || {
        viewport.get_untracked().map_or([0.0; 2], |v| {
            [v.client_width() as f64, v.client_height() as f64]
        })
    };
    // Zoom to `next`, and keep the world point below the screen point `at` in place.
    // True while the camera shows the last "Fit view". A window resize then fits again.
    let auto_fit = StoredValue::new(true);
    let zoom_at = move |next: f64, at: [f64; 2]| {
        auto_fit.set_value(false);
        let (z, n) = (zoom.get_untracked(), clamp_zoom(next));
        pan.update(|p| {
            p[0] = at[0] - (at[0] - p[0]) * n / z;
            p[1] = at[1] - (at[1] - p[1]) * n / z;
        });
        zoom.set(n);
    };
    let center = move || {
        let [w, h] = view_size();
        [w / 2.0, h / 2.0]
    };
    // The minimap box: the content and the visible part, so the view rectangle always shows.
    let mini_box = move || {
        let (w, h) = size();
        let (z, [px, py], f) = (zoom.get(), pan.get(), frame_bounds.get());
        let (vx, vy, vw, vh) = (-px / z, -py / z, f[2] / z, f[3] / z);
        let (x0, y0) = (vx.min(0.0), vy.min(0.0));
        [x0, y0, (vx + vw).max(w) - x0, (vy + vh).max(h) - y0]
    };
    // Fit every card in the part of the canvas that the floating controls leave free.
    let fit_view = move || {
        let b = world_points.with_untracked(|p| {
            p.iter()
                .fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, (id, q)| {
                    let h = box_of(id).map_or(CARD_H, |c| c.h);
                    [
                        b[0].min(q[0]),
                        b[1].min(q[1]),
                        b[2].max(q[0] + CARD_W),
                        b[3].max(q[1] + h),
                    ]
                })
        });
        let [w, h] = view_size();
        if b[0] > b[2] || w <= 0.0 {
            return;
        }
        let [top, right, bottom, left] = FIT_PAD;
        let (z, x, y) = fit(b, [w - left - right, h - top - bottom]);
        zoom.set(z);
        pan.set([x + left, y + top]);
        auto_fit.set_value(true);
    };
    let refit = window_event_listener(leptos::ev::resize, move |_| {
        if auto_fit.get_value() {
            request_animation_frame(fit_view);
        }
    });
    on_cleanup(move || refit.remove());
    // Fit once, when the first layout is ready.
    let fitted = StoredValue::new(false);
    Effect::new(move |_| {
        if sk.with(Option::is_some) && !fitted.get_value() {
            fitted.set_value(true);
            request_animation_frame(fit_view);
        }
    });
    // A pointer drag on the background pans. It starts after 4 px, so a click stays a click.
    let drag = StoredValue::new(None::<(i32, f64, f64, [f64; 2], bool)>);
    let panned = StoredValue::new(false);
    // Touch pointers, for a two-finger pinch: pointer ID and screen point.
    let touches = StoredValue::new(Vec::<(i32, f64, f64)>::new());
    let local_point = move |x: f64, y: f64| {
        viewport.get_untracked().map_or([x, y], |v| {
            let r = v.get_bounding_client_rect();
            [x - r.x(), y - r.y()]
        })
    };
    Effect::new(move |_| {
        if projection.with(Option::is_some) {
            list_view.set(false);
        }
    });
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
            aria-haspopup="dialog" disabled=move||projection.with(Option::is_some) on:click=move|_|open(p)>{icon(path)}</button>
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
      <section class="canvas-shell" aria-label="Topology canvas workspace"
        data-metric=move||match metric(){crate::sankey::Metric::LiveMbps=>"live",crate::sankey::Metric::Subscribers=>"subscribers"}>
        <div class="canvas-bar">
          <p class="canvas-facts">{move||data.with(|d|d.as_ref().map(|s|{
            let wans=s.nodes.iter().filter(|n|n.kind=="wan").count();
            format!("Policy revision {} · {wans} WANs · {} nodes · {} active subscribers",
              s.registry_revision.clone().unwrap_or("none".into()),s.nodes.len(),s.subscribers_active)
          }))}</p>
          <p class="canvas-status" role="status" aria-live="polite">{move||status.get()}</p>
        </div>
        <div class="canvas-frame" node_ref=frame style=move||{
          let [x,y,w,h]=frame_bounds.get();
          format!("--studio-left:{x}px;--studio-top:{y}px;--studio-width:{w}px;--studio-height:{h}px")
        }>
        <div class="sankey-toolbar">
          <div class="sankey-chips" role="group" aria-label="Topology state">
            {move||data.with(|d|d.as_ref().map(|s|{
              let saved=s.registry_revision.clone().map_or("No saved policy".to_string(),|r|format!("Saved rev {r}"));
              let applied=s.publication.as_ref().filter(|p|p.phase=="applied").and_then(|p|p.policy_revision.clone());
              let phase=s.publication.as_ref().map_or("no write yet".to_string(),|p|p.phase.replace('_'," "));
              view!{
                <span class="sankey-chip" data-state="ok">{saved}</span>
                {match applied {
                  Some(r)=>view!{<span class="sankey-chip" data-state="ok">{format!("Applied rev {r}")}</span>}.into_any(),
                  None=>view!{<span class="sankey-chip" data-state="warn" title=format!("Last write: {phase}")>"Not applied"</span>}.into_any(),
                }}
              }
            }))}
            {move||{
              let (text,state,why)=live_chip(live.get().as_ref(),now());
              view!{<span class="sankey-chip" data-state=state title=why>{text}</span>}
            }}
          </div>
          <ul class="sankey-legend" aria-label="WAN streams">
            {move||sk.with(|k|k.as_ref().map(|(i,o)|{
              let mut shown:Vec<Option<String>>=i.wans.iter().filter(|w|o.ribbons.iter().any(|r|r.wan.as_deref()==Some(w.as_str()))).cloned().map(Some).collect();
              if o.ribbons.iter().any(|r|r.wan.is_none()) && i.metric==crate::sankey::Metric::LiveMbps { shown.push(None); }
              shown.into_iter().map(|w|{
                let tone=wan_tone(&i.wans,w.as_deref());
                view!{<li><span class="sankey-swatch" data-tone=tone aria-hidden="true"></span>{wan_name(w.as_deref())}</li>}
              }).collect_view()
            }))}
          </ul>
        </div>
          {move||structural_blocked(data).filter(|_|data.with(Option::is_some)).map(|why|view!{<p class="notice canvas-readonly" role="note">{why}</p>})}
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
            aria-label="Topology canvas. Arrow keys move the view, plus and minus zoom, 0 fits. Tab moves between cards."
            style:background-size=move||{let g=24.0*zoom.get();format!("{g}px {g}px")}
            style:background-position=move||{let [x,y]=pan.get();format!("{x}px {y}px")}
            on:scroll=move|ev: web_sys::Event|{
              // A browser scroll (for example to show a focused card) becomes a camera move.
              let Some(v)=ev.current_target().and_then(|t|t.dyn_into::<web_sys::Element>().ok()) else {return};
              let (sx,sy)=(v.scroll_left() as f64,v.scroll_top() as f64);
              if sx!=0.0||sy!=0.0 { pan.update(|p|{p[0]-=sx;p[1]-=sy;}); v.set_scroll_left(0); v.set_scroll_top(0); }
            }
            on:pointerdown=move|ev: web_sys::PointerEvent|{
              if ev.pointer_type()=="touch" { touches.update_value(|t|{t.retain(|p|p.0!=ev.pointer_id());t.push((ev.pointer_id(),ev.client_x() as f64,ev.client_y() as f64));}); }
              if ev.button()!=0 { return; }
              let on_control=ev.target().and_then(|t|t.dyn_into::<web_sys::Element>().ok())
                .is_some_and(|e|e.closest("button, a, input, select, textarea, .sankey-popover, path.sankey-ribbon").ok().flatten().is_some());
              if !on_control { drag.set_value(Some((ev.pointer_id(),ev.client_x() as f64,ev.client_y() as f64,pan.get_untracked(),false))); }
            }
            on:pointermove=move|ev: web_sys::PointerEvent|{
              // Two touch pointers: the pinch zooms at their midpoint.
              if ev.pointer_type()=="touch" {
                let before=touches.get_value();
                touches.update_value(|t|if let Some(p)=t.iter_mut().find(|p|p.0==ev.pointer_id()){p.1=ev.client_x() as f64;p.2=ev.client_y() as f64;});
                let after=touches.get_value();
                if before.len()==2 && after.len()==2 {
                  let d=|t:&[(i32,f64,f64)]|((t[0].1-t[1].1).powi(2)+(t[0].2-t[1].2).powi(2)).sqrt();
                  if d(&before)>0.0 {
                    let mid=local_point((after[0].1+after[1].1)/2.0,(after[0].2+after[1].2)/2.0);
                    zoom_at(zoom.get_untracked()*d(&after)/d(&before),mid);
                  }
                  drag.set_value(None);
                  return;
                }
              }
              let Some((id,x0,y0,start,moved))=drag.get_value() else {return};
              if id!=ev.pointer_id() { return; }
              // The button was released outside the window: the drag is over.
              if ev.buttons()==0 {
                drag.set_value(None);
                if let Some(v)=viewport.get_untracked() { let _=v.class_list().remove_1("panning"); }
                return;
              }
              let (dx,dy)=(ev.client_x() as f64-x0,ev.client_y() as f64-y0);
              if !moved && dx.abs()+dy.abs()<4.0 { return; }
              if !moved {
                if let Some(v)=viewport.get_untracked() { let _=v.set_pointer_capture(ev.pointer_id()); let _=v.class_list().add_1("panning"); }
                drag.set_value(Some((id,x0,y0,start,true)));
              }
              auto_fit.set_value(false);
              pan.set([start[0]+dx,start[1]+dy]);
            }
            on:pointerup=move|ev: web_sys::PointerEvent|{
              touches.update_value(|t|t.retain(|p|p.0!=ev.pointer_id()));
              if let Some((_,_,_,_,true))=drag.get_value() { panned.set_value(true); }
              drag.set_value(None);
              if let Some(v)=viewport.get_untracked() { let _=v.class_list().remove_1("panning"); }
            }
            on:lostpointercapture=move|_|{
              if let Some((_,_,_,_,true))=drag.get_value() { panned.set_value(true); }
              drag.set_value(None);
              if let Some(v)=viewport.get_untracked() { let _=v.class_list().remove_1("panning"); }
            }
            on:pointercancel=move|ev: web_sys::PointerEvent|{
              touches.update_value(|t|t.retain(|p|p.0!=ev.pointer_id()));
              drag.set_value(None);
              if let Some(v)=viewport.get_untracked() { let _=v.class_list().remove_1("panning"); }
            }
            on:wheel=move|ev: web_sys::WheelEvent|{
              // The wheel zooms at the cursor. A line-mode wheel counts about 33 px per line.
              ev.prevent_default();
              let dy=ev.delta_y()*if ev.delta_mode()==1 {33.0} else {1.0};
              zoom_at(zoom.get_untracked()*(-dy*0.0015).exp(),local_point(ev.client_x() as f64,ev.client_y() as f64));
            }
            on:keydown=move|ev: web_sys::KeyboardEvent|{
              let typing=ev.target().and_then(|t|t.dyn_into::<web_sys::Element>().ok())
                .is_some_and(|e|e.closest("input, select, textarea, dialog").ok().flatten().is_some());
              if typing || ev.ctrl_key() || ev.meta_key() || ev.alt_key() { return; }
              let step=48.0;
              if ev.key().starts_with("Arrow") { auto_fit.set_value(false); }
              match ev.key().as_str() {
                "ArrowLeft"=>pan.update(|p|p[0]+=step),
                "ArrowRight"=>pan.update(|p|p[0]-=step),
                "ArrowUp"=>pan.update(|p|p[1]+=step),
                "ArrowDown"=>pan.update(|p|p[1]-=step),
                "+"|"="=>zoom_at(zoom.get_untracked()*1.2,center()),
                "-"|"_"=>zoom_at(zoom.get_untracked()/1.2,center()),
                "0"=>fit_view(),
                _=>return,
              }
              ev.prevent_default();
            }
            on:focusin=move|ev: web_sys::FocusEvent|{
              // A card that gets focus outside the view comes to the center of the view.
              let Some(e)=ev.target().and_then(|t|t.dyn_into::<web_sys::Element>().ok()) else {return};
              let Some(card)=e.closest(".sankey-card, .canvas-candidate").ok().flatten() else {return};
              let (Some(v),r)=(viewport.get_untracked(),card.get_bounding_client_rect()) else {return};
              let b=v.get_bounding_client_rect();
              let inside=r.left()>=b.left()&&r.right()<=b.right()&&r.top()>=b.top()&&r.bottom()<=b.bottom();
              if !inside {
                let (cx,cy)=((r.left()+r.right())/2.0,(r.top()+r.bottom())/2.0);
                let (vx,vy)=((b.left()+b.right())/2.0,(b.top()+b.bottom())/2.0);
                auto_fit.set_value(false);
                pan.update(|p|{p[0]+=vx-cx;p[1]+=vy-cy;});
              }
            }
            on:click=move|ev: web_sys::MouseEvent|{
              // The click at the end of a pan keeps the selection.
              if panned.get_value() { panned.set_value(false); return; }
              // A click on empty canvas clears the selection.
              let empty=ev.target().and_then(|t|t.dyn_into::<web_sys::Element>().ok())
                .is_some_and(|e|e.class_list().contains("canvas-layer")||e.class_list().contains("canvas-surface"));
              if empty { selected.set(None); selected_link.set(None); popover.set(false); }
            }>
            <div class="canvas-surface">
              <div class="canvas-layer" data-direction="left_right" style:width=move||format!("{}px",size().0) style:height=move||format!("{}px",size().1)
                style:transform=move||{let [x,y]=pan.get();format!("translate({x}px,{y}px) scale({})",zoom.get())}>
                <div class="canvas-saved-layer" style="display:contents" inert=move||projection.with(Option::is_some) data-preview=move||projection.with(Option::is_some).to_string()>
                {move||headers().into_iter().map(|(col,text)|view!{
                  <p class="sankey-column-head" style:left=format!("{}px",MARGIN+col as f64*(crate::sankey::CARD_W+crate::sankey::COL_GAP))>{text}</p>
                }).collect_view()}
                <svg class="canvas-wires sankey-ribbons" aria-hidden="true" width=move||size().0 height=move||size().1>
                  <g transform=format!("translate({MARGIN} {SANKEY_TOP})")>{ribbons}</g>
                </svg>
                <For each=move||card_list.get() key=|c|{let n=&c.node; format!("{}|{}|{}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",c.id,n.name,n.kind,n.geo,n.capacity,n.exported_mbps,c.members,n.router_id,(n.auto_members,&n.wan_health,n.routed,n.tree_excluded))} children=card/>
                {move||tiers().1.into_iter().filter(|c|tier_name(&c.id).is_some()).map(tier_card).collect_view()}
                <For each=move||candidate_cards(&candidates.get()) key=|c|c.0.clone() children=candidate/>
                </div>
                <crate::topology_studio_preview::CandidateOverlay projection saved=data points direction/>
                {details}

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
            <button type="button" class="canvas-tool" aria-label="Router source configuration" title="Router source configuration" on:click=move|_|{measure();studio_open.set(true);}>{icon("M4 4h16v16H4zM8 8h8M8 12h5M8 16h8")}</button>
            {tool(Panel::Allocation,"Subscriber allocation",ICON_ALLOCATION)}
            {tool(Panel::Preview,"Preview and evidence",ICON_PREVIEW)}
            {tool(Panel::Excluded,"Excluded sources",ICON_EXCLUDED)}
            <button type="button" class="canvas-tool" aria-label="Configuration information" title="Configuration information" on:click=move|_|{measure();help_open.set(true);if let Some(d)=help_dialog.get_untracked(){let _=d.show_modal();}}>{icon(ICON_HELP)}</button>
            <button type="button" class="canvas-tool" aria-label="List view" title="List view" disabled=move||projection.with(Option::is_some) aria-pressed=move||list_view.get().to_string() on:click=move|_|list_view.set(!list_view.get_untracked())>{icon(ICON_LIST)}</button>
            <Show when=move||selected.get().is_some()>
              <span class="canvas-zoom-sep" aria-hidden="true"></span>
              <button type="button" class="canvas-tool" aria-label="Card settings" title="Card settings" disabled=move||projection.with(Option::is_some) on:click=move|_|open(Panel::Node)>{icon(ICON_SETTINGS)}</button>
            </Show>

          </div>
          // The minimap: every card at a small scale and the visible part. "Fit view" is the
          // keyboard path, so the minimap is pointer-only.
          <svg class="sankey-minimap" aria-hidden="true"
            viewBox=move||{let [x,y,w,h]=mini_box();format!("{x} {y} {w} {h}")}
            on:click=move|ev: web_sys::MouseEvent|{
              // The clicked world point goes to the center of the view.
              let Some(t)=ev.current_target().and_then(|t|t.dyn_into::<web_sys::Element>().ok()) else {return};
              let r=t.get_bounding_client_rect();
              let z=zoom.get_untracked();let [cx,cy]=center();
              let [bx,by,bw,bh]=mini_box();
              // preserveAspectRatio: xMidYMid meet. One scale for both axes, centered.
              let k=(r.width()/bw).min(r.height()/bh);
              let (ox,oy)=((r.width()-bw*k)/2.0,(r.height()-bh*k)/2.0);
              let wx=bx+(ev.client_x() as f64-r.x()-ox)/k;
              let wy=by+(ev.client_y() as f64-r.y()-oy)/k;
              auto_fit.set_value(false);
              pan.set([cx-wx*z,cy-wy*z]);
            }>
            {move||world_points.get().into_iter().map(|(id,p)|{
              let h=box_of(&id).map_or(CARD_H,|c|c.h);
              view!{<rect class="sankey-minimap-card" x=p[0] y=p[1] width=CARD_W height=h rx="6"/>}
            }).collect_view()}
            <rect class="sankey-minimap-view" x=move||-pan.get()[0]/zoom.get() y=move||-pan.get()[1]/zoom.get()
              width=move||frame_bounds.get()[2]/zoom.get() height=move||frame_bounds.get()[3]/zoom.get()/>
          </svg>
          <div class="canvas-zoom" role="group" aria-label="Zoom">
            <button type="button" class="quiet" aria-label="Zoom out" on:click=move|_|zoom_at(((zoom.get_untracked()-0.1)*10.0).round()/10.0,center())
              title="Zoom out" disabled={move||zoom.get()<=0.26}>{icon(ICON_MINUS)}</button>
            <button type="button" class="quiet canvas-zoom-level" aria-label="Zoom to 100%" title="Zoom to 100%"
              on:click=move|_|zoom_at(1.0,center())>{icon(ICON_ACTUAL)}</button>
            <button type="button" class="quiet" aria-label="Zoom in" on:click=move|_|zoom_at(((zoom.get_untracked()+0.1)*10.0).round()/10.0,center())
              title="Zoom in" disabled={move||zoom.get()>=1.99}>{icon(ICON_PLUS)}</button>
            <span class="canvas-zoom-sep" aria-hidden="true"></span>
            <button type="button" class="quiet" aria-label="Fit view" title="Fit view" on:click=move|_|fit_view()>{icon(ICON_FIT)}</button>
          </div>
        <crate::topology_studio::Studio open=studio_open router_id=studio_router session data projection load_saved=Callback::new(move|()|{projection.set(None);reload.run(());})/>
        <crate::app::Theme icons=true/>
        <dialog node_ref=dialog class="topology-dialog canvas-dialog" data-panel=move||match panel.get(){Some(Panel::Inventory)=>"inventory",_=>"settings"} aria-labelledby="canvas-dialog-title"
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
            <Show when=move||selected_node().is_some_and(|n|n.router_id.is_some())>
              <button type="button" class="canvas-tool" aria-label="Configure source router" title="Configure source router" on:click=move|_|{
                if let Some(id)=selected_node().and_then(|n|n.router_id){close();measure();studio_router.set(Some(id));studio_open.set(true);}
              }>{icon(ICON_SETTINGS)}</button>
            </Show>
            <button type="button" class="canvas-tool" aria-label="Configuration information" title="Configuration information" on:click=move|_|{measure();help_open.set(true);if let Some(d)=help_dialog.get_untracked(){let _=d.show_modal();}}>{icon(ICON_HELP)}</button>
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
                <crate::topology_inventory::Inventory session data reload locked action=read_action
                  show=Callback::new(move|card:String|{
                    close();
                    selected.set(Some(card.clone()));
                    // Focus moves the card into view.
                    if let Ok(Some(el))=document().query_selector(&format!("[data-node-id=\"{card}\"]"))
                      && let Ok(el)=el.dyn_into::<web_sys::HtmlElement>() { let _=el.focus(); }
                  })
                  attach=Callback::new(move|id:String|act(Change::AttachSource(id,String::new(),None),under()))
                  send=Callback::new(submit)/>
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
            <button type="button" class="canvas-tool" aria-label="Close" title="Close" on:click=move|_|close()>{icon("M6 6l12 12M18 6 6 18")}</button>
          </footer>
        </dialog>
        <dialog node_ref=help_dialog class="topology-dialog canvas-dialog" data-panel="help" aria-labelledby="studio-help-title" on:close=move|_|help_open.set(false)>
            <header class="add-router-heading"><h2 id="studio-help-title">"Configuration workflow"</h2>
                <button type="button" class="canvas-tool" aria-label="Close" title="Close" on:click=move|_|{if let Some(d)=help_dialog.get_untracked(){d.close();}}>{icon("M6 6l12 12M18 6 6 18")}</button>
            </header>
            <div class="canvas-dialog-body"><Show when=move||help_open.get()><crate::topology_studio_help::StudioHelp selected=if panel.get()==Some(Panel::Inventory){None}else{selected_node()}/></Show></div>
        </dialog>
        <crate::topology_forms::ChangeDialog session data action=adopt reload/>
        </div>
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
