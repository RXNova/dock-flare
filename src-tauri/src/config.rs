use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Project {
    pub id: String,
    /// User-chosen label for the project (e.g. "Production"). Required at creation.
    #[serde(default)]
    pub name: String,
    /// Cloudflare zone, auto-discovered from the cert (browser) or API token (token).
    /// Empty until authentication completes.
    #[serde(default)]
    pub domain: String,
    pub auth_mode: String, // "token" | "browser"
    #[serde(default)]
    pub api_token: String,
    #[serde(default)]
    pub account_id: String,
    /// Set to true after a successful cloudflared login for this project.
    /// Stored so the sidebar auth dot works without an extra disk check on every render.
    #[serde(default)]
    pub browser_authed: bool,
    #[serde(default)]
    pub tunnels: Vec<TunnelMeta>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TunnelMeta {
    pub id: String,
    pub name: String,
    pub hostname: String,
    pub service: String,
    pub namespace: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TunnelConfig {
    pub tunnel_name: String,
    pub public_hostname: String,    pub target_type: String,    pub k8s_namespace: String,
    pub internal_service: String,
}

fn config_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("no app data dir")
        .join("projects.json")
}

pub fn load(app: &tauri::AppHandle) -> Vec<Project> {
    let path = config_path(app);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app: &tauri::AppHandle, projects: &[Project]) {
    let path = config_path(app);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(projects) {
        let _ = std::fs::write(path, json);
    }
}
