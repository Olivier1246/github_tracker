use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::models::{Notification, RepoState};

pub type SharedState = Arc<Mutex<AppState>>;

#[derive(Debug, Default)]
pub struct AppState {
    pub repos: HashMap<String, RepoState>,
    pub notifications: Vec<Notification>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_notification(&mut self, notif: Notification) {
        self.notifications.push(notif);
        // Keep only the 100 most recent notifications
        if self.notifications.len() > 100 {
            let drain_count = self.notifications.len() - 100;
            self.notifications.drain(0..drain_count);
        }
    }

    pub fn update_repo(&mut self, state: RepoState) {
        self.repos.insert(state.full_name.clone(), state);
    }
}
