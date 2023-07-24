use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;

use crate::args::CommonOptions;
use crate::{emitln, Command};

#[derive(Clone, Debug, Parser)]
pub struct Create;

#[async_trait]
impl Command for Create {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        _err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
    ) -> Result<()> {
        let client = common_options.new_client()?;

        let response = cronback_client::projects::create(&client).await?;

        let response = response.into_inner()?;

        emitln!(out, "Project '{}' was created successfully.", response.id);
        Ok(())
    }
}
