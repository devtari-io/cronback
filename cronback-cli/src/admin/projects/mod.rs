use anyhow::Result;
use async_trait::async_trait;
use clap::clap_derive::Parser;

use super::{AdminOptions, RunAdminCommand};
use crate::args::CommonOptions;

mod create;

#[derive(Parser, Debug, Clone)]
pub enum ProjectsCommand {
    /// Create a new API key
    Create(create::Create),
}

#[async_trait]
impl RunAdminCommand for ProjectsCommand {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
        admin_options: &AdminOptions,
    ) -> Result<()> {
        match self {
            | ProjectsCommand::Create(c) => {
                c.run(out, err, common_options, admin_options).await
            }
        }
    }
}
