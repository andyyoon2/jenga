use std::{
    io::{self, ErrorKind, Write},
    process::Command,
    str::FromStr,
};

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
async fn main() -> io::Result<()> {
    let args = Args::parse();

    match &args.command {
        Commands::Status {} => {
            println!("Welcome to jenga!\n");

            // Log test
            let output = Command::new("jj").args(["log", "-n10"]).output()?;
            io::stdout().write_all(&output.stdout)?;
            io::stderr().write_all(&output.stderr)?;

            // Get remote
            let output = Command::new("jj")
                .args(["git", "remote", "list"])
                .output()?;
            let remote_raw = String::from_utf8(output.stdout).expect("Not valid UTF-8");
            println!("remote_raw: {}", remote_raw);

            let mut remote_split = remote_raw.split(" ");
            let (Some(_), Some(url)) = (remote_split.next(), remote_split.next()) else {
                // TODO: Anyhow
                return Err(io::Error::new(ErrorKind::Other, "Invalid remote format"));
            };

            match GitHub::list_pull_requests(Remote::from_str(url).expect("URL Parse error")).await
            {
                Ok(page) => {
                    for item in page.items {
                        println!("{}\t{}\t{}", item.number, item.title, item.head.ref_field);
                    }
                }
                Err(e) => match e {
                    GitHubError::InvalidToken => {
                        eprintln!(
                            "Something went wrong: {}",
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
