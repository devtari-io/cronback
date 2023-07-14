use anyhow::Result;
use clap::clap_derive::Parser;

use crate::args::CommonOptions;

mod api_keys;

#[derive(Parser, Debug, Clone)]
pub enum AdminCommand {
    /// Commands for api key management. This subcommand requires admin
    /// privilliages.
    ApiKeys {
        #[command(subcommand)]
        command: api_keys::ApiKeysCommand,
    },
}

impl AdminCommand {
    pub async fn run<
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
        }
    }
}
