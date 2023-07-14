use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::clap_derive::Parser;
use cronback_client::ClientBuilder;

use crate::args::CommonOptions;
use crate::client::WrappedClient;

mod api_keys;

const CRONBACK_PROJECT_ID_VAR: &str = "CRONBACK_PROJECT_ID";

#[async_trait]
pub trait RunAdminCommand {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
        admin_options: &AdminOptions,
    ) -> Result<()>;
}

#[derive(Parser, Debug, Clone)]
pub struct AdminOptions {
    // Unfortunately, we can't make this required **and** global at the same
    // time. See [https://github.com/clap-rs/clap/issues/1546]
    #[arg(long, value_name = "PROJECT_ID", env(CRONBACK_PROJECT_ID_VAR))]
    /// The client will act on behalf of this project
    project_id: String,
}

impl AdminOptions {
    pub fn new_admin_client(
        &self,
        opts: &CommonOptions,
    ) -> Result<WrappedClient> {
        let base_url = opts.base_url();
        let inner = ClientBuilder::new()
            .base_url(base_url.clone())
            .context("Error while parsing base url")?
            .secret_token(opts.secret_token.clone())
            .on_behalf_of(self.project_id.clone())
            .build()?;
        Ok(WrappedClient {
            common_options: opts.clone(),
            inner,
        })
    }
}

#[derive(Parser, Debug, Clone)]
pub enum AdminCommand {
    /// Commands for api key management. This subcommand requires admin
    /// privilliages.
    ApiKeys {
        #[command(subcommand)]
        command: api_keys::ApiKeysCommand,
    },
}

#[async_trait]
impl RunAdminCommand for AdminCommand {
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
            | AdminCommand::ApiKeys { command } => {
                command.run(out, err, common_options, admin_options).await
            }
        }
    }
}
