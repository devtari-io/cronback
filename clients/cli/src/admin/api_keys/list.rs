use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use cronback::client::AdminApiExt;
use prettytable::{row, Table};

use crate::args::CommonOptions;
use crate::{emitln, RunCommand};

#[derive(Clone, Debug, Parser)]
pub struct List {}

#[async_trait]
impl RunCommand for List {
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

        let response = client.list_api_keys().await?;
        common_options.show_meta(&response, out, err).await?;

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
