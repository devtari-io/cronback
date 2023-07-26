use clap::Parser;

#[derive(clap::ValueEnum, Clone)]
pub enum LogFormat {
    Pretty,
    Compact,
    Json,
}

#[derive(Parser)]
#[command(version = "0.1")]
pub struct CliOpts {
    /// Sets the custom configuration file.
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<String>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[arg(short, long, default_value = "pretty")]
    pub log_format: LogFormat,

    /// The directory where the api tracing logs will be written to
    #[arg(short, long, default_value = "/tmp", value_name = "FILE")]
    pub api_tracing_dir: String,
}
