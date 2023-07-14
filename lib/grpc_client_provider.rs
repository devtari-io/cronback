use std::sync::RwLock;

use proto::dispatcher_proto::dispatcher_client::DispatcherClient as GenDispatcherClient;

pub type DispatcherClient = GenDispatcherClient<tonic::transport::Channel>;

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
