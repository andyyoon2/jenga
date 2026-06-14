use anyhow::Result;

use crate::{render::Renderer, utils::CliContext};

pub async fn run_status() -> Result<()> {
    let context = CliContext::new();
    let workspace = context.workspace().await?;
    let default_branch = context.default_branch().await?;
    let bookmark_graph = workspace.resolve_bookmarks_graph(default_branch).await?;
    let matcher = bookmark_graph.try_to_matcher()?;
    let remote_bookmarks = workspace.get_bookmarks_on_remote(&matcher, "origin");

    if bookmark_graph.is_empty() {
        eprintln!(
            "No relevant bookmarks found. No changes will be made.\n\
            hint: Bookmarks are checked from working copy to trunk. Try moving your working copy."
        );
        return Ok(());
    }

    // Check PRs for each bookmark
    let client = context.client()?;
    let default_branch = context.default_branch().await?;
    let pull_requests = context
        .fetch_pull_requests(
            remote_bookmarks
                .iter()
                .filter(|bookmark_name| *bookmark_name != default_branch),
        )
        .await?;

    let renderer = Renderer::new();
    println!(
        "{}",
        renderer.display_terminal(
            &bookmark_graph,
            &pull_requests,
            &client.remote,
            default_branch
        )
    );

    Ok(())
}
