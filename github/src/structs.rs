use serde::Deserialize;

// Semantic API of Octocrab has incorrect types.

#[derive(Deserialize, Debug)]
pub struct User {
    pub id: u64,
    pub login: String,
    pub avatar_url: String,
}

#[derive(Deserialize, Debug)]
pub struct Team {
    pub id: u64,
    pub slug: String,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Label {
    pub id: u64,
    pub name: String,
    pub color: String,
}

#[derive(Deserialize, Debug)]
pub struct Ref {
    pub label: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}

#[derive(Deserialize, Debug)]
pub struct PullRequest {
    pub id: u64,
    pub number: u64,
    pub state: String,
    pub title: String,
    pub user: User,
    pub body: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub merged_at: Option<String>,
    pub merge_commit_sha: Option<String>,
    pub head: Ref,
    pub base: Ref,
    pub assignees: Vec<User>,
    pub requested_reviewers: Vec<User>,
    pub requested_teams: Vec<Team>,
    pub labels: Vec<Label>,
    pub draft: bool,
    // pub auto_merge: Option<bool>,
    pub locked: bool,
    pub active_lock_reason: Option<bool>,
    // TODO: More fields
}
