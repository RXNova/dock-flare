use serde::Serialize;
use tauri::AppHandle;
use base64::Engine as _;

use crate::cloudflare;
use crate::config::{self, Project, TargetType, TunnelConfig};
use crate::keychain;
use crate::orchestrate;

pub struct DeployLock(pub tokio::sync::Mutex<()>);
pub struct AppClient(pub reqwest::Client);

// ── Return type for tunnel list ───────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct TunnelInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub hostname: Option<String>,
    pub service: Option<String>,
    pub namespace: Option<String>,
    pub target_type: Option<String>,
}

// ── Project CRUD ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_projects(app: AppHandle) -> Vec<Project> {
    let mut projects = config::load(&app);
    for p in projects.iter_mut() {
        if p.auth_mode == "token" && p.api_token.is_empty() {
            if let Some(token) = keychain::load_token(&p.id) {
                p.api_token = token;
            }
        }
    }
    projects
}

#[tauri::command]
pub fn upsert_project(app: AppHandle, mut project: Project) {
    if project.auth_mode == "token" && !project.api_token.is_empty() {
        if let Err(e) = keychain::store_token(&project.id, &project.api_token) {
            eprintln!("WARNING: keychain store failed: {}", e);
        }
        project.api_token = String::new();
    }
    let mut projects = config::load(&app);
    match projects.iter_mut().find(|p| p.id == project.id) {
        Some(p) => *p = project,
        None    => projects.push(project),
    }
    config::save(&app, &projects);
}

#[tauri::command]
pub fn delete_project(app: AppHandle, id: String) {
    keychain::delete_token(&id);
    let mut projects = config::load(&app);
    projects.retain(|p| p.id != id);
    config::save(&app, &projects);
}

// ── Tunnel list ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_project_tunnels(
    app: AppHandle,
    project: Project,
    client_state: tauri::State<'_, AppClient>,
) -> Result<Vec<TunnelInfo>, String> {
    let local = config::load(&app)
        .into_iter()
        .find(|p| p.id == project.id)
        .map(|p| p.tunnels)
        .unwrap_or_default();

    if project.auth_mode == "token" && !project.api_token.is_empty() {
        let client = &client_state.0;
        let cf_tunnels = cloudflare::list_tunnels(client, &project.api_token, &project.account_id)
            .await?;

        Ok(cf_tunnels.into_iter().map(|ct| {
            let meta = local.iter().find(|m| m.id == ct.id || m.name == ct.name);
            TunnelInfo {
                id:          ct.id,
                name:        ct.name,
                status:      ct.status,
                hostname:    meta.map(|m| m.hostname.clone()),
                service:     meta.map(|m| m.service.clone()),
                namespace:   meta.map(|m| m.namespace.clone()),
                target_type: meta.map(|m| match m.target_type {
                    TargetType::Local => "local".to_string(),
                    TargetType::K8s  => "k8s".to_string(),
                }),
            }
        }).collect())
    } else {
        let cert = orchestrate::project_cert_path(&project.id);
        if !cert.exists() {
            return Ok(vec![]);
        }
        let cert_str = cert.to_string_lossy();
        let output = tokio::process::Command::new("cloudflared")
            .args(&["--origincert", &cert_str, "tunnel", "list", "-o", "json"])
            .output()
            .await
            .map_err(|e| format!("Failed to run cloudflared tunnel list: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("cloudflared list failed: {}", err));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let cf_tunnels: Vec<serde_json::Value> = serde_json::from_str(&json_str)
            .unwrap_or_default();

        Ok(cf_tunnels.into_iter().map(|ct| {
            let id = ct["id"].as_str().unwrap_or_default().to_string();
            let name = ct["name"].as_str().unwrap_or_default().to_string();
            let meta = local.iter().find(|m| m.id == id || m.name == name);
            
            TunnelInfo {
                id,
                name,
                status: "unknown".to_string(), // cloudflared tunnel list doesn't return active status unfortunately
                hostname:    meta.map(|m| m.hostname.clone()),
                service:     meta.map(|m| m.service.clone()),
                namespace:   meta.map(|m| m.namespace.clone()),
                target_type: meta.map(|m| match m.target_type {
                    TargetType::Local => "local".to_string(),
                    TargetType::K8s  => "k8s".to_string(),
                }),
            }
        }).collect())
    }
}

// ── Deploy ────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn deploy_tunnel(
    app: AppHandle,
    state: tauri::State<'_, DeployLock>,
    client_state: tauri::State<'_, AppClient>,
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

    let health_hostname = tunnel.public_hostname.clone();
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

    // Best-effort health check — never fails the deploy
    let hostname = health_hostname;
    let app_hc = app.clone();
    let client_hc = client_state.0.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        match client_hc.head(format!("https://{}", hostname)).send().await {
            Ok(r) => orchestrate::emit(&app_hc, &format!("  -> Health check: reachable (HTTP {})", r.status().as_u16())),
            Err(_) => orchestrate::emit(&app_hc, "  -> Health check: not yet reachable — DNS may take a few seconds"),
        }
    });

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

    // Single load — find meta, run teardown, then update and save in one pass
    let mut projects = config::load(&app);
    let (target_type, pid) = projects
        .iter()
        .find(|p| p.id == project.id)
        .and_then(|p| p.tunnels.iter().find(|t| t.name == tunnel_name))
        .map(|m| (m.target_type.clone(), m.pid))
        .unwrap_or_else(|| (TargetType::default(), None));

    let result = if project.auth_mode == "browser" {
        orchestrate::teardown_browser(&app, &project.id, &tunnel_name, &namespace, &target_type, pid).await
    } else {
        orchestrate::teardown_token(&app, &project, &tunnel_name, &hostname, &namespace, &target_type, pid).await
    };

    if result.is_ok() {
        orchestrate::emit(&app, "=== Teardown complete ===");
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
pub async fn discover_zone(project: Project, client_state: tauri::State<'_, AppClient>) -> Result<ZoneInfo, String> {
    let client = &client_state.0;

    if project.auth_mode == "browser" {
        let cert = orchestrate::project_cert_path(&project.id);
        if !cert.exists() {
            return Err("No cert found — authorize this project first.".into());
        }
        let (zone_id, _account_id, api_token) = parse_argo_token(&cert)
            .ok_or("Could not read the cert credentials. Re-authorize.")?;
        let zone = cloudflare::get_zone_by_id(&client, &api_token, &zone_id).await?;
        Ok(ZoneInfo { zone: zone.name.clone(), all: vec![zone.name], account_id: String::new() })
    } else {
        if project.api_token.is_empty() {
            return Err("No API token set.".into());
        }
        // If no account_id was provided, discover it from the token.
        let resolved_account_id = if project.account_id.is_empty() {
            cloudflare::list_accounts(client, &project.api_token)
                .await?
                .into_iter()
                .next()
                .map(|a| a.id)
                .unwrap_or_default()
        } else {
            project.account_id.clone()
        };
        let zones = cloudflare::list_zones(client, &project.api_token, &resolved_account_id).await?;
        let all: Vec<String> = zones.into_iter().map(|z| z.name).collect();
        if all.is_empty() {
            return Err("This API token has no accessible zones.".into());
        }
        Ok(ZoneInfo { zone: all[0].clone(), all, account_id: resolved_account_id })
    }
}

#[derive(serde::Serialize)]
pub struct ZoneInfo {
    /// Primary zone (the project's domain).
    pub zone: String,
    /// All zones reachable with this credential (token mode may have several).
    pub all: Vec<String>,
    /// The Cloudflare account ID for this token (empty for browser mode).
    pub account_id: String,
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
    let backup       = orchestrate::cf_home_path().join(format!("cert.pem.{}.bak", project_id));
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

#[tauri::command]
pub async fn update_tunnel_service(
    app: AppHandle,
    client_state: tauri::State<'_, AppClient>,
    project: Project,
    tunnel_name: String,
    new_service: String,
) -> Result<(), String> {
    let client = &client_state.0;

    // Look up tunnel_id from stored metadata
    let tunnel_id = config::load(&app)
        .into_iter()
        .find(|p| p.id == project.id)
        .and_then(|p| p.tunnels.into_iter().find(|t| t.name == tunnel_name))
        .map(|m| m.id)
        .ok_or_else(|| format!("Tunnel '{}' not found in local metadata", tunnel_name))?;

    // Update ingress config on Cloudflare
    let hostname = config::load(&app)
        .into_iter()
        .find(|p| p.id == project.id)
        .and_then(|p| p.tunnels.into_iter().find(|t| t.id == tunnel_id))
        .map(|m| m.hostname)
        .unwrap_or_default();

    cloudflare::configure_ingress(client, &project.api_token, &project.account_id,
        &tunnel_id, &hostname, &new_service).await?;

    // Update stored metadata
    let mut projects = config::load(&app);
    if let Some(p) = projects.iter_mut().find(|p| p.id == project.id) {
        if let Some(t) = p.tunnels.iter_mut().find(|t| t.name == tunnel_name) {
            t.service = new_service;
        }
    }
    config::save(&app, &projects);
    Ok(())
}

/// Clear stored credentials for a token-mode project so the user can re-enter them.
/// Deletes the keychain entry and blanks api_token, account_id, and domain in config.
#[tauri::command]
pub fn reconfigure_project(app: AppHandle, project_id: String) {
    keychain::delete_token(&project_id);
    let mut projects = config::load(&app);
    if let Some(p) = projects.iter_mut().find(|p| p.id == project_id) {
        p.api_token  = String::new();
        p.account_id = String::new();
        p.domain     = String::new();
    }
    config::save(&app, &projects);
}

#[tauri::command]
pub fn get_log_file_path() -> String {
    orchestrate::log_file_path().to_string_lossy().to_string()
}

#[tauri::command]
pub fn clear_persistent_logs() {
    let _ = std::fs::remove_file(orchestrate::log_file_path());
}
