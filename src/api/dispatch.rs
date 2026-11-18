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

fn code_of(e: &anyhow::Error) -> ErrorCode {
    if e.chain().any(|c| c.is::<RevisionConflict>()) {
        ErrorCode::Conflict
    } else if e.chain().any(|c| c.is::<InvalidPolicy>()) {
        ErrorCode::Invalid
    } else {
        ErrorCode::Internal
    }
}

fn run<C: Connector>(runner: &mut Runner<C>, request: &Request) -> Result<Value, ApiError> {
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
        other => Err(ApiError::new(
            ErrorCode::UnknownOp,
            format!("unknown operation \"{other}\""),
        )),
    }
}

/// Answer one request on the task that owns the `Runner`. A lost lock returns `LostLock`.
pub fn handle<C: Connector>(runner: &mut Runner<C>, request: &Request) -> Result<String, LostLock> {
    if let Err(e) = runner.lock() {
        let error = ApiError::new(ErrorCode::Internal, format!("instance lock lost: {e:#}"));
        return Err(LostLock(response_err(&request.id, &error)));
    }
    Ok(match run(runner, request) {
        Ok(result) => response_ok(&request.id, result),
        Err(error) => response_err(&request.id, &error),
    })
}
