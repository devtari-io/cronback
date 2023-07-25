use anyhow::Result;
use async_trait::async_trait;
use clap::clap_derive::Parser;

use crate::args::CommonOptions;
use crate::Command;

mod create;

#[derive(Parser, Debug, Clone)]
pub enum ProjectsCommand {
    /// Create a new API key
    Create(create::Create),
}

#[async_trait]
impl Command for ProjectsCommand {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
    ) -> Result<()> {
        match self {
            | ProjectsCommand::Create(c) => {
                c.run(out, err, common_options).await
            }
        }
    }
}
