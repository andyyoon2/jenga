//! Code which integrates jj and remote operations.

use std::fmt::{self, Display, Formatter};

use jj_lib::refs::RefPushAction;

/// An action to be taken on remote
#[derive(Debug)]
pub enum Operation {
    Push(String),
    OpenPr,
    EditPr,
}

impl Display for Operation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Operation::Push(name) => write!(f, "push {}", name),
            Operation::OpenPr => write!(f, "open PR"),
            Operation::EditPr => write!(f, "edit PR"),
        }
    }
}

/// Convert from jj's internal RefPushAction to our simpler Operation enum to
/// determine which bookmarks need to be pushed.
// TODO: Define all the states with conflicted / new / deleted bookmarks
// See [jj]/cli/src/commands/git/push.rs::classify_bookmark_update
pub fn bookmark_push_operations(push_actions: &[(String, RefPushAction)]) -> Vec<Operation> {
    push_actions
        .iter()
        .filter_map(|(bookmark_name, push_action)| match push_action {
            RefPushAction::Update(_) => Some(Operation::Push(bookmark_name.clone())),
            _ => None,
        })
        .collect()
}
