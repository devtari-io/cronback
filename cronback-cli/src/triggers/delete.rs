use anyhow::Result;
use cling::prelude::*;

use crate::args::CommonOptions;
use crate::confirm_or_abort;

#[derive(Run, Collect, Args, Clone, Debug)]
#[cling(run = "delete")]
pub struct Delete {
    /// Trigger name
    name: String,
}

async fn delete(common_options: &CommonOptions, opts: &Delete) -> Result<()> {
    confirm_or_abort!(
        common_options,
        "Are you sure you want to permanently delete the trigger '{}'?",
        opts.name
    );

    let client = common_options.new_client()?;
    let response =
        cronback_client::triggers::delete(&client, &opts.name).await?;

    let response = response.into_inner();
    match response {
        | Ok(_) => {
            println!("Trigger '{}' has been deleted!", opts.name);
        }
        | Err(bad) => {
            return Err(bad.into());
        }
    };

    Ok(())
}
