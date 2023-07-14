use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;

use crate::args::CommonOptions;
use crate::ui::FancyToString;
use crate::{emitln, Command};

#[derive(Clone, Debug, Parser)]
pub struct Resume {
    /// Trigger name
    name: String,
}

#[async_trait]
impl Command for Resume {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        _err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
    ) -> Result<()> {
        let client = common_options.new_client()?;
        let response =
            cronback_client::triggers::resume(&client, &self.name).await?;

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
