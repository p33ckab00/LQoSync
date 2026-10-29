use crate::{
    model::compile,
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
fn same_inventories(
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
            normalizer::normalize(&work_settings, &work_inventories, &work_errors, now())?;
        let previous = store::load_assignments(&root)?;
        let assignments = crate::assignment::assign(
            &work_settings,
            &mut subscribers,
            &previous,
            &work_inventories,
            &work_errors,
            now(),
        )?;
        let plan = compile(&work_settings, &subscribers)?;
        Ok((subscribers, plan, assignments))
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
    let (subscribers, plan, assignments) = match result {
        Ok(value) => value,
        Err(e) => {
            a.status = format!("Not normalized: {e}. Last valid snapshot and generation retained.");
            return;
        }
    };
    // Serialize the commit, but do file I/O on a blocking worker, not an async worker.
    let _ = tokio::task::spawn_blocking(move || {
        match store::publish_assigned(&a.root, &plan, &subscribers, &assignments) {
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
    let (settings, routers) = {
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
        let selected: BTreeSet<_> = settings
            .nodes
            .iter()
            .filter(|n| {
                (normalizer::service(&n.kind).is_some() || n.kind == "wan")
                    && !n.source.is_empty()
                    && !n.router.is_empty()
            })
            .map(|n| n.router.as_str())
            .collect();
        let routers: Vec<_> = settings
            .routers
            .iter()
            .filter(|r| selected.contains(r.id.as_str()))
            .map(|r| (r.clone(), a.begin_read(&r.id)))
            .collect();
        a.status = "Reading selected routers. Last valid local generation remains active.".into();
        (settings, routers)
    };
    let mut pending = routers.into_iter();
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..4 {
        if let Some((router, ticket)) = pending.next() {
            tasks.spawn(async move {
                let result = routeros::inspect(&router).await;
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
            tasks.spawn(async move {
                let result = routeros::inspect(&router).await;
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
