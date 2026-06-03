use std::iter::zip;

use anyhow::Result;
use futures::future::join_all;
use github::github::{GitHub, GitHubError};
use jj::WorkspaceHelper;

use crate::utils::get_remote;

pub async fn run_status() -> Result<()> {
    let workspace = WorkspaceHelper::load_new().await?;
    let bookmark_graph = workspace.resolve_bookmarks_graph().await?;

    // TODO: DRY with `fetch_pull_requests` and pass to a rendering layer
    // Check PRs for each bookmark
    let remote = get_remote()?;
    let futures = bookmark_graph.iter().map(|node| {
        eprintln!("Checking {}...", node.name);
        GitHub::retrieve_pull_request(&remote, &node.name)
    });
    let results = join_all(futures).await;
    for (node, result) in zip(bookmark_graph, results) {
        match result {
            Ok(maybe_pr) => match maybe_pr {
                Some(pr) => {
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
