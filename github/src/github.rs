use std::{env, fmt, io, process::Command, string};

use octocrab::Octocrab;

use crate::{PullRequest, remote::Remote};

pub struct GitHub {}

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

impl GitHub {
    pub async fn list_pull_requests(
        remote: Remote,
    ) -> Result<Option<Vec<PullRequest>>, GitHubError> {
        let token = GitHub::get_access_token()?;
        let octocrab = Octocrab::builder().personal_token(token).build()?;
        Ok(octocrab
            .get(
                format!("/repos/{}/{}/pulls", remote.owner, remote.repository),
                None::<&()>,
            )
            .await?)
    }

    fn get_access_token() -> Result<String, GitHubError> {
        env::var("GITHUB_TOKEN")
            .or_else(|_| -> Result<String, GHError> {
                let output = Command::new("gh").args(["auth", "token"]).output()?;
                Ok(String::from_utf8(output.stdout)?.trim().to_string())
            })
            .or(Err(GitHubError::InvalidToken))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
