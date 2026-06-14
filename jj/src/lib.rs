use anyhow::{Context, Result, anyhow};
use futures::{StreamExt, TryStreamExt, stream::LocalBoxStream};
use std::{collections::HashMap, env, rc::Rc, sync::Arc};

use jj_lib::{
    backend::CommitId,
    commit::Commit,
    config::StackedConfig,
    graph::{GraphEdge, TopoGroupedGraph},
    id_prefix::IdPrefixContext,
    ref_name::RemoteName,
    refs::{RefPushAction, classify_ref_push_action},
    repo::{ReadonlyRepo, Repo, StoreFactories},
    revset::{Revset, RevsetExtensions, SymbolResolver, UserRevsetExpression},
    settings::UserSettings,
    str_util::{StringExpression, StringMatcher, StringPattern},
    workspace::{Workspace, default_working_copy_factories},
};

/// Owns workspace and relevant state
pub struct WorkspaceContext {
    workspace: Workspace,
    repo: Arc<ReadonlyRepo>,
    bookmarks: HashMap<CommitId, String>,
}

impl WorkspaceContext {
    /// Loads relevant repo info from the jj workspace on disk.
    pub async fn try_load_new() -> Result<Self> {
        let workspace = load_workspace()?;
        let repo = load_repo(&workspace).await?;
        let bookmarks = build_bookmarks_map(&repo);

        Ok(Self {
            workspace,
            repo,
            bookmarks,
        })
    }

    /// Check push actions for each bookmark
    /// See [jj]/cli/src/commands/git/push.rs::find_bookmarks_to_push L1121
    pub fn get_bookmark_push_actions<'a>(
        &'a self,
        // TODO: This is weird to require caller to pass it when we already own it in the struct. Lifetimes issue.
        matcher: &'a StringMatcher,
        remote_name: &'a str,
    ) -> Vec<(String, RefPushAction)> {
        self.repo
            .view()
            .local_remote_bookmarks_matching(matcher, RemoteName::new(remote_name))
            .map(|(name, targets)| {
                (
                    name.as_symbol().to_string(),
                    classify_ref_push_action(targets),
                )
            })
            .collect()
    }

    /// Get bookmarks which exist on the given remote
    pub fn get_bookmarks_on_remote<'a>(
        &'a self,
        // TODO: This is weird to require caller to pass it when we already own it in the struct. Lifetimes issue.
        matcher: &'a StringMatcher,
        remote_name: &'a str,
    ) -> Vec<String> {
        self.repo
            .view()
            .local_remote_bookmarks_matching(matcher, RemoteName::new(remote_name))
            .filter_map(|(bookmark_name, local_remote_ref)| {
                if local_remote_ref.remote_ref.is_present() {
                    Some(bookmark_name.as_symbol().to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Build a dependency graph of local bookmarks
    pub async fn resolve_bookmarks_graph(&self, default_branch: &str) -> Result<BookmarkGraph> {
        let revset = get_valid_bookmarks_revset(&self.workspace, &self.repo, default_branch)
            .context("Failed to get revset")?;
        // Deref the Box (Box<dyn Revset>), then get a ref to it (&dyn Revset)
        build_bookmark_graph_from_revset(&*revset, &self.bookmarks).await
    }

    pub fn get_commit(&self, commit_id: &CommitId) -> Result<Commit> {
        self.repo
            .store()
            .get_commit(commit_id)
            .context(format!("Failed to read commit {}", commit_id))
    }
}

fn load_workspace() -> Result<Workspace> {
    let config = StackedConfig::with_defaults();
    let user_settings =
        UserSettings::from_config(config).context("Failed to load user settings")?;
    let cwd = env::current_dir()?;
    let store_factories = StoreFactories::default();
    let wc_factories = default_working_copy_factories();

    Workspace::load(&user_settings, &cwd, &store_factories, &wc_factories)
        .context("Failed to load workspace")
}

async fn load_repo(workspace: &Workspace) -> Result<Arc<ReadonlyRepo>> {
    workspace
        .repo_loader()
        .load_at_head()
        .await
        .context("Failed to load repo")
}

// See [jj]/cli/src/commands/git/push.rs
fn get_valid_bookmarks_revset<'repo>(
    workspace: &Workspace,
    repo: &'repo ReadonlyRepo,
    default_branch: &str,
) -> Result<Box<dyn Revset + 'repo>> {
    // See [jj]/cli/src/revset_util.rs L108
    let wc = UserRevsetExpression::working_copy(workspace.workspace_name().to_owned());
    let trunk = UserRevsetExpression::bookmarks(StringExpression::exact(default_branch));
    let bookmarks = UserRevsetExpression::bookmarks(StringExpression::all());
    // TODO: Doing a lot of repeated stuff from jj cli, clean it up
    let expr = wc
        .ancestors()
        .minus(&trunk.ancestors())
        .intersection(&bookmarks);
    let extensions = Arc::new(RevsetExtensions::default());
    let id_prefix_context = IdPrefixContext::new(extensions.clone());
    let symbol_resolver = SymbolResolver::new(repo, extensions.symbol_resolvers())
        .with_id_prefix_context(&id_prefix_context);
    let resolved_expr = expr
        .resolve_user_expression(repo, &symbol_resolver)
        .context("Failed to resolve revset expression")?;
    resolved_expr
        .evaluate(repo)
        .context("Failed to evaluate revset")
}

#[derive(Debug)]
pub struct BookmarkNode {
    pub commit_id: CommitId,
    pub name: String,
    pub parent_name: Option<String>,
}

impl BookmarkNode {
    pub fn new(commit_id: CommitId, name: String, parent_name: Option<String>) -> Self {
        Self {
            commit_id,
            name,
            parent_name,
        }
    }
}

/// Linear dependency graph of bookmarks
#[derive(Debug)]
pub struct BookmarkGraph(Vec<Rc<BookmarkNode>>);

impl BookmarkGraph {
    pub fn iter(&self) -> impl Iterator<Item = &Rc<BookmarkNode>> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn try_to_matcher(&self) -> Result<StringMatcher> {
        let combined_bookmark_names = self
            .iter()
            .map(|node| node.name.clone())
            .collect::<Vec<_>>()
            .join("|");
        Ok(StringPattern::regex(&format!("^({})$", combined_bookmark_names))?.to_matcher())
    }
}

impl IntoIterator for BookmarkGraph {
    type Item = Rc<BookmarkNode>;
    type IntoIter = std::vec::IntoIter<Rc<BookmarkNode>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a BookmarkGraph {
    type Item = &'a Rc<BookmarkNode>;
    type IntoIter = std::slice::Iter<'a, Rc<BookmarkNode>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// [jj]/cli/src/commands/log.rs
/// Walk the revset graph and convert to a linear dependency graph of bookmarks.
/// The revset should already be filtered to bookmarks by calling get_valid_bookmarks_revset.
/// TODO: Make the structure/api better
async fn build_bookmark_graph_from_revset(
    revset: &dyn Revset,
    bookmarks_map: &HashMap<CommitId, String>,
) -> Result<BookmarkGraph> {
    // Don't understand this yet
    let revset_graph = TopoGroupedGraph::new(revset.stream_graph(), |id| id);
    let mut stream: LocalBoxStream<_> = revset_graph.stream().boxed_local();

    // Reverse stream so we can build parent nodes before children and validate parent relationships
    let mut entries: Vec<(CommitId, Vec<GraphEdge<CommitId>>)> = vec![];
    while let Some(entry) = stream
        .try_next()
        .await
        .context("Failed to iterate stream")?
    {
        entries.push(entry);
    }
    entries.reverse();

    let mut seen_nodes: HashMap<CommitId, Rc<BookmarkNode>> = HashMap::new();
    let mut ordered_bookmarks = vec![];

    // Iterate from trunk -> leaves
    for (commit_id, edges) in entries {
        let name = bookmarks_map
            .get(&commit_id)
            .ok_or(anyhow!("No bookmark found for commit {}", commit_id))?;

        let parents: Vec<_> = edges
            .iter()
            .filter_map(|e| seen_nodes.get(&e.target).cloned())
            .collect();
        if parents.len() > 1 {
            return Err(anyhow!(
                "{} has multiple parent bookmarks ({:?}). Only linear history is supported. \
                Reorder your stack so each bookmark has only one parent.",
                name,
                parents.iter().map(|n| &n.name[..]).collect::<Vec<_>>()
            ));
        }

        let node = Rc::new(BookmarkNode::new(
            commit_id.clone(),
            name.clone(),
            parents.first().map(|n| n.name.clone()),
        ));
        seen_nodes.insert(commit_id.clone(), node.clone());
        ordered_bookmarks.push(node);
    }
    Ok(BookmarkGraph(ordered_bookmarks))
}

/// Build a mapping from CommitId -> bookmark name for future lookups.
fn build_bookmarks_map(repo: &ReadonlyRepo) -> HashMap<CommitId, String> {
    let mut map = HashMap::new();
    let view = repo.view();
    for (name, target) in view.bookmarks() {
        if let Some(commit_id) = target.local_target.as_normal()
            && let Some(_) = target
                .remote_refs
                .iter()
                // "git" remote is used internally in jj, for our purposes it means a local bookmark.
                .find(|(remote_name, _)| remote_name == "git")
        {
            map.insert(commit_id.clone(), name.as_symbol().to_string());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
}
