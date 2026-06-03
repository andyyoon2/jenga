//! Code which integrates jj and remote operations.

use std::fmt::{self, Display, Formatter};

use github::PullRequest;
use jj::BookmarkGraph;
use jj_lib::refs::RefPushAction;

/// An action to be taken on remote
#[derive(Debug)]
pub enum Operation {
    Push(String),
    /// head, base
    OpenPullRequest(String, String),
    /// head, base
    EditPullRequest(String, String),
}

impl Display for Operation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Operation::Push(name) => write!(f, "Push {}", name),
            Operation::OpenPullRequest(head, base) => write!(f, "Open PR {} -> {}", head, base),
            Operation::EditPullRequest(head, base) => write!(f, "Edit PR {} -> {}", head, base),
        }
    }
}

/// Convert from jj's internal RefPushAction to our simpler Operation enum to
/// determine which bookmarks need to be pushed.
// TODO: Define all the states with conflicted / new / deleted bookmarks
// See [jj]/cli/src/commands/git/push.rs::classify_bookmark_update
pub fn get_bookmark_push_operations(
    remote_bookmarks: &[(String, RefPushAction)],
) -> Vec<Operation> {
    remote_bookmarks
        .iter()
        .filter_map(|(bookmark_name, push_action)| match push_action {
            RefPushAction::Update(_) => Some(Operation::Push(bookmark_name.clone())),
            _ => None,
        })
        .collect()
}

pub fn get_operations_for_pull_requests(
    bookmark_graph: &BookmarkGraph,
    pull_requests: &[Option<PullRequest>],
    default_branch: &str,
) -> Vec<Operation> {
    bookmark_graph
        .iter()
        .filter_map(|node| {
            // TODO: Not great but let's try this
            match pull_requests.iter().find(|maybe_pr| {
                // This feels pretty wrong
                if let Some(pr) = maybe_pr {
                    pr.head.ref_name == node.name
                } else {
                    false
                }
            }) {
                // If a PR exists, check that its base matches local
                Some(Some(pr)) => {
                    match &node.parent_name {
                        Some(local_base_name) => {
                            if pr.base.ref_name == *local_base_name {
                                None
                            } else {
                                // TODO: So much cloning. Can we use the Arc better
                                Some(Operation::EditPullRequest(
                                    node.name.clone(),
                                    node.parent_name
                                        .clone()
                                        .unwrap_or(default_branch.to_owned()),
                                ))
                            }
                        }
                        None => {
                            if pr.base.ref_name == default_branch {
                                None
                            } else {
                                Some(Operation::EditPullRequest(
                                    node.name.clone(),
                                    node.parent_name
                                        .clone()
                                        .unwrap_or(default_branch.to_owned()),
                                ))
                            }
                        }
                    }
                }
                // If a PR doesn't exist, open a new PR
                _ => Some(Operation::OpenPullRequest(
                    node.name.clone(),
                    node.parent_name
                        .clone()
                        .unwrap_or(default_branch.to_owned()),
                )),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use jj_lib::merge::Diff;

    use super::*;

    #[test]
    fn handles_empty_slice() {
        let push_actions = vec![];
        let operations = get_bookmark_push_operations(&push_actions);
        assert_eq!(operations.len(), 0);
    }

    #[test]
    fn returns_one_push_operation_for_each_update_action() {
        let push_actions = vec![
            (
                "branch-1".to_string(),
                RefPushAction::Update(Diff::new(None, None)),
            ),
            (
                "branch-2".to_string(),
                RefPushAction::Update(Diff::new(None, None)),
            ),
        ];
        let operations = get_bookmark_push_operations(&push_actions);
        assert_eq!(operations.len(), 2);
    }

    #[test]
    fn filters_non_push_operations() {
        let push_actions = vec![
            (
                "branch-1".to_string(),
                RefPushAction::Update(Diff::new(None, None)),
            ),
            ("branch-2".to_string(), RefPushAction::AlreadyMatches),
            ("branch-3".to_string(), RefPushAction::LocalConflicted),
            ("branch-4".to_string(), RefPushAction::RemoteConflicted),
            ("branch-5".to_string(), RefPushAction::RemoteUntracked),
        ];
        let operations = get_bookmark_push_operations(&push_actions);
        assert_eq!(operations.len(), 1);
    }
}
