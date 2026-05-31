use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TargetType {
    #[default]
    Local,
    K8s,
}

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
    #[serde(default)]
    pub target_type: TargetType,
    #[serde(default)]
    pub pid: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TunnelConfig {
    pub tunnel_name: String,
    pub public_hostname: String,
    pub target_type: TargetType,
    pub k8s_namespace: String,
    pub internal_service: String,
}

fn config_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join(".dock-flare")
        })
        .join("projects.json")
}

pub fn load(app: &tauri::AppHandle) -> Vec<Project> {
    let path = config_path(app);
    let s = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    match serde_json::from_str(&s) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "WARNING: projects.json is corrupted ({}). Starting with no projects. \
                 Back up and delete {} to recover.",
                e, path.display()
            );
            vec![]
        }
    }
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
