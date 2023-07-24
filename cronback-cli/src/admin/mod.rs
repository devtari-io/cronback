use anyhow::Result;
use async_trait::async_trait;
use clap::clap_derive::Parser;

use crate::args::CommonOptions;
use crate::Command;

mod api_keys;
mod projects;

#[derive(Parser, Debug, Clone)]
pub enum AdminCommand {
    /// Commands for api key management. This subcommand requires admin
    /// privilliages.
    ApiKeys {
        #[command(subcommand)]
        command: api_keys::ApiKeysCommand,
    },

    Projects {
        #[command(subcommand)]
        command: projects::ProjectsCommand,
    },
}

#[async_trait]
impl Command for AdminCommand {
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
            | AdminCommand::ApiKeys { command } => {
                command.run(out, err, common_options).await
            }
            | AdminCommand::Projects { command } => {
                command.run(out, err, common_options, admin_options).await
            }
        }
    }
}
