use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use prettytable::{row, Table};

use crate::admin::{AdminOptions, RunAdminCommand};
use crate::args::CommonOptions;
use crate::emitln;

#[derive(Clone, Debug, Parser)]
pub struct List {}

#[async_trait]
impl RunAdminCommand for List {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        _err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
        admin_options: &AdminOptions,
    ) -> Result<()> {
        let client = admin_options.new_admin_client(&common_options)?;

        let response = cronback::api_keys::list(&client).await?;

        let response = response.into_inner()?;
        // Print Table
        if !response.data.is_empty() {
            let mut table = Table::new();
            table.set_titles(row!["Id", "Name", "Created At"]);
            for key in response.data {
                table.add_row(row![
                    key.id,
                    key.name,
                    key.created_at.to_rfc2822(),
                ]);
            }

            emitln!(out, "{}", table);
        }

        Ok(())
    }
}
