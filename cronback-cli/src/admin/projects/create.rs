use anyhow::Result;
use cling::prelude::*;

use crate::args::CommonOptions;

#[derive(Run, Args, Clone, Debug)]
#[cling(run = "create")]
pub struct Create;

async fn create(common_options: &CommonOptions) -> Result<()> {
    let client = common_options.new_client()?;

    let response = cronback_client::projects::create(&client).await?;

    let response = response.into_inner()?;

    println!("Project '{}' was created successfully.", response.id);
    Ok(())
}
