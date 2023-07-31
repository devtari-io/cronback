use anyhow::Result;
use clap::Parser;
use cling::prelude::*;

use crate::args::CommonOptions;
use crate::ui::FancyToString;

#[derive(CliRunnable, CliParam, Clone, Debug, Parser)]
#[cling(run = "resume")]
pub struct Resume {
    /// Trigger name
    name: String,
}

async fn resume(common_options: &CommonOptions, opts: &Resume) -> Result<()> {
    let client = common_options.new_client()?;
    let response =
        cronback_client::triggers::resume(&client, &opts.name).await?;

    let response = response.into_inner();
    match response {
        | Ok(trigger) => {
            println!(
                "Trigger '{}' is now {}!",
                opts.name,
                trigger.status.fancy(),
            );
        }
        | Err(bad) => {
            return Err(bad.into());
        }
    };
    Ok(())
}
