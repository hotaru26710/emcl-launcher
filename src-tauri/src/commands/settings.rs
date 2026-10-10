use anyhow::Context;
use rusqlite::Connection;
use tauri::State;

use crate::db::models::{Settings, UpdateSettingsInput};
use crate::db::schema::{
    get_setting, set_setting, DEFAULT_DOWNLOAD_CONCURRENCY, DEFAULT_DOWNLOAD_MIRROR,
};
use crate::AppState;

pub(crate) fn load_settings(state: &AppState) -> Result<Settings, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    settings_from_connection(state, &connection).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_default_java(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    get_setting(&connection, "default_java_path").map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_default_java(
    state: State<'_, AppState>,
    java_path: Option<String>,
) -> Result<(), String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    if let Some(path) = java_path {
        if path.trim().is_empty() {
            return Err("java path cannot be empty".to_string());
        }
        set_setting(&connection, "default_java_path", path.trim())
            .map_err(|error| error.to_string())?;
    } else {
        connection
            .execute("DELETE FROM settings WHERE key = 'default_java_path'", [])
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    load_settings(&state)
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, AppState>,
    input: UpdateSettingsInput,
) -> Result<Settings, String> {
    if let Some(mirror) = &input.download_mirror {
        if mirror != "official" && mirror != "bmclapi" {
            return Err("download mirror must be official or bmclapi".to_string());
        }
    }

    let connection = state.db.lock().map_err(|error| error.to_string())?;
    if let Some(mirror) = input.download_mirror {
        set_setting(&connection, "download_mirror", &mirror).map_err(|error| error.to_string())?;
    }
    if let Some(concurrency) = input.download_concurrency {
        set_setting(
            &connection,
            "download_concurrency",
            &concurrency.clamp(1, 16).to_string(),
        )
        .map_err(|error| error.to_string())?;
    }
    if let Some(client_id) = input.microsoft_client_id {
        if client_id.trim().is_empty() {
            connection
                .execute("DELETE FROM settings WHERE key = 'microsoft_client_id'", [])
                .map_err(|error| error.to_string())?;
        } else {
            set_setting(&connection, "microsoft_client_id", client_id.trim())
                .map_err(|error| error.to_string())?;
        }
    }

    settings_from_connection(&state, &connection).map_err(|error| error.to_string())
}

fn settings_from_connection(state: &AppState, connection: &Connection) -> anyhow::Result<Settings> {
    let mirror = get_setting(connection, "download_mirror")?
        .unwrap_or_else(|| DEFAULT_DOWNLOAD_MIRROR.to_string());
    let concurrency = get_setting(connection, "download_concurrency")?
        .unwrap_or_else(|| DEFAULT_DOWNLOAD_CONCURRENCY.to_string())
        .parse::<u32>()
        .context("invalid download concurrency setting")?
        .clamp(1, 16);
    let default_java_path = get_setting(connection, "default_java_path")?;
    let microsoft_client_id = get_setting(connection, "microsoft_client_id")?;

    Ok(Settings {
        download_mirror: mirror,
        download_concurrency: concurrency,
        default_java_path,
        microsoft_client_id,
        shared_directory: state.paths.shared_dir().to_string_lossy().into_owned(),
        data_directory: state.paths.root().to_string_lossy().into_owned(),
    })
}
