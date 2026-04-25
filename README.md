# GitHub Tracker

Surveillance de dépôts GitHub publics avec notifications Telegram et interface web locale (LAN).

## Fonctionnalités

- **Surveillance automatique** : vérifie les dépôts à intervalle configurable
- **Notifications Telegram** : alertes pour nouvelles releases, changements de stars et de forks
- **Interface web LAN** : tableau de bord accessible depuis tout appareil du réseau
- **Configuration simple** : fichier `.env` et `repos.toml` pour tout paramétrer
- **Démarrage automatique** : service systemd intégré
- **API JSON** : endpoints `/api/repos` et `/api/notifications`

## Prérequis

- VM Debian/Ubuntu (Proxmox ou autre)
- Accès Internet depuis la VM
- Un bot Telegram (créé via @BotFather)

## Installation rapide

```bash
sudo bash install.sh
```

Le script installe Rust si nécessaire, compile, crée un utilisateur système dédié,
configure le service systemd et démarre l'application.

## Configuration

### 1. Fichier .env

```bash
cp .env.example .env
nano .env
```

| Variable                 | Description                           | Défaut    |
|--------------------------|---------------------------------------|-----------|
| `TELEGRAM_BOT_TOKEN`     | Token du bot Telegram                 | *requis*  |
| `TELEGRAM_CHAT_ID`       | ID du chat de destination             | *requis*  |
| `CHECK_INTERVAL_SECONDS` | Intervalle entre vérifications (s)    | `300`     |
| `WEB_HOST`               | Adresse d'écoute                      | `0.0.0.0` |
| `WEB_PORT`               | Port du serveur web                   | `8080`    |
| `GITHUB_TOKEN`           | Token GitHub (5 000 req/h au lieu 60) | optionnel |

### 2. Liste des dépôts — repos.toml

```toml
[[repos]]
owner = "rust-lang"
repo = "rust"
notify_releases = true
notify_stars    = true
notify_forks    = false
```

### 3. Compilation manuelle

```bash
cargo build --release
./target/release/github-tracker
```

## Interface web

`http://<IP-VM>:<WEB_PORT>` — auto-refresh toutes les 60 secondes.

## API REST

| Endpoint               | Description                      |
|------------------------|----------------------------------|
| `GET /`                | Interface web HTML               |
| `GET /api/repos`       | État de tous les dépôts (JSON)   |
| `GET /api/notifications` | Historique des notifications   |

## Gestion du service

```bash
systemctl status github-tracker
journalctl -u github-tracker -f
systemctl restart github-tracker   # après modif repos.toml
```

## Limites API GitHub

| Contexte            | Limite          |
|---------------------|-----------------|
| Sans token          | 60 req/h        |
| Avec GITHUB_TOKEN   | 5 000 req/h     |

2 requêtes par dépôt par vérification.

## Structure

Voir [docs/STRUCTURE.md](docs/STRUCTURE.md) et [docs/DOCUMENTATION.md](docs/DOCUMENTATION.md).
