use anyhow::Result;
use cling::prelude::*;
use colored::Colorize;

use crate::args::CommonOptions;

#[derive(Run, Collect, Clone, Debug, Parser)]
#[cling(run = "whoami")]
pub struct WhoAmI {
    #[arg(long)]
    /// Reveal the configured API secret token
    show_secret_token: bool,
}

async fn whoami(common_options: &CommonOptions, opts: &WhoAmI) -> Result<()> {
    println!(
        "Cronback Service: {}",
        common_options.base_url().to_string().green()
    );
    if opts.show_secret_token {
        println!("Secret Token: {}", common_options.secret_token.yellow());
    }
    Ok(())
}
