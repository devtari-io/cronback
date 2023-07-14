#[cfg(feature = "admin")]
mod admin;
mod args;
mod command;
mod confirm;
mod runs;
mod triggers;
mod ui;
mod whoami;

use anyhow::Result;
pub use command::RunCommand;
pub(crate) use confirm::confirm_or_abort;
use tokio::io::AsyncWriteExt;
use tracing::log::info;

pub use self::args::Cli;

pub async fn run_cli(args: Cli) -> Result<()> {
    info!("Base url: {}", args.common.base_url());
    let stdout = tokio::io::stdout();
    let mut stdout = tokio::io::BufWriter::new(stdout);

    let stderr = tokio::io::stderr();
    let mut stderr = tokio::io::BufWriter::new(stderr);
    let res = args
        .command
        .run(&mut stdout, &mut stderr, &args.common)
        .await;
    stdout.flush().await?;
    stderr.flush().await?;
    res
}

macro_rules! emitln {
    ($dst: expr) => {
        {
            tokio::io::AsyncWriteExt::write_all($dst, b"\n").await?
        }
    };
    ($dst: expr, $fmt: expr) => {
        {
            use std::io::Write;
            let mut buf = Vec::<u8>::new();
            writeln!(buf, $fmt)?;
            tokio::io::AsyncWriteExt::write_all($dst, &buf).await?
        }
    };
    ($dst: expr, $fmt: expr, $($arg: tt)*) => {
        {
            use std::io::Write;
            let mut buf = Vec::<u8>::new();
            writeln!(buf, $fmt, $( $arg )*)?;
            tokio::io::AsyncWriteExt::write_all($dst, &buf).await?
        }
    };
}

pub(crate) use emitln;
