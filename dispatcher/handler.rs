use chrono::Utc;
use metrics::counter;
use tonic::{Request, Response, Status};

use chrono_tz::UTC;
use proto::dispatcher_proto::{
    dispatcher_server::Dispatcher, DispatchRequest, DispatchResponse,
};
use shared::{
    service::ServiceContext,
    types::{
        Emit, Invocation, InvocationId, InvocationStatus, OwnerId, TriggerId,
        WebhookStatus,
    },
};

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

        let owner_id = OwnerId::from(request.owner_id);
        let invocation_id = InvocationId::new(&owner_id);

        let invocation = Invocation {
            id: invocation_id.clone(),
            trigger_id: request.trigger_id.into(),
            owner_id,
            created_at: Utc::now().with_timezone(&UTC),
            payload: request.payload.unwrap().into(),
            status: request
                .emits
                .into_iter()
                .map(|e|
                    match Emit::from(e) {
                        | Emit::Webhook(webhook) => {
                            InvocationStatus::WebhookStatus(WebhookStatus {
                                webhook,
                                delivery_status: shared::types::WebhookDeliveryStatus::Attempting,
                            })
                        }
                    }
                )
                .collect(),
        };

        counter!("dispatcher.invocations_total", 1);
        self.dispatch_manager
            .register_invocation(invocation)
            .unwrap();

        Ok(Response::new(DispatchResponse {
            invocation_id: invocation_id.to_string(),
        }))
    }
}
