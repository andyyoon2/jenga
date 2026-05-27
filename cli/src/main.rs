use std::{process::Command, str::FromStr};

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use github::{
    github::{GitHub, GitHubError},
    remote::Remote,
};

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

// TODO: Likely better to instantiate rt when needed
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match &args.command {
        Commands::Status {} => {
            // Get remote
            let output = Command::new("jj")
                .args(["git", "remote", "list"])
                .output()
                .context("Failed to list jj remotes")?;
            let remote_raw = String::from_utf8(output.stdout).expect("Not valid UTF-8");

            let mut remote_split = remote_raw.split(" ");
            let (Some(_), Some(url)) = (remote_split.next(), remote_split.next()) else {
                return Err(anyhow!("Invalid remote format"));
            };

            match GitHub::list_pull_requests(Remote::from_str(url).expect("URL Parse error")).await
            {
                Ok(Some(items)) => {
                    for item in items {
                        println!("{}\t{}\t{}", item.number, item.title, item.head.ref_name);
                    }
                }
                Ok(None) => {
                    eprintln!("No pull requests found");
                }
                Err(e) => match e {
                    GitHubError::InvalidToken => {
                        eprintln!(
                            "Invalid github token. Set GITHUB_TOKEN or log in with the gh CLI."
                        );
                    }
                    _ => {
                        eprintln!("Something went wrong: {}", e);
                    }
                },
            }
        }
    }

    Ok(())
}
