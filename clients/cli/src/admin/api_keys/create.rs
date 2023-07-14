use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use cronback::client::AdminApiExt;
use cronback_api_model::admin::APIKeyMetaData;

use crate::args::CommonOptions;
use crate::{emitln, RunCommand};

#[derive(Clone, Debug, Parser)]
pub struct Create {
    /// The name of the key to be created
    name: String,
}

#[async_trait]
impl RunCommand for Create {
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

        let response = client
            .gen_api_key(&self.name, APIKeyMetaData::default())
            .await?;

        common_options.show_meta(&response, out, err).await?;
        let response = response.into_inner()?;

        emitln!(
            out,
            "API key with the name '{}' was created successfully.",
            self.name
        );
        emitln!(out, "Secret key: '{}'", response.key);
        Ok(())
    }
}
