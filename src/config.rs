use std::env;
use std::fs;

use crate::models::{RepoConfig, ReposConfig};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub telegram_bot_token: String,
    pub telegram_chat_id: String,
    pub check_interval_secs: u64,
    pub web_host: String,
    pub web_port: u16,
    pub github_token: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let telegram_bot_token = env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| "TELEGRAM_BOT_TOKEN non défini dans .env")?;

        let telegram_chat_id = env::var("TELEGRAM_CHAT_ID")
            .map_err(|_| "TELEGRAM_CHAT_ID non défini dans .env")?;

        let check_interval_secs = env::var("CHECK_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<u64>()
            .map_err(|_| "CHECK_INTERVAL_SECONDS doit être un nombre")?;

        let web_host = env::var("WEB_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let web_port = env::var("WEB_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|_| "WEB_PORT doit être un numéro de port valide")?;

        let github_token = env::var("GITHUB_TOKEN").ok();

        Ok(Self {
            telegram_bot_token,
            telegram_chat_id,
            check_interval_secs,
            web_host,
            web_port,
            github_token,
        })
    }
}

pub fn load_repos(path: &str) -> Result<ReposConfig, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Impossible de lire {}: {}", path, e))?;
    toml::from_str(&content)
        .map_err(|e| format!("Impossible de parser {}: {}", path, e))
}

pub fn save_repos(path: &str, repos: &[RepoConfig]) -> Result<(), String> {
    let config = ReposConfig { repos: repos.to_vec() };
    let body = toml::to_string_pretty(&config)
        .map_err(|e| format!("Erreur de sérialisation: {}", e))?;
    let content = format!(
        "# Géré automatiquement par l'interface web GitHub Tracker\n\n{}",
        body
    );
    fs::write(path, content)
        .map_err(|e| format!("Impossible d'écrire {}: {}", path, e))
}
