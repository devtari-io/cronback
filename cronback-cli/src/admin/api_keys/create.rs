use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use cronback_api_model::admin::APIKeyMetaData;

use crate::admin::{AdminOptions, RunAdminCommand};
use crate::args::CommonOptions;
use crate::emitln;

#[derive(Clone, Debug, Parser)]
pub struct Create {
    /// The name of the key to be created
    name: String,
}

#[async_trait]
impl RunAdminCommand for Create {
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
        let client = admin_options.new_admin_client(common_options)?;

        let response = cronback_client::api_keys::gen(
            &client,
            &self.name,
            APIKeyMetaData::default(),
        )
        .await?;

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
