use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;

use crate::args::CommonOptions;
use crate::confirm::confirm_or_abort;
use crate::{emitln, Command};

#[derive(Clone, Debug, Parser)]
pub struct Revoke {
    /// The Id of the key to be revoked
    id: String,

    /// Ignore the confirmation prompt and always answer "yes"
    #[arg(long, short)]
    yes: bool,
}

#[async_trait]
impl Command for Revoke {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        _err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
    ) -> Result<()> {
        confirm_or_abort!(
            self,
            "Are you sure you want to revoke the key '{}'? All API calls with \
             this key will start failing.",
            self.id
        );
        let client = common_options.new_client()?;

        let response =
            cronback_client::api_keys::revoke(&client, &self.id).await?;

        // Ensure that the request actually succeeded
        response.into_inner()?;

        emitln!(out, "Key with id '{}' was revoked!", self.id);

        Ok(())
    }
}
