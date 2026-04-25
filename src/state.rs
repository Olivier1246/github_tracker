use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::models::{Notification, RepoConfig, RepoState};

pub type SharedState = Arc<Mutex<AppState>>;

#[derive(Debug)]
pub struct AppState {
    pub repos: Vec<RepoConfig>,
    pub repo_states: HashMap<String, RepoState>,
    pub notifications: Vec<Notification>,
}

impl AppState {
    pub fn new(repos: Vec<RepoConfig>) -> Self {
        Self {
            repos,
            repo_states: HashMap::new(),
            notifications: Vec::new(),
        }
    }

    pub fn add_repo(&mut self, config: RepoConfig) -> bool {
        let full = config.full_name();
        if self.repos.iter().any(|r| r.full_name() == full) {
            return false;
        }
        self.repos.push(config);
        true
    }

    pub fn remove_repo(&mut self, full_name: &str) {
        self.repos.retain(|r| r.full_name() != full_name);
        self.repo_states.remove(full_name);
    }

    pub fn toggle_notify(&mut self, full_name: &str, field: &str) {
        if let Some(repo) = self.repos.iter_mut().find(|r| r.full_name() == full_name) {
            match field {
                "stars"    => repo.notify_stars    = !repo.notify_stars,
                "forks"    => repo.notify_forks    = !repo.notify_forks,
                "releases" => repo.notify_releases = !repo.notify_releases,
                _ => {}
            }
        }
    }

    pub fn add_notification(&mut self, notif: Notification) {
        self.notifications.push(notif);
        if self.notifications.len() > 100 {
            let drain = self.notifications.len() - 100;
            self.notifications.drain(0..drain);
        }
    }

    pub fn update_repo_state(&mut self, state: RepoState) {
        self.repo_states.insert(state.full_name.clone(), state);
    }
}
