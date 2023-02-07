use anyhow::Result;
use shared::config::CoreConfig;
use tokio::time::{sleep, Duration};
use tracing::*;

pub async fn start_dispatcher(_config: CoreConfig) -> Result<()> {
    for i in 1..3 {
        info!("-> {}", i);
        sleep(Duration::from_secs(1)).await;
    }
    Ok(())
}
