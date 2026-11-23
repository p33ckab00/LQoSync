//! Map API requests to `Runner` operations and errors to API error codes.

use super::protocol::{ApiError, ErrorCode, Request, response_err, response_ok};
use crate::routeros::Connector;
use crate::runner::Runner;
use crate::topology_manager::store::{InvalidPolicy, RevisionConflict};
use serde::Deserialize;
use serde_json::{Value, json};

/// The service lost its instance lock. The line is the `internal` reply; the service then stops.
#[derive(Debug)]
pub struct LostLock(pub String);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommitArgs {
    expected_revision: u64,
    catalog: crate::topology_manager::catalog::TopologyCatalog,
    source_settings: crate::topology_manager::settings::SourceSettings,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditProposeArgs {
    expected_revision: u64,
    edit: crate::topology_manager::edits::Edit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RouterTestArgs {
    router_id: String,
    actor: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WanDiscoverArgs {
    #[serde(default)]
    routers: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposalArgs {
    proposal_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanCreateArgs {
    expected_revision: u64,
    name: String,
    down_mbps: u64,
    up_mbps: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanUpdateArgs {
    expected_revision: u64,
    plan_id: String,
    name: Option<String>,
    down_mbps: Option<u64>,
    up_mbps: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanDeleteArgs {
    expected_revision: u64,
    plan_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityArgs {
    router_id: String,
    source_id: String,
    kind: crate::topology_manager::model::SourceKind,
    account_ref: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SubscriberPlanArgs {
    expected_revision: u64,
    identity: IdentityArgs,
    plan_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WebMutateArgs {
    key: String,
    actor: String,
    command: web_contract::topology::WebCommand,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WebOperationArgs {
    key: String,
    actor: String,
}

fn parse<T: serde::de::DeserializeOwned>(request: &Request) -> Result<T, ApiError> {
    serde_json::from_value(request.args.clone())
        .map_err(|e| ApiError::new(ErrorCode::BadRequest, format!("invalid arguments: {e}")))
}

fn code_of(e: &anyhow::Error) -> ErrorCode {
    use crate::topology_manager::web_receipts::KeyReused;
    if e.chain()
        .any(|c| c.is::<RevisionConflict>() || c.is::<KeyReused>())
    {
        ErrorCode::Conflict
    } else if e.chain().any(|c| c.is::<InvalidPolicy>()) {
        ErrorCode::Invalid
    } else {
        ErrorCode::Internal
    }
}

async fn run<C>(runner: &mut Runner<C>, request: &Request) -> Result<Value, ApiError>
where
    C: Connector + 'static,
    C::Session: 'static,
{
    let fail = |e: anyhow::Error| ApiError::new(code_of(&e), format!("{e:#}"));
    match request.op.as_str() {
        "status" => runner.api_status().map_err(fail),
        "registry.get" => runner
            .api_registry()
            .map_err(fail)
            .and_then(|d| serde_json::to_value(d).map_err(|e| fail(e.into()))),
        "policy.commit" => {
            let args: CommitArgs = serde_json::from_value(request.args.clone()).map_err(|e| {
                ApiError::new(ErrorCode::BadRequest, format!("invalid arguments: {e}"))
            })?;
            runner
                .api_policy_commit(args.expected_revision, args.catalog, args.source_settings)
                .map(|revision| json!({"revision": revision}))
                .map_err(fail)
        }
        "topology.preview.refresh" => runner.api_preview_refresh().await.map_err(fail),
        "topology.preview.get" => runner.api_preview_get().map_err(fail),
        "topology.policies" => runner.api_policies().map_err(fail),
        "wan.discover" => {
            let a: WanDiscoverArgs = parse(request)?;
            runner.api_wan_discover(&a.routers).await.map_err(fail)
        }
        "router.test" => {
            let args: RouterTestArgs = parse(request)?;
            runner
                .api_router_test(&args.router_id, &args.actor)
                .await
                .map_err(fail)
        }
        "topology.tree" => runner.api_tree().map_err(fail),
        "topology.sources.candidates" => runner.api_source_candidates().await.map_err(fail),
        "topology.edit.propose" => {
            let a: EditProposeArgs = parse(request)?;
            runner
                .api_edit_propose(a.expected_revision, a.edit)
                .await
                .map_err(fail)
        }
        "topology.edit.confirm" => {
            let a: ProposalArgs = parse(request)?;
            runner.api_edit_confirm(&a.proposal_id).map_err(fail)
        }
        "topology.edit.cancel" => {
            let a: ProposalArgs = parse(request)?;
            runner.api_edit_cancel(&a.proposal_id).map_err(fail)
        }
        "plan.create" => {
            let a: PlanCreateArgs = parse(request)?;
            runner
                .api_plan_create(a.expected_revision, &a.name, a.down_mbps, a.up_mbps)
                .map_err(fail)
        }
        "plan.update" => {
            let a: PlanUpdateArgs = parse(request)?;
            runner
                .api_plan_update(
                    a.expected_revision,
                    &a.plan_id,
                    a.name.as_deref(),
                    a.down_mbps,
                    a.up_mbps,
                )
                .map_err(fail)
        }
        "plan.delete" => {
            let a: PlanDeleteArgs = parse(request)?;
            runner
                .api_plan_delete(a.expected_revision, &a.plan_id)
                .map_err(fail)
        }
        "subscriber.plan" => {
            let a: SubscriberPlanArgs = parse(request)?;
            let identity = crate::topology_manager::model::IdentityRef {
                scope: crate::topology_manager::model::SourceScope {
                    router_id: a.identity.router_id,
                    source_id: a.identity.source_id,
                    kind: a.identity.kind,
                },
                account_ref: a.identity.account_ref,
            };
            runner
                .api_subscriber_plan(a.expected_revision, identity, a.plan_id)
                .map_err(fail)
        }
        "topology.web.mutate" => {
            let a: WebMutateArgs = parse(request)?;
            if !crate::topology_manager::web_receipts::valid_key(&a.key) {
                return Err(ApiError::new(ErrorCode::BadRequest, "invalid request key"));
            }
            runner
                .api_web_mutate(&a.key, &a.actor, a.command)
                .await
                .map_err(fail)
        }
        "topology.layout.save" => {
            let a: web_contract::topology::LayoutRequest = parse(request)?;
            runner
                .api_layout_save(a.expected_revision, a.direction, a.positions)
                .map_err(fail)
        }
        "topology.web.operation" => {
            let a: WebOperationArgs = parse(request)?;
            runner.api_web_operation(&a.key, &a.actor).map_err(fail)
        }
        other => Err(ApiError::new(
            ErrorCode::UnknownOp,
            format!("unknown operation \"{other}\""),
        )),
    }
}

/// Answer one request on the task that owns the `Runner`. A lost lock returns `LostLock`.
pub async fn handle<C>(runner: &mut Runner<C>, request: &Request) -> Result<String, LostLock>
where
    C: Connector + 'static,
    C::Session: 'static,
{
    if let Err(e) = runner.lock() {
        let error = ApiError::new(ErrorCode::Internal, format!("instance lock lost: {e:#}"));
        return Err(LostLock(response_err(&request.id, &error)));
    }
    Ok(match run(runner, request).await {
        Ok(result) => response_ok(&request.id, result),
        Err(error) => response_err(&request.id, &error),
    })
}
