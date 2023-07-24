use anyhow::Result;
use async_trait::async_trait;
use clap::clap_derive::Parser;

use crate::args::CommonOptions;
use crate::Command;

mod create;
mod list;
mod revoke;

#[derive(Parser, Debug, Clone)]
pub enum ApiKeysCommand {
    /// List API keys
    #[command(visible_alias = "ls")]
    List(list::List),
    /// Create a new API key
    Create(create::Create),
    /// Revokes an API key
    Revoke(revoke::Revoke),
}

#[async_trait]
impl Command for ApiKeysCommand {
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
            | ApiKeysCommand::List(c) => c.run(out, err, common_options).await,
            | ApiKeysCommand::Create(c) => {
                c.run(out, err, common_options).await
            }
            | ApiKeysCommand::Revoke(c) => {
                c.run(out, err, common_options).await
            }
        }
    }
}
