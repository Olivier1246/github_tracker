use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub owner: String,
    pub repo: String,
    #[serde(default = "default_true")]
    pub notify_releases: bool,
    #[serde(default = "default_true")]
    pub notify_stars: bool,
    #[serde(default)]
    pub notify_forks: bool,
}

fn default_true() -> bool {
    true
}

impl RepoConfig {
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReposConfig {
    pub repos: Vec<RepoConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoState {
    pub full_name: String,
    pub stars: u64,
    pub forks: u64,
    pub latest_release: Option<String>,
    pub description: Option<String>,
    pub html_url: String,
    pub last_checked: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepo {
    pub full_name: String,
    pub stargazers_count: u64,
    pub forks_count: u64,
    pub description: Option<String>,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub html_url: String,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub repo: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub kind: NotificationKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationKind {
    NewRelease,
    StarChange,
    ForkChange,
    Info,
    Error,
}

impl std::fmt::Display for NotificationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewRelease => write!(f, "Release"),
            Self::StarChange => write!(f, "Stars"),
            Self::ForkChange => write!(f, "Forks"),
            Self::Info => write!(f, "Info"),
            Self::Error => write!(f, "Erreur"),
        }
    }
}
