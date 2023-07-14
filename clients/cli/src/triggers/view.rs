use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use colored::Colorize;

use crate::args::CommonOptions;
use crate::{emitln, RunCommand};

#[derive(Clone, Debug, Parser)]
pub struct View {
    name: String,
    #[arg(long)]
    extended: bool,
    //output_format: Option<Formatting>,
}

#[async_trait]
impl RunCommand for View {
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
        let response = client.get_trigger(&self.name).await?;
        if self.extended {
            // Print extra information.
            emitln!(
                out,
                "{}",
                "-------------------------------------------------".green()
            );
            emitln!(out, "Status Code: {}", response.status_code());
            emitln!(
                out,
                "Request Id: {}",
                response.request_id().clone().unwrap_or_default().green()
            );
            emitln!(
                out,
                "Project Id: {}",
                response.project_id().clone().unwrap_or_default().green()
            );
            emitln!(
                out,
                "{}",
                "-------------------------------------------------".green()
            );
        }

        emitln!(out);
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
