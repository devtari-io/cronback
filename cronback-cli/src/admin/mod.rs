use cling::prelude::*;

mod api_keys;
mod projects;

#[derive(Run, Subcommand, Debug, Clone)]
pub enum AdminCommand {
    /// Commands for api key management. This subcommand requires admin
    /// privilliages.
    #[command(subcommand)]
    ApiKeys(api_keys::ApiKeysCommand),
    #[command(subcommand)]
    Projects(projects::ProjectsCommand),
}
