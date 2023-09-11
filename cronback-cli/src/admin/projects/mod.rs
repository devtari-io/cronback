use cling::prelude::*;
mod create;

#[derive(Run, Subcommand, Debug, Clone)]
pub enum ProjectsCommand {
    /// Create a new API key
    Create(create::Create),
}
