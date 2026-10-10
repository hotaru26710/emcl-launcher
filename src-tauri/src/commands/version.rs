use tauri::State;

use crate::commands::settings::load_settings;
use crate::core::version::{self, VersionDetails, VersionManifest};
use crate::AppState;

#[tauri::command]
pub async fn get_version_manifest(
    state: State<'_, AppState>,
    force_refresh: Option<bool>,
) -> Result<VersionManifest, String> {
    let settings = load_settings(&state)?;
    version::fetch_version_manifest(
        &state.http,
        &state.paths,
        &settings.download_mirror,
        force_refresh.unwrap_or(false),
    )
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_version_details(
    state: State<'_, AppState>,
    version_id: String,
) -> Result<VersionDetails, String> {
    let settings = load_settings(&state)?;
    version::fetch_version_details(
        &state.http,
        &state.paths,
        &version_id,
        &settings.download_mirror,
    )
    .await
    .map_err(|error| error.to_string())
}
