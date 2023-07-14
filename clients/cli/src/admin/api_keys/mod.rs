use anyhow::Result;
use clap::clap_derive::Parser;

use crate::args::CommonOptions;
use crate::RunCommand;

mod create;
mod list;
mod revoke;

#[derive(Parser, Debug, Clone)]
pub enum ApiKeysCommand {
    /// List api keys
    #[command(visible_alias = "ls")]
    List(list::List),
    /// Create a new api key
    Create(create::Create),
    /// Revokes an API key
    Revoke(revoke::Revoke),
}

impl ApiKeysCommand {
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
