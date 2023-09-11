use anyhow::Result;
use cling::prelude::*;

use crate::args::CommonOptions;
use crate::confirm::confirm_or_abort;

#[derive(Run, Collect, Args, Clone, Debug)]
#[cling(run = "revoke")]
pub struct Revoke {
    /// The Id of the key to be revoked
    id: String,

    /// Ignore the confirmation prompt and always answer "yes"
    #[arg(long, short)]
    yes: bool,
}

async fn revoke(common_options: &CommonOptions, opts: &Revoke) -> Result<()> {
    confirm_or_abort!(
        opts,
        "Are you sure you want to revoke the key '{}'? All API calls with \
         this key will start failing.",
        opts.id
    );
    let client = common_options.new_client()?;

    let response = cronback_client::api_keys::revoke(&client, &opts.id).await?;

    // Ensure that the request actually succeeded
    response.into_inner()?;

    println!("Key with id '{}' was revoked!", opts.id);

    Ok(())
}
