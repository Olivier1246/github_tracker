mod config;
mod github;
mod models;
mod state;
mod telegram;
mod web;

use chrono::Utc;
use std::sync::{Arc, Mutex};
use tokio::time::Duration;

use config::{load_repos, AppConfig};
use github::GitHubClient;
use models::{Notification, NotificationKind, RepoConfig, RepoState};
use state::AppState;
use telegram::TelegramClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    dotenvy::dotenv().ok();

    let app_config = AppConfig::from_env().unwrap_or_else(|e| {
        eprintln!("[ERREUR] Configuration invalide: {}", e);
        eprintln!("Copiez .env.example vers .env et renseignez vos valeurs.");
        std::process::exit(1);
    });

    let initial_repos = load_repos("repos.toml")
        .map(|c| c.repos)
        .unwrap_or_else(|e| {
            log::warn!("repos.toml non chargé ({}), démarrage sans dépôts.", e);
            Vec::new()
        });

    log::info!("=== GitHub Tracker démarré ===");
    log::info!("Dépôts initiaux : {}", initial_repos.len());
    log::info!("Intervalle de vérification : {}s", app_config.check_interval_secs);

    let shared_state = Arc::new(Mutex::new(AppState::new(initial_repos)));

    let github_client = Arc::new(GitHubClient::new(app_config.github_token.clone()));
    let telegram_client = Arc::new(TelegramClient::new(
        app_config.telegram_bot_token.clone(),
        app_config.telegram_chat_id.clone(),
    ));

    // Monitoring loop — reads repos dynamically from state
    {
        let state = Arc::clone(&shared_state);
        let github = Arc::clone(&github_client);
        let telegram = Arc::clone(&telegram_client);
        let interval_secs = app_config.check_interval_secs;

        tokio::spawn(async move {
            loop {
                let repos: Vec<RepoConfig> = {
                    let s = state.lock().unwrap();
                    s.repos.clone()
                };
                if !repos.is_empty() {
                    check_repos(&repos, &github, &telegram, &state).await;
                }
                tokio::time::sleep(Duration::from_secs(interval_secs)).await;
            }
        });
    }

    web::start_web_server(
        Arc::clone(&shared_state),
        app_config.web_host,
        app_config.web_port,
    )
    .await?;

    Ok(())
}

async fn check_repos(
    repos: &[RepoConfig],
    github: &GitHubClient,
    telegram: &TelegramClient,
    state: &Arc<Mutex<AppState>>,
) {
    log::info!("Vérification de {} dépôt(s)...", repos.len());

    for repo_cfg in repos {
        let owner = &repo_cfg.owner;
        let name = &repo_cfg.repo;
        let full_name = repo_cfg.full_name();

        let gh_repo = match github.get_repo(owner, name).await {
            Ok(r) => r,
            Err(e) => {
                log::error!("Échec de récupération de {}: {}", full_name, e);
                let mut s = state.lock().unwrap();
                s.add_notification(Notification {
                    repo: full_name.clone(),
                    message: format!("Erreur API: {}", e),
                    timestamp: Utc::now(),
                    kind: NotificationKind::Error,
                });
                continue;
            }
        };

        let latest_release = match github.get_latest_release(owner, name).await {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Impossible de récupérer la release de {}: {}", full_name, e);
                None
            }
        };

        let latest_release_tag = latest_release.as_ref().map(|r| r.tag_name.clone());

        let prev_state: Option<RepoState> = {
            let s = state.lock().unwrap();
            s.repo_states.get(&full_name).cloned()
        };

        let mut new_notifications: Vec<Notification> = Vec::new();

        if let Some(prev) = &prev_state {
            if repo_cfg.notify_stars && gh_repo.stargazers_count != prev.stars {
                let diff = gh_repo.stargazers_count as i64 - prev.stars as i64;
                let tg_msg = format!(
                    "{} <b>{}</b>\nStars : {} → {} ({:+})\n<a href=\"{}\">{}</a>",
                    if diff > 0 { "📈" } else { "📉" },
                    full_name, prev.stars, gh_repo.stargazers_count, diff,
                    gh_repo.html_url, gh_repo.html_url,
                );
                log::info!("{}: stars {} → {} ({:+})", full_name, prev.stars, gh_repo.stargazers_count, diff);
                send_telegram(telegram, &tg_msg).await;
                new_notifications.push(Notification {
                    repo: full_name.clone(),
                    message: format!("Stars : {} → {} ({:+})", prev.stars, gh_repo.stargazers_count, diff),
                    timestamp: Utc::now(),
                    kind: NotificationKind::StarChange,
                });
            }

            if repo_cfg.notify_forks && gh_repo.forks_count != prev.forks {
                let diff = gh_repo.forks_count as i64 - prev.forks as i64;
                let tg_msg = format!(
                    "🍴 <b>{}</b>\nForks : {} → {} ({:+})\n<a href=\"{}\">{}</a>",
                    full_name, prev.forks, gh_repo.forks_count, diff,
                    gh_repo.html_url, gh_repo.html_url,
                );
                log::info!("{}: forks {} → {} ({:+})", full_name, prev.forks, gh_repo.forks_count, diff);
                send_telegram(telegram, &tg_msg).await;
                new_notifications.push(Notification {
                    repo: full_name.clone(),
                    message: format!("Forks : {} → {} ({:+})", prev.forks, gh_repo.forks_count, diff),
                    timestamp: Utc::now(),
                    kind: NotificationKind::ForkChange,
                });
            }

            if repo_cfg.notify_releases {
                if let Some(new_tag) = &latest_release_tag {
                    let is_new = prev.latest_release.as_ref().map_or(true, |old| old != new_tag);
                    if is_new {
                        let release = latest_release.as_ref().unwrap();
                        let tg_msg = format!(
                            "🚀 <b>{}</b>\nNouvelle release : <b>{}</b>\n<a href=\"{}\">{}</a>",
                            full_name, new_tag, release.html_url, release.html_url,
                        );
                        log::info!("{}: nouvelle release {}", full_name, new_tag);
                        send_telegram(telegram, &tg_msg).await;
                        new_notifications.push(Notification {
                            repo: full_name.clone(),
                            message: format!("Nouvelle release : {}", new_tag),
                            timestamp: Utc::now(),
                            kind: NotificationKind::NewRelease,
                        });
                    }
                }
            }
        } else {
            log::info!(
                "Premier passage pour {} — Stars : {}, Forks : {}",
                full_name, gh_repo.stargazers_count, gh_repo.forks_count
            );
        }

        {
            let mut s = state.lock().unwrap();
            s.update_repo_state(RepoState {
                full_name: full_name.clone(),
                stars: gh_repo.stargazers_count,
                forks: gh_repo.forks_count,
                latest_release: latest_release_tag,
                description: gh_repo.description.clone(),
                html_url: gh_repo.html_url.clone(),
                last_checked: Utc::now(),
            });
            for notif in new_notifications {
                s.add_notification(notif);
            }
        }

        log::info!("✓ {} — ⭐ {} 🍴 {}", full_name, gh_repo.stargazers_count, gh_repo.forks_count);
    }
}

async fn send_telegram(client: &TelegramClient, msg: &str) {
    if let Err(e) = client.send_message(msg).await {
        log::error!("Échec envoi Telegram: {}", e);
    }
}
