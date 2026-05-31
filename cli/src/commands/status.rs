use std::iter::zip;

use anyhow::Result;
use futures::future::join_all;
use github::github::{GitHub, GitHubError};
use jj::WorkspaceHelper;

use crate::utils::get_remote;

pub async fn run_status() -> Result<()> {
    let workspace = WorkspaceHelper::load_new().await?;
    let ordered_bookmarks = workspace.resolve_bookmarks_graph().await?;
    eprintln!("{:#?}", ordered_bookmarks);

    // Check PRs for each bookmark
    let remote = get_remote()?;
    let futures = ordered_bookmarks.iter().filter_map(|node| {
        // TODO: This is not right
        if node.remote_name == "git" {
            return None;
        }
        println!("Checking {}...", node.name);
        Some(GitHub::retrieve_pull_request(&remote, &node.name))
    });
    let results = join_all(futures).await;
    for (node, result) in zip(ordered_bookmarks, results) {
        match result {
            Ok(maybe_pr) => match maybe_pr {
                Some(pr) => {
                    // eprintln!("Found PR {:#?}", pr);
                    println!(
                        "{}: https://github.com/{}/{}/pull/{}",
                        node.name, remote.owner, remote.repository, pr.number
                    );
                }
                None => {
                    println!("{}: No PR found", node.name);
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

    Ok(())
}
