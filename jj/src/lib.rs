use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    env,
    error::Error,
    fmt,
    io::{self, BufRead, BufReader},
    process::Command,
    sync::Arc,
};

use jj_lib::{
    backend::CommitId,
    config::{ConfigGetError, StackedConfig},
    op_store::RemoteRef,
    repo::{ReadonlyRepo, RepoLoaderError, StoreFactories},
    settings::UserSettings,
    workspace::{Workspace, WorkspaceLoadError, default_working_copy_factories},
};

pub fn list_commits() -> io::Result<Vec<String>> {
    let output = Command::new("jj")
        .args([
            "log",
            "-r",
            "(::@ ~ ::trunk())", // NOTE: both @ and trunk() are not in jj-lib
            "--no-graph",
            "--reversed",
            "-T",
            "commit_id ++ \"\n\"",
        ])
        .output()?;
    let reader = BufReader::new(&output.stdout[..]);
    reader.lines().collect()
}

#[derive(Debug)]
pub enum JJError {
    ConfigGet(ConfigGetError),
    Io(io::Error),
    WorkspaceLoad(WorkspaceLoadError),
    RepoLoader(RepoLoaderError),
}

impl fmt::Display for JJError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigGet(e) => write!(f, "ConfigGetError: {e}"),
            Self::Io(e) => write!(f, "io Error: {e}"),
            Self::WorkspaceLoad(e) => write!(f, "WorkspaceLoadError: {e}"),
            Self::RepoLoader(e) => write!(f, "RepoLoaderError: {e}"),
        }
    }
}

impl Error for JJError {}

impl From<ConfigGetError> for JJError {
    fn from(value: ConfigGetError) -> Self {
        Self::ConfigGet(value)
    }
}

impl From<io::Error> for JJError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<WorkspaceLoadError> for JJError {
    fn from(value: WorkspaceLoadError) -> Self {
        Self::WorkspaceLoad(value)
    }
}

impl From<RepoLoaderError> for JJError {
    fn from(value: RepoLoaderError) -> Self {
        Self::RepoLoader(value)
    }
}

pub async fn load_repo() -> Result<Arc<ReadonlyRepo>> {
    let config = StackedConfig::with_defaults();
    let user_settings =
        UserSettings::from_config(config).context("Failed to load user settings")?;
    let cwd = env::current_dir()?;
    let store_factories = StoreFactories::default();
    let wc_factories = default_working_copy_factories();

    let workspace = Workspace::load(&user_settings, &cwd, &store_factories, &wc_factories)
        .context("Failed to load workspace")?;
    let repo = workspace
        .repo_loader()
        .load_at_head()
        .await
        .context("Failed to load repo")?;
    Ok(repo)
}

/// Walk list of commits and build a dep graph of bookmarks. Linear history supported only for now.
/// commit_ids should be ordered by closest to trunk -> farthest.
pub async fn walk_commits<'a>(
    commit_ids: &'a Vec<String>,
    bookmarks_map: &'a HashMap<CommitId, (String, RemoteRef)>,
) -> Vec<(String, RemoteRef)> {
    let mut bookmarks = vec![];
    for id in commit_ids.iter() {
        // TODO: Try also from_bytes, then we don't have to parse to string in list_commits
        let Some(commit_id) = CommitId::try_from_hex(id) else {
            eprintln!("Failed to parse commit id: {}", id);
            continue;
        };
        if let Some(bookmark) = bookmarks_map.get(&commit_id) {
            bookmarks.push(bookmark.clone());
        }
    }
    bookmarks
}

/// Build a mapping from CommitId -> (String (bookmark name), RemoteRef). Filters for "origin" remote only for now.
/// TODO: Support other remote names
pub fn build_bookmarks_map(repo: &ReadonlyRepo) -> HashMap<CommitId, (String, RemoteRef)> {
    let mut map = HashMap::new();
    let view = repo.view();
    for (name, target) in view.bookmarks() {
        if let Some(commit_id) = target.local_target.as_normal()
            && let Some((_remote_name, remote_ref)) = target
                .remote_refs
                .into_iter()
                .find(|(name, _)| name.as_str() == "origin")
        {
            map.insert(
                commit_id.clone(),
                (name.as_symbol().to_string(), remote_ref.clone()),
            );
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
}
