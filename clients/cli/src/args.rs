use std::env::VarError;

use anyhow::{bail, Context, Result};
use clap::clap_derive::Parser;
use cronback::{
    Client,
    ClientBuilder,
    Response,
    BASE_URL_ENV,
    DEFAULT_BASE_URL,
};
use once_cell::sync::OnceCell;
use tokio::io::AsyncWriteExt;
use url::Url;

#[cfg(feature = "admin")]
use crate::admin;
use crate::{emitln, runs, triggers, whoami, RunCommand};

const CRONBACK_SECRET_TOKEN_VAR: &str = "CRONBACK_SECRET_TOKEN";
const CRONBACK_PROJECT_ID_VAR: &str = "CRONBACK_PROJECT_ID";

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
    localhost: bool,
    #[arg(long, global = true, value_name = "URL", env(BASE_URL_ENV))]
    base_url: Option<Url>,
    #[arg(
        long,
        global = true,
        value_name = "TOKEN",
        env(CRONBACK_SECRET_TOKEN_VAR),
        hide_env_values = true
    )]
    /// The API secret token. We attempt to read from `.env` if environment
    /// variable is not set
    secret_token: Option<String>,

    #[arg(
        long,
        global = true,
        value_name = "PROJECT_ID",
        env(CRONBACK_PROJECT_ID_VAR),
        hide_env_values = true,
        hide = true
    )]
    /// If the secret_token is an admin key, the client will act on behalf of
    /// the project passed here. This flag is for cronback admin use only.
    /// For normal users, the project id is infered from the secret token.
    /// We attempt to read from `.env` if environment variable is not set, then
    /// fallback to `$HOME/.cronback/config`
    project_id: Option<String>,
    #[arg(long, global = true)]
    /// Displays a table with meta information about the response
    show_meta: bool,
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
    pub fn secret_token(&self) -> Result<String> {
        if let Some(ref token) = self.secret_token {
            return Ok(token.to_string());
        }

        // is it set in env (loaded from .env)
        let maybe_token = match std::env::var(CRONBACK_SECRET_TOKEN_VAR) {
            | Ok(t) => Some(t),
            | Err(VarError::NotPresent) => None,
            | e => {
                // Note that we land here, only when the environment is loaded
                // through the .env file. If the environment variable was set
                // directly, then self.secret_token would have been set.
                return e.with_context(|| {
                    format!(
                        "Failed to load value of `{}` from .env file",
                        CRONBACK_SECRET_TOKEN_VAR
                    )
                });
            }
        };

        if let Some(token) = maybe_token {
            return Ok(token);
        }

        bail!("No secret token was specified!")
    }

    pub fn project_id(&self) -> Result<Option<String>> {
        if let Some(ref token) = self.project_id {
            return Ok(Some(token.to_string()));
        }

        // is it set in env (loaded from .env)
        let maybe_token = match std::env::var(CRONBACK_PROJECT_ID_VAR) {
            | Ok(t) => Some(t),
            | Err(VarError::NotPresent) => None,
            | e => {
                // Note that we land here, only when the environment is loaded
                // through the .env file. If the environment variable was set
                // directly, then self.secret_token would have been set.
                return e
                    .with_context(|| {
                        format!(
                            "Failed to load value of `{}` from .env file",
                            CRONBACK_PROJECT_ID_VAR
                        )
                    })
                    .map(|_| None);
            }
        };

        Ok(maybe_token)
    }

    pub fn base_url(&self) -> &Url {
        if self.localhost {
            static LOCALHOST_URL: OnceCell<Url> = OnceCell::new();
            LOCALHOST_URL
                .get_or_init(|| Url::parse("http://localhost:8888").unwrap())
        } else {
            self.base_url.as_ref().unwrap_or(&DEFAULT_BASE_URL)
        }
    }

    pub fn new_client(&self) -> Result<Client> {
        let base_url = self.base_url();
        let secret_token = self.secret_token()?;
        let project_id = self.project_id()?;
        Ok(ClientBuilder::new()
            .base_url(base_url.clone())
            .context("Error while parsing base url")?
            .secret_token(secret_token)
            .on_behalf_of(project_id)
            .build()?)
    }

    pub async fn show_meta<
        T,
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        response: &Response<T>,
        _out: &mut tokio::io::BufWriter<A>,
        err: &mut tokio::io::BufWriter<B>,
    ) -> Result<()> {
        use colored::Colorize;
        // Print extra information.
        if self.show_meta {
            emitln!(err);
            emitln!(
                err,
                "{}",
                "-------------------------------------------------".green()
            );
            emitln!(err, "URL: {}", response.url());
            emitln!(err, "Status Code: {}", response.status_code());
            emitln!(
                err,
                "Request Id: {}",
                response.request_id().clone().unwrap_or_default().green()
            );
            emitln!(
                err,
                "Project Id: {}",
                response.project_id().clone().unwrap_or_default().green()
            );
            emitln!(
                err,
                "{}",
                "-------------------------------------------------".green()
            );
            emitln!(err);
        }
        err.flush().await?;
        Ok(())
    }
}

// TODO: Macro-fy this.
impl CliCommand {
    pub async fn run<
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
            | CliCommand::Admin { command } => {
                command.run(out, err, common_options).await
            }
            | CliCommand::WhoAmI(c) => c.run(out, err, common_options).await,
        }
    }
}

impl TriggerCommand {
    pub async fn run<
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

impl RunsCommand {
    pub async fn run<
        A: tokio::io::AsyncWrite + Send + Sync + Unpin,
        B: tokio::io::AsyncWrite + Send + Sync + Unpin,
    >(
        &self,
        out: &mut tokio::io::BufWriter<A>,
        err: &mut tokio::io::BufWriter<B>,
        common_options: &CommonOptions,
    ) -> Result<()> {
        match self {
            | RunsCommand::View(c) => c.run(out, err, common_options).await,
        }
    }
}
