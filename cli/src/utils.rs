use std::{
    io,
    iter::{self, zip},
    process::{Command, ExitStatus, Stdio},
};

use anyhow::{Context, Result, anyhow};
use futures::future::join_all;
use github::{
    PullRequest,
    github::{GitHubClient, GitHubError},
    remote::Remote,
};
use stacks::Operation;

/// Get github remote from jj cli
pub fn get_remote() -> Result<Remote> {
    let output = Command::new("jj")
        .args(["git", "remote", "list"])
        .output()
        .context("Failed to list jj remotes")?;
    let remote_raw = String::from_utf8(output.stdout).context("Not valid UTF-8")?;

    let mut remote_split = remote_raw.split(" ");
    let (Some(name), Some(url)) = (remote_split.next(), remote_split.next()) else {
        return Err(anyhow!("Invalid remote format"));
    };

    Remote::from_url_str(url, name).context("URL Parse error")
}

pub fn push_bookmarks<'a>(
    bookmarks: impl Iterator<Item = &'a String>,
    dry_run: bool,
) -> io::Result<ExitStatus> {
    let mut args = vec!["git", "push"];
    if dry_run {
        args.push("--dry-run");
    }
    Command::new("jj")
        .args(args)
        .args(zip(iter::repeat("-b"), bookmarks).flat_map(|(f, n)| [f, n]))
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .status()
}

pub async fn fetch_pull_requests(bookmarks: &[String]) -> Result<Vec<Option<PullRequest>>> {
    if bookmarks.is_empty() {
        return Ok(vec![]);
    }

    // Check PRs for each bookmark
    let client = GitHubClient::try_load_new()?;
    let futures = bookmarks.iter().map(|name| {
        eprintln!("Checking {}...", name);
        client.retrieve_pull_request(name)
    });

    let results = join_all(futures).await;
    results
        .into_iter()
        .map(|r| {
            r.map_err(|e| match e {
                GitHubError::InvalidToken => anyhow::Error::from(e)
                    .context("Invalid github token. Set GITHUB_TOKEN or log in with the gh CLI."),
                _ => anyhow::Error::from(e).context("Failed to fetch PR info"),
            })
        })
        .collect::<Result<Vec<_>, _>>()
}

// TODO: Obviously we should not be writing this twice. Cleanup the GitHub/Remote abstraction.
pub async fn get_default_branch() -> Result<String> {
    let client = GitHubClient::try_load_new()?;
    client
        .get_default_branch()
        .await
        .context("Failed to get default branch")
}

pub async fn take_pull_request_operations(operations: &[Operation]) -> Result<Vec<PullRequest>> {
    if operations.is_empty() {
        return Ok(vec![]);
    }

    // TODO: Should be one client per CLI invocation, lazy loaded
    let client = GitHubClient::try_load_new()?;
    let futures = operations.iter().filter_map(|operation| match operation {
        Operation::OpenPullRequest(head, base) => {
            Some(client.create_pull_request(head.clone(), base.clone()))
        }
        Operation::EditPullRequest(head, base) => None, // TODO
        _ => None,
    });

    let results = join_all(futures).await;
    results
        .into_iter()
        .map(|r| {
            r.map_err(|e| match e {
                GitHubError::InvalidToken => anyhow::Error::from(e)
                    .context("Invalid github token. Set GITHUB_TOKEN or log in with the gh CLI."),
                _ => anyhow::Error::from(e),
            })
        })
        .collect::<Result<Vec<_>, _>>()
}

#[derive(Debug)]
pub enum UserOperationConfirmation {
    Confirm,
    Cancel,
    NoOp,
}

impl UserOperationConfirmation {
    pub fn is_confirm(&self) -> bool {
        matches!(self, UserOperationConfirmation::Confirm)
    }
}

/// Print the operations to take and get user confirmation
pub fn confirm_operations<'a>(
    operations: impl Iterator<Item = &'a Operation>,
    dry_run: bool,
) -> UserOperationConfirmation {
    let mut peekable = operations.peekable();
    if peekable.peek().is_none() {
        eprintln!("Your local stacks match remote. No changes will be made.");
        eprintln!("hint: Run `jj git fetch` to update your view of remote.");
        return UserOperationConfirmation::NoOp;
    }

    if dry_run {
        eprintln!("jenga would perform the following actions:");
        for operation in peekable {
            eprintln!("    {}", operation);
        }
        eprintln!("Dry-run: No actions taken.");
        return UserOperationConfirmation::NoOp;
    }

    eprintln!("jenga will perform the following actions:");
    for operation in peekable {
        eprintln!("    {}", operation);
    }
    eprint!("Continue? (Y/n) ");
    let mut confirmation = String::new();
    io::stdin()
        .read_line(&mut confirmation)
        .expect("Failed to read input");
    let confirmation = confirmation.trim();
    if !confirmation.is_empty() && confirmation.to_lowercase() != "y" {
        eprintln!("Operation cancelled.");
        return UserOperationConfirmation::Cancel;
    }
    UserOperationConfirmation::Confirm
}
