mod cloudflare;
mod commands;
mod config;
mod orchestrate;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            commands::cancel_login,
            commands::open_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
