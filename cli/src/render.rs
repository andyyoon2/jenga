//! Rendering utils for terminal output and GH comments

use std::iter::zip;

use github::{PullRequest, remote::Remote};
use jj::BookmarkGraph;

pub struct Renderer {}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        Self {}
    }

    // Ideas
    // ▿▽│─╯╰└╭┌△◆○
    // ╭─ Stack 2
    // ▽  https://github.com/org/repo/pull/2
    // ├─ Stack 1
    // ▽  https://github.com/org/repo/pull/1
    // ╰─ main
    //
    // ○  Stack 2
    // ↓  https://github.com/org/repo/pull/2
    // ○  Stack 1
    // ↓  https://github.com/org/repo/pull/1
    // ○  main
    //
    // ↓  Stack 2
    // │  https://github.com/org/repo/pull/2
    // ↓  Stack 1
    // │  https://github.com/org/repo/pull/1
    // ○  main
    //
    // ○  Stack 2
    // │  https://github.com/org/repo/pull/2
    // │ ○  Stack 2a
    // ├─╯  https://github.com/org/repo/pull/3
    // ○  Stack 1
    // │  https://github.com/org/repo/pull/1
    // ◆  main
    //
    // ○  Stack 2
    // │  https://github.com/org/repo/pull/2
    // ○  Stack 1
    // │  https://github.com/org/repo/pull/1
    // ◆  main
    //
    // ╭─ Stack 2
    // ▽  https://github.com/org/repo/pull/2
    // ├─ Stack 1
    // ▽  https://github.com/org/repo/pull/1
    // ◆  main
    //
    // ╭─ Stack 2
    // ▽  https://github.com/org/repo/pull/2
    // │  ╭─ Stack 2a
    // │  ▽  https://github.com/org/repo/pull/3
    // ├─ Stack 1
    // ▽  https://github.com/org/repo/pull/1
    // ○  main
    //
    // ╭─ Stack 2
    // ▽  https://github.com/org/repo/pull/2
    // │
    // ├─ Stack 1
    // ▽  https://github.com/org/repo/pull/1
    // │
    // ○  main
    //
    //   ╭── Stack 2
    //   ▽  https://github.com/org/repo/pull/2
    // ╭─ Stack 1
    // ▽  https://github.com/org/repo/pull/1
    // │
    // ○  main
    //
    // │  Stack 2
    // ▽  https://github.com/org/repo/pull/2
    // │  Stack 1
    // ▽  https://github.com/org/repo/pull/1
    // ○  main
    //
    // ▽  Stack 2
    // │  https://github.com/org/repo/pull/2
    // ▽  Stack 1
    // │  https://github.com/org/repo/pull/1
    // ○  main
    // TODO: This doesn't display branches yet
    pub fn display_terminal(
        &self,
        graph: &BookmarkGraph,
        pull_requests: &[Option<PullRequest>],
        remote: &Remote,
        default_branch: &str,
    ) -> String {
        // TODO: Maybe integrate sapling-renderdag?
        let mut lines = vec![format!("◆  {}", default_branch)];
        for (node, maybe_pr) in zip(graph, pull_requests) {
            let pr_status = match maybe_pr {
                Some(pr) => {
                    if pr.draft {
                        "DRAFT"
                    } else if pr.merged_at.is_some() {
                        "MERGED"
                    } else if pr.state == "closed" {
                        "CLOSED"
                    } else {
                        "OPEN"
                    }
                }
                None => "",
            };
            let pr_link = match maybe_pr {
                Some(pr) => format!(
                    "{}/{}/{}/pull/{}",
                    remote.base_url, remote.owner, remote.repository, pr.number
                ),
                None => "No PR found".to_string(),
            };
            lines.push(format!("│  {}", pr_link));
            lines.push(format!("▽  {} {}", &node.name, pr_status));
        }
        lines.reverse();
        lines.join("\n")
    }
}
