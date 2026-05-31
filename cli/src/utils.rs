use std::{
    io,
    iter::{self, zip},
    process::{Command, ExitStatus, Stdio},
};

use anyhow::{Context, Result, anyhow};
use github::remote::Remote;
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
pub fn confirm_operations(operations: &[Operation], dry_run: bool) -> UserOperationConfirmation {
    if operations.is_empty() {
        eprintln!("Your local stacks match remote. No changes will be made.");
        eprintln!("hint: Run `jj git fetch` to update your view of remote.");
        return UserOperationConfirmation::NoOp;
    }

    if dry_run {
        eprintln!("jenga would perform the following actions:");
        for operation in operations.iter() {
            eprintln!("  {}", operation);
        }
        eprintln!("Dry-run: No actions taken.");
        return UserOperationConfirmation::NoOp;
    }

    eprintln!("jenga will perform the following actions:");
    for operation in operations.iter() {
        eprintln!("  {}", operation);
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
