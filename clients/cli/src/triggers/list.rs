use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;

use crate::args::CommonOptions;
use crate::RunCommand;

#[derive(Clone, Debug, Parser)]
pub struct List {}

#[async_trait]
impl RunCommand for List {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        _out: &mut tokio::io::BufWriter<A>,
        _err: &mut tokio::io::BufWriter<B>,
        _common_options: &CommonOptions,
    ) -> Result<()> {
        todo!()
    }
}
