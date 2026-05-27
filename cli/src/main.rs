use std::{iter::zip, process::Command, str::FromStr};

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use futures::future::join_all;
use github::{
    github::{GitHub, GitHubError},
    remote::Remote,
};
use jj::{build_bookmarks_map, list_commits, load_repo, walk_commits};

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
            // Quick and dirty
            let commit_ids = list_commits().context("Failed to list commits")?;
            let repo = load_repo().await.context("Failed to load jj repo")?;
            let bookmarks_map = build_bookmarks_map(&repo);
            let bookmarks_graph = walk_commits(&commit_ids, &bookmarks_map).await;

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

            let remote = Remote::from_str(url).expect("URL Parse error");

            // Check PRs for each bookmark
            let futures = bookmarks_graph.iter().map(|(name, _target)| {
                println!("Checking {}...", name);
                GitHub::retrieve_pull_request(&remote, &name)
            });
            let results = join_all(futures).await;
            for ((bookmark_name, _), result) in zip(bookmarks_graph, results) {
                match result {
                    Ok(maybe_pr) => match maybe_pr {
                        Some(pr) => {
                            // eprintln!("Found PR {:#?}", pr);
                            println!(
                                "{}: https://github.com/{}/{}/pull/{}",
                                bookmark_name, remote.owner, remote.repository, pr.number
                            );
                        }
                        None => {
                            println!("{}: No PR found", bookmark_name);
                        }
                    },
                    Err(e) => match e {
                        GitHubError::InvalidToken => {
                            return Err(anyhow::Error::from(e).context(
                                "Invalid github token. Set GITHUB_TOKEN or log in with the gh CLI.",
                            ));
                        }
                        _ => {
                            return Err(anyhow::Error::from(e).context("Failed to fetch PR info"));
                        }
                    },
                }
            }
        }
    }

    Ok(())
}
