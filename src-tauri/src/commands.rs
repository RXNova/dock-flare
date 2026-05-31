use serde::Serialize;
use tauri::AppHandle;
use base64::Engine as _;

use crate::cloudflare;
use crate::config::{self, Project, TunnelConfig};
use crate::orchestrate;

pub struct DeployLock(pub tokio::sync::Mutex<()>);

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
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;
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
    state: tauri::State<'_, DeployLock>,
    project: Project,
    tunnel: TunnelConfig,
) -> Result<(), String> {
    let _lock = state.0.try_lock()
        .map_err(|_| "Another operation is already in progress".to_string())?;

    orchestrate::emit(&app, &format!("=== DockFlare Deploy [{}] ===", project.domain));

    let (tunnel_id, local_pid) = if project.auth_mode == "browser" {
        orchestrate::deploy_browser(&app, &project.id, &tunnel).await
    } else {
        orchestrate::deploy_token(&app, &project, &tunnel).await
    }?;

    orchestrate::emit(&app, "=== Deploy complete ===");

    let mut projects = config::load(&app);
    if let Some(p) = projects.iter_mut().find(|p| p.id == project.id) {
        p.tunnels.retain(|t| t.name != tunnel.tunnel_name);
        p.tunnels.push(config::TunnelMeta {
            id:          tunnel_id,
            name:        tunnel.tunnel_name,
            hostname:    tunnel.public_hostname,
            service:     tunnel.internal_service,
            namespace:   tunnel.k8s_namespace,
            target_type: tunnel.target_type,
            pid:         local_pid,
        });
    }
    config::save(&app, &projects);

    Ok(())
}

// ── Teardown ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn teardown_tunnel(
    app: AppHandle,
    state: tauri::State<'_, DeployLock>,
    project: Project,
    tunnel_name: String,
    hostname: String,
    namespace: String,
) -> Result<(), String> {
    let _lock = state.0.try_lock()
        .map_err(|_| "Another operation is already in progress".to_string())?;

    orchestrate::emit(&app, &format!("=== DockFlare Teardown [{}] ===", project.domain));

    // Look up stored target_type and pid for correct cleanup
    let (target_type, pid) = config::load(&app)
        .into_iter()
        .find(|p| p.id == project.id)
        .and_then(|p| p.tunnels.into_iter().find(|t| t.name == tunnel_name))
        .map(|m| (m.target_type, m.pid))
        .unwrap_or_else(|| (String::new(), None));

    let result = if project.auth_mode == "browser" {
        orchestrate::teardown_browser(&app, &project.id, &tunnel_name, &namespace, &target_type, pid).await
    } else {
        orchestrate::teardown_token(&app, &project, &tunnel_name, &hostname, &namespace, &target_type, pid).await
    };

    if result.is_ok() {
        orchestrate::emit(&app, "=== Teardown complete ===");
        let mut projects = config::load(&app);
        if let Some(p) = projects.iter_mut().find(|p| p.id == project.id) {
            p.tunnels.retain(|t| t.name != tunnel_name);
        }
        config::save(&app, &projects);
    }

    result
}

// ── cloudflared helpers ───────────────────────────────────────────────────────

/// Parse the ARGO TUNNEL TOKEN from a cert.pem file.
/// Returns (zone_id, account_id, api_token) or None if the format is unrecognised.
fn parse_argo_token(cert_path: &std::path::PathBuf) -> Option<(String, String, String)> {
    let content = std::fs::read_to_string(cert_path).ok()?;
    let start = content.find("-----BEGIN ARGO TUNNEL TOKEN-----")?
        + "-----BEGIN ARGO TUNNEL TOKEN-----".len();
    let end = content.find("-----END ARGO TUNNEL TOKEN-----")?;
    let b64: String = content[start..end].split_whitespace().collect();
    let decoded = base64::engine::general_purpose::STANDARD.decode(&b64).ok()?;
    let json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    let zone_id    = json["zoneID"].as_str()?.to_string();
    let account_id = json["accountID"].as_str()?.to_string();
    let api_token  = json["apiToken"].as_str()?.to_string();
    Some((zone_id, account_id, api_token))
}

/// Discover the Cloudflare zone(s) this project is authorized for.
///
/// - Browser mode: the cert.pem holds an ARGO TUNNEL TOKEN block — base64 JSON
///   `{ "zoneID", "accountID", "apiToken" }`. We resolve zoneID → zone name.
/// - Token mode: list the zones the API token can access.
///
/// The discovered zone becomes the project's domain — the user never types it.
#[tauri::command]
pub async fn discover_zone(project: Project) -> Result<ZoneInfo, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    if project.auth_mode == "browser" {
        let cert = orchestrate::project_cert_path(&project.id);
        if !cert.exists() {
            return Err("No cert found — authorize this project first.".into());
        }
        let (zone_id, _account_id, api_token) = parse_argo_token(&cert)
            .ok_or("Could not read the cert credentials. Re-authorize.")?;
        let zone = cloudflare::get_zone_by_id(&client, &api_token, &zone_id).await?;
        Ok(ZoneInfo { zone: zone.name.clone(), all: vec![zone.name] })
    } else {
        if project.api_token.is_empty() {
            return Err("No API token set.".into());
        }
        let zones = cloudflare::list_zones(&client, &project.api_token, &project.account_id).await?;
        let all: Vec<String> = zones.into_iter().map(|z| z.name).collect();
        if all.is_empty() {
            return Err("This API token has no accessible zones.".into());
        }
        Ok(ZoneInfo { zone: all[0].clone(), all })
    }
}

#[derive(serde::Serialize)]
pub struct ZoneInfo {
    /// Primary zone (the project's domain).
    pub zone: String,
    /// All zones reachable with this credential (token mode may have several).
    pub all: Vec<String>,
}

#[tauri::command]
pub async fn cancel_login(app: AppHandle) -> Result<(), String> {
    // Kill the cloudflared process that is sitting in "Waiting for login..."
    tokio::process::Command::new("pkill")
        .args(["-f", "cloudflared tunnel login"])
        .output()
        .await
        .ok();
    orchestrate::emit(&app, "Login cancelled.");
    Ok(())
}

#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    tokio::process::Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|e| format!("open: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn check_cloudflared() -> bool {
    orchestrate::check_bin("cloudflared").await.is_ok()
}

#[tauri::command]
pub fn check_cf_auth(project_id: String) -> bool {
    orchestrate::project_cert_path(&project_id).exists()
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
pub async fn cloudflared_login(app: AppHandle, project_id: String) -> Result<(), String> {
    let default_cert = orchestrate::cf_home_path().join("cert.pem");
    let backup       = orchestrate::cf_home_path().join("cert.pem.dockflare-bak");
    let project_cert = orchestrate::project_cert_path(&project_id);

    // cloudflared refuses to run `tunnel login` if cert.pem already exists.
    // Move it aside so the login can write a fresh cert for this account,
    // then restore it afterwards so other tools are unaffected.
    if default_cert.exists() {
        std::fs::rename(&default_cert, &backup)
            .map_err(|e| format!("Could not move existing cert.pem: {}", e))?;
    }

    orchestrate::emit(&app, "Opening Cloudflare authorization in your browser...");
    orchestrate::emit(&app, "(If the browser doesn't open, copy the URL printed below.)");

    let result = orchestrate::run_streamed(&app, "cloudflared", &["tunnel", "login"]).await;

    if let Err(e) = result {
        // Restore backup so we don't leave things in a broken state
        if backup.exists() { let _ = std::fs::rename(&backup, &default_cert); }
        return Err(e);
    }

    // Copy the freshly-written cert to the project-specific location
    std::fs::copy(&default_cert, &project_cert)
        .map_err(|e| format!("Login succeeded but could not save project cert: {}", e))?;

    // Restore the old cert.pem so other tools keep working
    if backup.exists() {
        let _ = std::fs::rename(&backup, &default_cert);
    }

    orchestrate::emit(&app, &format!("Authorized. Cert saved to {}", project_cert.display()));
    Ok(())
}
