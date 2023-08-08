use cling::prelude::*;
mod create;

#[derive(CliRunnable, Subcommand, Debug, Clone)]
pub enum ProjectsCommand {
    /// Create a new API key
    Create(create::Create),
}
