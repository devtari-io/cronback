use tonic::service::Interceptor;

use crate::model::ValidShardedId;
use crate::prelude::*;
use crate::types::{ProjectId, RequestId};

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: RequestId,
    pub project_id: ValidShardedId<ProjectId>,
}

impl RequestContext {
    pub fn new(
        request_id: RequestId,
        project_id: ValidShardedId<ProjectId>,
    ) -> Self {
        Self {
            request_id,
            project_id,
        }
    }
}

// Injects tracing headers (x-parent-span-id, and x-cronback-request-id) into
// outgoing gRPC requests
pub struct GrpcRequestInterceptor {
    pub project_id: Option<ValidShardedId<ProjectId>>,
    pub request_id: Option<RequestId>,
}

impl Interceptor for GrpcRequestInterceptor {
    fn call(
        &mut self,
        mut req: tonic::Request<()>,
    ) -> Result<tonic::Request<()>, tonic::Status> {
        if let Some(span_id) = tracing::Span::current().id() {
            let span_id = format!("{}", span_id.into_u64());
            req.metadata_mut()
                .insert(PARENT_SPAN_HEADER, span_id.parse().unwrap());
        }

        if let Some(ref request_id) = self.request_id {
            // Injects request-id to request metadata to avoid sending in every
            // message payload.
            req.metadata_mut().set_request_id(request_id.clone());
        }

        if let Some(ref project_id) = self.project_id {
            // Injects project-id to request metadata to avoid sending in every
            // message payload.
            req.metadata_mut().set_project_id(project_id.clone());
        }
        Ok(req)
    }
}
