use anyhow::Result;
use cling::prelude::*;

use crate::args::CommonOptions;

#[derive(Run, Collect, Args, Clone, Debug)]
#[cling(run = "view")]
pub struct View {
    /// Trigger name
    name: String,
}

async fn view(common_options: &CommonOptions, opts: &View) -> Result<()> {
    let client = common_options.new_client()?;
    let response = cronback_client::triggers::get(&client, &opts.name).await?;

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
