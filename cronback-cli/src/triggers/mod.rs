//! Trigger subcommands
mod cancel;
mod create;
mod delete;
mod list;
mod pause;
mod resume;
mod run;
mod runs;
mod view;

use anyhow::Result;
use async_trait::async_trait;
pub(crate) use cancel::Cancel;
pub(crate) use create::Create;
pub(crate) use delete::Delete;
pub(crate) use list::List;
pub(crate) use pause::Pause;
pub(crate) use resume::Resume;
pub(crate) use run::RunArgs as Run;
pub(crate) use runs::ListRuns;
pub(crate) use view::View;

use crate::args::{CommonOptions, TriggerCommand};
use crate::Command;

#[async_trait]
impl Command for TriggerCommand {
    async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
    ) -> Result<()> {
        match self {
            | TriggerCommand::List(c) => c.run(out, err, common_options).await,
            | TriggerCommand::ListRuns(c) => {
                c.run(out, err, common_options).await
            }
            | TriggerCommand::Create(c) => {
                c.run(out, err, common_options).await
            }
            | TriggerCommand::View(c) => c.run(out, err, common_options).await,
            | TriggerCommand::Run(c) => c.run(out, err, common_options).await,
            | TriggerCommand::Delete(c) => {
                c.run(out, err, common_options).await
            }
            | TriggerCommand::Resume(c) => {
                c.run(out, err, common_options).await
            }
            | TriggerCommand::Cancel(c) => {
                c.run(out, err, common_options).await
            }
            | TriggerCommand::Pause(c) => c.run(out, err, common_options).await,
        }
    }
}
