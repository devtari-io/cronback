use anyhow::Result;
use cling::prelude::*;
use cronback_api_model::RunMode;
use spinners::{Spinner, Spinners};

use crate::args::CommonOptions;
use crate::confirm_or_abort;

#[derive(Run, Collect, Args, Clone, Debug)]
#[cling(run = "run")]
pub struct RunArgs {
    /// Trigger name
    name: String,

    /// (or --wait) Awaits the run to complete
    #[arg(long, short, alias = "wait")]
    r#await: bool,
}

async fn run(common_options: &CommonOptions, opts: &RunArgs) -> Result<()> {
    confirm_or_abort!(
        common_options,
        "Are you sure you want to run the trigger '{}' immediately?",
        opts.name
    );

    let client = common_options.new_client()?;
    let mode = if opts.r#await {
        RunMode::Sync
    } else {
        RunMode::Async
    };

    let spinner = if mode == RunMode::Sync {
        Some(Spinner::new(
            Spinners::Dots9,
            "Awaiting the run to complete...".to_owned(),
        ))
    } else {
        None
    };

    let response =
        cronback_client::triggers::run(&client, &opts.name, mode).await?;
    if let Some(mut spinner) = spinner {
        spinner.stop_with_message("".to_string());
    }

    let response = response.into_inner();
    match response {
        | Ok(good) => {
            let json = serde_json::to_value(good)?;
            let colored = colored_json::to_colored_json_auto(&json)?;
            println!("{}", colored);
        }
        | Err(bad) => {
            return Err(bad.into());
        }
    };
    Ok(())
}
