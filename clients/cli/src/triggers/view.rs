use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;

use crate::args::CommonOptions;
use crate::RunCommand;

#[derive(Clone, Debug, Parser)]
pub struct View {}

#[async_trait]
impl RunCommand for View {
    async fn run(&self, _common_options: &CommonOptions) -> Result<()> {
        todo!()
    }
}
