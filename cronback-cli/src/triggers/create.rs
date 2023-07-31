use anyhow::{anyhow, Result};
use clap_stdin::FileOrStdin;
use cling::prelude::*;

use crate::args::CommonOptions;
use crate::confirm_or_abort;

#[derive(CliRunnable, CliParam, Clone, Debug, Parser)]
#[cling(run = "create")]
pub struct Create {
    /// JSON file name with the trigger definition, or use - for stdin
    file: FileOrStdin<String>,
}

// Three ways to create a trigger
// 1. Command line arguments [future]
// 2. Interactive (prompt) [future]
// 3. Provide a file with the trigger definition [now]
async fn create(common_options: &CommonOptions, opts: &Create) -> Result<()> {
    let json_raw: serde_json::Value = serde_json::from_str(&opts.file)
        .map_err(|e| anyhow!("Failed to parse JSON: {e}"))?;

    let colored_req = colored_json::to_colored_json_auto(&json_raw)?;

    println!("You are about to send this trigger definition to be created:");

    println!("----");
    println!("{colored_req}");
    println!("----");

    confirm_or_abort!(
        common_options,
        "Are you sure you want to create this trigger?",
    );

    let client = common_options.new_client()?;
    let response =
        cronback_client::triggers::create_from_json(&client, json_raw).await?;

    let response = response.into_inner();
    match response {
        | Ok(good) => {
            let name = good.name.clone().expect("name is required");
            let json = serde_json::to_value(good)?;
            let colored = colored_json::to_colored_json_auto(&json)?;
            println!("{}", colored);
            println!();
            println!("Trigger '{}' was created successfully", name);
        }
        | Err(bad) => {
            return Err(bad.into());
        }
    };

    Ok(())
}
