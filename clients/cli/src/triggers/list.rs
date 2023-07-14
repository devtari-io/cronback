use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;

use crate::args::CommonOptions;
use crate::RunCommand;

#[derive(Clone, Debug, Parser)]
pub struct List {}

#[async_trait]
impl RunCommand for List {
    async fn run(&self, _common_options: &CommonOptions) -> Result<()> {
        todo!()
    }
}
