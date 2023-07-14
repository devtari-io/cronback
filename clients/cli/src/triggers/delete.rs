use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;

use crate::args::CommonOptions;
use crate::{confirm_or_abort, emitln, RunCommand};

#[derive(Clone, Debug, Parser)]
pub struct Delete {
    /// Trigger name
    name: String,
    /// Ignore the confirmation prompt and always answer "yes"
    #[arg(long, short)]
    yes: bool,
}

#[async_trait]
impl RunCommand for Delete {
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
            "Are you sure you want to permanently delete the trigger '{}'?",
            self.name
        );

        let client = common_options.new_client()?;
        let response = client.delete_trigger(&self.name).await?;
        common_options.show_meta(&response, out, err).await?;

        let response = response.into_inner();
        match response {
            | Ok(_) => {
                emitln!(out, "Trigger '{}' has been deleted!", self.name);
            }
            | Err(bad) => {
                return Err(bad.into());
            }
        };

        Ok(())
    }
}
