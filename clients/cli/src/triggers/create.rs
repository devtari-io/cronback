use anyhow::{anyhow, Result};
use async_trait::async_trait;
use clap::Parser;
use clap_stdin::FileOrStdin;
use tokio::io::AsyncWriteExt;

use crate::args::CommonOptions;
use crate::{confirm_or_abort, emitln, RunCommand};

#[derive(Clone, Debug, Parser)]
pub struct Create {
    /// JSON file name with the trigger definition, or use - for stdin
    file: FileOrStdin<String>,
}

// Three ways to create a trigger
// 1. Command line arguments [future]
// 2. Interactive (prompt) [future]
// 3. Provide a file with the trigger definition [now]
#[async_trait]
impl RunCommand for Create {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
    ) -> Result<()> {
        let json_raw: serde_json::Value = serde_json::from_str(&self.file)
            .map_err(|e| anyhow!("Failed to parse JSON: {e}"))?;

        let colored_req = colored_json::to_colored_json_auto(&json_raw)?;

        emitln!(
            out,
            "You are about to send this trigger definition to be created:"
        );

        emitln!(out, "----");
        emitln!(out, "{colored_req}");
        emitln!(out, "----");

        out.flush().await?;

        confirm_or_abort!(
            common_options,
            "Are you sure you want to create this trigger?",
        );

        let client = common_options.new_client()?;
        let response = client.create_trigger_from_json(json_raw).await?;
        common_options.show_meta(&response, out, err).await?;

        let response = response.into_inner();
        match response {
            | Ok(good) => {
                let name = good.name.clone().expect("name is required");
                let json = serde_json::to_value(good)?;
                let colored = colored_json::to_colored_json_auto(&json)?;
                emitln!(out, "{}", colored);
                emitln!(out);
                emitln!(out, "Trigger '{}' was created successfully", name);
            }
            | Err(bad) => {
                return Err(bad.into());
            }
        };

        Ok(())
    }
}
