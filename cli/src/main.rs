use anyhow::Result;
use clap::{Parser, Subcommand};

use cli::commands::{
    status::run_status,
    submit::{SubmitArgs, run_submit},
};

/// Submit your jj stack to GitHub.
#[derive(Parser, Debug)]
#[command(name = "jenga")]
#[command(version, about, long_about = None)]
#[command(arg_required_else_help = true)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// View GitHub status
    Status,
    Submit(SubmitArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match &args.command {
        Command::Status => run_status().await,
        Command::Submit(args) => run_submit(args).await,
    }
}
