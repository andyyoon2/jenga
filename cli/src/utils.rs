use anyhow::{Context, Result};
use dialoguer::{Confirm, Input, theme::ColorfulTheme};
use futures::future::join_all;
use jj::WorkspaceContext;
use std::{
    io,
    iter::{self, zip},
    process::{Command, ExitStatus, Stdio},
};

use github::{
    PullRequest,
    github::{GitHubClient, GitHubError},
};
use stacks::Operation;

/// Holds shared state for a command invocation
pub struct CliContext {
    client: once_cell::sync::OnceCell<GitHubClient>,
    default_branch: tokio::sync::OnceCell<String>,
    workspace: tokio::sync::OnceCell<WorkspaceContext>,
}

impl Default for CliContext {
    fn default() -> Self {
        Self::new()
    }
}

impl CliContext {
    pub fn new() -> Self {
        Self {
            client: once_cell::sync::OnceCell::new(),
            default_branch: tokio::sync::OnceCell::new(),
            workspace: tokio::sync::OnceCell::new(),
        }
    }

    pub fn client(&self) -> Result<&GitHubClient> {
        self.client.get_or_try_init(GitHubClient::try_load_new)
    }

    pub async fn default_branch(&self) -> Result<&String> {
        self.default_branch
            .get_or_try_init(async || {
                self.client()?
                    .get_default_branch()
                    .await
                    .context("Failed to get default branch")
            })
            .await
    }

    pub async fn workspace(&self) -> Result<&WorkspaceContext> {
        self.workspace
            .get_or_try_init(async || WorkspaceContext::try_load_new().await)
            .await
    }

    pub async fn fetch_pull_requests<'a, I>(&self, bookmarks: I) -> Result<Vec<Option<PullRequest>>>
    where
        I: Iterator<Item = &'a String>,
    {
        let mut bookmarks = bookmarks.peekable();
        if bookmarks.peek().is_none() {
            return Ok(vec![]);
        }

        // Check PRs for each bookmark
        let client = self.client()?;
        let futures = bookmarks.map(|name| {
            eprintln!("Checking {}...", name);
            client.retrieve_pull_request(name)
        });

        let results = join_all(futures).await;
        results
            .into_iter()
            .map(|r| {
                r.map_err(|e| match e {
                    GitHubError::InvalidToken => anyhow::Error::from(e).context(
                        "Invalid github token. Set GITHUB_TOKEN or log in with the gh CLI.",
                    ),
                    _ => anyhow::Error::from(e).context("Failed to fetch PR info"),
                })
            })
            .collect::<Result<Vec<_>, _>>()
    }
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
) -> Result<UserOperationConfirmation> {
    let mut peekable = operations.peekable();
    if peekable.peek().is_none() {
        eprintln!("Your local stacks match remote. No changes will be made.");
        eprintln!("hint: Run `jj git fetch` to update your view of remote.");
        return Ok(UserOperationConfirmation::NoOp);
    }

    if dry_run {
        eprintln!("jenga would perform the following actions:");
        for operation in peekable {
            eprintln!("    {}", operation);
        }
        eprintln!("Dry-run: No actions taken.");
        return Ok(UserOperationConfirmation::NoOp);
    }

    eprintln!("jenga will perform the following actions:");
    for operation in peekable {
        eprintln!("    {}", operation);
    }
    let confirmation = prompt_confirm("Continue?", true)?;
    if confirmation {
        Ok(UserOperationConfirmation::Confirm)
    } else {
        eprintln!("Operation cancelled.");
        Ok(UserOperationConfirmation::Cancel)
    }
}

pub fn prompt_confirm(prompt: &str, default: bool) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default)
        .interact()
        .context("Failed to read input")
}

pub fn prompt_input(prompt: &str) -> Result<String> {
    Input::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact()
        .context("Failed to read input")
}
