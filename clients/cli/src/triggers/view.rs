use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;

use crate::args::CommonOptions;
use crate::{emitln, Command};

#[derive(Clone, Debug, Parser)]
pub struct View {
    /// Trigger name
    name: String,
}

#[async_trait]
impl Command for View {
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
        let response = cronback::triggers::get(&client, &self.name).await?;

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
