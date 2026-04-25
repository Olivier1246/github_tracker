#!/usr/bin/env bash
# ============================================================
# GitHub Tracker — Script d'installation
# Cible : Debian / Ubuntu (VM Proxmox)
# Usage : sudo bash install.sh
# ============================================================
set -euo pipefail

INSTALL_DIR="/opt/github-tracker"
SERVICE_USER="github-tracker"
SERVICE_FILE="/etc/systemd/system/github-tracker.service"
BINARY="github-tracker"

# --- Couleurs ---
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'

info()    { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()      { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error()   { echo -e "${RED}[ERREUR]${NC} $*"; exit 1; }
header()  { echo -e "\n${BOLD}$*${NC}"; }

# ---- Vérifications préalables --------------------------------
[ "$EUID" -eq 0 ] || error "Lancez ce script en root : sudo bash install.sh"
[ -f "Cargo.toml" ] || error "Cargo.toml introuvable. Exécutez install.sh depuis la racine du projet."

header "=== Installation de GitHub Tracker ==="

# ---- Dépendances système -------------------------------------
header "1/6 — Dépendances système"
apt-get update -qq
apt-get install -y -qq curl build-essential pkg-config libssl-dev
ok "Dépendances installées"

# ---- Rust ----------------------------------------------------
header "2/6 — Rust toolchain"
if ! command -v cargo &>/dev/null; then
    info "Rust non trouvé — installation en cours..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --no-modify-path --profile minimal
    # Rendre cargo disponible dans la session actuelle
    export PATH="$HOME/.cargo/bin:$PATH"
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env" 2>/dev/null || true
fi
ok "Rust : $(rustc --version)"

# ---- Compilation ----------------------------------------------
header "3/6 — Compilation (peut prendre quelques minutes)"
cargo build --release
ok "Binaire compilé : target/release/$BINARY"

# ---- Utilisateur système --------------------------------------
header "4/6 — Utilisateur de service"
if ! id -u "$SERVICE_USER" &>/dev/null; then
    useradd --system --shell /usr/sbin/nologin \
            --home-dir "$INSTALL_DIR" --create-home "$SERVICE_USER"
    ok "Utilisateur '$SERVICE_USER' créé"
else
    warn "L'utilisateur '$SERVICE_USER' existe déjà — ignoré"
fi

# ---- Installation des fichiers --------------------------------
header "5/6 — Installation des fichiers"
mkdir -p "$INSTALL_DIR"

# Binaire
install -m 755 "target/release/$BINARY" "$INSTALL_DIR/$BINARY"
ok "Binaire → $INSTALL_DIR/$BINARY"

# repos.toml (ne pas écraser si déjà présent)
if [ ! -f "$INSTALL_DIR/repos.toml" ]; then
    cp repos.toml "$INSTALL_DIR/repos.toml"
    ok "repos.toml → $INSTALL_DIR/repos.toml"
else
    warn "repos.toml déjà présent dans $INSTALL_DIR — non écrasé"
fi

# Configuration .env (interactive)
if [ ! -f "$INSTALL_DIR/.env" ]; then
    echo ""
    echo -e "${BOLD}Configuration Telegram${NC}"
    read -rp "  Token du bot Telegram          : " TG_TOKEN
    read -rp "  Chat ID Telegram               : " TG_CHAT
    read -rp "  Intervalle de vérif. (s) [300] : " INTERVAL
    INTERVAL="${INTERVAL:-300}"
    read -rp "  Port web [8080]                : " WEB_PORT
    WEB_PORT="${WEB_PORT:-8080}"
    read -rp "  Token GitHub (optionnel)       : " GH_TOKEN

    {
        echo "TELEGRAM_BOT_TOKEN=$TG_TOKEN"
        echo "TELEGRAM_CHAT_ID=$TG_CHAT"
        echo "CHECK_INTERVAL_SECONDS=$INTERVAL"
        echo "WEB_HOST=0.0.0.0"
        echo "WEB_PORT=$WEB_PORT"
        [ -n "$GH_TOKEN" ] && echo "GITHUB_TOKEN=$GH_TOKEN"
    } > "$INSTALL_DIR/.env"

    chmod 600 "$INSTALL_DIR/.env"
    ok ".env configuré dans $INSTALL_DIR/.env"
else
    warn ".env déjà présent dans $INSTALL_DIR — non écrasé"
fi

chown -R "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR"
ok "Permissions appliquées sur $INSTALL_DIR"

# ---- Service systemd -----------------------------------------
header "6/6 — Service systemd"
cat > "$SERVICE_FILE" << EOF
[Unit]
Description=GitHub Tracker — Surveillance de dépôts GitHub
Documentation=https://github.com
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$SERVICE_USER
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/$BINARY
Restart=on-failure
RestartSec=15
Environment=RUST_LOG=info

# Sécurité minimale
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=$INSTALL_DIR

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now github-tracker
ok "Service activé et démarré"

# ---- Résumé --------------------------------------------------
LOCAL_IP=$(hostname -I | awk '{print $1}')
WEB_PORT_DISPLAY=$(grep -E '^WEB_PORT=' "$INSTALL_DIR/.env" | cut -d= -f2 || echo "8080")

echo ""
echo -e "${GREEN}${BOLD}============================================${NC}"
echo -e "${GREEN}${BOLD}  Installation terminée avec succès !${NC}"
echo -e "${GREEN}${BOLD}============================================${NC}"
echo ""
echo -e "  Interface web  : ${BOLD}http://$LOCAL_IP:$WEB_PORT_DISPLAY${NC}"
echo -e "  Statut service : ${BOLD}systemctl status github-tracker${NC}"
echo -e "  Journaux       : ${BOLD}journalctl -u github-tracker -f${NC}"
echo -e "  Modifier repos : ${BOLD}nano $INSTALL_DIR/repos.toml && systemctl restart github-tracker${NC}"
echo ""
