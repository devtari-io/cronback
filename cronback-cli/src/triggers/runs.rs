use anyhow::Result;
use cling::prelude::*;
use colored::Colorize;
use cronback_client::Pagination;
use prettytable::{row, Table};

use crate::args::CommonOptions;
use crate::ui::FancyToString;

#[derive(CliRunnable, CliParam, Clone, Debug, Parser)]
#[cling(run = "list_runs")]
pub struct ListRuns {
    /// Cursor to start listing from
    #[clap(long)]
    cursor: Option<String>,
    /// Limit the number of results
    #[clap(long, default_value = "20")]
    limit: Option<i32>,
    /// Trigger name
    name: String,
}

async fn list_runs(
    common_options: &CommonOptions,
    opts: &ListRuns,
) -> Result<()> {
    let client = common_options.new_client()?;
    let pagination = Some(Pagination {
        cursor: opts.cursor.clone(),
        limit: opts.limit,
    });

    let response =
        cronback_client::runs::list(&client, pagination, &opts.name).await?;

    let response = response.into_inner()?;

    // Print Table
    if !response.data.is_empty() {
        let len = response.data.len();

        let mut table = Table::new();
        table.set_titles(row![
            "Created At",
            "Status",
            "No. of Attempts",
            "Latest Attempt At",
            "Latest Attempt Status",
            "Id",
        ]);
        for run in response.data {
            let latest_attempt = run.latest_attempt;
            let latest_status = latest_attempt
                .as_ref()
                .map(|a| a.details.status_message())
                .unwrap_or("-".to_owned());

            table.add_row(row![
                run.created_at.to_rfc2822(),
                run.status.fancy(),
                latest_attempt
                    .as_ref()
                    .map(|a| a.attempt_num.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                latest_attempt
                    .as_ref()
                    .map(|a| a.created_at.to_rfc2822())
                    .unwrap_or_else(|| "-".to_string()),
                latest_status,
                run.id,
            ]);
        }

        println!("{}", table);

        // Print Pagination Metadata
        eprintln!("{len} Runs Shown");
        if let Some(next_page_cursor) = response.meta.next_cursor {
            eprintln!(
                "View next page by {}{}",
                "--cursor=".bold(),
                next_page_cursor.bold()
            );
        }
    }
    Ok(())
}
