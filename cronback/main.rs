use anyhow::Result;
use cronback::Cronback;
use cronback_services::api::ApiService;
use cronback_services::dispatcher::DispatcherService;
use cronback_services::metadata::MetadataService;
use cronback_services::scheduler::SchedulerService;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    // Registering services.
    <(
        ApiService,
        DispatcherService,
        SchedulerService,
        MetadataService,
    )>::run_cronback()
    .await
}
