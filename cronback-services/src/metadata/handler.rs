use std::sync::Arc;

use chrono::Utc;
use lib::database::DatabaseError;
use lib::e;
use lib::prelude::ModelId;
use lib::service::ServiceContext;
use lib::types::ProjectId;
use proto::metadata_svc::metadata_svc_server::MetadataSvc;
use proto::metadata_svc::{
    CreateProjectRequest,
    CreateProjectResponse,
    GetProjectStatusRequest,
    GetProjectStatusResponse,
    ProjectExistsRequest,
    ProjectExistsResponse,
    SetProjectStatusRequest,
    SetProjectStatusResponse,
};
use thiserror::Error;
use tonic::{Request, Response, Status};

use super::db_model::{Project, ProjectStatus};
use super::metadata_store::MetadataStore;

pub(crate) struct MetadataSvcHandler {
    #[allow(unused)]
    context: ServiceContext,
    project_store: Arc<dyn MetadataStore + Send + Sync>,
}

impl MetadataSvcHandler {
    pub fn new(
        context: ServiceContext,
        project_store: Arc<dyn MetadataStore + Send + Sync>,
    ) -> Self {
        Self {
            context,
            project_store,
        }
    }
}

#[tonic::async_trait]
impl MetadataSvc for MetadataSvcHandler {
    async fn create_project(
        &self,
        request: Request<CreateProjectRequest>,
    ) -> Result<Response<CreateProjectResponse>, Status> {
        let (_, _, req) = request.into_parts();
        let id: ProjectId = req.id.unwrap().into();
        let id = id
            .validated()
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let project = Project {
            id: id.clone(),
            created_at: Utc::now(),
            changed_at: Utc::now(),
            status: ProjectStatus::Enabled,
        };

        self.project_store
            .store_project(project)
            .await
            .map_err(ProjectStoreHandlerError::Store)?;

        e!(project_id = id, ProjectCreated {});

        Ok(Response::new(CreateProjectResponse {
            id: Some(id.into_inner().into()),
        }))
    }

    async fn get_project_status(
        &self,
        request: Request<GetProjectStatusRequest>,
    ) -> Result<Response<GetProjectStatusResponse>, Status> {
        let (_, _, req) = request.into_parts();
        let project_id: ProjectId = req.id.unwrap().into();
        let project_id = project_id
            .validated()
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let status = self
            .project_store
            .get_status(&project_id)
            .await
            .map_err(ProjectStoreHandlerError::Store)?;
        match status {
            | Some(st) => {
                Ok(Response::new(GetProjectStatusResponse {
                    status: st.into(),
                }))
            }
            | None => {
                Err(ProjectStoreHandlerError::NotFound(format!(
                    "{}",
                    project_id
                )))?
            }
        }
    }

    async fn set_project_status(
        &self,
        request: Request<SetProjectStatusRequest>,
    ) -> Result<Response<SetProjectStatusResponse>, Status> {
        let (_, _, req) = request.into_parts();
        let project_id: ProjectId = req.id.unwrap().into();
        let project_id = project_id
            .validated()
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let old_status = self
            .project_store
            .get_status(&project_id)
            .await
            .map_err(ProjectStoreHandlerError::Store)?;

        let Some(old_status) = old_status else {
            return Err(ProjectStoreHandlerError::NotFound(
                project_id.to_string(),
            )
            .into());
        };

        self.project_store
            .set_status(&project_id, req.status.into())
            .await
            .map_err(ProjectStoreHandlerError::Store)?;

        // NOTE: The `old_status` here might not always be 100% accurate as it
        // might race with another update.
        e!(
            project_id = project_id,
            ProjectStatusUpdated {
                old_status: old_status.clone().into(),
                new_status: req.status,
            }
        );

        Ok(Response::new(SetProjectStatusResponse {
            old_status: old_status.into(),
        }))
    }

    async fn project_exists(
        &self,
        request: Request<ProjectExistsRequest>,
    ) -> Result<Response<ProjectExistsResponse>, Status> {
        let (_, _, req) = request.into_parts();
        let project_id: ProjectId = req.id.unwrap().into();
        let project_id = project_id
            .validated()
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let exists = self
            .project_store
            .exists(&project_id)
            .await
            .map_err(ProjectStoreHandlerError::Store)?;
        Ok(Response::new(ProjectExistsResponse { exists }))
    }
}

#[derive(Error, Debug)]
pub(crate) enum ProjectStoreHandlerError {
    #[error("Project '{0}' is unknown to the store!")]
    NotFound(String),
    #[error("Operation on underlying database failed: {0}")]
    Store(#[from] DatabaseError),
}

impl From<ProjectStoreHandlerError> for Status {
    fn from(e: ProjectStoreHandlerError) -> Self {
        // match variants of TriggerError
        match e {
            | ProjectStoreHandlerError::NotFound(e) => Status::not_found(e),
            | e => Status::invalid_argument(e.to_string()),
        }
    }
}
