use cling::prelude::*;

mod create;
mod list;
mod revoke;

#[derive(CliRunnable, Subcommand, Debug, Clone)]
pub enum ApiKeysCommand {
    /// List API keys
    #[command(visible_alias = "ls")]
    List(list::List),
    /// Create a new API key
    Create(create::Create),
    /// Revokes an API key
    Revoke(revoke::Revoke),
}
