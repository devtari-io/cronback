use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;

use crate::args::CommonOptions;
use crate::{confirm_or_abort, emitln, Command};

#[derive(Clone, Debug, Parser)]
pub struct Delete {
    /// Trigger name
    name: String,
}

#[async_trait]
impl Command for Delete {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        _err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
    ) -> Result<()> {
        confirm_or_abort!(
            common_options,
            "Are you sure you want to permanently delete the trigger '{}'?",
            self.name
        );

        let client = common_options.new_client()?;
        let response = cronback::triggers::delete(&client, &self.name).await?;

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
