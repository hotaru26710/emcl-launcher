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
    if let Some(memory) = input.default_memory_mb {
        set_setting(
            &connection,
            "default_memory_mb",
            &memory.clamp(512, 65536).to_string(),
        )
        .map_err(|error| error.to_string())?;
    }
    if let Some(width) = input.game_width {
        set_setting(
            &connection,
            "game_width",
            &width.clamp(320, 7680).to_string(),
        )
        .map_err(|error| error.to_string())?;
    }
    if let Some(height) = input.game_height {
        set_setting(
            &connection,
            "game_height",
            &height.clamp(240, 4320).to_string(),
        )
        .map_err(|error| error.to_string())?;
    }
    if let Some(fullscreen) = input.fullscreen {
        set_setting(
            &connection,
            "fullscreen",
            if fullscreen { "true" } else { "false" },
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
    let theme = get_setting(connection, "theme")?.unwrap_or_else(|| "system".to_string());
    let custom_theme = get_setting(connection, "custom_theme_json")?;
    let default_memory_mb = get_setting(connection, "default_memory_mb")?
        .unwrap_or_else(|| "4096".to_string())
        .parse::<u32>()
        .context("invalid default memory setting")?
        .clamp(512, 65536);
    let game_width = get_setting(connection, "game_width")?
        .unwrap_or_else(|| "854".to_string())
        .parse::<u32>()
        .context("invalid game width setting")?
        .clamp(320, 7680);
    let game_height = get_setting(connection, "game_height")?
        .unwrap_or_else(|| "480".to_string())
        .parse::<u32>()
        .context("invalid game height setting")?
        .clamp(240, 4320);
    let fullscreen = get_setting(connection, "fullscreen")?
        .map(|value| value == "true")
        .unwrap_or(false);

    Ok(Settings {
        download_mirror: mirror,
        download_concurrency: concurrency,
        default_java_path,
        microsoft_client_id,
        theme,
        custom_theme,
        default_memory_mb,
        game_width,
        game_height,
        fullscreen,
        shared_directory: state.paths.shared_dir().to_string_lossy().into_owned(),
        data_directory: state.paths.root().to_string_lossy().into_owned(),
    })
}
