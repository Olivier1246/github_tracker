# Structure du projet GitHub Tracker

```
github-tracker/
├── src/
│   ├── main.rs         Point d'entrée — orchestration, boucle de surveillance
│   ├── config.rs       Chargement de la configuration (.env et repos.toml)
│   ├── models.rs       Structures de données partagées
│   ├── github.rs       Client HTTP pour l'API GitHub
│   ├── telegram.rs     Envoi de messages via l'API Telegram Bot
│   ├── web.rs          Serveur web actix-web (dashboard + API JSON)
│   └── state.rs        État partagé (Arc<Mutex<AppState>>)
│
├── docs/
│   ├── STRUCTURE.md    Ce fichier — arborescence et rôle de chaque composant
│   └── DOCUMENTATION.md Documentation technique complète
│
├── Cargo.toml          Manifest du projet Rust et dépendances
├── .env.example        Modèle de configuration — copier en .env et remplir
├── repos.toml          Liste des dépôts GitHub à surveiller
├── install.sh          Script d'installation pour Debian/Ubuntu
├── github-tracker.service  Unité systemd de référence
├── .gitignore          Fichiers ignorés par git
└── README.md           Documentation principale
```

## Rôle de chaque module

### `src/main.rs`
Initialise le logger, charge la configuration, crée les clients GitHub et Telegram,
lance la boucle de surveillance dans une tâche Tokio en arrière-plan,
puis démarre le serveur web (bloquant).

### `src/config.rs`
- `AppConfig::from_env()` : lit les variables d'environnement (via dotenvy)
- `load_repos(path)` : parse `repos.toml` avec la crate `toml`

### `src/models.rs`
Toutes les structures `serde::{Serialize, Deserialize}` :
- `RepoConfig` — configuration d'un dépôt (owner, repo, flags de notification)
- `ReposConfig` — liste de `RepoConfig`, racine du TOML
- `RepoState` — état courant d'un dépôt (stars, forks, dernière release, URL…)
- `GitHubRepo` / `GitHubRelease` — réponses désérialisées de l'API GitHub
- `Notification` / `NotificationKind` — événements stockés dans l'état partagé

### `src/github.rs`
`GitHubClient` : client `reqwest` avec User-Agent et token optionnel.
- `get_repo(owner, repo)` → `GitHubRepo`
- `get_latest_release(owner, repo)` → `Option<GitHubRelease>`

### `src/telegram.rs`
`TelegramClient` : appelle `sendMessage` de l'API Bot Telegram (HTML parse mode).

### `src/state.rs`
`AppState` : `HashMap<String, RepoState>` + `Vec<Notification>` (plafond 100).
Partagé via `Arc<Mutex<AppState>>` entre la boucle de surveillance et le serveur web.

### `src/web.rs`
Trois routes actix-web :
- `GET /` — dashboard HTML généré dynamiquement, auto-refresh 60s
- `GET /api/repos` — JSON des états de dépôts
- `GET /api/notifications` — JSON de l'historique des notifications

## Flux de données

```
repos.toml ──► AppConfig
                   │
                   ▼
            check_repos() ──► GitHubClient ──► api.github.com
                   │
                   ├── diff avec état précédent ──► TelegramClient ──► Telegram
                   │
                   └── Arc<Mutex<AppState>> ──► actix-web handlers ──► navigateur LAN
```

## Dépendances Cargo

| Crate       | Usage                                |
|-------------|--------------------------------------|
| tokio       | Runtime async (multi-thread)         |
| reqwest     | Client HTTP (GitHub + Telegram)      |
| serde       | Sérialisation/désérialisation        |
| serde_json  | JSON pour Telegram et API interne    |
| toml        | Parsing de repos.toml                |
| dotenvy     | Chargement du fichier .env           |
| actix-web   | Serveur web                          |
| chrono      | Horodatage des notifications         |
| log         | Macros de journalisation             |
| env_logger  | Backend de log configurable via RUST_LOG |
