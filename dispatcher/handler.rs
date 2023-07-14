use chrono::Utc;
use chrono_tz::UTC;
use lib::model::ModelId;
use lib::service::ServiceContext;
use lib::types::{
    Emit,
    Invocation,
    InvocationId,
    InvocationStatus,
    ProjectId,
    WebhookStatus,
};
use metrics::counter;
use proto::dispatcher_proto::dispatcher_server::Dispatcher;
use proto::dispatcher_proto::{DispatchRequest, DispatchResponse};
use tonic::{Request, Response, Status};

use crate::dispatch_manager::DispatchManager;

pub(crate) struct DispatcherAPIHandler {
    #[allow(unused)]
    context: ServiceContext,
    dispatch_manager: DispatchManager,
}

impl DispatcherAPIHandler {
    pub fn new(
        context: ServiceContext,
        dispatch_manager: DispatchManager,
    ) -> Self {
        Self {
            context,
            dispatch_manager,
        }
    }
}

#[tonic::async_trait]
impl Dispatcher for DispatcherAPIHandler {
    async fn dispatch(
        &self,
        request: Request<DispatchRequest>,
    ) -> Result<Response<DispatchResponse>, Status> {
        let (_metadata, _extensions, request) = request.into_parts();

        let dispatch_mode = request.mode();
        let project = ProjectId::from(request.project_id).validated()?;
        let invocation_id = InvocationId::generate(&project);

        let invocation = Invocation {
            id: invocation_id.into(),
            trigger: request.trigger_id.into(),
            project,
            created_at: Utc::now().with_timezone(&UTC),
            payload: request.payload.map(|p| p.into()),
            status: request
                .emits
                .into_iter()
                .map(|e| {
                    match Emit::from(e) {
                        | Emit::Webhook(webhook) => {
                            InvocationStatus::WebhookStatus(WebhookStatus {
                                webhook,
                                delivery_status: lib::types::WebhookDeliveryStatus::Attempting,
                            })
                        }
                        | Emit::Event(_) => unimplemented!(),
                    }
                })
                .collect(),
        };

        counter!("dispatcher.invocations_total", 1);
        let invocation = self
            .dispatch_manager
            .invoke(invocation, dispatch_mode)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DispatchResponse {
            invocation: Some(invocation.into()),
        }))
    }
}
