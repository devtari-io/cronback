use anyhow::Result;
use cling::prelude::*;

use crate::args::CommonOptions;

#[derive(Run, Collect, Args, Clone, Debug)]
#[cling(run = "view")]
pub struct View {
    /// Run Id
    id: String,
}

async fn view(common_options: &CommonOptions, opts: &View) -> Result<()> {
    let client = common_options.new_client()?;
    let response = cronback_client::runs::get(&client, &opts.id).await?;

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
