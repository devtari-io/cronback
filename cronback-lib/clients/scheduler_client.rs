use std::collections::HashMap;

use async_trait::async_trait;
use derive_more::{Deref, DerefMut};
use proto::scheduler_svc::scheduler_svc_client::SchedulerSvcClient as GenSchedulerSvcClient;
use tonic::codegen::InterceptedService;

use crate::config::MainConfig;
use crate::grpc_client_provider::{GrpcClientType, ScopedGrpcClient};
use crate::grpc_helpers::GrpcRequestInterceptor;
use crate::prelude::ValidShardedId;
use crate::types::{ProjectId, RequestId};

type SchedulerSvcClient = GenSchedulerSvcClient<
    InterceptedService<tonic::transport::Channel, GrpcRequestInterceptor>,
>;

#[derive(Deref, DerefMut)]
pub struct ScopedSchedulerSvcClient(ScopedGrpcClient<SchedulerSvcClient>);

#[async_trait]
impl GrpcClientType for ScopedSchedulerSvcClient {
    type RawGrpcClient = SchedulerSvcClient;

    fn get_mut(&mut self) -> &mut ScopedGrpcClient<Self::RawGrpcClient> {
        &mut self.0
    }

    fn address_map(config: &MainConfig) -> &HashMap<u64, String> {
        &config.scheduler_cell_map
    }

    fn create_scoped_client(
        project_id: ValidShardedId<ProjectId>,
        request_id: RequestId,
        channel: tonic::transport::Channel,
        interceptor: GrpcRequestInterceptor,
    ) -> Self {
        let client =
            GenSchedulerSvcClient::with_interceptor(channel, interceptor);

        Self(ScopedGrpcClient::new(project_id, request_id, client))
    }
}
