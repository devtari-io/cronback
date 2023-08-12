use std::sync::Arc;
use std::time::Duration;

use futures::Future;
use moka::future::Cache;
use moka::Expiry;
use thiserror::Error;

use crate::clients::ScopedMetadataSvcClient;
use crate::model::ValidShardedId;
use crate::types::{ProjectId, RequestId};
use crate::GrpcClientFactory;

type MetadataClientFactory = Arc<
    Box<dyn GrpcClientFactory<ClientType = ScopedMetadataSvcClient> + Send>,
>;

#[derive(Error, Debug, Clone)]
pub enum ProjectSettingError {
    // Optimally this should wrap a `GrpcClientError` but unfortauntly, it's
    // not `Clone` and it's not possible to make it Clone. Error being `Clone`
    // is a requirement for the cache though.
    #[error("failed to create metadata svc client: {0}")]
    Client(String),
    #[error("setting grpc called failed with code {0}")]
    Server(#[from] tonic::Status),
}

/// A TTL-based read-through async project setting cache. A single instance of
/// this struct caches only a single setting type.
///
/// Reading from the cache will return the value immediately if it exists, and
/// will fetch it if it doesn't exist. Notes:
/// 1. If multiple concurrent callers request the same uncached value, only one
///    of them will fetch it and the rest will wait for the result.
/// 2. If the cached value exceeded the TTL, it will get refetched on the next
///    read.
/// 3. If the fetch from source fails, waiting callers will get fulfilled with
///    the failed Result. However, the value will get refetched on the next read
///    attempt (even if it didn't exceed the TTL).
/// 4. Cache evictions happen in the background in a separate thread pool.
pub struct ProjectSetting<V, F> {
    client_factory: MetadataClientFactory,
    fetcher: F,
    cache: Cache<ProjectId, Result<V, ProjectSettingError>>,
}

impl<V, F, Fut> ProjectSetting<V, F>
where
    V: Send + Clone + Sync + 'static,
    F: Fn(&ValidShardedId<ProjectId>, ScopedMetadataSvcClient) -> Fut,
    Fut: Future<Output = Result<V, tonic::Status>>,
{
    pub fn new(
        client_factory: MetadataClientFactory,
        fetcher: F,
        ttl: Duration,
    ) -> Self {
        Self {
            client_factory,
            fetcher,
            cache: Cache::builder()
                .expire_after(ExpirationPolicy { ttl })
                .build(),
        }
    }

    /// Returns the setting value if it's cached, and requests it if it's not in
    /// the cache. If multiple concurrent calls attempt to read the same key,
    /// it'll only be fetched once and returned to all the callers.
    pub async fn get(
        &self,
        project_id: &ValidShardedId<ProjectId>,
    ) -> Result<V, ProjectSettingError> {
        self.cache
            .get_with_by_ref(project_id.inner(), async move {
                let client = self
                    .client_factory
                    .get_client(&RequestId::new(), project_id)
                    .await
                    .map_err(|e| ProjectSettingError::Client(e.to_string()))?;
                Ok((self.fetcher)(project_id, client).await?)
            })
            .await
    }

    /// Returns the value if it exists in the cache and didn't expire, and None
    /// otherwise.
    pub async fn get_cached(
        &self,
        project_id: &ValidShardedId<ProjectId>,
    ) -> Option<Result<V, ProjectSettingError>> {
        self.cache.get(project_id)
    }
}

struct ExpirationPolicy {
    ttl: Duration,
}

impl<V> Expiry<ProjectId, Result<V, ProjectSettingError>> for ExpirationPolicy {
    fn expire_after_create(
        &self,
        _key: &ProjectId,
        value: &Result<V, ProjectSettingError>,
        _current_time: std::time::Instant,
    ) -> Option<Duration> {
        if value.is_ok() {
            Some(self.ttl)
        } else {
            // If the cached value is an error, let it expire asap.
            Some(Duration::default())
        }
    }
}
