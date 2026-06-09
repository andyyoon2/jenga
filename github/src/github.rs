use std::{env, error::Error, fmt, io, process::Command, string};

use anyhow::{Context, anyhow};
use octocrab::Octocrab;

use crate::{PullRequest, remote::Remote, structs::PullRequestCreateBody};

#[derive(Debug)]
pub enum GitHubError {
    InvalidToken,
    Octocrab(octocrab::Error),
}

impl fmt::Display for GitHubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToken => {
                write!(f, "Invalid access token")
            }
            Self::Octocrab(e) => {
                // TODO: Better backtrace handling
                match e {
                    octocrab::Error::GitHub {
                        source,
                        backtrace: _,
                    } => {
                        write!(
                            f,
                            "Octocrab {}\n{} {} {:#?}",
                            e, source.status_code, source.message, source.errors
                        )
                    }
                    octocrab::Error::Serde {
                        source: _,
                        backtrace,
                    } => {
                        write!(f, "Octocrab {}\n{}", e, backtrace)
                    }
                    _ => write!(f, "Octocrab {}", e),
                }
            }
        }
    }
}

impl Error for GitHubError {}

impl From<octocrab::Error> for GitHubError {
    fn from(value: octocrab::Error) -> Self {
        Self::Octocrab(value)
    }
}

/// Errors when running the `gh` command
enum GHError {
    Io,
    InvalidUtf8,
}

impl From<io::Error> for GHError {
    fn from(_: io::Error) -> Self {
        Self::Io
    }
}

impl From<string::FromUtf8Error> for GHError {
    fn from(_: string::FromUtf8Error) -> Self {
        Self::InvalidUtf8
    }
}

pub struct GitHubClient {
    remote: Remote,
}

impl GitHubClient {
    pub fn try_load_new() -> anyhow::Result<Self> {
        Ok(Self {
            remote: GitHubClient::get_remote()?,
        })
    }

    pub async fn list_pull_requests(&self) -> Result<Option<Vec<PullRequest>>, GitHubError> {
        let token = GitHubClient::get_access_token()?;
        let octocrab = Octocrab::builder().personal_token(token).build()?;
        Ok(octocrab
            .get(
                format!(
                    "/repos/{}/{}/pulls",
                    &self.remote.owner, &self.remote.repository
                ),
                None::<&()>,
            )
            .await?)
    }

    pub async fn retrieve_pull_request(
        &self,
        head: &str,
    ) -> Result<Option<PullRequest>, GitHubError> {
        let token = GitHubClient::get_access_token()?;
        let octocrab = Octocrab::builder().personal_token(token).build()?;
        let results: Vec<PullRequest> = octocrab
            .get(
                format!(
                    "/repos/{}/{}/pulls?head={}:{}",
                    &self.remote.owner, &self.remote.repository, &self.remote.owner, head
                ),
                None::<&()>,
            )
            .await?;

        // Take ownership and return the first value
        Ok(results.into_iter().next())
    }

    pub async fn create_pull_request(
        &self,
        head: String,
        base: String,
        title: Option<String>,
        body: Option<String>,
        draft: Option<bool>,
    ) -> Result<PullRequest, GitHubError> {
        let body = PullRequestCreateBody {
            head,
            base,
            title,
            body,
            head_repo: None,
            maintainer_can_modify: None,
            draft,
            issue: None,
        };

        let token = GitHubClient::get_access_token()?;
        let octocrab = Octocrab::builder().personal_token(token).build()?;
        let pull_request = octocrab
            .post::<PullRequestCreateBody, PullRequest>(
                format!(
                    "/repos/{}/{}/pulls",
                    &self.remote.owner, &self.remote.repository
                ),
                Some(&body),
            )
            .await?;
        Ok(pull_request)
    }

    pub async fn get_default_branch(&self) -> Result<String, GitHubError> {
        let token = GitHubClient::get_access_token()?;
        let octocrab = Octocrab::builder().personal_token(token).build()?;
        let repo = octocrab
            .repos(&self.remote.owner, &self.remote.repository)
            .get()
            .await?;
        Ok(repo.default_branch.unwrap_or("main".to_string()))
    }

    fn get_access_token() -> Result<String, GitHubError> {
        env::var("GITHUB_TOKEN")
            .or_else(|_| -> Result<String, GHError> {
                let output = Command::new("gh").args(["auth", "token"]).output()?;
                Ok(String::from_utf8(output.stdout)?.trim().to_string())
            })
            .or(Err(GitHubError::InvalidToken))
    }

    // TODO: Better err handling
    fn get_remote() -> anyhow::Result<Remote> {
        let output = Command::new("jj")
            .args(["git", "remote", "list"])
            .output()
            .context("Failed to list jj remotes")?;
        let remote_raw = String::from_utf8(output.stdout).context("Not valid UTF-8")?;

        let mut remote_split = remote_raw.split(" ");
        let (Some(name), Some(url)) = (remote_split.next(), remote_split.next()) else {
            return Err(anyhow!("Invalid remote format"));
        };

        Remote::from_url_str(url, name).context("URL Parse error")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
