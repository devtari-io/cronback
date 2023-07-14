use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use colored::Colorize;

use crate::args::CommonOptions;
use crate::{emitln, RunCommand};
#[derive(Parser, Debug, Clone)]
pub struct WhoAmI {
    #[arg(long)]
    /// Reveal the configured API secret token
    show_secret_token: bool,
}

#[async_trait]
impl RunCommand for WhoAmI {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
    ) -> Result<()> {
        emitln!(
            out,
            "Cronback Service: {}",
            common_options.base_url().to_string().green()
        );
        match common_options.secret_token() {
            | Ok(token) if self.show_secret_token => {
                emitln!(out, "Secret Token: {}", token.yellow());
            }
            | Ok(_) => {}
            | Err(_) => {
                emitln!(err, "{}", "WARNING: NO API SECRET TOKEN IS SET".red());
            }
        }

        Ok(())
    }
}
