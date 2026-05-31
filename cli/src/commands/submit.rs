use anyhow::Result;
use clap::Args;
use jj::WorkspaceHelper;
use stacks::{Operation, bookmark_push_operations};

use crate::utils::{confirm_operations, push_bookmarks};

/// Submit a local stack to remote
#[derive(Args, Debug)]
pub struct SubmitArgs {
    /// Print what operations would be done
    #[arg(long)]
    dry_run: bool,
}

pub async fn run_submit(args: &SubmitArgs) -> Result<()> {
    let workspace = WorkspaceHelper::load_new().await?;
    // TODO: extremely weird, design better pls
    let bookmarks_graph = workspace.resolve_bookmarks_graph().await?;
    let matcher = bookmarks_graph.try_to_matcher()?;
    let push_actions = workspace.bookmark_push_actions(&matcher);

    let operations = bookmark_push_operations(&push_actions);
    let user_confirmation = confirm_operations(&operations, args.dry_run);
    if !user_confirmation.is_confirm() {
        return Ok(()); // TODO: Exit code if canceled
    }

    push_bookmarks(
        operations.iter().filter_map(|operation| match operation {
            Operation::Push(b) => Some(b),
            _ => None,
        }),
        false,
    )?;
    Ok(())
}
