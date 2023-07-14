use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::{debug_handler, Extension, Json};
use lib::model::ValidShardedId;
use lib::prelude::{ModelId, OptionExt};
use lib::types::{ProjectId, RequestId, RunId};
use proto::common::TriggerId;
use proto::dispatcher_proto::{GetRunRequest, ListRunsRequest};
use proto::scheduler_proto::GetTriggerIdRequest;
use validator::Validate;

use crate::errors::ApiError;
use crate::model::{GetRunResponse, Run};
use crate::paginated::{Paginated, Pagination};
use crate::AppState;

async fn get_trigger_id(
    state: &State<Arc<AppState>>,
    name: &str,
    project: &ValidShardedId<ProjectId>,
    request_id: &RequestId,
) -> Result<TriggerId, ApiError> {
    let mut scheduler = state
        .scheduler_clients
        .get_client(request_id, project)
        .await?;

    Ok(scheduler
        .get_trigger_id(GetTriggerIdRequest {
            name: name.to_string(),
        })
        .await?
        .into_inner()
        .id
        .unwrap())
}

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn get(
    state: State<Arc<AppState>>,
    Path((name, run_id)): Path<(String, RunId)>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<GetRunResponse>, ApiError> {
    let run_id = run_id
        .clone()
        .validated()
        .map_err(|_| ApiError::NotFound(run_id.to_string()))?;

    // Validate that the trigger exists for better user experience
    // We use `-` as a wildcard symbol.
    let trigger_id = if name == "-" {
        None
    } else {
        Some(get_trigger_id(&state, &name, &project, &request_id).await?)
    };

    let mut dispatcher = state
        .dispatcher_clients
        .get_client(&request_id, &project)
        .await?;

    let resp = dispatcher
        .get_run(GetRunRequest {
            run_id: Some(run_id.clone().into()),
        })
        .await?
        .into_inner();

    if let Some(trigger_id) = trigger_id {
        // Validate that the run actually belongs to this trigger
        // If not, fail the request with NotFound.
        if resp.run.unwrap_ref().trigger_id.unwrap_ref() != &trigger_id {
            return Err(ApiError::NotFound(run_id.to_string()));
        }
    }

    let resp = GetRunResponse::from(resp);

    Ok(Json(resp))
}

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    Query(pagination): Query<Pagination>,
    state: State<Arc<AppState>>,
    Path(name): Path<String>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Paginated<Run>, ApiError> {
    pagination.validate()?;

    let trigger_id =
        get_trigger_id(&state, &name, &project, &request_id).await?;

    let mut dispatcher = state
        .dispatcher_clients
        .get_client(&request_id, &project)
        .await?;

    let response = dispatcher
        .list_runs(ListRunsRequest {
            trigger_id: Some(trigger_id),
            pagination: Some(pagination.into()),
        })
        .await?
        .into_inner();

    Ok(Paginated::from(
        response.runs,
        response.pagination.unwrap_or_default(),
    ))
}
