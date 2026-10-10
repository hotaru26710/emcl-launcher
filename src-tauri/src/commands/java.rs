use tauri::State;

use crate::commands::settings::load_settings;
use crate::core::java;
use crate::core::version;
use crate::AppState;

#[tauri::command]
pub async fn detect_java() -> Result<Vec<java::JavaInstallation>, String> {
    java::detect_java_installations()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn recommend_java(
    state: State<'_, AppState>,
    version_id: String,
) -> Result<java::JavaRecommendation, String> {
    let settings = load_settings(&state)?;
    let details = version::fetch_version_details(
        &state.http,
        &state.paths,
        &version_id,
        &settings.download_mirror,
    )
    .await
    .map_err(|error| error.to_string())?;
    let installations = java::detect_java_installations()
        .await
        .map_err(|error| error.to_string())?;
    Ok(java::recommend_java(&installations, &details))
}
