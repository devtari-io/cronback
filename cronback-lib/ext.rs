use tonic::metadata::MetadataMap;

use crate::consts::{PROJECT_ID_HEADER, REQUEST_ID_HEADER};
use crate::grpc_helpers::RequestContext;
use crate::model::{ModelIdError, ValidShardedId};
use crate::types::{ProjectId, RequestId};

pub trait OptionExt {
    type Item;
    fn unwrap_ref(&self) -> &Self::Item;
    fn unwrap_mut(&mut self) -> &mut Self::Item;
}
impl<T> OptionExt for Option<T> {
    type Item = T;

    fn unwrap_ref(&self) -> &T {
        self.as_ref().unwrap()
    }

    fn unwrap_mut(&mut self) -> &mut T {
        self.as_mut().unwrap()
    }
}

pub trait GrpcMetadataMapExt {
    fn set_project_id(&mut self, project_id: ValidShardedId<ProjectId>);
    fn project_id(&self) -> Option<ProjectId>;

    fn set_request_id(&mut self, request_id: RequestId);
    fn request_id(&self) -> Option<RequestId>;
}

impl GrpcMetadataMapExt for MetadataMap {
    fn set_project_id(&mut self, project_id: ValidShardedId<ProjectId>) {
        self.insert(
            PROJECT_ID_HEADER,
            project_id
                .to_string()
                .parse()
                .expect("ProjectId is not HTTP header-friendly!"),
        );
    }

    fn set_request_id(&mut self, request_id: RequestId) {
        self.insert(
            REQUEST_ID_HEADER,
            request_id
                .to_string()
                .parse()
                .expect("RequestId is not HTTP header-friendly!"),
        );
    }

    fn project_id(&self) -> Option<ProjectId> {
        self.get(PROJECT_ID_HEADER)
            .and_then(|project_id| project_id.to_str().ok())
            .map(|project_id| ProjectId::from(project_id.to_string()))
    }

    fn request_id(&self) -> Option<RequestId> {
        self.get(REQUEST_ID_HEADER)
            .and_then(|request_id| request_id.to_str().ok())
            .map(|request_id| RequestId::from(request_id.to_string()))
    }
}

pub trait TonicRequestExt {
    fn project_id(&self) -> Result<&ValidShardedId<ProjectId>, tonic::Status>;
    fn request_id(&self) -> Result<&RequestId, tonic::Status>;
    fn context(&self) -> Result<RequestContext, tonic::Status>;
}

impl<T> TonicRequestExt for tonic::Request<T> {
    // ProjectId is added to extensions in CronbackRpcMiddleware
    fn project_id(&self) -> Result<&ValidShardedId<ProjectId>, tonic::Status> {
        let raw_id = self
            .extensions()
            .get::<Result<ValidShardedId<ProjectId>, ModelIdError>>()
            .ok_or_else(|| {
                tonic::Status::internal(format!(
                    "{} was not set in GRPC headers!",
                    PROJECT_ID_HEADER
                ))
            })?;
        match raw_id {
            | Ok(id) => Ok(id),
            | Err(err) => {
                Err(tonic::Status::internal(format!(
                    "Invalid ProjectId was passed in headers: {}",
                    err,
                )))
            }
        }
    }

    fn request_id(&self) -> Result<&RequestId, tonic::Status> {
        self.extensions().get::<RequestId>().ok_or_else(|| {
            tonic::Status::internal(format!(
                "{} was not set in GRPC headers!",
                REQUEST_ID_HEADER,
            ))
        })
    }

    fn context(&self) -> Result<RequestContext, tonic::Status> {
        Ok(RequestContext::new(
            self.request_id()?.clone(),
            self.project_id()?.clone(),
        ))
    }
}

// re-export Option extension for Prost lazy defaults
pub use dto::traits::ProstOptionExt;
