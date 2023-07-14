use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use colored::Colorize;

use crate::args::CommonOptions;
use crate::RunCommand;
#[derive(Parser, Debug, Clone)]
pub struct WhoAmI {
    #[arg(long)]
    /// Reveal the configured API secret token
    show_secret_token: bool,
}

#[async_trait]
impl RunCommand for WhoAmI {
    async fn run(&self, common_options: &CommonOptions) -> Result<()> {
        println!(
            "Cronback Service: {}",
            common_options.base_url().to_string().green()
        );
        match common_options.secret_token() {
            | Ok(token) if self.show_secret_token => {
                println!("Secret Token: {}", token.yellow());
            }
            | Ok(_) => {}
            | Err(_) => {
                println!("{}", "WARNING: NO API SECRET TOKEN IS SET".red())
            }
        }

        Ok(())
    }
}
