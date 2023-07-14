use anyhow::Result;
use async_trait::async_trait;

use crate::args::CommonOptions;

#[async_trait]
pub trait RunCommand {
    async fn run(&self, common_options: &CommonOptions) -> Result<()>;
}
