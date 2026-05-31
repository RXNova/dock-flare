mod cloudflare;
mod commands;
mod config;
mod keychain;
mod orchestrate;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("HTTP client");

    tauri::Builder::default()
        .manage(commands::DeployLock(tokio::sync::Mutex::new(())))
        .manage(commands::AppClient(http_client))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_projects,
            commands::upsert_project,
            commands::delete_project,
            commands::list_project_tunnels,
            commands::deploy_tunnel,
            commands::teardown_tunnel,
            commands::check_cloudflared,
            commands::check_cf_auth,
            commands::install_cloudflared,
            commands::cloudflared_login,
            commands::discover_zone,
            commands::cancel_login,
            commands::open_url,
            commands::reconfigure_project,
            commands::get_log_file_path,
            commands::clear_persistent_logs,
            commands::update_tunnel_service,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
