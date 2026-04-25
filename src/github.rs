use reqwest::Client;

use crate::models::{GitHubRelease, GitHubRepo};

pub struct GitHubClient {
    client: Client,
    token: Option<String>,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Self {
        let client = Client::builder()
            .user_agent("github-tracker/0.1.0")
            .build()
            .expect("Échec de construction du client HTTP");
        Self { client, token }
    }

    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {}", t))
    }

    pub async fn get_repo(&self, owner: &str, repo: &str) -> Result<GitHubRepo, String> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        let mut req = self.client.get(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("Requête échouée: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("GitHub API a retourné {}", resp.status()));
        }

        resp.json::<GitHubRepo>()
            .await
            .map_err(|e| format!("Échec de parsing de la réponse: {}", e))
    }

    pub async fn get_latest_release(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Option<GitHubRelease>, String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            owner, repo
        );
        let mut req = self.client.get(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("Requête échouée: {}", e))?;

        // 404 means no releases yet
        if resp.status().as_u16() == 404 {
            return Ok(None);
        }

        if !resp.status().is_success() {
            return Err(format!("GitHub API a retourné {}", resp.status()));
        }

        let release = resp
            .json::<GitHubRelease>()
            .await
            .map_err(|e| format!("Échec de parsing de la réponse: {}", e))?;
        Ok(Some(release))
    }
}
