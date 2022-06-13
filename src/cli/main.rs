use anyhow::Error;
// use auditor::client::AuditorClient;
use auditor::domain::{RecordAdd, RecordUpdate};
use auditor::telemetry::{get_subscriber, init_subscriber};
// use tracing::{debug, info};
// use clap::{Args, Parser, Subcommand};
use clap::{Parser, Subcommand};
use serde::de::Deserialize;

#[derive(Debug, Parser)]
#[clap(name = "auditor-cli")]
#[clap(about = "A command line interface for AUDITOR", long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Get all records from server
    // #[clap(arg_required_else_help = true)]
    Get {},
    /// Push a record to the server
    #[clap(arg_required_else_help = true)]
    Add {
        #[clap(required = true, parse(try_from_str = from_json))]
        record: RecordAdd,
    },
    /// Update a record on the server
    #[clap(arg_required_else_help = true)]
    Update {
        #[clap(required = true, parse(try_from_str = from_json))]
        record: RecordUpdate,
    },
}

fn from_json<'a, T>(s: &'a str) -> Result<T, &'static str>
where
    T: Deserialize<'a>,
{
    serde_json::from_str(s).map_err(|_| "Could not parse JSON")
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Set up logging
    let subscriber = get_subscriber("AUDITOR-cli".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let args = Cli::parse();

    match args.command {
        Commands::Get {} => {
            println!("get")
        }
        Commands::Add { record } => {
            println!("add: {:?}", record)
        }
        Commands::Update { record } => {
            println!("update: {:?}", record)
        }
    }

    Ok(())
}
