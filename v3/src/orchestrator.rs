use crate::{
    normalizer,
    routeros::{self},
    store,
    web::Shared,
};
use axum::{
    extract::{Request, State},
    http::Method,
    middleware::Next,
    response::Response,
};
use std::{
    collections::BTreeSet,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
pub(crate) fn same_inventories(
    a: &std::collections::BTreeMap<String, Arc<routeros::Inventory>>,
    b: &std::collections::BTreeMap<String, Arc<routeros::Inventory>>,
) -> bool {
    a.len() == b.len()
        && a.iter()
            .all(|(id, value)| b.get(id).is_some_and(|other| Arc::ptr_eq(value, other)))
}
pub async fn after_mutation(State(s): State<Shared>, request: Request, next: Next) -> Response {
    let post = request.method() == Method::POST;
    let response = next.run(request).await;
    if post && response.status().is_redirection() {
        refresh(s).await;
    }
    response
}
pub async fn refresh(s: Shared) {
    let (settings, inventories, errors, epochs, root, generation) = {
        let a = s.lock().await;
        if !a.settings.auto_stage || !normalizer::has_bindings(&a.settings) {
            return;
        }
        (
            a.settings.clone(),
            a.inventories.clone(),
            a.inventory_errors.clone(),
            a.read_epochs.clone(),
            a.root.clone(),
            a.generation.clone(),
        )
    };
    let work_settings = settings.clone();
    let work_inventories = inventories.clone();
    let work_errors = errors.clone();
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let mut subscribers =
            normalizer::collect(&work_settings, &work_inventories, &work_errors, now())?;
        let previous = store::load_assignments(&root)?;
        let count_mode = crate::dynamic::count_mode(&work_settings);
        let (session_baseline, revision) = if count_mode {
            let (baseline, revision) = crate::sessions::load_for_update(&root)?;
            (baseline, Some(revision))
        } else {
            (None, None)
        };
        let observation = if count_mode {
            Some(crate::sessions::plan(
                &work_settings,
                &mut subscribers,
                &previous,
                session_baseline.as_ref(),
                &work_inventories,
                &work_errors,
                now(),
            )?)
        } else {
            None
        };
        let assignments = if let Some(o) = &observation {
            o.assignments.clone()
        } else {
            crate::assignment::assign(
                &work_settings,
                &mut subscribers,
                &previous,
                &work_inventories,
                &work_errors,
                now(),
            )?
        };
        // A healthy absence remains useful even if output admission later fails.
        let output = (|| -> anyhow::Result<_> {
            anyhow::ensure!(
                !subscribers.is_empty(),
                "Selected scan is empty; last valid snapshot is retained"
            );
            crate::minimums::allocate_assigned(&work_settings, &mut subscribers, &assignments)?;
            let plan = crate::model::compile_assigned(&work_settings, &subscribers, &assignments)?;
            let core = if work_settings.core_lists.bindings.is_empty() {
                None
            } else {
                Some(crate::core_plan::build(
                    &work_settings,
                    &subscribers,
                    &assignments,
                    &work_inventories,
                    &work_errors,
                    now(),
                )?)
            };
            Ok((subscribers, plan, core))
        })();
        Ok((observation, revision, assignments, output))
    })
    .await;
    let mut a = s.clone().lock_owned().await;
    if a.settings != settings
        || a.generation != generation
        || a.inventory_errors != errors
        || a.read_epochs != epochs
        || !same_inventories(&a.inventories, &inventories)
    {
        a.status="Normalization result discarded because settings or inventory changed. Last valid generation retained.".into();
        return;
    }
    let result = match result {
        Ok(value) => value,
        Err(e) => Err(e.into()),
    };
    let (observation, revision, assignments, output) = match result {
        Ok(value) => value,
        Err(e) => {
            a.status = format!("Not normalized: {e}. Last valid snapshot and generation retained.");
            return;
        }
    };
    // Serialize candidate evidence and publication after the same freshness checks.
    let _ = tokio::task::spawn_blocking(move || {
        if let Some(observation) = observation {
            let current = now();
            let fresh = observation.collected.values().all(|t| current >= *t && current - *t <= normalizer::MAX_INVENTORY_AGE_SECONDS);
            if !fresh {a.status="Session observation expired; last valid snapshot and generation retained.".into();return;}
            if let Err(e) = revision.expect("count-mode observation has its validated revision").save(&a.root, &observation) {
                a.status=format!("Candidate session state could not be saved: {e}. Last valid generation retained.");return;
            }
        }
        let (subscribers, plan, core) = match output {
            Ok(value) => value,
            Err(e) => {a.status=format!("Not normalized: {e}. Last valid snapshot and generation retained.");return;}
        };
        let result = (|| -> anyhow::Result<_> {
            if let Some(core) = &core {
                crate::core_plan::validate_fresh(core, now())?;
            }
            store::publish_orchestrated(&a.root, &plan, &subscribers, &assignments, core.as_ref())
        })();
        match result {
            Ok(p) => {
                a.generation = Some(p.generation);
                a.status = format!(
                    "{} normalized subscribers staged locally. {}",
                    subscribers.len(),
                    p.mirror_warning.unwrap_or_else(|| {
                        "Snapshot and output pair committed together. Production is untouched."
                            .into()
                    })
                );
            }
            Err(e) => {
                a.status = format!("Local generation could not be committed: {e}");
            }
        }
    })
    .await;
}

pub async fn cycle(s: Shared) {
    let (settings, routers, root) = {
        let mut a = s.lock().await;
        if !a.settings.auto_stage {
            return;
        }
        if !normalizer::has_bindings(&a.settings) {
            drop(a);
            let mut a = s.clone().lock_owned().await;
            let _ = tokio::task::spawn_blocking(move || a.reconcile()).await;
            return;
        }
        let settings = a.settings.clone();
        let mut selected: BTreeSet<_> = settings
            .nodes
            .iter()
            .filter(|n| {
                (normalizer::service(&n.kind).is_some() || n.kind == "wan")
                    && !n.source.is_empty()
                    && !n.router.is_empty()
            })
            .map(|n| n.router.as_str())
            .collect();
        selected.extend(normalizer::selected_routers(&settings));
        if let Some(c) = &settings.dynamic {
            selected.extend(c.wans.iter().map(|w| w.router.as_str()));
        }
        let routers: Vec<_> = settings
            .routers
            .iter()
            .filter(|r| selected.contains(r.id.as_str()))
            .map(|r| (r.clone(), a.begin_read(&r.id)))
            .collect();
        a.status = "Reading selected routers. Last valid local generation remains active.".into();
        (settings, routers, a.root.clone())
    };
    let mut pending = routers.into_iter();
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..4 {
        if let Some((router, ticket)) = pending.next() {
            let root = root.clone();
            tasks.spawn(async move {
                let result = routeros::inspect_from(&router, &root).await;
                (router.id, ticket, result)
            });
        }
    }
    let mut results = vec![];
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(value) => results.push(value),
            Err(_) => {
                s.lock().await.status =
                    "Router read worker failed; last valid generation retained.".into();
                return;
            }
        }
        {
            let a = s.lock().await;
            if !a.settings.auto_stage || a.settings != settings {
                tasks.abort_all();
                return;
            }
        }
        if let Some((router, ticket)) = pending.next() {
            let root = root.clone();
            tasks.spawn(async move {
                let result = routeros::inspect_from(&router, &root).await;
                (router.id, ticket, result)
            });
        }
    }
    let mut a = s.clone().lock_owned().await;
    if a.settings != settings
        || results
            .iter()
            .any(|(id, ticket, _)| a.read_epochs.get(id) != Some(ticket))
    {
        a.status="Read round superseded by a newer read or settings change. Last valid generation retained.".into();
        return;
    }
    let _ = tokio::task::spawn_blocking(move || {
        for (id, _, result) in results {
            a.record_inventory(id, result);
        }
    })
    .await;
    refresh(s).await;
}
