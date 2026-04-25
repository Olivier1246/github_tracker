# Documentation technique — GitHub Tracker

## Architecture générale

GitHub Tracker est une application Rust mono-binaire organisée autour de deux tâches Tokio concurrentes :

1. **Boucle de surveillance** (tokio::spawn) — interroge périodiquement l'API GitHub,
   compare les résultats avec l'état précédent et envoie des notifications Telegram.
2. **Serveur web** (actix-web, bloquant) — expose un tableau de bord HTML et une API JSON.

L'état applicatif est partagé entre ces deux tâches via `Arc<Mutex<AppState>>`.

---

## Installation sur Proxmox (VM Debian/Ubuntu)

### Créer la VM

1. Dans Proxmox, créer une VM avec :
   - OS : Debian 12 ou Ubuntu 22.04 LTS
   - RAM : 256 Mo minimum (512 Mo recommandé)
   - Disque : 4 Go minimum
   - Réseau : bridge sur le LAN (ex. `vmbr0`)

2. Démarrer la VM et noter son adresse IP (`ip a`).

### Déployer l'application

```bash
# Sur la VM, en tant que root
apt update && apt install -y git

# Cloner le projet
git clone <url> /root/github-tracker
cd /root/github-tracker

# Lancer l'installation (interactive)
bash install.sh
```

L'installateur :
- Installe les dépendances système (`build-essential`, `libssl-dev`, etc.)
- Installe la toolchain Rust via rustup si absente
- Compile le binaire en mode release
- Crée l'utilisateur système `github-tracker`
- Installe les fichiers dans `/opt/github-tracker/`
- Crée et active le service systemd `github-tracker`

### Accéder au tableau de bord

Depuis n'importe quel appareil du LAN :
```
http://<IP-de-la-VM>:8080
```

---

## Configuration détaillée

### Obtenir un token Telegram

1. Ouvrir Telegram et contacter **@BotFather**
2. Envoyer `/newbot`, suivre les instructions
3. Récupérer le token (format `123456789:ABCdef...`)
4. Pour obtenir votre Chat ID : envoyer un message à votre bot,
   puis appeler `https://api.telegram.org/bot<TOKEN>/getUpdates`
   et relever `message.chat.id`

### Limites de l'API GitHub

L'API publique de GitHub autorise :
- **60 requêtes/heure** sans authentification (par IP)
- **5 000 requêtes/heure** avec un Personal Access Token

Chaque cycle de vérification consomme **2 requêtes par dépôt** :
une pour les métadonnées (`/repos/{owner}/{repo}`) et
une pour la dernière release (`/releases/latest`).

Calcul de l'intervalle minimum sans token :
```
intervalle_min = (nb_repos * 2) / 60 * 3600 secondes
```
Exemple : 10 dépôts → intervalle_min = 1200s (20 min)

Avec un token GitHub (recommandé) : 10 dépôts toutes les 5 minutes = 240 req/h, très en-dessous de la limite.

**Créer un token sans scope** (lecture seule publique) :
[https://github.com/settings/tokens/new](https://github.com/settings/tokens/new) — décocher tous les scopes.

---

## Fonctionnement de la surveillance

### Premier passage

Lors du démarrage, la boucle effectue un premier passage pour **peupler l'état initial**.
Aucune notification n'est envoyée à ce stade (pas d'état précédent à comparer).

### Passages suivants

Pour chaque dépôt, les valeurs actuelles sont comparées à l'état mémorisé :

| Changement détecté | Condition d'activation | Message Telegram |
|---|---|---|
| Nouvelle release | `notify_releases = true` et tag différent | 🚀 Nom du dépôt + lien |
| Variation de stars | `notify_stars = true` et count différent | 📈/📉 Ancienne → Nouvelle valeur |
| Variation de forks | `notify_forks = true` et count différent | 🍴 Ancienne → Nouvelle valeur |

L'état est persisté en mémoire uniquement (redémarrage = reprise à zéro, premier passage sans notification).

### Gestion d'erreurs

- Erreur API GitHub : loguée, notification d'erreur ajoutée au dashboard, dépôt ignoré pour ce cycle
- Erreur Telegram : loguée, pas de retry (l'état est quand même mis à jour)
- Réponse 404 sur `/releases/latest` : interprétée comme "pas de release" (normal)

---

## Interface web

Le dashboard HTML est généré à chaque requête `GET /` directement en Rust (pas de moteur de template externe). Il contient :

- **Grille de cartes** : un dépôt par carte (stars, forks, dernière release, lien GitHub)
- **Liste de notifications** : les 50 plus récentes, ordre anti-chronologique
- **Auto-refresh** : `<meta http-equiv="refresh" content="60">`
- **Design dark** : compatible GitHub dark theme

### API JSON

```bash
# État des dépôts
curl http://<IP>:8080/api/repos | jq .

# Historique des notifications
curl http://<IP>:8080/api/notifications | jq .
```

Réponses conformes aux structures `RepoState` et `Notification` définies dans `src/models.rs`.

---

## Journaux

Le niveau de log est contrôlé par la variable `RUST_LOG` (définie dans le service systemd à `info`).

Niveaux disponibles : `error`, `warn`, `info`, `debug`, `trace`

```bash
# Journaux en temps réel
journalctl -u github-tracker -f

# Journaux depuis le démarrage
journalctl -u github-tracker --since today

# Activer le mode debug (temporaire)
systemctl edit github-tracker --force
# Ajouter dans la section [Service] :
# Environment=RUST_LOG=debug
systemctl restart github-tracker
```

---

## Mise à jour

```bash
cd /root/github-tracker   # ou l'endroit où vous avez cloné le projet
git pull

# Recompiler
cargo build --release

# Installer la nouvelle version
sudo systemctl stop github-tracker
sudo install -m 755 target/release/github-tracker /opt/github-tracker/github-tracker
sudo systemctl start github-tracker
```

---

## Sécurité

- Le service tourne sous l'utilisateur `github-tracker` (pas de privilèges root)
- `NoNewPrivileges=true` empêche l'escalade de privilèges
- `ProtectSystem=strict` monte le système de fichiers en lecture seule (sauf `/opt/github-tracker`)
- `PrivateTmp=true` isole `/tmp`
- Le fichier `.env` a les permissions `600` (lecture propriétaire uniquement)
- Les tokens ne transitent jamais dans les logs (seuls les messages d'erreur HTTP sont loggés)

---

## Dépannage

| Symptôme | Cause probable | Solution |
|---|---|---|
| Service ne démarre pas | `.env` absent ou incomplet | Vérifier `/opt/github-tracker/.env` |
| Pas de notification Telegram | Token ou Chat ID incorrect | Tester via `curl` l'API Telegram |
| Erreur 403 GitHub | Rate limit dépassé | Ajouter un `GITHUB_TOKEN` ou augmenter l'intervalle |
| Tableau de bord vide | Premier passage en cours | Attendre la fin du premier cycle |
| Port 8080 inaccessible | Pare-feu VM ou hôte | `ufw allow 8080` sur la VM |
