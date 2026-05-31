use serde::Serialize;
use tauri::AppHandle;

use crate::cloudflare;
use crate::config::{self, Project, TunnelConfig};
use crate::orchestrate;

// ── Return type for tunnel list ───────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct TunnelInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub hostname: Option<String>,
    pub service: Option<String>,
    pub namespace: Option<String>,
}

// ── Project CRUD ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_projects(app: AppHandle) -> Vec<Project> {
    config::load(&app)
}

#[tauri::command]
pub fn upsert_project(app: AppHandle, project: Project) {
    let mut projects = config::load(&app);
    match projects.iter_mut().find(|p| p.id == project.id) {
        Some(p) => *p = project,
        None    => projects.push(project),
    }
    config::save(&app, &projects);
}

#[tauri::command]
pub fn delete_project(app: AppHandle, id: String) {
    let mut projects = config::load(&app);
    projects.retain(|p| p.id != id);
    config::save(&app, &projects);
}

// ── Tunnel list ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_project_tunnels(
    app: AppHandle,
    project: Project,
) -> Result<Vec<TunnelInfo>, String> {
    // Always load local metadata from disk (the JS-side project may lack tunnel array)
    let local = config::load(&app)
        .into_iter()
        .find(|p| p.id == project.id)
        .map(|p| p.tunnels)
        .unwrap_or_default();

    if project.auth_mode == "token" && !project.api_token.is_empty() {
        let client = reqwest::Client::new();
        let cf_tunnels = cloudflare::list_tunnels(&client, &project.api_token, &project.account_id)
            .await?;

        Ok(cf_tunnels.into_iter().map(|ct| {
            let meta = local.iter().find(|m| m.id == ct.id || m.name == ct.name);
            TunnelInfo {
                id:       ct.id,
                name:     ct.name,
                status:   ct.status,
                hostname: meta.map(|m| m.hostname.clone()),
                service:  meta.map(|m| m.service.clone()),
                namespace:meta.map(|m| m.namespace.clone()),
            }
        }).collect())
    } else {
        // Browser mode: show locally tracked tunnels only
        Ok(local.into_iter().map(|m| TunnelInfo {
            id:       m.id,
            name:     m.name,
            status:   "unknown".to_string(),
            hostname: Some(m.hostname),
            service:  Some(m.service),
            namespace:Some(m.namespace),
        }).collect())
    }
}

// ── Deploy ────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn deploy_tunnel(
    app: AppHandle,
    project: Project,
    tunnel: TunnelConfig,
) -> Result<(), String> {
    orchestrate::emit(&app, &format!("=== DockFlare Deploy [{}] ===", project.domain));

    let tunnel_id = if project.auth_mode == "browser" {
        orchestrate::deploy_browser(&app, &tunnel).await
    } else {
        orchestrate::deploy_token(&app, &project, &tunnel).await
    }?;

    orchestrate::emit(&app, "=== Deploy complete ===");

    // Persist tunnel metadata so list/teardown work correctly
    let mut projects = config::load(&app);
    if let Some(p) = projects.iter_mut().find(|p| p.id == project.id) {
        p.tunnels.retain(|t| t.name != tunnel.tunnel_name);
        p.tunnels.push(config::TunnelMeta {
            id:        tunnel_id,
            name:      tunnel.tunnel_name,
            hostname:  tunnel.public_hostname,
            service:   tunnel.internal_service,
            namespace: tunnel.k8s_namespace,
        });
    }
    config::save(&app, &projects);

    Ok(())
}

// ── Teardown ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn teardown_tunnel(
    app: AppHandle,
    project: Project,
    tunnel_name: String,
    hostname: String,
    namespace: String,
) -> Result<(), String> {
    orchestrate::emit(&app, &format!("=== DockFlare Teardown [{}] ===", project.domain));

    let result = if project.auth_mode == "browser" {
        orchestrate::teardown_browser(&app, &tunnel_name, &namespace).await
    } else {
        orchestrate::teardown_token(&app, &project, &tunnel_name, &hostname, &namespace).await
    };

    if result.is_ok() {
        orchestrate::emit(&app, "=== Teardown complete ===");
        // Remove from local metadata
        let mut projects = config::load(&app);
        if let Some(p) = projects.iter_mut().find(|p| p.id == project.id) {
            p.tunnels.retain(|t| t.name != tunnel_name);
        }
        config::save(&app, &projects);
    }

    result
}

// ── cloudflared helpers ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn check_cloudflared() -> bool {
    orchestrate::check_bin("cloudflared").await.is_ok()
}

#[tauri::command]
pub fn check_cf_auth() -> bool {
    std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".cloudflared/cert.pem")
        .exists()
}

#[tauri::command]
pub async fn install_cloudflared(app: AppHandle) -> Result<(), String> {
    if orchestrate::check_bin("brew").await.is_err() {
        let msg = "Homebrew not found — install it from https://brew.sh";
        orchestrate::emit(&app, &format!("ERROR: {}", msg));
        return Err(msg.to_string());
    }
    orchestrate::emit(&app, "Running: brew install cloudflared");
    orchestrate::run_streamed(&app, "brew", &["install", "cloudflared"]).await
}

#[tauri::command]
pub async fn cloudflared_login(app: AppHandle) -> Result<(), String> {
    orchestrate::emit(&app, "Opening Cloudflare authorization in your browser...");
    orchestrate::emit(&app, "(If the browser doesn't open, copy the URL printed below.)");
    orchestrate::run_streamed(&app, "cloudflared", &["tunnel", "login"]).await?;
    orchestrate::emit(&app, "Authorization complete.");
    Ok(())
}
