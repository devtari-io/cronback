use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use cronback_api_model::admin::{
    CreateProjectResponse as CreateProjectHttpResponse,
    NotificationSettings,
};
use hyper::StatusCode;
use lib::prelude::*;
use proto::metadata_svc::{
    CreateProjectRequest,
    GetProjectNotificationSettingsRequest,
    SetProjectNotificationSettingsRequest,
    SetProjectStatusRequest,
};
use proto::projects::ProjectStatus;

use crate::api::errors::ApiError;
use crate::api::extractors::ValidatedJson;
use crate::api::AppState;

#[tracing::instrument(skip(state))]
pub(crate) async fn create(
    state: State<Arc<AppState>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let id = ProjectId::generate();
    let mut metadata = state
        .metadata_svc_clients
        .get_client(&request_id, &id)
        .await?;
    let (_, resp, _) = metadata
        .create_project(CreateProjectRequest {
            id: Some(id.clone().into()),
        })
        .await?
        .into_parts();
    let response = CreateProjectHttpResponse {
        id: resp.id.unwrap().value,
    };
    Ok((StatusCode::CREATED, Json(response)).into_response())
}

async fn set_project_status(
    state: &State<Arc<AppState>>,
    project_id_str: String,
    request_id: RequestId,
    status: ProjectStatus,
) -> Result<impl IntoResponse, ApiError> {
    let project_id = ProjectId::from(project_id_str.clone())
        .validated()
        .map_err(move |_| ApiError::NotFound(project_id_str))?;

    let mut metadata = state
        .metadata_svc_clients
        .get_client(&request_id, &project_id)
        .await?;
    metadata
        .set_project_status(SetProjectStatusRequest {
            id: Some(project_id.into()),
            status: status.into(),
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[tracing::instrument(skip(state))]
pub(crate) async fn enable(
    state: State<Arc<AppState>>,
    Path(project_id_str): Path<String>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    set_project_status(
        &state,
        project_id_str,
        request_id,
        ProjectStatus::Enabled,
    )
    .await
}

#[tracing::instrument(skip(state))]
pub(crate) async fn disable(
    state: State<Arc<AppState>>,
    Path(project_id_str): Path<String>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    set_project_status(
        &state,
        project_id_str,
        request_id,
        ProjectStatus::Disabled,
    )
    .await
}

#[tracing::instrument(skip(state))]
pub(crate) async fn get_notification_settings(
    state: State<Arc<AppState>>,
    Path(project_id_str): Path<String>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let project_id = ProjectId::from(project_id_str.clone())
        .validated()
        .map_err(move |_| ApiError::NotFound(project_id_str))?;

    let mut metadata = state
        .metadata_svc_clients
        .get_client(&request_id, &project_id)
        .await?;
    let resp = metadata
        .get_project_notification_settings(
            GetProjectNotificationSettingsRequest {
                id: Some(project_id.into()),
            },
        )
        .await?
        .into_inner();

    let settings: NotificationSettings = resp.settings.unwrap().into();

    Ok(Json(settings))
}

#[tracing::instrument(skip(state))]
pub(crate) async fn set_notification_settings(
    state: State<Arc<AppState>>,
    Path(project_id_str): Path<String>,
    Extension(request_id): Extension<RequestId>,
    ValidatedJson(settings): ValidatedJson<NotificationSettings>,
) -> Result<impl IntoResponse, ApiError> {
    let project_id = ProjectId::from(project_id_str.clone())
        .validated()
        .map_err(move |_| ApiError::NotFound(project_id_str))?;

    let mut metadata = state
        .metadata_svc_clients
        .get_client(&request_id, &project_id)
        .await?;
    metadata
        .set_project_notification_settings(
            SetProjectNotificationSettingsRequest {
                id: Some(project_id.into()),
                settings: Some(settings.into()),
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
