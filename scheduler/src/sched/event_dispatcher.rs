use std::sync::Arc;

use proto::dispatcher_proto::DispatchRequest;
use shared::{grpc_client_provider::DispatcherClientProvider, types::Trigger};

pub(crate) struct DispatchJob {
    dispatch_request: DispatchRequest,
    dispatcher_client_provider: Arc<DispatcherClientProvider>,
}

impl DispatchJob {
    pub fn from_trigger(
        trigger: Trigger,
        dispatcher_client_provider: Arc<DispatcherClientProvider>,
    ) -> Self {
        Self {
            dispatch_request: DispatchRequest {
                trigger_id: trigger.id.to_string(),
                owner_id: trigger.owner_id.to_string(),
                emits: trigger
                    .emit
                    .into_iter()
                    .map(|e| e.into())
                    .collect::<Vec<_>>(),
                payload: Some(trigger.payload.into()),
                on_success: None, // TODO
                on_failure: None, // TODO
            },
            dispatcher_client_provider,
        }
    }

    pub fn id(&self) -> &str {
        &self.dispatch_request.trigger_id
    }

    pub async fn run(self) {
        // TODO: How to handle infra failures?
        let mut client = self
            .dispatcher_client_provider
            .get_or_create()
            .await
            .unwrap();

        loop {
            // Retry forever until the dispatcher accepts our message.
            // TODO: Obviously that's not ideal

            let resp = client.dispatch(self.dispatch_request.clone()).await;
            if resp.is_ok() {
                break;
            }
        }
    }
}
