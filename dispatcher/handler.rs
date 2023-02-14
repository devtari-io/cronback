use std::sync::Arc;

use proto::{
    dispatcher_proto::{
        dispatcher_server::Dispatcher, DispatchEventRequest, DispatchEventResponse,
    },
    event_proto::EventInstanceStatus,
    trigger_proto::endpoint::Endpoint,
};
use tonic::{Request, Response, Status};

use shared::config::ConfigLoader;

use crate::{validators, webhook};

pub(crate) struct DispatcherAPIHandler {
    pub config_loader: Arc<ConfigLoader>,
}

#[tonic::async_trait]
impl Dispatcher for DispatcherAPIHandler {
    async fn dispatch_event(
        &self,
        request: Request<DispatchEventRequest>,
    ) -> Result<Response<DispatchEventResponse>, Status> {
        let event = request.get_ref().event.as_ref().unwrap();
        let event_request = event.request.as_ref().unwrap();

        if let Err(e) = validators::validate_dispatch_request(event_request) {
            return Ok(Response::new(DispatchEventResponse {
                status: EventInstanceStatus::InvalidRequest.into(),
                response: None,
                error_message: Some(format!("Invalid request: {}", e)),
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
            Endpoint::Webhook(w) => webhook::dispatch_webhook(
                w,
                event_request.request_payload.as_ref().unwrap(),
                event_request.timeout.as_ref().unwrap(),
            ).await,
        };
        // TODO: Send notifications here

        Ok(Response::new(response))
    }
}
