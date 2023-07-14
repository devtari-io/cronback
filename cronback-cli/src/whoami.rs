use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use colored::Colorize;

use crate::args::CommonOptions;
use crate::{emitln, Command};
#[derive(Parser, Debug, Clone)]
pub struct WhoAmI {
    #[arg(long)]
    /// Reveal the configured API secret token
    show_secret_token: bool,
}

#[async_trait]
impl Command for WhoAmI {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        _err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
    ) -> Result<()> {
        emitln!(
            out,
            "Cronback Service: {}",
            common_options.base_url().to_string().green()
        );
        emitln!(
            out,
            "Secret Token: {}",
            common_options.secret_token.yellow()
        );

        Ok(())
    }
}
