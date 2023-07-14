use std::collections::HashMap;

use async_trait::async_trait;
use derive_more::{Deref, DerefMut};
use proto::dispatcher_proto::dispatcher_client::DispatcherClient as GenDispatcherClient;
use tonic::codegen::InterceptedService;

use crate::config::MainConfig;
use crate::grpc_client_provider::{GrpcClientType, ScopedGrpcClient};
use crate::grpc_helpers::GrpcRequestInterceptor;
use crate::prelude::ValidShardedId;
use crate::types::{ProjectId, RequestId};

type DispatcherClient = GenDispatcherClient<
    InterceptedService<tonic::transport::Channel, GrpcRequestInterceptor>,
>;

#[derive(Deref, DerefMut)]
pub struct ScopedDispatcherClient(ScopedGrpcClient<DispatcherClient>);

#[async_trait]
impl GrpcClientType for ScopedDispatcherClient {
    type RawGrpcClient = DispatcherClient;

    fn get_mut(&mut self) -> &mut ScopedGrpcClient<Self::RawGrpcClient> {
        &mut self.0
    }

    fn address_map(config: &MainConfig) -> &HashMap<u64, String> {
        &config.dispatcher_cell_map
    }

    fn create_scoped_client(
        project_id: ValidShardedId<ProjectId>,
        request_id: RequestId,
        channel: tonic::transport::Channel,
        interceptor: GrpcRequestInterceptor,
    ) -> Self {
        let client =
            GenDispatcherClient::with_interceptor(channel, interceptor);

        Self(ScopedGrpcClient::new(project_id, request_id, client))
    }
}
