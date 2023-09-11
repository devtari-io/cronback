use anyhow::Result;
use cling::prelude::*;
use prettytable::{row, Table};

use crate::args::CommonOptions;

#[derive(Run, Args, Clone, Debug)]
#[cling(run = "list")]
pub struct List;

async fn list(common_options: &CommonOptions) -> Result<()> {
    let client = common_options.new_client()?;

    let response = cronback_client::api_keys::list(&client).await?;

    let response = response.into_inner()?;
    // Print Table
    if !response.data.is_empty() {
        let mut table = Table::new();
        table.set_titles(row!["Id", "Name", "Created At"]);
        for key in response.data {
            table.add_row(row![key.id, key.name, key.created_at.to_rfc2822(),]);
        }

        println!("{}", table);
    }

    Ok(())
}
