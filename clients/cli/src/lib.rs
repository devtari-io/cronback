mod args;
mod command;
mod triggers;
mod whoami;

use anyhow::Result;
pub use command::RunCommand;
use tracing::log::info;

pub use self::args::Cli;

pub async fn run_cli(args: Cli) -> Result<()> {
    info!("Base url: {}", args.common.base_url());
    args.command.run(&args.common).await
}
