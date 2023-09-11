use anyhow::Result;
use cling::prelude::*;

use crate::args::CommonOptions;
use crate::confirm_or_abort;
use crate::ui::FancyToString;

#[derive(Run, Collect, Args, Clone, Debug)]
#[cling(run = "pause")]
pub struct Pause {
    /// Trigger name
    name: String,
}

async fn pause(common_options: &CommonOptions, opts: &Pause) -> Result<()> {
    confirm_or_abort!(
        common_options,
        "Are you sure you want to pause the trigger '{}'?",
        opts.name
    );

    let client = common_options.new_client()?;
    let response =
        cronback_client::triggers::pause(&client, &opts.name).await?;

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
