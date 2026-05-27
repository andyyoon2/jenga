use std::env;

use octocrab::{Octocrab, Page, models::pulls::SimplePullRequest};

use crate::remote::Remote;

pub struct GitHub {}

impl GitHub {
    pub async fn list_pull_requests(
        remote: Remote,
    ) -> Result<Page<SimplePullRequest>, octocrab::Error> {
        let token = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN env variable is required");
        let octocrab = Octocrab::builder().personal_token(token).build()?;
        octocrab
            .pulls(remote.owner, remote.repository)
            .list()
            .send()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
