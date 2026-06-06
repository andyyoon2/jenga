use anyhow::Result;
use clap::Args;
use jj::WorkspaceHelper;
use stacks::{Operation, get_bookmark_push_operations, get_operations_for_pull_requests};

use crate::utils::{
    confirm_operations, do_pull_request_operations, fetch_pull_requests, get_default_branch,
    push_bookmarks,
};

/// Submit a local stack to remote
#[derive(Args, Debug)]
pub struct SubmitArgs {
    /// Print what operations would be done
    #[arg(long)]
    dry_run: bool,
}

pub async fn run_submit(args: &SubmitArgs) -> Result<()> {
    let workspace = WorkspaceHelper::try_load_new().await?;
    // TODO: extremely weird, design better pls
    let bookmark_graph = workspace.resolve_bookmarks_graph().await?;
    let matcher = bookmark_graph.try_to_matcher()?;
    let remote_bookmarks = workspace.get_bookmarks_on_remote(&matcher, "origin");

    let push_actions = workspace.get_bookmark_push_actions(&matcher, "origin");

    // TODO: Improve parallelism
    let default_branch = get_default_branch().await?;
    let pull_requests = fetch_pull_requests(
        remote_bookmarks
            .iter()
            .filter(|bookmark_name| **bookmark_name != default_branch),
    )
    .await?;

    let push_operations = get_bookmark_push_operations(&push_actions);
    let pr_operations =
        get_operations_for_pull_requests(&bookmark_graph, &pull_requests, &default_branch);

    // Get confirmation. TODO: Configure a bypass
    let user_confirmation = confirm_operations(
        push_operations.iter().chain(pr_operations.iter()),
        args.dry_run,
    );
    if !user_confirmation.is_confirm() {
        return Ok(()); // TODO: Exit code if canceled
    }

    // Take the actions
    if !push_operations.is_empty() {
        push_bookmarks(
            push_operations
                .iter()
                .filter_map(|operation| match operation {
                    Operation::Push(b) => Some(b),
                    _ => None,
                }),
            false,
        )?;
    }

    // TODO: Take input for title/body/other params
    do_pull_request_operations(&pr_operations).await?;

    Ok(())
}
