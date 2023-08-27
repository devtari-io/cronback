use cling::prelude::*;
use cronback_cli::Cli;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ClingFinished<Cli> {
    Cling::parse_and_run().await
}
