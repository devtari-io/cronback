use tonic::{Request, Response, Status};

use crate::{validators, webhook};
use proto::{
    dispatcher_proto::{
        dispatcher_server::Dispatcher, DispatchEventRequest, DispatchEventResponse,
    },
    event_proto::EventInstanceStatus,
    trigger_proto::endpoint::Endpoint,
};
use shared::service::ServiceContext;

pub(crate) struct DispatcherAPIHandler {
    #[allow(unused)]
    context: ServiceContext,
}

impl DispatcherAPIHandler {
    pub fn new(context: ServiceContext) -> Self {
        Self { context }
    }
}

#[tonic::async_trait]
impl Dispatcher for DispatcherAPIHandler {
    async fn dispatch_event(
        &self,
        request: Request<DispatchEventRequest>,
    ) -> Result<Response<DispatchEventResponse>, Status> {
        let (_metadata, _extensions, request) = request.into_parts();

        let event = request.event.expect("event must be set to a value");
        let event_request = event.request.expect("An event must have a request set");

        if let Err(e) = validators::validate_dispatch_request(&event_request) {
            return Ok(Response::new(DispatchEventResponse {
                status: EventInstanceStatus::InvalidRequest.into(),
                response: None,
                error_message: Some(format!("Invalid request: {e}")),
            }));
        }

        let endpoint = event_request
            .endpoint
            .as_ref()
            .unwrap()
            .endpoint
            .as_ref()
            .unwrap();

        let response = match endpoint {
            Endpoint::Webhook(w) => {
                webhook::dispatch_webhook(
                    w,
                    event_request.request_payload.as_ref().unwrap(),
                    event_request.timeout.as_ref().unwrap(),
                )
                .await
            }
        };
        // TODO: Send notifications here

        Ok(Response::new(response))
    }
}
