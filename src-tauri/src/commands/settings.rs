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

    Ok(Settings {
        download_mirror: mirror,
        download_concurrency: concurrency,
        shared_directory: state.paths.shared_dir().to_string_lossy().into_owned(),
        data_directory: state.paths.root().to_string_lossy().into_owned(),
    })
}
