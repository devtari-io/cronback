use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::clap_derive::Parser;
use cronback_client::{
    ClientBuilder,
    Response,
    BASE_URL_ENV,
    DEFAULT_BASE_URL,
};
use once_cell::sync::OnceCell;
use url::Url;

#[cfg(feature = "admin")]
use crate::admin;
#[cfg(feature = "admin")]
use crate::admin::RunAdminCommand;
use crate::client::WrappedClient;
use crate::ui::FancyToString;
use crate::{runs, triggers, whoami, Command};

const CRONBACK_SECRET_TOKEN_VAR: &str = "CRONBACK_SECRET_TOKEN";

#[derive(Parser, Debug, Clone)]
/// Command-line utility to manage cronback projects
pub struct Cli {
    #[clap(flatten)]
    pub common: CommonOptions,
    #[clap(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,

    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Parser, Debug, Clone)]
pub struct CommonOptions {
    #[arg(long, global = true)]
    /// Connect to a local cronback service (http://localhost:8888)
    pub localhost: bool,
    #[arg(long, global = true, value_name = "URL", env(BASE_URL_ENV))]
    pub base_url: Option<Url>,
    // Unfortunately, we can't make this required **and** global at the same
    // time. See [https://github.com/clap-rs/clap/issues/1546]
    #[arg(
        long,
        value_name = "TOKEN",
        env(CRONBACK_SECRET_TOKEN_VAR),
        hide_env_values = true
    )]
    /// The API secret token. We attempt to read from `.env` if environment
    /// variable is not set
    pub secret_token: String,

    #[arg(long, global = true)]
    /// Displays a table with meta information about the response
    pub show_meta: bool,
    /// Ignore the confirmation prompt and always answer "yes"
    #[arg(long, short, global = true)]
    pub yes: bool,
}

#[derive(Parser, Debug, Clone)]
pub enum CliCommand {
    /// Commands for triggers
    Triggers {
        #[command(subcommand)]
        command: TriggerCommand,
    },
    /// Commands for trigger runs
    Runs {
        #[command(subcommand)]
        command: RunsCommand,
    },
    #[command(name = "whoami")]
    /// Prints information about the current context/environment
    WhoAmI(whoami::WhoAmI),

    /// Set of commands that require admin privillages.
    #[cfg(feature = "admin")]
    Admin {
        #[clap(flatten)]
        admin_options: admin::AdminOptions,
        #[command(subcommand)]
        command: admin::AdminCommand,
    },
}

#[derive(Parser, Debug, Clone)]
pub enum TriggerCommand {
    /// List triggers
    #[command(visible_alias = "ls")]
    List(triggers::List),
    /// List runs of a trigger
    #[command(visible_alias = "lr")]
    ListRuns(triggers::ListRuns),
    /// Create a new trigger
    Create(triggers::Create),
    /// View details about a given trigger
    #[command(visible_alias = "v")]
    View(triggers::View),
    /// Cancel a scheduled trigger.
    Cancel(triggers::Cancel),
    /// Invoke an adhoc run for a given trigger
    Run(triggers::Run),
    /// Pause a scheduled trigger.
    Pause(triggers::Pause),
    /// Resume a paused trigger
    Resume(triggers::Resume),
    /// Delete a trigger
    Delete(triggers::Delete),
}

#[derive(Parser, Debug, Clone)]
pub enum RunsCommand {
    /// View details about a given trigger run
    View(runs::View),
}

impl CommonOptions {
    pub fn base_url(&self) -> &Url {
        if self.localhost {
            static LOCALHOST_URL: OnceCell<Url> = OnceCell::new();
            LOCALHOST_URL
                .get_or_init(|| Url::parse("http://localhost:8888").unwrap())
        } else {
            self.base_url.as_ref().unwrap_or(&DEFAULT_BASE_URL)
        }
    }

    pub fn new_client(&self) -> Result<WrappedClient> {
        let base_url = self.base_url();
        let inner = ClientBuilder::new()
            .base_url(base_url.clone())
            .context("Error while parsing base url")?
            .secret_token(self.secret_token.clone())
            .build()?;
        Ok(WrappedClient {
            common_options: self.clone(),
            inner,
        })
    }

    pub fn show_response_meta<T>(&self, response: &Response<T>) {
        use colored::Colorize;
        // Print extra information.
        if self.show_meta {
            eprintln!();
            eprintln!(
                "{}",
                "<<-------------------------------------------------".green()
            );
            eprintln!("Path: {}", response.url());
            eprintln!("Status Code: {}", response.status_code().fancy());
            eprintln!(
                "Request Id: {}",
                response.request_id().clone().unwrap_or_default().green()
            );
            eprintln!(
                "Project Id: {}",
                response.project_id().clone().unwrap_or_default().green()
            );
            eprintln!(
                "{}",
                "-------------------------------------------------".green()
            );
            eprintln!();
        }
    }
}

// TODO: Macro-fy this.
#[async_trait]
impl Command for CliCommand {
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
            | CliCommand::Triggers { command } => {
                command.run(out, err, common_options).await
            }
            | CliCommand::Runs { command } => {
                command.run(out, err, common_options).await
            }
            #[cfg(feature = "admin")]
            | CliCommand::Admin {
                admin_options,
                command,
            } => command.run(out, err, common_options, admin_options).await,
            | CliCommand::WhoAmI(c) => c.run(out, err, common_options).await,
        }
    }
}
