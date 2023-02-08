use clap::Parser;

#[derive(Parser)]
#[command(version = "0.1")]
pub struct CliOpts {
    /// Sets the custom configuration file.
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<String>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}
