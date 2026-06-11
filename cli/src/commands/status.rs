use std::iter::zip;

use anyhow::Result;
use futures::future::join_all;
use github::github::GitHubError;

use crate::utils::CliContext;

pub async fn run_status() -> Result<()> {
    let context = CliContext::new();
    let workspace = context.workspace().await?;
    let bookmark_graph = workspace.resolve_bookmarks_graph().await?;

    // TODO: DRY with `fetch_pull_requests` and pass to a rendering layer
    // Check PRs for each bookmark
    let client = context.client()?;
    let futures = bookmark_graph.iter().map(|node| {
        eprintln!("Checking {}...", node.name);
        client.retrieve_pull_request(&node.name)
    });
    let results = join_all(futures).await;
    for (node, result) in zip(bookmark_graph, results) {
        match result {
            Ok(maybe_pr) => match maybe_pr {
                Some(pr) => {
                    println!(
                        "{}: {}/{}/{}/pull/{}",
                        node.name,
                        client.remote.base_url,
                        client.remote.owner,
                        client.remote.repository,
                        pr.number
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
