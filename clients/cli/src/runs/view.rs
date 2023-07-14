use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;

use crate::args::CommonOptions;
use crate::{emitln, RunCommand};

#[derive(Clone, Debug, Parser)]
pub struct View {
    /// Run Id
    id: String,
}

#[async_trait]
impl RunCommand for View {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
    ) -> Result<()> {
        let client = common_options.new_client()?;
        let response = client.get_run(&self.id).await?;
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
