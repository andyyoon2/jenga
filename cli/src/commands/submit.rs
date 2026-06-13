use anyhow::{Context, Result, anyhow};
use clap::Args;
use stacks::{Operation, get_bookmark_push_operations, get_operations_for_pull_requests};

use crate::utils::{CliContext, confirm_operations, prompt_confirm, prompt_input, push_bookmarks};

/// Submit a local stack to remote
#[derive(Args, Debug)]
pub struct SubmitArgs {
    /// Print what operations would be done
    #[arg(long)]
    dry_run: bool,
}

pub async fn run_submit(args: &SubmitArgs) -> Result<()> {
    let context = CliContext::new();
    let workspace = context.workspace().await?;
    // TODO: extremely weird, design better pls
    let bookmark_graph = workspace.resolve_bookmarks_graph().await?;
    let matcher = bookmark_graph.try_to_matcher()?;
    let remote_bookmarks = workspace.get_bookmarks_on_remote(&matcher, "origin");
    let push_actions = workspace.get_bookmark_push_actions(&matcher, "origin");

    // TODO: Improve parallelism
    let default_branch = context.default_branch().await?;
    let pull_requests = context
        .fetch_pull_requests(
            remote_bookmarks
                .iter()
                .filter(|bookmark_name| *bookmark_name != default_branch),
        )
        .await?;

    let push_operations = get_bookmark_push_operations(&push_actions);
    let pr_operations =
        get_operations_for_pull_requests(&bookmark_graph, &pull_requests, default_branch);
    // eprintln!("{:#?}", bookmark_graph);
    // eprintln!("{:#?}", push_operations);
    // eprintln!("{:#?}", pr_operations);

    // Get confirmation. TODO: Configure a bypass
    let user_confirmation = confirm_operations(
        push_operations.iter().chain(pr_operations.iter()),
        args.dry_run,
        &default_branch,
    )?;
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

    let client = context.client()?;
    for operation in &pr_operations {
        match operation {
            Operation::CreatePullRequest(node) => {
                let head = &node.name;
                let base = node.parent_name.as_deref().unwrap_or(default_branch);
                eprintln!("\nCreating PR {} -> {}", head, base);
                // TODO: Get defaults for title/body from commit msg
                let title = prompt_input("Title (required)", true)?;
                if title.is_empty() {
                    return Err(anyhow!("Operation cancelled."));
                }
                let body = prompt_input("Body", false)?;
                let draft = prompt_confirm("Draft?", false)?;
                let pr = client
                    .create_pull_request(
                        head.clone(),
                        base.to_string(),
                        Some(title),
                        Some(body),
                        Some(draft),
                    )
                    .await
                    .context("Failed to create pull request")?;
                eprintln!(
                    "Created PR {} -> {}\n{}/{}/{}/pull/{}",
                    pr.head.ref_name,
                    pr.base.ref_name,
                    client.remote.base_url,
                    client.remote.owner,
                    client.remote.repository,
                    pr.number
                );
            }
            // Operation::EditPullRequest(head, base) => None, // TODO
            _ => {}
        }
    }

    Ok(())
}
