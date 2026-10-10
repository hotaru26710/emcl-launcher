mod commands;
mod core;
mod db;

use std::sync::Mutex;
use std::time::Duration;

use anyhow::Result;
use core::launch::ProcessRegistry;
use core::paths::LauncherPaths;
use db::schema;
use reqwest::Client;
use rusqlite::Connection;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub http: Client,
    pub paths: LauncherPaths,
    pub processes: ProcessRegistry,
}

fn bootstrap(app: &mut tauri::App) -> Result<()> {
    let paths = LauncherPaths::discover()?;
    paths.ensure()?;

    let connection = schema::open_database(&paths.database_file())?;
    let http = Client::builder()
        .user_agent("EMCL-Launcher/0.1.0")
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(300))
        .pool_max_idle_per_host(32)
        .build()?;

    app.manage(AppState {
        db: Mutex::new(connection),
        http,
        paths,
        processes: ProcessRegistry::default(),
    });

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            bootstrap(app).map_err(|error| {
                Box::new(std::io::Error::other(error.to_string())) as Box<dyn std::error::Error>
            })
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_launcher_info,
            commands::instance::list_instances,
            commands::instance::get_instance,
            commands::instance::create_instance,
            commands::instance::update_instance,
            commands::instance::delete_instance,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::version::get_version_manifest,
            commands::version::get_version_details,
            commands::install::install_version,
            commands::java::detect_java,
            commands::java::recommend_java,
            commands::settings::get_default_java,
            commands::settings::set_default_java,
            commands::auth::create_offline_account,
            commands::auth::list_accounts,
            commands::auth::delete_account,
            commands::auth::set_active_account,
            commands::auth::get_active_account,
            commands::launch::launch_instance,
            commands::launch::stop_instance,
            commands::launch::is_instance_running,
        ])
        .run(tauri::generate_context!())
        .expect("error while running EMCL");
}
