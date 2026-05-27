use std::{
    io::{self, Write},
    process::Command,
};

use clap::{Parser, Subcommand};

/// Submit your jj stack to GitHub.
#[derive(Parser, Debug)]
#[command(name = "jenga")]
#[command(version, about, long_about = None)]
#[command(arg_required_else_help = true)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// View GitHub status
    Status {},
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    match &args.command {
        Commands::Status {} => {
            println!("Welcome to jenga!\n");
            let output = Command::new("jj").args(["log"]).output()?;
            io::stdout().write_all(&output.stdout)?;
            io::stderr().write_all(&output.stderr)?;
        }
    }

    Ok(())
}
