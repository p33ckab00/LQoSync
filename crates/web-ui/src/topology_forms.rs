//! The Builder change dialog: draft, service review, save. The service decides every rule and
//! every impact; the browser only shapes the request and shows the answer.
use crate::topology::{get, node_name};
use crate::topology_model::*;
use gloo_net::http::Request;
use leptos::prelude::*;
use web_contract::Principal;
use web_contract::topology::*;

/// A request key for one user action: time and random bits, 16 to 64 characters.
pub(crate) fn new_key() -> String {
    let r = || (js_sys::Math::random() * 4_503_599_627_370_496.0) as u64;
    format!("{:x}-{:013x}-{:013x}", js_sys::Date::now() as u64, r(), r())
}

/// One state-changing POST with the session CSRF token. `Ok(None)`: done with no body.
pub(crate) async fn post<B: serde::Serialize>(
    session: RwSignal<Option<Principal>>,
    url: &str,
    body: &B,
) -> Result<Option<Operation>, &'static str> {
    let csrf = session.get_untracked().map(|p| p.csrf).unwrap_or_default();
    let (signal, _timer) = crate::app::deadline();
    let request = Request::post(url)
        .abort_signal(Some(&signal))
        .header("x-csrf-token", &csrf)
        .json(body)
        .map_err(|_| "The change could not be prepared.")?;
    match request.send().await {
        Ok(r) if r.status() == 202 => r.json::<Operation>().await.map(Some).map_err(
            |_| "The service answer is not valid. Reload the page to see the saved state.",
        ),
        Ok(r) if r.ok() => Ok(None),
        Ok(r) if r.status() == 401 => {
            session.set(None);
            Err("Your session ended. Sign in again; your draft is kept until you leave the page.")
        }
        Ok(r) if r.status() == 409 => {
            Err("Another change used this request. Review the change again.")
        }
        Ok(r) if r.status() == 410 => {
            Err("The review expired or another editor replaced it. Review the change again.")
        }
        Ok(r) if r.status() == 429 => Err("Too many changes are waiting. Try again in a minute."),
        Ok(r) if r.status() == 400 => Err("The service did not accept this request."),
        _ => Err("The service did not answer. Check Changes before you try again."),
    }
}

/// Read the operation until it leaves `queued` and `running`. The service can wait for the end
/// of a cycle, so this waits up to ten minutes.
pub(crate) async fn settle(
    session: RwSignal<Option<Principal>>,
    key: &str,
) -> Result<Operation, &'static str> {
    for _ in 0..600 {
        match get::<Operation>(session, &format!("/api/v1/topology/operations/{key}")).await {
            Ok(op) if matches!(op.state.as_str(), "queued" | "running") => {}
            Ok(op) => return Ok(op),
            Err(_) => {
                return Err(
                    "The change status could not load. Reload the page: the service keeps the result.",
                );
            }
        }
        gloo_timers::future::TimeoutFuture::new(1000).await;
    }
    Err("The service is still working. Reload the page later to see the result.")
}

#[derive(Clone, PartialEq)]
enum Step {
    Draft,
    Reviewing,
    Review(Box<Review>),
    Saving,
    Saved(String),
}

fn title(action: &Change) -> String {
    match action {
        Change::Initialize => "Start empty topology".into(),
        Change::Create => "Create node".into(),
        Change::CreateChild(_) => "Create child node".into(),
        Change::AttachSource(_, name, _) => format!("Attach source {name}"),
        Change::AddParent(n) => format!("Add parent for {}", n.name),
        Change::Edit(n) => format!("Edit {}", n.name),
        Change::Move(n) => format!("Move {}", n.name),
        Change::Remove(n) => format!("Remove {}", n.name),
        Change::CreateWan(_, router, interface) => format!("Create WAN on {router} {interface}"),
        Change::AddSource(_, name) => format!("Add source {name}"),
        Change::CreatePlan => "Create plan".into(),
        Change::EditPlan(p) => format!("Edit plan {}", p.name),
        Change::DeletePlan(p) => format!("Delete plan {}", p.name),
        Change::SubscriberPlan(id, _) => format!("Change plan of {id}"),
        Change::SubscriberPin(id, _) => format!("Pin {id} to a WAN"),
        Change::Rebalance(_, label) => format!("Rebalance {label}"),
        Change::SpeedDefaults(_) => "Edit speed rules".into(),
        Change::GroupFactor(f) => format!("Edit factor of {}", f.name),
        Change::SetFallback(_) => "Set fallback".into(),
        Change::ClearFallback => "Clear fallback".into(),
    }
}

/// Run one explicit router read and wait for its answer.
pub(crate) async fn run_read(
    session: RwSignal<Option<Principal>>,
    job: ReadJob,
) -> Result<Operation, String> {
    let request = ReadRequest {
        key: new_key(),
        job,
    };
    post(session, "/api/v1/topology/reads", &request)
        .await
        .map_err(str::to_owned)?;
    let op = settle(session, &request.key).await.map_err(str::to_owned)?;
    match op.state.as_str() {
        "done" => Ok(op),
        _ => Err(op
            .error
            .unwrap_or("The read did not finish. Check the router state and try again.".into())),
    }
}

/// Why changes are unavailable, or `None` when the service accepts them.
pub(crate) fn blocked(data: RwSignal<Option<TopologySummary>>) -> Option<&'static str> {
    structural_blocked(data).or_else(|| {
        data.with(|d| {
            d.as_ref()
                .filter(|s| !s.enrolled)
                .map(|_| "Enroll the topology policy before you change subscriber rules.")
        })
    })
}

/// Explicit frontend drafts accept structural edits before enrollment.
pub(crate) fn structural_blocked(data: RwSignal<Option<TopologySummary>>) -> Option<&'static str> {
    data.with(|d| match d {
        None => Some("Loading the topology view."),
        Some(s) if s.registry_revision.is_none() => {
            Some("Start an empty topology to build the hierarchy.")
        }
        Some(s) if !s.enrolled && !(s.frontend_initialized && s.writer == "legacy") => {
            Some("Enroll the existing policy before you change it.")
        }
        Some(s) if s.writer == "unknown" => {
            Some("The shaping writer is unknown. Changes stay off until recovery finishes.")
        }
        Some(_) => None,
    })
}

/// The change dialog. Set `action` to open it; it closes itself and sets `action` to `None`.
#[component]
pub(crate) fn ChangeDialog(
    session: RwSignal<Option<Principal>>,
    data: RwSignal<Option<TopologySummary>>,
    action: RwSignal<Option<Change>>,
    reload: Callback<()>,
) -> impl IntoView {
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let draft = RwSignal::new(Draft::default());
    let step = RwSignal::new(Step::Draft);
    let failure = RwSignal::new(String::new());
    let saved_any = RwSignal::new(false);
    Effect::new(move |_| {
        if let Some(a) = action.get() {
            draft.set(Draft::of(&a));
            step.set(Step::Draft);
            failure.set(String::new());
            if let Some(d) = dialog.get() {
                let _ = d.show_modal();
            }
        }
    });
    let cancel_review = move || {
        if let Step::Review(r) = step.get_untracked() {
            let url = format!("/api/v1/topology/proposals/{}/cancel", r.proposal);
            leptos::task::spawn_local(async move {
                let _ = post(session, &url, &serde_json::json!({})).await;
            });
        }
    };
    let close = move || {
        if let Some(d) = dialog.get_untracked() {
            d.close();
        }
    };
    let review = move || {
        let Some(a) = action.get_untracked() else {
            return;
        };
        let edit = match build_edit(&a, &draft.get_untracked()) {
            Ok(e) => e,
            Err(e) => return failure.set(e.into()),
        };
        let Some(revision) = data.with_untracked(|d| {
            d.as_ref()
                .and_then(|s| s.registry_revision.as_ref())
                .and_then(|r| r.parse::<u64>().ok())
                .or_else(|| matches!(a, Change::Initialize).then_some(0))
        }) else {
            return failure.set("The saved revision is unknown. Reload the view.".into());
        };
        failure.set(String::new());
        step.set(Step::Reviewing);
        let request = ProposalRequest {
            key: new_key(),
            expected_revision: revision,
            edit,
        };
        leptos::task::spawn_local(async move {
            let result = match post(session, "/api/v1/topology/proposals", &request).await {
                Ok(_) => settle(session, &request.key).await,
                Err(e) => Err(e),
            };
            match result {
                Ok(op) if op.state == "review_ready" && op.review.is_some() => {
                    step.set(Step::Review(Box::new(op.review.unwrap_or_default())));
                }
                Ok(op) => {
                    failure.set(op.error.unwrap_or(
                        "The service could not review this change. Check the values and try again."
                            .into(),
                    ));
                    step.set(Step::Draft);
                }
                Err(e) => {
                    failure.set(e.into());
                    step.set(Step::Draft);
                }
            }
        });
    };
    let save = move || {
        let Step::Review(r) = step.get_untracked() else {
            return;
        };
        step.set(Step::Saving);
        let body = ConfirmRequest { key: new_key() };
        let url = format!("/api/v1/topology/proposals/{}/confirm", r.proposal);
        leptos::task::spawn_local(async move {
            let result = match post(session, &url, &body).await {
                Ok(_) => settle(session, &body.key).await,
                Err(e) => Err(e),
            };
            match result {
                Ok(op) if op.state == "saved" => {
                    let revision = op
                        .receipt
                        .and_then(|r| r.revision)
                        .unwrap_or_else(|| "—".into());
                    saved_any.set(true);
                    step.set(Step::Saved(revision));
                }
                Ok(op) => {
                    failure.set(match op.state.as_str() {
                        "outcome_unknown" => {
                            "The result is not known yet. Reload the view before you try again."
                                .to_string()
                        }
                        _ => op
                            .receipt
                            .and_then(|r| r.error)
                            .or(op.error)
                            .map(|e| format!("Not saved: {e}. Review the change again."))
                            .unwrap_or("Not saved. Review the change again.".into()),
                    });
                    step.set(Step::Draft);
                }
                Err(e) => {
                    failure.set(e.into());
                    step.set(Step::Draft);
                }
            }
        });
    };
    let busy = move || matches!(step.get(), Step::Reviewing | Step::Saving);
    let parents = move |exclude: Option<String>| {
        data.with(|d| {
            d.as_ref()
                .map(|s| {
                    tree_rows(&s.nodes)
                        .into_iter()
                        .filter(|(_, n)| Some(&n.id) != exclude.as_ref())
                        .map(|(depth, n)| {
                            (n.id.clone(), format!("{}{}", "– ".repeat(depth), n.name))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    };
    view! {
      <dialog node_ref=dialog class="topology-dialog" aria-labelledby="change-title" aria-describedby="change-help"
        on:cancel=move|ev: web_sys::Event|{if busy(){ev.prevent_default();}}
        on:close=move|_|{
          cancel_review();
          action.set(None);
          if saved_any.get_untracked() { saved_any.set(false); reload.run(()); }
        }>
        <header class="add-router-heading">
          <div>
            <h2 id="change-title">{move||action.get().map(|a|title(&a)).unwrap_or_default()}</h2>
            <p id="change-help">"The service reviews the change first and shows its impact. Nothing changes until you save."</p>
          </div>
          <button type="button" class="router-icon quiet" aria-label="Close" title="Close" disabled=busy on:click=move|_|close()>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18"/></svg>
          </button>
        </header>
        <form class="router-form" on:submit=move|ev|{
          ev.prevent_default();
          match step.get_untracked() { Step::Draft=>review(), Step::Review(_)=>save(), _=>{} }
        }>
          {move||match (action.get(), step.get()) {
            (None,_)=>().into_any(),
            (Some(_),Step::Saved(revision))=>view!{
              <p class="saved-notice" role="status">{if data.with(|d|d.as_ref().is_some_and(|s|s.enrolled && s.writer=="topology")){format!("Saved as policy revision {revision}. The service publishes it in its next cycle; Changes shows the publication.")}else{format!("Saved draft revision {revision}. Continue building here. Enrollment and writer activation are required before publication.")}}</p>
            }.into_any(),
            (Some(_),Step::Review(r))=>review_view(&r, data).into_any(),
            (Some(a),_)=>{
              let locked=busy();
              // Cache age updates must not replace focused draft inputs.
              let controls=untrack(||fields(&a, draft, parents, data));
              view!{<fieldset disabled=locked>{controls}</fieldset>}.into_any()
            }
          }}
          <p class="sr-only" role="status">{move||match step.get() {Step::Reviewing=>"Reviewing the change…",Step::Saving=>"Saving the change…",_=>""}}</p>
          <Show when=move||!failure.get().is_empty()><p class="form-failure" role="alert">{move||failure.get()}</p></Show>
          <div class="form-actions">
            {move||match step.get() {
              Step::Saved(_)=>view!{<button type="button" class="primary" on:click=move|_|close()>"Close"</button>}.into_any(),
              Step::Review(_)|Step::Saving=>view!{
                <button type="button" class="quiet" disabled=busy on:click=move|_|{cancel_review();step.set(Step::Draft);}>"Back to the change"</button>
                <button type="submit" class="primary" disabled=busy>{move||if matches!(step.get(),Step::Saving){"Saving…"}else{"Save change"}}</button>
              }.into_any(),
              _=>view!{
                <button type="button" class="quiet" disabled=busy on:click=move|_|close()>"Cancel"</button>
                <button type="submit" class="primary" disabled=busy>{move||if matches!(step.get(),Step::Reviewing){"Reviewing…"}else{"Review change"}}</button>
              }.into_any(),
            }}
          </div>
        </form>
      </dialog>
    }
}

fn fields(
    action: &Change,
    draft: RwSignal<Draft>,
    parents: impl Fn(Option<String>) -> Vec<(String, String)> + Copy + Send + Sync + 'static,
    data: RwSignal<Option<TopologySummary>>,
) -> impl IntoView {
    let text = move |f: fn(&Draft) -> String| move || draft.with(f);
    let set =
        move |f: fn(&mut Draft, String)| move |ev| draft.update(|d| f(d, event_target_value(&ev)));
    let choice = move |label: &'static str, none: &'static str, options: Vec<(String, String)>| {
        view! {
          <div class="field"><label for="change-choice">{label}</label>
            <select id="change-choice" on:change=set(|d,v|d.parent=v)>
              <option value="" selected=move||draft.with(|d|d.parent.is_empty())>{none}</option>
              {options.into_iter().map(|(id,label)|view!{<option value=id.clone() selected=move||draft.with(|d|d.parent==id)>{label}</option>}).collect_view()}
            </select>
          </div>
        }
    };
    let rates = move || {
        view! {
          <div class="field"><label for="change-name">"Name"</label>
            <input id="change-name" type="text" maxlength="64" required autofocus autocomplete="off" prop:value=text(|d|d.name.clone()) on:input=set(|d,v|d.name=v)/></div>
          <div class="field-row">
            <div class="field"><label for="change-down">"Download, Mbps"</label>
              <input id="change-down" type="number" inputmode="numeric" min="1" step="1" required prop:value=text(|d|d.down.clone()) on:input=set(|d,v|d.down=v)/></div>
            <div class="field"><label for="change-up">"Upload, Mbps"</label>
              <input id="change-up" type="number" inputmode="numeric" min="1" step="1" required prop:value=text(|d|d.up.clone()) on:input=set(|d,v|d.up=v)/></div>
          </div>
        }
    };
    let parent_select = move |exclude: Option<String>| {
        view! {
          <div class="field"><label for="change-parent">"Parent"</label>
            <select id="change-parent" on:change=set(|d,v|d.parent=v) prop:value=text(|d|d.parent.clone())>
              <option value="">"None: a root node"</option>
              {parents(exclude).into_iter().map(|(id,label)|view!{<option value=id.clone() selected=move||draft.with(|d|d.parent==id)>{label}</option>}).collect_view()}
            </select>
          </div>
        }
    };
    let capacity = move || {
        view! {
          <div class="field"><label for="change-mode">"Capacity"</label>
            <select id="change-mode" on:change=set(|d,v|d.mode=v) prop:value=text(|d|d.mode.clone())>
              <option value="automatic" selected=move||draft.with(|d|d.mode=="automatic")>"Automatic: sum of the plans below"</option>
              <option value="inherit" selected=move||draft.with(|d|d.mode=="inherit")>"Inherit from the parent"</option>
              <option value="fixed" selected=move||draft.with(|d|d.mode=="fixed")>"Fixed Mbps"</option>
              <option value="wan_percent" selected=move||draft.with(|d|d.mode=="wan_percent")>"Share of the WAN, in percent"</option>
            </select>
          </div>
          <Show when=move||draft.with(|d|d.mode=="fixed"||d.mode=="wan_percent")>
            <div class="field-row">
              <div class="field"><label for="change-down">{move||if draft.with(|d|d.mode=="fixed"){"Download, Mbps"}else{"Download, percent"}}</label>
                <input id="change-down" type="number" inputmode="numeric" min="1" max=move||draft.with(|d|if d.mode=="wan_percent"{"100"}else{""}) step="1" required prop:value=text(|d|d.down.clone()) on:input=set(|d,v|d.down=v)/></div>
              <div class="field"><label for="change-up">{move||if draft.with(|d|d.mode=="fixed"){"Upload, Mbps"}else{"Upload, percent"}}</label>
                <input id="change-up" type="number" inputmode="numeric" min="1" max=move||draft.with(|d|if d.mode=="wan_percent"{"100"}else{""}) step="1" required prop:value=text(|d|d.up.clone()) on:input=set(|d,v|d.up=v)/></div>
            </div>
          </Show>
        }
    };
    match action.clone() {
        Change::Initialize => view! {<p class="panel-note">"Create an empty topology draft. Configured routers remain available for discovery. No nodes, sources, subscriber assignments, or shaping files are imported. Review before saving."</p>}.into_any(),
        Change::Create | Change::CreateChild(_) => view! {
          <div class="field"><label for="change-name">"Name"</label>
            <input id="change-name" type="text" maxlength="64" required autofocus autocomplete="off" prop:value=text(|d|d.name.clone()) on:input=set(|d,v|d.name=v)/></div>
          {parent_select(None)}
          <div class="field"><label for="change-kind">"Node type"</label>
            <select id="change-kind" on:change=set(|d,v|d.kind=v)>
              <option value="intermediate">"Intermediate: holds other nodes"</option>
              <option value="group">"Subscriber group without a source"</option>
            </select></div>
          {capacity()}
        }
        .into_any(),
        Change::AttachSource(_, name, _) => view! {
          <p class="panel-note">{format!("Add an instance of {name}. Choose its parent in the hierarchy. The source identity stays shared.")}</p>
          {parent_select(None)}
        }.into_any(),
        Change::AddParent(n) => {
            let options=data.with_untracked(|d|d.as_ref().map(|s|additional_parents(&s.nodes,&n)).unwrap_or_default());
            view!{
              <p class="panel-note">"Add a placement of this shared source beneath another parent. The current placement stays. Each subscriber keeps one effective placement."</p>
              {choice("Additional parent","Select a parent…",options)}
            }.into_any()
        },
        Change::Edit(_) => view! {
          <div class="field"><label for="change-name">"Name"</label>
            <input id="change-name" type="text" maxlength="64" required autofocus autocomplete="off" prop:value=text(|d|d.name.clone()) on:input=set(|d,v|d.name=v)/></div>
          {capacity()}
        }
        .into_any(),
        Change::Move(n) => view! {
          <p>"Moving a node moves its whole branch. The review shows the subscribers that change place."</p>
          {parent_select(Some(n.id.clone()))}
        }
        .into_any(),
        Change::Remove(n) => view! {
          <p>{format!("Remove {} from the topology. The service refuses this when the node still has children or sources.", n.name)}</p>
        }
        .into_any(),
        Change::CreateWan(..) => view! {
          <div class="field"><label for="change-name">"Name"</label>
            <input id="change-name" type="text" maxlength="64" required autofocus autocomplete="off" prop:value=text(|d|d.name.clone()) on:input=set(|d,v|d.name=v)/></div>
          <p class="field-help">"The WAN capacity is the contracted rate of this uplink."</p>
          <div class="field-row">
            <div class="field"><label for="change-down">"Download, Mbps"</label>
              <input id="change-down" type="number" inputmode="numeric" min="1" step="1" required prop:value=text(|d|d.down.clone()) on:input=set(|d,v|d.down=v)/></div>
            <div class="field"><label for="change-up">"Upload, Mbps"</label>
              <input id="change-up" type="number" inputmode="numeric" min="1" step="1" required prop:value=text(|d|d.up.clone()) on:input=set(|d,v|d.up=v)/></div>
          </div>
        }
        .into_any(),
        Change::AddSource(..) => view! {
          <p>"The service reads the routers again for this source. The review shows the subscribers that it places."</p>
          {parent_select(None)}
        }
        .into_any(),
        Change::CreatePlan => view! {
          <p>"One plan serves PPPoE and DHCP subscribers alike."</p>
          {rates()}
        }
        .into_any(),
        Change::EditPlan(p) => view! {
          <p>{format!("{} subscribers use this plan, over all sources. The review lists their new rates.", p.subscribers)}</p>
          {rates()}
        }
        .into_any(),
        Change::DeletePlan(p) => view! {
          <p>{format!("Delete {}. The service refuses this while a subscriber uses the plan.", p.name)}</p>
        }
        .into_any(),
        Change::SubscriberPlan(..) => {
          let plans = data.with(|d| d.as_ref().map(|s| s.plans.iter().map(|p| (p.id.clone(), format!("{} · {} / {} Mbps", p.name, p.down_mbps, p.up_mbps))).collect()).unwrap_or_default());
          view! {
            <p>"The plan applies to the subscriber record, so every verified identity of it gets the same plan."</p>
            {choice("Local plan", "No local plan: use the source rate", plans)}
          }
          .into_any()
        }
        Change::SubscriberPin(..) => {
          let wans = data.with(|d| d.as_ref().map(|s| s.nodes.iter().filter(|n| n.kind == "wan").map(|n| (n.id.clone(), n.name.clone())).collect()).unwrap_or_default());
          view! {
            <p>"A topology pin places the subscriber below one WAN. It writes no router rule. Pinning needs the subscriber online; unpinning does not."</p>
            {choice("WAN", "No pin", wans)}
          }
          .into_any()
        }
        Change::Rebalance(_, label) => view! {
          <p>{format!("Place the subscribers of {label} again across their nodes. The service reads the routers for the review.")}</p>
        }
        .into_any(),
        Change::SpeedDefaults(sd) => view! {
          <p>{format!("These rules apply to every subscriber. Saving resets the own rules of {} profiles to them and replaces imported legacy rules. The review lists the subscribers whose rates change.", sd.overrides)}</p>
          {RULES.iter().enumerate().map(|(i,label)|{
            let id=format!("rule-{i}");
            view!{<div class="field-row">
              <div class="field"><label for=id.clone()>{*label}</label>
                <input id=id type="text" inputmode="decimal" required autocomplete="off"
                  prop:value=move||draft.with(|d|d.rules[i].1.clone()) on:input=move|ev|draft.update(|d|d.rules[i].1=event_target_value(&ev))/></div>
              <div class="field"><label for=format!("rule-unit-{i}")>{format!("{label} unit")}</label>
                <select id=format!("rule-unit-{i}") on:change=move|ev|draft.update(|d|d.rules[i].0=event_target_value(&ev))>
                  <option value="percent" selected=move||draft.with(|d|d.rules[i].0!="mbps")>"Percent of the plan"</option>
                  <option value="mbps" selected=move||draft.with(|d|d.rules[i].0=="mbps")>"Fixed Mbps"</option>
                </select></div>
            </div>}
          }).collect_view()}
          <p class="field-help">"Minimums: 0.1 to 100 percent. Maximums: 1 to 1000 percent. One decimal at most."</p>
        }
        .into_any(),
        Change::GroupFactor(f) => view! {
          <p>{format!("The factor multiplies the plan rates of {} when the service sizes its group. Node factor overrides stay.", f.name)}</p>
          <div class="field-row">
            <div class="field"><label for="change-down">"Download factor"</label>
              <input id="change-down" type="text" inputmode="decimal" required autocomplete="off" prop:value=text(|d|d.down.clone()) on:input=set(|d,v|d.down=v)/></div>
            <div class="field"><label for="change-up">"Upload factor"</label>
              <input id="change-up" type="text" inputmode="decimal" required autocomplete="off" prop:value=text(|d|d.up.clone()) on:input=set(|d,v|d.up=v)/></div>
          </div>
        }
        .into_any(),
        Change::SetFallback(_) => {
          let groups: Vec<(String, String)> = data.with(|d| d.as_ref().map(|s| s.nodes.iter().filter(|n| n.kind == "group").map(|n| (n.id.clone(), n.name.clone())).collect()).unwrap_or_default());
          view! {
            <p>"Subscribers that no source knows go to these groups at the default rate."</p>
            <fieldset class="field"><legend>"Fallback groups"</legend>
              {groups.into_iter().map(|(id,label)|{
                let (check,key)=(id.clone(),id.clone());
                view!{<label class="check"><input type="checkbox" prop:checked=move||draft.with(|d|d.nodes.contains(&check))
                  on:change=move|ev|{let on=event_target_checked(&ev);let key=key.clone();draft.update(|d|{d.nodes.retain(|n|*n!=key);if on{d.nodes.push(key);}})}/>{label}</label>}
              }).collect_view()}
            </fieldset>
            <div class="field-row">
              <div class="field"><label for="change-down">"Default download, Mbps"</label>
                <input id="change-down" type="number" inputmode="numeric" min="1" step="1" required prop:value=text(|d|d.down.clone()) on:input=set(|d,v|d.down=v)/></div>
              <div class="field"><label for="change-up">"Default upload, Mbps"</label>
                <input id="change-up" type="number" inputmode="numeric" min="1" step="1" required prop:value=text(|d|d.up.clone()) on:input=set(|d,v|d.up=v)/></div>
            </div>
          }
          .into_any()
        }
        Change::ClearFallback => view! {
          <p>"Clear the fallback. The service refuses this while subscribers use a fallback group."</p>
        }
        .into_any(),
    }
}

fn review_view(r: &Review, data: RwSignal<Option<TopologySummary>>) -> impl IntoView {
    let expires = crate::dashboard::local_time(r.expires_at);
    let place = move |id: &Option<String>| {
        id.as_ref()
            .map(|id| node_name(data, id))
            .unwrap_or("no placement".into())
    };
    let more = r.moves.saturating_sub(r.moved.len() as u64);
    view! {
      <section aria-labelledby="impact-title">
        <h3 id="impact-title">"Impact"</h3>
        <dl class="router-facts">
          <div><dt>"Subscribers that move"</dt><dd>{r.moves}</dd></div>
          <div><dt>"Offline placements cleared"</dt><dd>{r.cleared}</dd></div>
          <div><dt>"Newly placed subscribers"</dt><dd>{r.assigned}</dd></div>
          <div><dt>"Based on revision"</dt><dd>{r.base_revision.clone()}</dd></div>
        </dl>
        {(!r.nodes.is_empty()).then(||view!{
          <h3>"Node changes"</h3>
          <ul class="plain-list">{r.nodes.iter().map(|n|view!{<li><span class="chip neutral">{n.change.clone()}</span><strong>{n.name.clone()}</strong></li>}).collect_view()}</ul>
        })}
        {(!r.moved.is_empty()).then(||view!{
          <h3>"Moves"</h3>
          <ul class="plain-list">{r.moved.iter().map(|m|view!{<li><strong>{m.label.clone().unwrap_or(m.subscriber_id.clone())}</strong><small>{format!("{} to {}",place(&m.from),place(&m.to))}</small></li>}).collect_view()}</ul>
          {(more>0).then(||view!{<p class="field-help">{format!("{more} more moves are not listed.")}</p>})}
        })}
        {(!r.warnings.is_empty()).then(||view!{
          <div class="notice" role="note"><ul>{r.warnings.iter().map(|w|view!{<li>{w.clone()}</li>}).collect_view()}</ul></div>
        })}
        {(r.rates_changed>0).then(||view!{
          <h3>{format!("Rate changes: {} subscribers",r.rates_changed)}</h3>
          <ul class="plain-list">{r.rates.iter().map(|x|view!{<li><strong>{x.label.clone().unwrap_or(x.subscriber_id.clone())}</strong><small>{format!("{} to {}",x.before,x.after)}</small></li>}).collect_view()}</ul>
          {(r.rates_changed>r.rates.len() as u64).then(||view!{<p class="field-help">{format!("{} more rate changes are not listed.",r.rates_changed-r.rates.len() as u64)}</p>})}
        })}
        {(!r.capacities.is_empty()).then(||view!{
          <h3>"Capacity changes"</h3>
          <ul class="plain-list">{r.capacities.iter().map(|c|view!{<li><strong>{node_name(data,&c.node_id)}</strong><small>{format!("{} to {}",c.before,c.after)}</small></li>}).collect_view()}</ul>
        })}
        {(!r.sources.is_empty()).then(||view!{<p class="field-help">"The review read the current subscriber sources from their routers."</p>})}
        <p class="field-help">{format!("Save before {expires}. After that time, review the change again.")}</p>
      </section>
    }
}
