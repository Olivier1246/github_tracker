use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono::Utc;
use serde::Deserialize;
use std::sync::{Arc, Mutex};

use crate::config::save_repos;
use crate::models::{Notification, NotificationKind, RepoConfig, RepoState};
use crate::state::AppState;

pub async fn start_web_server(
    state: Arc<Mutex<AppState>>,
    host: String,
    port: u16,
) -> std::io::Result<()> {
    let data = web::Data::new(state);
    log::info!("Interface web disponible sur http://{}:{}", host, port);

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .route("/", web::get().to(dashboard))
            .route("/repos", web::post().to(add_repo))
            .route("/repos/remove", web::post().to(remove_repo))
            .route("/api/repos", web::get().to(api_repos))
            .route("/api/notifications", web::get().to(api_notifications))
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}

// ── Form structs ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AddRepoForm {
    url: String,
}

#[derive(Deserialize)]
struct RemoveRepoForm {
    full_name: String,
}

// ── URL parser ───────────────────────────────────────────────────────────────

fn parse_github_url(input: &str) -> Option<(String, String)> {
    let s = input.trim().trim_end_matches('/');
    let s = s.strip_suffix(".git").unwrap_or(s);

    let path = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"))
        .or_else(|| s.strip_prefix("github.com/"))
        .unwrap_or(s);

    let parts: Vec<&str> = path.splitn(3, '/').collect();
    if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn add_repo(
    state: web::Data<Arc<Mutex<AppState>>>,
    form: web::Form<AddRepoForm>,
) -> impl Responder {
    match parse_github_url(&form.url) {
        None => {
            log::warn!("URL invalide reçue : {}", form.url);
        }
        Some((owner, repo)) => {
            let config = RepoConfig {
                owner: owner.clone(),
                repo: repo.clone(),
                notify_releases: true,
                notify_stars: true,
                notify_forks: false,
            };
            let repos = {
                let mut s = state.lock().unwrap();
                if s.add_repo(config) {
                    log::info!("Dépôt ajouté : {}/{}", owner, repo);
                } else {
                    log::warn!("Dépôt déjà surveillé : {}/{}", owner, repo);
                }
                s.repos.clone()
            };
            if let Err(e) = save_repos("repos.toml", &repos) {
                log::error!("Impossible de sauvegarder repos.toml : {}", e);
            }
        }
    }

    redirect("/")
}

async fn remove_repo(
    state: web::Data<Arc<Mutex<AppState>>>,
    form: web::Form<RemoveRepoForm>,
) -> impl Responder {
    let repos = {
        let mut s = state.lock().unwrap();
        s.remove_repo(&form.full_name);
        log::info!("Dépôt supprimé : {}", form.full_name);
        s.repos.clone()
    };
    if let Err(e) = save_repos("repos.toml", &repos) {
        log::error!("Impossible de sauvegarder repos.toml : {}", e);
    }
    redirect("/")
}

fn redirect(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header(("Location", location))
        .finish()
}

// ── Dashboard ────────────────────────────────────────────────────────────────

async fn dashboard(state: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let (repos_html, notifs_html, repo_count, notif_count) = {
        let s = state.lock().unwrap();

        let repos_html: String = if s.repos.is_empty() {
            r#"<p class="empty">Aucun dépôt surveillé. Ajoutez-en un via le formulaire ci-dessus.</p>"#
                .to_string()
        } else {
            s.repos.iter().map(|cfg| {
                let full_name = cfg.full_name();
                match s.repo_states.get(&full_name) {
                    Some(repo) => render_card(repo, &full_name),
                    None       => render_pending_card(&full_name),
                }
            }).collect()
        };

        let notifs_html: String = if s.notifications.is_empty() {
            r#"<p class="empty">Aucune notification pour l'instant.</p>"#.to_string()
        } else {
            s.notifications.iter().rev().take(50).map(render_notification).collect()
        };

        (repos_html, notifs_html, s.repos.len(), s.notifications.len())
    };

    let now = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="fr">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0">
<meta http-equiv="refresh" content="60">
<title>GitHub Tracker</title>
<style>
  *{{box-sizing:border-box;margin:0;padding:0}}
  body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:#0d1117;color:#c9d1d9;min-height:100vh}}
  header{{background:#161b22;border-bottom:1px solid #30363d;padding:16px 24px;position:sticky;top:0;z-index:10}}
  header h1{{font-size:1.3rem;color:#f0f6fc;font-weight:700}}
  .subtitle{{color:#8b949e;font-size:.8rem;margin-top:2px}}
  main{{max-width:1280px;margin:0 auto;padding:24px 24px 80px}}
  h2{{font-size:1rem;color:#f0f6fc;margin:24px 0 12px;padding-bottom:8px;border-bottom:1px solid #21262d;font-weight:600}}
  .add-wrap{{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:14px 16px;margin-bottom:20px;display:flex;gap:10px;align-items:center}}
  .add-wrap form{{display:flex;flex:1;gap:10px}}
  .url-input{{flex:1;background:#0d1117;border:1px solid #30363d;border-radius:6px;padding:8px 12px;color:#c9d1d9;font-size:.9rem;outline:none;min-width:0}}
  .url-input:focus{{border-color:#388bfd}}
  .url-input::placeholder{{color:#484f58}}
  .btn-add{{background:#238636;color:#fff;border:none;border-radius:6px;padding:8px 18px;cursor:pointer;font-weight:600;white-space:nowrap;font-size:.9rem}}
  .btn-add:hover{{background:#2ea043}}
  .repos-grid{{display:grid;grid-template-columns:repeat(auto-fill,minmax(300px,1fr));gap:14px;margin-bottom:8px}}
  .card{{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:16px;transition:border-color .15s}}
  .card:hover{{border-color:#388bfd}}
  .card-pending{{border-style:dashed;opacity:.75}}
  .card-header{{display:flex;justify-content:space-between;align-items:flex-start;margin-bottom:6px;gap:8px}}
  .card-header a{{color:#58a6ff;text-decoration:none;font-weight:600;font-size:.95rem;word-break:break-all;flex:1}}
  .card-header a:hover{{text-decoration:underline}}
  .btn-remove{{background:transparent;border:1px solid #6e7681;color:#6e7681;border-radius:4px;padding:2px 7px;cursor:pointer;font-size:.75rem;line-height:1.4;flex-shrink:0}}
  .btn-remove:hover{{border-color:#da3633;color:#da3633}}
  .desc{{color:#8b949e;font-size:.8rem;margin-bottom:10px;line-height:1.5;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}}
  .pending-msg{{color:#8b949e;font-style:italic;font-size:.85rem;margin:12px 0}}
  .stats{{display:flex;gap:8px;margin-bottom:10px}}
  .stat{{display:flex;flex-direction:column;align-items:center;background:#0d1117;border-radius:6px;padding:8px 6px;flex:1;gap:2px}}
  .val{{font-weight:700;color:#f0f6fc;font-size:.95rem}}
  .lbl{{color:#6e7681;font-size:.7rem}}
  .meta{{color:#6e7681;font-size:.72rem}}
  .notifs-wrap{{background:#161b22;border:1px solid #30363d;border-radius:8px;overflow:hidden}}
  .notif{{display:grid;grid-template-columns:auto auto 1fr auto;gap:10px;align-items:center;padding:11px 16px;border-bottom:1px solid #21262d;font-size:.85rem}}
  .notif:last-child{{border-bottom:none}}
  .badge{{padding:3px 8px;border-radius:10px;font-size:.72rem;font-weight:600;white-space:nowrap}}
  .badge-release{{background:#1f6feb;color:#fff}}
  .badge-stars{{background:#9e6a03;color:#fff}}
  .badge-forks{{background:#1a7f37;color:#fff}}
  .badge-error{{background:#da3633;color:#fff}}
  .badge-info{{background:#388bfd;color:#fff}}
  .notif-repo{{font-weight:600;color:#58a6ff;white-space:nowrap}}
  .notif-msg{{color:#c9d1d9}}
  .notif-time{{color:#6e7681;font-size:.75rem;white-space:nowrap}}
  .empty{{color:#8b949e;font-style:italic;padding:32px;text-align:center}}
  footer{{background:#161b22;border-top:1px solid #30363d;padding:10px 24px;display:flex;gap:20px;font-size:.78rem;color:#6e7681;position:fixed;bottom:0;left:0;right:0}}
  footer span{{color:#8b949e}}
  @media(max-width:600px){{
    .add-wrap form{{flex-direction:column}}
    .notif{{grid-template-columns:1fr 1fr}}
    .notif-msg,.notif-time{{grid-column:span 2}}
    .repos-grid{{grid-template-columns:1fr}}
  }}
</style>
</head>
<body>
<header>
  <h1>&#128269; GitHub Tracker</h1>
  <div class="subtitle">Surveillance de d&eacute;p&ocirc;ts GitHub &bull; Actualisation auto : 60s</div>
</header>
<main>
  <h2>&#128230; D&eacute;p&ocirc;ts surveill&eacute;s ({repo_count})</h2>
  <div class="add-wrap">
    <form method="POST" action="/repos">
      <input class="url-input" type="text" name="url"
             placeholder="https://github.com/owner/repo" required>
      <button class="btn-add" type="submit">+ Ajouter</button>
    </form>
  </div>
  <div class="repos-grid">{repos_html}</div>
  <h2>&#128276; Notifications r&eacute;centes</h2>
  <div class="notifs-wrap">{notifs_html}</div>
</main>
<footer>
  <div>D&eacute;p&ocirc;ts : <span>{repo_count}</span></div>
  <div>Notifications : <span>{notif_count}</span></div>
  <div>G&eacute;n&eacute;r&eacute; : <span>{generated_at}</span></div>
</footer>
</body>
</html>"#,
        repos_html = repos_html,
        notifs_html = notifs_html,
        repo_count = repo_count,
        notif_count = notif_count,
        generated_at = now,
    );

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

fn render_card(repo: &RepoState, full_name: &str) -> String {
    let release = repo.latest_release.as_deref().unwrap_or("—");
    let desc = repo.description.as_deref().unwrap_or("Aucune description");
    format!(
        r#"<div class="card">
  <div class="card-header">
    <a href="{url}" target="_blank" rel="noopener">{name}</a>
    <form method="POST" action="/repos/remove">
      <input type="hidden" name="full_name" value="{name}">
      <button class="btn-remove" type="submit" title="Supprimer">&#10005;</button>
    </form>
  </div>
  <p class="desc">{desc}</p>
  <div class="stats">
    <div class="stat"><span class="val">{stars}</span><span class="lbl">&#11088; Stars</span></div>
    <div class="stat"><span class="val">{forks}</span><span class="lbl">&#129380; Forks</span></div>
    <div class="stat"><span class="val">{release}</span><span class="lbl">&#127991; Release</span></div>
  </div>
  <div class="meta">V&eacute;rifi&eacute; : {checked}</div>
</div>"#,
        url = repo.html_url,
        name = full_name,
        desc = desc,
        stars = repo.stars,
        forks = repo.forks,
        release = release,
        checked = repo.last_checked.format("%Y-%m-%d %H:%M UTC"),
    )
}

fn render_pending_card(full_name: &str) -> String {
    format!(
        r#"<div class="card card-pending">
  <div class="card-header">
    <a href="https://github.com/{name}" target="_blank" rel="noopener">{name}</a>
    <form method="POST" action="/repos/remove">
      <input type="hidden" name="full_name" value="{name}">
      <button class="btn-remove" type="submit" title="Supprimer">&#10005;</button>
    </form>
  </div>
  <p class="pending-msg">&#9203; En attente de la prochaine v&eacute;rification...</p>
</div>"#,
        name = full_name,
    )
}

fn render_notification(n: &Notification) -> String {
    let (badge_class, icon) = match n.kind {
        NotificationKind::NewRelease => ("badge-release", "&#128640;"),
        NotificationKind::StarChange => ("badge-stars",   "&#11088;"),
        NotificationKind::ForkChange => ("badge-forks",   "&#129380;"),
        NotificationKind::Error      => ("badge-error",   "&#10060;"),
        NotificationKind::Info       => ("badge-info",    "&#8505;"),
    };
    format!(
        r#"<div class="notif">
  <span class="badge {badge_class}">{icon} {kind}</span>
  <span class="notif-repo">{repo}</span>
  <span class="notif-msg">{msg}</span>
  <span class="notif-time">{time}</span>
</div>"#,
        badge_class = badge_class,
        icon = icon,
        kind = n.kind,
        repo = n.repo,
        msg = n.message,
        time = n.timestamp.format("%Y-%m-%d %H:%M UTC"),
    )
}

// ── JSON API ─────────────────────────────────────────────────────────────────

async fn api_repos(state: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let repos: Vec<RepoState> = {
        let s = state.lock().unwrap();
        s.repo_states.values().cloned().collect()
    };
    HttpResponse::Ok().json(&repos)
}

async fn api_notifications(state: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let notifications: Vec<Notification> = {
        let s = state.lock().unwrap();
        s.notifications.clone()
    };
    HttpResponse::Ok().json(&notifications)
}
