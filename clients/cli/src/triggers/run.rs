use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use cronback_api_model::RunMode;
use spinners::{Spinner, Spinners};

use crate::args::CommonOptions;
use crate::{confirm_or_abort, emitln, RunCommand};

#[derive(Clone, Debug, Parser)]
pub struct RunArgs {
    /// Trigger name
    name: String,

    /// (or --wait) Awaits the run to complete
    #[arg(long, short, alias = "wait")]
    r#await: bool,
}

#[async_trait]
impl RunCommand for RunArgs {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
    ) -> Result<()> {
        confirm_or_abort!(
            common_options,
            "Are you sure you want to run the trigger '{}' immediately?",
            self.name
        );

        let client = common_options.new_client()?;
        let mode = if self.r#await {
            RunMode::Sync
        } else {
            RunMode::Async
        };

        let spinner = if mode == RunMode::Sync {
            Some(Spinner::new(
                Spinners::Dots9,
                "Awaiting the run to complete...".to_owned(),
            ))
        } else {
            None
        };

        let response = client.run_trigger(&self.name, mode).await?;
        if let Some(mut spinner) = spinner {
            spinner.stop_with_message("".to_string());
        }

        common_options.show_meta(&response, out, err).await?;

        let response = response.into_inner();
        match response {
            | Ok(good) => {
                let json = serde_json::to_value(good)?;
                let colored = colored_json::to_colored_json_auto(&json)?;
                emitln!(out, "{}", colored);
            }
            | Err(bad) => {
                return Err(bad.into());
            }
        };
        Ok(())
    }
}
