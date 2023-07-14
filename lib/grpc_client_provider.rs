use std::collections::HashMap;
use std::str::FromStr;
use std::sync::RwLock;

use async_trait::async_trait;
use derive_more::{Deref, DerefMut};
use thiserror::Error;
use tonic::transport::{Channel, Endpoint};

use crate::config::MainConfig;
use crate::model::ValidShardedId;
use crate::prelude::{GrpcRequestInterceptor, Shard};
use crate::service::ServiceContext;
use crate::types::{ProjectId, RequestId};

#[derive(Debug, Error)]
pub enum GrpcClientError {
    #[error(transparent)]
    Connect(#[from] tonic::transport::Error),
    #[error("Internal data routing error: {0}")]
    Routing(String),
    #[error("Malformed grpc endpoint address: {0}")]
    BadAddress(String),
}

// Wraps a raw gRPC client with the project ID and request ID, allows users to
// access project_id and request_id at any time.
//
// Deref/DerefMut allow ScopedGrpcClient to be used as a T (smart pointer-like)
#[derive(Deref, DerefMut)]
pub struct ScopedGrpcClient<T> {
    pub project_id: ValidShardedId<ProjectId>,
    pub request_id: RequestId,
    #[deref]
    #[deref_mut]
    inner: T,
}

impl<T> ScopedGrpcClient<T> {
    pub fn new(
        project_id: ValidShardedId<ProjectId>,
        request_id: RequestId,
        inner: T,
    ) -> Self {
        Self {
            project_id,
            request_id,
            inner,
        }
    }
}

#[async_trait]
pub trait GrpcClientType: Sync + Send {
    type RawGrpcClient;

    fn create_scoped_client(
        project_id: ValidShardedId<ProjectId>,
        request_id: RequestId,
        channel: tonic::transport::Channel,
        interceptor: GrpcRequestInterceptor,
    ) -> Self;

    fn get_mut(&mut self) -> &mut ScopedGrpcClient<Self::RawGrpcClient>;

    // Concrete default implementations
    async fn create_channel(
        address: &str,
    ) -> Result<tonic::transport::Channel, GrpcClientError> {
        let channel = Endpoint::from_str(address)
            .map_err(|_| GrpcClientError::BadAddress(address.to_string()))?
            .connect()
            .await?;
        Ok(channel)
    }

    fn address_map(config: &MainConfig) -> &HashMap<u64, String>;

    fn get_address(
        config: &MainConfig,
        _project_id: &ValidShardedId<ProjectId>,
    ) -> Result<String, GrpcClientError> {
        // For now, we'll assume everything is on Cell 0
        // TODO: support multiple cells
        let shard = Shard(0);

        let address =
            Self::address_map(config).get(&shard.0).ok_or_else(|| {
                GrpcClientError::Routing(format!(
                    "No endpoint was found for shard {shard} in config (grpc \
                     client type {:?})",
                    std::any::type_name::<Self>(),
                ))
            })?;
        Ok(address.clone())
    }
}

#[async_trait]
pub trait GrpcClientFactory: Send + Sync {
    type ClientType;
    async fn get_client(
        &self,
        request_id: &RequestId,
        project_id: &ValidShardedId<ProjectId>,
    ) -> Result<Self::ClientType, GrpcClientError>;
}

// A concrete channel-caching implementation of the GrpcClientFactory used in
// production.
pub struct GrpcClientProvider<T> {
    service_context: ServiceContext,
    channel_cache: RwLock<HashMap<String, Channel>>,
    phantom: std::marker::PhantomData<T>,
}

impl<T: GrpcClientType> GrpcClientProvider<T> {
    pub fn new(service_context: ServiceContext) -> Self {
        Self {
            service_context,
            channel_cache: Default::default(),
            phantom: Default::default(),
        }
    }
}

#[async_trait]
impl<T: GrpcClientType> GrpcClientFactory for GrpcClientProvider<T> {
    type ClientType = T;

    async fn get_client(
        &self,
        request_id: &RequestId,
        project_id: &ValidShardedId<ProjectId>,
    ) -> Result<Self::ClientType, GrpcClientError> {
        // resolve shard -> cell
        let address = T::get_address(
            &self.service_context.get_config().main,
            project_id,
        )?;

        let mut channel = None;
        {
            let cache = self.channel_cache.read().unwrap();
            if let Some(ch) = cache.get(&address) {
                channel = Some(ch.clone());
            }
        }
        if channel.is_none() {
            // We attempt to create a new channel anyway because we don't want
            // to block the write lock during connection.
            let temp_new_ch = T::create_channel(&address).await?;
            {
                // Only upgrade to a write lock if we need to create a new
                let mut cache = self.channel_cache.write().unwrap();
                // check again, someone might have already created the channel
                if let Some(ch) = cache.get(&address) {
                    channel = Some(ch.clone());
                    // temp_new_ch dropped here
                } else {
                    cache.insert(address.clone(), temp_new_ch.clone());
                    channel = Some(temp_new_ch);
                }
            }
        }

        assert!(channel.is_some());

        let interceptor = GrpcRequestInterceptor {
            project_id: Some(project_id.clone()),
            request_id: Some(request_id.clone()),
        };

        Ok(T::create_scoped_client(
            project_id.clone(),
            request_id.clone(),
            channel.unwrap(),
            interceptor,
        ))
    }
}

pub mod test_helpers {
    use std::sync::Arc;

    use hyper::Uri;
    use tempfile::TempPath;
    use tokio::net::UnixStream;
    use tower::service_fn;

    use super::*;
    // An implementation of the GrpcClientFactory used in tests that uses unix
    // domain socket.
    pub struct TestGrpcClientProvider<T> {
        cell_to_socket_path: HashMap<u16, Arc<TempPath>>,
        channel_cache: RwLock<HashMap<u16, Channel>>,
        phantom: std::marker::PhantomData<T>,
    }

    impl<T: GrpcClientType> TestGrpcClientProvider<T> {
        pub fn new(cell_to_socket_path: HashMap<u16, Arc<TempPath>>) -> Self {
            Self {
                cell_to_socket_path,
                channel_cache: Default::default(),
                phantom: Default::default(),
            }
        }

        pub fn new_single_shard(socket_path: Arc<TempPath>) -> Self {
            let mut cell_to_socket_path = HashMap::with_capacity(1);
            cell_to_socket_path.insert(0, socket_path);

            Self {
                cell_to_socket_path,
                channel_cache: Default::default(),
                phantom: Default::default(),
            }
        }
    }

    #[async_trait]
    impl<T: GrpcClientType> GrpcClientFactory for TestGrpcClientProvider<T> {
        type ClientType = T;

        async fn get_client(
            &self,
            request_id: &RequestId,
            project_id: &ValidShardedId<ProjectId>,
        ) -> Result<Self::ClientType, GrpcClientError> {
            // do we have a channel in cache?
            // TODO: support multiple cells
            let _shard = Shard(0);
            let cell: u16 = 0;

            let socket = self
                .cell_to_socket_path
                .get(&cell)
                .expect("Cell not found!")
                .clone();

            let mut channel = None;
            {
                let cache = self.channel_cache.read().unwrap();
                if let Some(ch) = cache.get(&cell) {
                    channel = Some(ch.clone());
                }
            }
            if channel.is_none() {
                // We attempt to create a new channel anyway because we don't
                // want to block the write lock during
                // connection.
                // Connect to the server over a Unix socket. The URL will be
                // ignored.
                let temp_new_ch = Endpoint::try_from("http://example.url")
                    .unwrap()
                    .connect_with_connector(service_fn(move |_: Uri| {
                        let socket = Arc::clone(&socket);
                        async move { UnixStream::connect(&*socket).await }
                    }))
                    .await?;
                {
                    // Only upgrade to a write lock if we need to create a new
                    let mut cache = self.channel_cache.write().unwrap();
                    // check again, someone might have already created the
                    // channel
                    if let Some(ch) = cache.get(&cell) {
                        channel = Some(ch.clone());
                        // temp_new_ch dropped here
                    } else {
                        cache.insert(cell, temp_new_ch.clone());
                        channel = Some(temp_new_ch);
                    }
                }
            }

            assert!(channel.is_some());

            let interceptor = GrpcRequestInterceptor {
                project_id: Some(project_id.clone()),
                request_id: Some(request_id.clone()),
            };

            Ok(T::create_scoped_client(
                project_id.clone(),
                request_id.clone(),
                channel.unwrap(),
                interceptor,
            ))
        }
    }
}
