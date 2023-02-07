use anyhow::Result;
use chrono::Utc;
use saffron::Cron;
use shared::config::CoreConfig;
use tokio::time::{sleep, Duration};
use tracing::*;

pub async fn start_scheduler(_config: CoreConfig) -> Result<()> {
    // println!("sk_{} !", Uuid::new_v4().as_simple());
    let cron_expr: Cron = "2 4 * * *"
        .parse()
        .expect("Failed to parse cron expression");
    for datetime in cron_expr.iter_after(Utc::now()).take(10) {
        info!("-> {}", datetime);
        sleep(Duration::from_secs(1)).await;
    }
    Ok(())
}
