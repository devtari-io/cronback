use anyhow::Result;
use cling::prelude::*;
use cronback_api_model::admin::APIKeyMetaData;

use crate::args::CommonOptions;

#[derive(Run, Collect, Clone, Debug, Parser)]
#[cling(run = "create")]
pub struct Create {
    /// The name of the key to be created
    name: String,
}

async fn create(common_options: &CommonOptions, opts: &Create) -> Result<()> {
    let client = common_options.new_client()?;

    let response = cronback_client::api_keys::gen(
        &client,
        &opts.name,
        APIKeyMetaData::default(),
    )
    .await?;

    let response = response.into_inner()?;

    println!(
        "API key with the name '{}' was created successfully.",
        opts.name
    );
    println!("Secret key: '{}'", response.key);
    Ok(())
}
