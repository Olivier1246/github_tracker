use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono::Utc;
use std::sync::{Arc, Mutex};

use crate::models::{Notification, NotificationKind, RepoState};
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
            .route("/api/repos", web::get().to(api_repos))
            .route("/api/notifications", web::get().to(api_notifications))
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}

async fn dashboard(state: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let (repos_html, notifs_html, repo_count, notif_count) = {
        let s = state.lock().unwrap();

        let repos_html = if s.repos.is_empty() {
            r#"<p class="empty">Aucun dépôt vérifié pour l'instant. En attente du premier passage...</p>"#
                .to_string()
        } else {
            let mut sorted: Vec<&RepoState> = s.repos.values().collect();
            sorted.sort_by(|a, b| b.stars.cmp(&a.stars));
            sorted.iter().map(|repo| {
                let release = repo.latest_release.as_deref().unwrap_or("—");
                let desc = repo.description.as_deref().unwrap_or("Aucune description");
                format!(
                    r#"<div class="card">
  <div class="card-header"><a href="{url}" target="_blank" rel="noopener">{name}</a></div>
  <p class="desc">{desc}</p>
  <div class="stats">
    <div class="stat"><span class="icon">&#11088;</span><span class="val">{stars}</span><span class="lbl">Stars</span></div>
    <div class="stat"><span class="icon">&#129380;</span><span class="val">{forks}</span><span class="lbl">Forks</span></div>
    <div class="stat"><span class="icon">&#127991;</span><span class="val">{release}</span><span class="lbl">Release</span></div>
  </div>
  <div class="meta">Dernière vérification : {checked}</div>
</div>"#,
                    url = repo.html_url,
                    name = repo.full_name,
                    desc = desc,
                    stars = repo.stars,
                    forks = repo.forks,
                    release = release,
                    checked = repo.last_checked.format("%Y-%m-%d %H:%M UTC"),
                )
            }).collect()
        };

        let notifs_html = if s.notifications.is_empty() {
            r#"<p class="empty">Aucune notification pour l'instant.</p>"#.to_string()
        } else {
            s.notifications.iter().rev().take(50).map(|n| {
                let (badge_class, icon) = match n.kind {
                    NotificationKind::NewRelease => ("badge-release", "&#128640;"),
                    NotificationKind::StarChange  => ("badge-stars",   "&#11088;"),
                    NotificationKind::ForkChange  => ("badge-forks",   "&#129380;"),
                    NotificationKind::Error       => ("badge-error",   "&#10060;"),
                    NotificationKind::Info        => ("badge-info",    "&#8505;"),
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
            }).collect()
        };

        let rc = s.repos.len();
        let nc = s.notifications.len();
        (repos_html, notifs_html, rc, nc)
    };

    let now = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="fr">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta http-equiv="refresh" content="60">
<title>GitHub Tracker</title>
<style>
  *{{box-sizing:border-box;margin:0;padding:0}}
  body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:#0d1117;color:#c9d1d9;min-height:100vh}}
  header{{background:#161b22;border-bottom:1px solid #30363d;padding:16px 24px;display:flex;align-items:center;justify-content:space-between;position:sticky;top:0;z-index:10}}
  .logo{{display:flex;align-items:center;gap:10px}}
  header h1{{font-size:1.3rem;color:#f0f6fc;font-weight:700}}
  .subtitle{{color:#8b949e;font-size:0.8rem;margin-top:2px}}
  main{{max-width:1280px;margin:0 auto;padding:24px 24px 80px}}
  h2{{font-size:1rem;color:#f0f6fc;margin:24px 0 14px;padding-bottom:8px;border-bottom:1px solid #21262d;font-weight:600}}
  .repos-grid{{display:grid;grid-template-columns:repeat(auto-fill,minmax(300px,1fr));gap:14px;margin-bottom:8px}}
  .card{{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:16px;transition:border-color .15s}}
  .card:hover{{border-color:#388bfd}}
  .card-header a{{color:#58a6ff;text-decoration:none;font-weight:600;font-size:.95rem;word-break:break-all}}
  .card-header a:hover{{text-decoration:underline}}
  .desc{{color:#8b949e;font-size:.8rem;margin:8px 0 10px;line-height:1.5;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}}
  .stats{{display:flex;gap:8px;margin-bottom:10px}}
  .stat{{display:flex;flex-direction:column;align-items:center;background:#0d1117;border-radius:6px;padding:8px 6px;flex:1;gap:2px}}
  .icon{{font-size:1.1rem}}
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
    .notif{{grid-template-columns:1fr 1fr}}
    .notif-msg,.notif-time{{grid-column:span 2}}
    .repos-grid{{grid-template-columns:1fr}}
  }}
</style>
</head>
<body>
<header>
  <div class="logo">
    <div>
      <h1>&#128269; GitHub Tracker</h1>
      <div class="subtitle">Surveillance de d&eacute;p&ocirc;ts GitHub &bull; Actualisation auto : 60s</div>
    </div>
  </div>
</header>
<main>
  <h2>&#128230; D&eacute;p&ocirc;ts surveill&eacute;s ({repo_count})</h2>
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

async fn api_repos(state: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let repos: Vec<RepoState> = {
        let s = state.lock().unwrap();
        s.repos.values().cloned().collect()
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
