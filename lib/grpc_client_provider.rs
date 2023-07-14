use std::sync::RwLock;

use proto::dispatcher_proto::dispatcher_client::DispatcherClient as GenDispatcherClient;
use tonic::service::Interceptor;

use crate::types::RequestId;

pub type DispatcherClient = GenDispatcherClient<tonic::transport::Channel>;

// Injects tracing headers (parent-span-id, and cronback-request-id) into gRPC
// requests
pub struct GrpcRequestTracingInterceptor(pub RequestId);
impl Interceptor for GrpcRequestTracingInterceptor {
    fn call(
        &mut self,
        mut req: tonic::Request<()>,
    ) -> Result<tonic::Request<()>, tonic::Status> {
        if let Some(span_id) = tracing::Span::current().id() {
            let span_id = format!("{}", span_id.into_u64());
            req.metadata_mut()
                .insert("cronback-parent-span-id", span_id.parse().unwrap());
        }

        // TODO: Consider adding project-id to request metadata whenever
        // possible.
        req.metadata_mut()
            .insert("cronback-request-id", self.0.to_string().parse().unwrap());
        Ok(req)
    }
}

pub struct DispatcherClientProvider {
    inner: RwLock<Option<DispatcherClient>>,
    address: String,
}

impl DispatcherClientProvider {
    pub fn new(address: String) -> Self {
        DispatcherClientProvider {
            inner: Default::default(),
            address,
        }
    }

    pub async fn get_or_create(
        &self,
    ) -> Result<DispatcherClient, tonic::transport::Error> {
        {
            // This mutex will always be acquirable after the first client
            // initialization.
            let c = self.inner.read().unwrap();

            if let Some(ref client) = *c {
                return Ok(client.clone());
            }
        }

        let client = DispatcherClient::connect(self.address.clone()).await?;

        // The client not initalized, upgrade to a write lock
        let mut c = self.inner.write().unwrap();

        // Between releasing the read lock and acquiring the write lock, someone
        // might have already initialized it. In that case, let's just reuse
        // their client
        if let Some(ref client) = *c {
            return Ok(client.clone());
        }

        *c = Some(client.clone());

        Ok(client)
    }
}
