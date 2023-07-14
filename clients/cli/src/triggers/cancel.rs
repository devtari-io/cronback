use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;

use crate::args::CommonOptions;
use crate::ui::FancyToString;
use crate::{confirm_or_abort, emitln, RunCommand};

#[derive(Clone, Debug, Parser)]
pub struct Cancel {
    /// Trigger name
    name: String,
    /// Ignore the confirmation prompt and always answer "yes"
    #[arg(long, short)]
    yes: bool,
}

#[async_trait]
impl RunCommand for Cancel {
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
            self,
            "Are you sure you want to cancel the trigger '{}'?",
            self.name
        );

        let client = common_options.new_client()?;
        let response = client.cancel_trigger(&self.name).await?;
        common_options.show_meta(&response, out, err).await?;

        let response = response.into_inner();
        match response {
            | Ok(trigger) => {
                emitln!(
                    out,
                    "Trigger '{}' is now {}!",
                    self.name,
                    trigger.status.fancy(),
                );
            }
            | Err(bad) => {
                return Err(bad.into());
            }
        };
        Ok(())
    }
}
