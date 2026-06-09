use anyhow::{Result, anyhow};
use clap::Args;
use jj::WorkspaceHelper;
use stacks::{Operation, get_bookmark_push_operations, get_operations_for_pull_requests};

use crate::utils::{
    confirm_operations, create_pull_request, fetch_pull_requests, get_default_branch,
    prompt_and_read_line, push_bookmarks,
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
    // eprintln!("{:#?}", bookmark_graph);
    // eprintln!("{:#?}", push_operations);
    // eprintln!("{:#?}", pr_operations);

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
    // do_pull_request_operations(&pr_operations).await?;

    for operation in &pr_operations {
        match operation {
            Operation::OpenPullRequest(head, base) => {
                eprintln!("\nCreating PR {} -> {}\n", head, base);
                // TODO: Get defaults for title/body from commit msg
                let title = prompt_and_read_line("Title (required)");
                if title.is_empty() {
                    return Err(anyhow!("Operation cancelled."));
                }
                let body = prompt_and_read_line("Body");
                let draft = prompt_and_read_line("Draft? (y/N)");
                let draft = draft.to_lowercase() == "y";
                let pr = create_pull_request(
                    head.clone(),
                    base.clone(),
                    Some(title),
                    Some(body),
                    Some(draft),
                )
                .await?;
                // TODO: Get URL via the client remote address, include in message
                eprintln!(
                    "Created pull request #{}, {} -> {}",
                    pr.number, pr.head.ref_name, pr.base.ref_name
                );
            }
            // Operation::EditPullRequest(head, base) => None, // TODO
            _ => {}
        }
    }

    Ok(())
}
