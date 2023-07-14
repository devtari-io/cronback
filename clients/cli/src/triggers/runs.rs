use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use colored::Colorize;
use cronback::Pagination;
use prettytable::{row, Table};

use crate::args::CommonOptions;
use crate::ui::FancyToString;
use crate::{emitln, RunCommand};

#[derive(Clone, Debug, Parser)]
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

#[async_trait]
impl RunCommand for ListRuns {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
    ) -> Result<()> {
        let client = common_options.new_client()?;
        let pagination = Some(Pagination {
            cursor: self.cursor.clone(),
            limit: self.limit,
        });

        let response = client.list_runs(pagination, &self.name).await?;
        common_options.show_meta(&response, out, err).await?;

        let response = response.into_inner()?;

        // Print Table
        if !response.data.is_empty() {
            let len = response.data.len();

            let mut table = Table::new();
            table.set_titles(row![
                "Created At",
                "Status",
                // TODO: Attempt....
                "Id",
            ]);
            for run in response.data {
                table.add_row(row![
                    run.created_at.to_rfc2822(),
                    run.status.fancy(),
                    run.id,
                ]);
            }

            emitln!(out, "{}", table);

            // Print Pagination Metadata
            emitln!(err, "{len} Runs Shown");
            if let Some(next_page_cursor) = response.meta.next_cursor {
                emitln!(
                    err,
                    "View next page by {}{}",
                    "--cursor=".bold(),
                    next_page_cursor.bold()
                );
            }
        }
        Ok(())
    }
}
