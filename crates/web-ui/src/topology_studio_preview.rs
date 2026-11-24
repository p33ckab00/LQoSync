//! Read-only discovery and candidate inspection. The saved canvas remains a separate projection.
use crate::topology_studio_api as api;
use crate::topology_studio_model::*;
use leptos::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use web_contract::{
    Principal,
    source_studio::*,
    topology::{Operation, TopologySummary},
};
#[derive(Clone)]
struct PendingRead {
    request: StudioJobRequest,
    stamp: DraftStamp,
}
#[derive(Clone, Copy)]
pub(crate) struct Preview {
    session: RwSignal<Option<Principal>>,
    draft: RwSignal<Option<StudioDraft>>,
    caps: RwSignal<Option<StudioCapabilities>>,
    config: RwSignal<Option<RouterSourceConfig>>,
    data: RwSignal<Option<TopologySummary>>,
    pub snapshot: RwSignal<Option<SnapshotRef>>,
    candidate: RwSignal<Option<BoundCandidate>>,
    pending: RwSignal<Option<PendingRead>>,
    pub busy: RwSignal<bool>,
    pub message: RwSignal<String>,
    pub clock: RwSignal<u64>,
    issues: RwSignal<DetailPage<ValidationIssue>>,
    records: RwSignal<DetailPage<ResolvedRecord>>,
    page_busy: RwSignal<bool>,
    page_generation: StoredValue<u64>,
    page_error: RwSignal<String>,
    records_tab: RwSignal<bool>,
    details: NodeRef<leptos::html::Dialog>,
    show: RwSignal<bool>,
    open: RwSignal<bool>,
    on_edit: Callback<()>,
    on_saved: Callback<()>,
    alive: StoredValue<Arc<AtomicBool>>,
}
pub(crate) struct Inputs {
    pub session: RwSignal<Option<Principal>>,
    pub draft: RwSignal<Option<StudioDraft>>,
    pub caps: RwSignal<Option<StudioCapabilities>>,
    pub config: RwSignal<Option<RouterSourceConfig>>,
    pub data: RwSignal<Option<TopologySummary>>,
    pub open: RwSignal<bool>,
    pub projection: RwSignal<Option<CandidateSummary>>,
    pub on_edit: Callback<()>,
    pub on_saved: Callback<()>,
}
pub(crate) fn controller(input: Inputs) -> Preview {
    let Inputs {
        session,
        draft,
        caps,
        config,
        data,
        open,
        projection,
        on_edit,
        on_saved,
    } = input;
    let state = Preview {
        session,
        draft,
        caps,
        config,
        data,
        open,
        on_edit,
        on_saved,
        snapshot: RwSignal::new(None),
        candidate: RwSignal::new(None),
        pending: RwSignal::new(None),
        busy: RwSignal::new(false),
        message: RwSignal::new(String::new()),
        clock: RwSignal::new(crate::topology::now()),
        issues: RwSignal::new(DetailPage::default()),
        records: RwSignal::new(DetailPage::default()),
        page_busy: RwSignal::new(false),
        page_generation: StoredValue::new(0),
        page_error: RwSignal::new(String::new()),
        records_tab: RwSignal::new(false),
        details: NodeRef::new(),
        show: RwSignal::new(false),
        alive: StoredValue::new(Arc::new(AtomicBool::new(true))),
    };
    let alive = state.alive.get_value();
    on_cleanup(move || alive.store(false, Ordering::Relaxed));
    let timer = set_interval_with_handle(
        move || state.clock.set(crate::topology::now()),
        std::time::Duration::from_secs(1),
    )
    .expect("preview clock");
    on_cleanup(move || timer.clear());
    Effect::new(move |_| {
        if let Some(c) = config.get() {
            let old = state.snapshot.get_untracked();
            if old
                .as_ref()
                .is_none_or(|s| s.router_id != c.router_id || s.config_digest != c.config_digest)
            {
                state
                    .snapshot
                    .set(c.snapshots.into_iter().max_by_key(|s| s.collected_to));
            }
        }
    });
    Effect::new(move |_| {
        let next = if state.show.get() && state.current() {
            state.candidate.get().map(|c| c.summary)
        } else {
            None
        };
        if projection.with_untracked(|old| *old != next) {
            projection.set(next);
        }
    });
    state
}
impl Preview {
    fn actor(self) -> String {
        self.session.get().map(|p| p.username).unwrap_or_default()
    }
    fn matches(self, stamp: &DraftStamp) -> bool {
        self.draft.with(|d| {
            d.as_ref()
                .is_some_and(|d| stamp.matches(&self.actor(), d, &stamp.snapshot_id))
        })
    }
    fn current(self) -> bool {
        self.candidate.with(|c| {
            c.as_ref().is_some_and(|c| {
                if self.config.with(|cfg| {
                    cfg.as_ref().is_none_or(|cfg| {
                        cfg.config_digest != c.stamp.config_digest
                            || cfg.loaded_config_digest != c.stamp.config_digest
                    })
                }) {
                    return false;
                }
                if self.snapshot.with(|s| {
                    s.as_ref()
                        .is_none_or(|s| s.id != c.summary.reference.snapshot_id)
                }) {
                    return false;
                }
                if self.data.with(|d| {
                    d.as_ref()
                        .and_then(|d| d.registry_revision.as_deref())
                        .map(str::to_string)
                }) != c.stamp.revision
                {
                    return false;
                }
                self.draft.with(|d| {
                    d.as_ref()
                        .is_some_and(|d| c.valid(&self.actor(), d, self.clock.get()))
                })
            })
        })
    }
    pub fn reason(self, evaluate: bool) -> Option<String> {
        if self.busy.get() {
            return Some("A read job is pending.".into());
        }
        let Some(caps) = self.caps.get() else {
            return Some("Studio discovery is unavailable on this backend.".into());
        };
        let feature = if evaluate {
            StudioFeature::PolicyPreview
        } else {
            StudioFeature::SourceCollect
        };
        if !caps.features.contains(&feature) {
            return Some("This backend does not support this discovery operation.".into());
        }
        let Some(c) = self.config.get() else {
            return Some("Load the router configuration first.".into());
        };
        if c.config_digest != c.loaded_config_digest {
            return Some("Wait for the service to load the saved access configuration.".into());
        }
        if !evaluate && !c.enabled {
            return Some("Enable the registered router before discovery. Disabled routers make no connections.".into());
        }
        let Some(d) = self.draft.get() else {
            return Some("Select a router draft.".into());
        };
        if evaluate {
            if d.policy_revision.is_none()
                || self
                    .data
                    .with(|d| d.as_ref().is_none_or(|d| d.registry_revision.is_none()))
            {
                return Some("Initialize the topology before evaluation. Connection editing and collection remain available.".into());
            }
            let Some(s) = self.snapshot.get() else {
                return Some("Refresh discovery or select a cached snapshot first.".into());
            };
            if s.router_id != d.router_id || s.config_digest != d.config_digest {
                return Some("The snapshot does not match this router and configuration.".into());
            }
            if s.expires_at <= self.clock.get() {
                return Some("The snapshot expired. Refresh discovery explicitly.".into());
            }
            if let Err(e) = evaluation_policies(&d) {
                return Some(e);
            }
        } else if d.policies.is_empty() {
            return Some("Select at least one source type in the draft.".into());
        }
        None
    }
    pub fn collect(self) {
        if let Some(why) = self.reason(false) {
            self.message.set(why);
            return;
        }
        let Some(d) = self.draft.get_untracked() else {
            return;
        };
        let mut sources = d
            .policies
            .iter()
            .map(|p| p.policy.source)
            .collect::<Vec<_>>();
        sources.sort();
        sources.dedup();
        let Some(stamp) = DraftStamp::new(&self.actor(), &d, "") else {
            return;
        };
        self.execute(
            PendingRead {
                stamp,
                request: StudioJobRequest {
                    key: crate::topology_forms::new_key(),
                    job: StudioJob::Collect {
                        router_id: d.router_id,
                        expected_config_digest: d.config_digest,
                        sources,
                    },
                },
            },
            false,
        );
    }
    pub fn evaluate(self) {
        if let Some(why) = self.reason(true) {
            self.message.set(why);
            return;
        }
        let (Some(d), Some(snapshot)) = (self.draft.get_untracked(), self.snapshot.get_untracked())
        else {
            return;
        };
        let Some(stamp) = DraftStamp::new(&self.actor(), &d, &snapshot.id) else {
            return;
        };
        let Ok(policies) = evaluation_policies(&d) else {
            return;
        };
        let request = StudioJobRequest {
            key: crate::topology_forms::new_key(),
            job: StudioJob::Evaluate {
                router_id: d.router_id,
                expected_revision: d.policy_revision.unwrap(),
                expected_config_digest: d.config_digest,
                snapshot_id: snapshot.id,
                policies,
            },
        };
        self.open.set(false);
        self.execute(PendingRead { request, stamp }, false);
    }
    fn recover(self) {
        if self.busy.get() {
            return;
        }
        if let Some(p) = self.pending.get_untracked() {
            if self.matches(&p.stamp) {
                self.execute(p, true);
            } else {
                self.message
                    .set("The draft changed. Start a new read for its current inputs.".into());
            }
        }
    }
    fn execute(self, pending: PendingRead, resume: bool) {
        self.busy.set(true);
        self.message.set(String::new());
        self.show.set(false);
        self.pending.set(Some(pending.clone()));
        let alive = self.alive.get_value();
        let seconds = self.caps.with_untracked(|c| {
            c.as_ref()
                .map(|c| c.limits.job_deadline_secs)
                .unwrap_or(JOB_DEADLINE_SECS)
        });
        leptos::task::spawn_local(async move {
            let result = api::job(self.session, &pending.request, seconds, resume).await;
            if !alive.load(Ordering::Relaxed) {
                return;
            }
            if self.pending.with_untracked(|p| {
                p.as_ref()
                    .is_none_or(|p| p.request.key != pending.request.key)
            }) {
                return;
            }
            self.busy.set(false);
            if !self.matches(&pending.stamp) {
                self.message
                    .set("The draft changed. The old read result was ignored.".into());
                return;
            }
            match result {
                Ok(Operation {
                    source_result: Some(StudioJobResult::Snapshot(snapshot)),
                    ..
                }) if matches!(pending.request.job, StudioJob::Collect { .. }) => {
                    if snapshot.router_id == pending.stamp.router_id
                        && snapshot.config_digest == pending.stamp.config_digest
                    {
                        self.snapshot.set(Some(snapshot));
                        self.candidate.set(None);
                        self.pending.set(None);
                        self.message.set("Discovery snapshot loaded. A partial read is not empty complete evidence.".into());
                    } else {
                        self.message
                            .set("The snapshot does not match this request.".into());
                    }
                }
                Ok(Operation {
                    source_result: Some(StudioJobResult::Candidate(reference)),
                    ..
                }) if matches!(pending.request.job, StudioJob::Evaluate { .. }) => {
                    if !pending.stamp.accepts_reference(&reference) {
                        self.message
                            .set("The candidate does not match the evaluation inputs.".into());
                        return;
                    }
                    self.busy.set(true);
                    let result = api::get::<CandidateSummary>(
                        self.session,
                        &format!(
                            "/api/v1/topology/source-candidates/{}",
                            api::encoded(&reference.id)
                        ),
                    )
                    .await;
                    if !alive.load(Ordering::Relaxed) {
                        return;
                    }
                    if self.pending.with_untracked(|p| {
                        p.as_ref()
                            .is_none_or(|p| p.request.key != pending.request.key)
                    }) {
                        return;
                    }
                    self.busy.set(false);
                    if !self.matches(&pending.stamp) {
                        self.message
                            .set("The draft changed. The old preview was ignored.".into());
                        return;
                    }
                    match result {
                        Ok(summary) if summary.reference == reference => {
                            self.candidate.set(Some(BoundCandidate {
                                stamp: pending.stamp,
                                summary,
                                unavailable: false,
                            }));
                            self.page_generation
                                .update_value(|n| *n = n.wrapping_add(1));
                            self.page_busy.set(false);
                            self.issues.set(DetailPage::default());
                            self.records.set(DetailPage::default());
                            self.page_error.set(String::new());
                            self.pending.set(None);
                            self.show.set(true);
                            self.load_page(false, false);
                        }
                        Ok(_) => self.message.set(
                            "The candidate answer does not match its operation reference.".into(),
                        ),
                        Err(e) => self.fail(e),
                    }
                }
                Ok(_) => self
                    .message
                    .set("The read result has an unexpected type.".into()),
                Err(e) => self.fail(e),
            }
        });
    }
    fn fail(self, error: api::ApiError) {
        if matches!(error.status, Some(410 | 404)) {
            self.candidate.update(|c| {
                if let Some(c) = c {
                    c.unavailable = true;
                }
            });
            self.show.set(false);
        }
        self.message.set(error.message);
    }
    fn load_page(self, records: bool, more: bool) {
        if self.page_busy.get() || !self.current() {
            return;
        }
        let Some(bound) = self.candidate.get_untracked() else {
            return;
        };
        let cursor = if more {
            if records {
                self.records.with_untracked(|p| p.next_cursor.clone())
            } else {
                self.issues.with_untracked(|p| p.next_cursor.clone())
            }
        } else {
            None
        };
        if more && cursor.is_none() {
            return;
        }
        let limit = self.caps.with_untracked(|c| {
            c.as_ref()
                .map(|c| {
                    c.limits
                        .page_default
                        .min(c.limits.page_max)
                        .clamp(1, PAGE_MAX)
                })
                .unwrap_or(PAGE_DEFAULT)
        });
        let url = format!(
            "/api/v1/topology/source-candidates/{}/{}?limit={limit}{}",
            api::encoded(&bound.summary.reference.id),
            if records { "records" } else { "issues" },
            cursor
                .as_ref()
                .map(|c| format!("&cursor={}", api::encoded(c)))
                .unwrap_or_default()
        );
        self.page_busy.set(true);
        self.page_generation
            .update_value(|n| *n = n.wrapping_add(1));
        let page_generation = self.page_generation.get_value();
        self.page_error.set(String::new());
        let alive = self.alive.get_value();
        leptos::task::spawn_local(async move {
            enum Answer {
                Records(CandidatePage<ResolvedRecord>),
                Issues(CandidatePage<ValidationIssue>),
            }
            let result = if records {
                api::get::<CandidatePage<ResolvedRecord>>(self.session, &url)
                    .await
                    .map(Answer::Records)
            } else {
                api::get::<CandidatePage<ValidationIssue>>(self.session, &url)
                    .await
                    .map(Answer::Issues)
            };
            if !alive.load(Ordering::Relaxed) || self.page_generation.get_value() != page_generation
            {
                return;
            }
            self.page_busy.set(false);
            if !self.current()
                || self.candidate.with_untracked(|c| {
                    c.as_ref()
                        .is_none_or(|c| c.summary.reference != bound.summary.reference)
                })
            {
                return;
            }
            match result {
                Ok(Answer::Records(page)) => self.records.update(|state| {
                    if let Err(e) =
                        state.append(&bound.summary.reference.id, cursor.as_deref(), page)
                    {
                        self.page_error.set(e.into());
                    }
                }),
                Ok(Answer::Issues(page)) => self.issues.update(|state| {
                    if let Err(e) =
                        state.append(&bound.summary.reference.id, cursor.as_deref(), page)
                    {
                        self.page_error.set(e.into());
                    }
                }),
                Err(e) => {
                    self.page_error.set(e.message.clone());
                    if matches!(e.status, Some(410 | 404)) {
                        self.fail(e);
                    }
                }
            }
            if self.page_error.get_untracked().is_empty()
                && self.records_tab.get_untracked() != records
            {
                let selected = self.records_tab.get_untracked();
                let missing = if selected {
                    self.records.with_untracked(|p| p.total.is_none())
                } else {
                    self.issues.with_untracked(|p| p.total.is_none())
                };
                if missing {
                    self.load_page(selected, false);
                }
            }
        });
    }

    fn details(self) {
        if let Some(d) = self.details.get_untracked() {
            let _ = d.show_modal();
        }
        if self.records_tab.get_untracked() && self.records.with_untracked(|p| p.total.is_none()) {
            self.load_page(true, false);
        }
    }
    fn status(self) -> String {
        if self.busy.get() {
            return "Reading source evidence…".into();
        }
        if !self.message.get().is_empty() {
            return self.message.get();
        }
        if let Some(c) = self.candidate.get() {
            if c.summary.reference.expires_at <= self.clock.get() || c.unavailable {
                return "Preview expired or unavailable. Evaluate again; your draft stays.".into();
            }
            if !self.current() {
                return "The preview is outdated. Evaluate the current draft again.".into();
            }
            if !self.show.get() {
                return "Saved topology is visible. The candidate is kept for inspection.".into();
            }
            return if !c.summary.complete {
                "Incomplete evaluation"
            } else if !c.summary.publishable || c.summary.issues.blockers > 0 {
                "Read-only preview · evaluation blocked"
            } else {
                "Read-only preview · complete evaluation"
            }
            .into();
        }
        "No candidate preview.".into()
    }
}
fn icon(path: &'static str) -> impl IntoView {
    crate::topology_canvas::icon(path)
}
#[component]
pub(crate) fn EvidencePane(state: Preview) -> impl IntoView {
    view! {<section class="studio-discovery" aria-label="Discovery evidence"><h4>"Read-only discovery"</h4>
      <p class="field-help">"Opening this panel reads cached metadata only. Refresh contacts the enabled router for the source types in this draft, using saved access configuration. Preview never sends secret input."</p>
      <button type="button" class="canvas-tool" aria-label="Refresh discovery" title="Refresh discovery" disabled=move||state.reason(false).is_some() on:click=move|_|state.collect()>{icon("M20 11a8 8 0 1 0-2 6M20 4v7h-7")}</button>
      <p class="field-help">{move||state.reason(false)}</p>
      <label>"Cached snapshot"<select aria-label="Cached snapshot" prop:value=move||state.snapshot.get().map(|s|s.id).unwrap_or_default() on:change=move|ev|{let id=event_target_value(&ev);let next=state.config.with(|c|c.as_ref().and_then(|c|c.snapshots.iter().find(|s|s.id==id).cloned()));if let Some(next)=next{state.snapshot.set(Some(next));state.show.set(false);}}>
        <option value="">"No cached snapshot"</option>{move||{let mut snapshots=state.config.with(|c|c.as_ref().map(|c|c.snapshots.clone()).unwrap_or_default());if let Some(s)=state.snapshot.get()&&!snapshots.iter().any(|x|x.id==s.id){snapshots.push(s);}snapshots.into_iter().map(|s|view!{<option value=s.id.clone()>{format!("Snapshot {} · {} tables",s.id,s.tables.len())}</option>}).collect_view()}}
      </select></label>
      {move||state.snapshot.get().map(|s|view!{<div class="studio-snapshot"><code>{s.id}</code><p>{format!("Collected {} seconds ago · {}",state.clock.get().saturating_sub(s.collected_to),if s.expires_at>state.clock.get(){"snapshot unexpired"}else{"snapshot expired"})}</p>
        <p class="field-help">"Completeness describes that collection. It is not live traffic. Partial, failed, and unrequested tables do not prove absence."</p>
        <ul class="studio-table-evidence">{s.tables.into_iter().map(|t|view!{<li><strong>{key(&t.table)}</strong><span>{key(&t.state)}</span><span>{format!("{} records",t.records)}</span><small>{t.issue_code.unwrap_or_default()}</small></li>}).collect_view()}</ul>
      </div>})}
      <button type="button" class="canvas-tool" aria-label="Preview details" title="Preview details" disabled=move||state.candidate.get().is_none() on:click=move|_|state.details()>{icon("M12 11v6M12 7v1M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0")}</button>
      <Show when=move||state.pending.get().is_some()&&!state.busy.get()>
        <button type="button" class="canvas-tool" aria-label="Recover read request" title="Query the previous operation; repeat this read only if it is absent" on:click=move|_|state.recover()>{icon("M20 11a8 8 0 1 0-2 6M20 4v7h-7")}</button>
        <p class="field-help">"Recovery checks the existing operation first. If absent, it repeats this read with the original key and input."</p>
      </Show>
      <p role="status">{move||state.status()}</p><p class="field-help">{move||state.reason(true)}</p>
    </section>}
}
#[component]
pub(crate) fn PreviewUi(state: Preview) -> impl IntoView {
    view! {
      <Show when=move||state.show.get()||state.busy.get()||!state.message.get().is_empty()>
        <div class="studio-preview-banner" role="region" aria-label="Read-only source preview"><div><strong>{move||state.status()}</strong><small>"Line width: subscribers. Rates show capacity, not throughput."</small></div>
          <button type="button" class="canvas-tool" aria-label="Preview details" title="Preview details" disabled=move||state.candidate.get().is_none() on:click=move|_|state.details()>{icon("M12 11v6M12 7v1M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0")}</button>
          <button type="button" class="canvas-tool" aria-label="Edit source draft" title="Edit source draft" on:click=move|_|{state.show.set(false);state.on_edit.run(());}>{icon("M16 3l5 5-12 12-6 1 1-6z")}</button>
          <button type="button" class="canvas-tool" aria-label="Load saved topology" title="Load saved topology" on:click=move|_|{state.show.set(false);state.message.set(String::new());state.on_saved.run(());}>{icon("M3 5h18v14H3zM7 9h10M7 13h6")}</button>
        </div>
      </Show>
      <dialog node_ref=state.details class="topology-dialog canvas-dialog studio-preview-dialog" data-panel="source-preview" aria-labelledby="studio-preview-title">
        <header class="add-router-heading"><div><h2 id="studio-preview-title">"Discovery preview"</h2><p>{move||state.status()}</p></div><button type="button" class="canvas-tool" aria-label="Close preview details" title="Close preview details" on:click=move|_|if let Some(d)=state.details.get_untracked(){d.close();}>{icon("M6 6l12 12M18 6 6 18")}</button></header>
        <div class="canvas-dialog-body">
          {move||state.candidate.get().map(|c|view!{<p class="studio-preview-counts">{format!("{} blockers · {} warnings · {} information · {} added · {} removed · {} changed",c.summary.issues.blockers,c.summary.issues.warnings,c.summary.issues.information,c.summary.diff.added,c.summary.diff.removed,c.summary.diff.changed)}</p>
            {(c.summary.graph.nodes_truncated||c.summary.graph.flows_truncated||c.summary.diff.markers_truncated).then(||view!{<p class="notice">"The graph or diff markers are truncated. Counts describe the full evaluation."</p>})}
            <p class="field-help">{format!("Candidate {} · revision {} · snapshot {}. This is a display result, not a saved or applied policy.",c.summary.reference.id,c.summary.reference.base_revision,c.summary.reference.snapshot_id)}</p>
          })}
          <nav class="studio-inline" aria-label="Preview details"><button type="button" class="canvas-tool" aria-label="Validation issues" title="Validation issues" aria-pressed=move||(!state.records_tab.get()).to_string() on:click=move|_|{state.records_tab.set(false);if state.issues.with(|p|p.total.is_none()){state.load_page(false,false);}}>{icon("M12 3l10 18H2zM12 9v5M12 17v1")}</button>
            <button type="button" class="canvas-tool" aria-label="Resolved records" title="Resolved records" aria-pressed=move||state.records_tab.get().to_string() on:click=move|_|{state.records_tab.set(true);if state.records.with(|p|p.total.is_none()){state.load_page(true,false);}}>{icon("M4 3h16v18H4zM8 7h8M8 12h8M8 17h5")}</button>
          </nav>
          <p role="status">{move||state.page_error.get()}</p>
          <Show when=move||!state.page_error.get().is_empty()><button type="button" class="canvas-tool" aria-label="Retry details" title="Retry details" disabled=move||state.page_busy.get()||!state.current() on:click=move|_|state.load_page(state.records_tab.get_untracked(),false)>{icon("M20 11a8 8 0 1 0-2 6M20 4v7h-7")}</button></Show>
          <Show when=move||!state.records_tab.get() fallback=move||view!{
            <p>{move||state.records.with(|p|format!("{} of {} records loaded",p.items.len(),p.total.map(|n|n.to_string()).unwrap_or("unknown".into())))}</p>
            <Show when=move||state.records.with(DetailPage::incomplete)><p class="notice">"Record details are incomplete. The missing rows are not empty results."</p></Show>
            <ul class="studio-resolved-records">{move||state.records.with(|p|p.items.iter().map(|r|view!{<li><strong>{r.record_ref.clone()}</strong><p>{if r.identity_ref.is_some(){"Existing registry identity"}else{"Unresolved identity"}}</p><dl><dt>"Download (bps)"</dt><dd>{r.down_bps.clone().unwrap_or("Unknown".into())}</dd><dt>"Upload (bps)"</dt><dd>{r.up_bps.clone().unwrap_or("Unknown".into())}</dd><dt>"Rate source"</dt><dd>{r.speed_source.as_ref().map(key).unwrap_or("Unresolved".into())}</dd><dt>"Group"</dt><dd>{r.group_ref.clone().unwrap_or("Unresolved".into())}</dd></dl>
              <ul>{r.provenance.iter().map(|s|view!{<li>{format!("{} · {} · {} · {}",key(&s.table),s.snapshot_id,s.record_ref,s.fields.iter().map(key).collect::<Vec<_>>().join(", "))}</li>}).collect_view()}</ul>
            </li>}).collect_view())}</ul>
            <button type="button" class="canvas-tool" aria-label="More records" title="More records" disabled=move||state.page_busy.get()||!state.current()||state.records.with(|p|p.next_cursor.is_none()) on:click=move|_|state.load_page(true,true)>{icon("M6 9l6 6 6-6")}</button>
          }>
            <p>{move||state.issues.with(|p|format!("{} of {} issues loaded",p.items.len(),p.total.map(|n|n.to_string()).unwrap_or("unknown".into())))}</p>
            <Show when=move||state.issues.with(DetailPage::incomplete)><p class="notice">"Issue details are incomplete. The summary counts still include all blockers."</p></Show>
            <ul class="studio-validation-issues">{move||state.issues.with(|p|p.items.iter().map(|i|view!{<li data-severity=key(&i.severity)><strong>{format!("{} · {}",key(&i.severity),i.code)}</strong><p>{i.message.clone()}</p>{i.remedy.clone().map(|r|view!{<p>{r}</p>})}<small>{i.entity_ref.clone()}</small></li>}).collect_view())}</ul>
            <button type="button" class="canvas-tool" aria-label="More issues" title="More issues" disabled=move||state.page_busy.get()||!state.current()||state.issues.with(|p|p.next_cursor.is_none()) on:click=move|_|state.load_page(false,true)>{icon("M6 9l6 6 6-6")}</button>
          </Show>
          <Show when=move||state.pending.get().is_some()&&!state.busy.get()><button type="button" class="canvas-tool" aria-label="Recover read request" title="Query the existing operation. If absent, repeat this read with the same key." on:click=move|_|state.recover()>{icon("M20 11a8 8 0 1 0-2 6M20 4v7h-7")}</button></Show>
        </div>
      </dialog>
    }
}

pub(crate) fn candidate_geometry(
    summary: &CandidateSummary,
    saved: Option<&TopologySummary>,
    points: &[(String, [f64; 2])],
    direction: web_contract::topology::Direction,
) -> (
    Vec<crate::topology_canvas_model::Card>,
    std::collections::BTreeMap<String, [f64; 2]>,
) {
    use crate::canvas_layout::{CardBox, layout_in};
    use crate::topology_canvas_model::{SIZE, card_of, cards, node_tier};
    let mut nodes = summary.graph.nodes.clone();
    if let Some(saved) = saved.cloned() {
        for marker in &summary.diff.markers {
            if marker.change == ChangeKind::Removed
                && let Some(n) = saved
                    .nodes
                    .iter()
                    .find(|n| n.id == marker.entity_ref || card_of(n) == marker.entity_ref)
                && !nodes.iter().any(|other| other.id == n.id)
            {
                nodes.push(n.clone());
            }
        }
    }
    let cards = cards(&nodes);
    let boxes = cards
        .iter()
        .map(|c| CardBox {
            id: c.id.clone(),
            tier: node_tier(&c.node),
            parent: c.node.parent.as_ref().and_then(|p| {
                cards
                    .iter()
                    .find(|c| c.nodes.contains(p))
                    .map(|c| c.id.clone())
            }),
            size: SIZE,
        })
        .collect::<Vec<_>>();
    let manual = points
        .iter()
        .filter(|&(id, _)| cards.iter().any(|c| &c.id == id))
        .cloned()
        .collect();
    let positions = layout_in(&boxes, &manual, direction);
    (cards, positions)
}

#[component]
pub(crate) fn CandidateOverlay(
    projection: RwSignal<Option<CandidateSummary>>,
    saved: RwSignal<Option<TopologySummary>>,
    points: Memo<Vec<(String, [f64; 2])>>,
    direction: RwSignal<web_contract::topology::Direction>,
) -> impl IntoView {
    use crate::topology_canvas_model::port;
    let geometry = Memo::new(move |_| {
        projection
            .get()
            .map(|s| candidate_geometry(&s, saved.get().as_ref(), &points.get(), direction.get()))
            .unwrap_or_default()
    });
    view! {<Show when=move||projection.with(Option::is_some)>
      <div class="studio-candidate-layer" aria-label="Read-only candidate graph">
        <svg class="canvas-wires studio-candidate-wires" aria-hidden="true" width="100%" height="100%">
          {move||projection.with(|s|s.as_ref().map(|s|{let (_,positions)=geometry.get();let max=s.graph.flows.iter().map(|f|f.subscribers).max().unwrap_or(1).max(1);s.graph.flows.iter().filter_map(|f|{
            let (Some(a),Some(b))=(positions.get(&f.from),positions.get(&f.to))else{return None};let start=port(*a,direction.get(),true);let end=port(*b,direction.get(),false);let mid=(start[1]+end[1])/2.;
            Some(view!{<g class="studio-preview-flow"><path d=format!("M{},{} C{},{} {},{} {},{}",start[0],start[1],start[0],mid,end[0],mid,end[0],end[1]) stroke-width=(1.5+8.5*f.subscribers as f64/max as f64).to_string()/><text x=((start[0]+end[0])/2.).to_string() y=(mid-6.).to_string()>{format!("{} subscribers",f.subscribers)}</text></g>})
          }).collect_view()}))}
        </svg>
        {move||{let (cards,at)=geometry.get();let summary=projection.get();cards.into_iter().map(|c|{let p=at.get(&c.id).copied().unwrap_or([0.,0.]);let change=summary.as_ref().and_then(|s|s.diff.markers.iter().find(|m|m.entity_ref==c.id||c.nodes.contains(&m.entity_ref)).map(|m|m.change));let change=match change{Some(ChangeKind::Added)=>"Added",Some(ChangeKind::Changed)=>"Changed",Some(ChangeKind::Removed)=>"Removed",None=>"Unchanged"};
          view!{<div class="studio-preview-card" tabindex="0" role="group" aria-label=format!("{}, {change}, read-only",c.node.name) data-change=change style:left=format!("{}px",p[0]) style:top=format!("{}px",p[1])><small>{format!("{change} · read-only")}</small><strong>{c.node.name.clone()}</strong><span>{c.node.calculated_mbps.map(|r|format!("{} / {} Mbps capacity",r[0],r[1])).unwrap_or("Unknown capacity".into())}</span></div>}
        }).collect_view()}}
      </div>
    </Show>}
}
