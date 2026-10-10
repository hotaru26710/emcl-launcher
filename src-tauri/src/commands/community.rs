use std::sync::Arc;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::core::community;
use crate::core::install::{self, ProgressReporter};
use crate::core::instance as instance_core;
use crate::core::mods::ModInfo;
use crate::AppState;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchModrinthInput {
    pub query: String,
    pub game_version: String,
    pub loader: Option<String>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallModrinthInput {
    pub instance_id: String,
    pub version_id: String,
}

#[tauri::command]
pub async fn search_modrinth_mods(
    state: State<'_, AppState>,
    input: SearchModrinthInput,
) -> Result<Vec<community::ModrinthSearchResult>, String> {
    community::search_modrinth(
        &state.http,
        &input.query,
        &input.game_version,
        input.loader.as_deref(),
        input.offset.unwrap_or(0),
    )
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_modrinth_versions(
    state: State<'_, AppState>,
    project_id: String,
    game_version: String,
    loader: Option<String>,
) -> Result<Vec<community::ModrinthVersionInfo>, String> {
    community::get_modrinth_versions(&state.http, &project_id, &game_version, loader.as_deref())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn install_modrinth_version(
    app: AppHandle,
    state: State<'_, AppState>,
    input: InstallModrinthInput,
) -> Result<Vec<ModInfo>, String> {
    let instance = {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        instance_core::get_instance(&connection, &input.instance_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "实例不存在".to_string())?
    };
    let job_id = format!("community-{}", Uuid::new_v4());
    let event_handle = app.clone();
    let sink: install::ProgressSink = Arc::new(move |progress| {
        let _ = event_handle.emit("install-progress", progress);
    });
    let reporter = ProgressReporter::new(job_id, instance.version_id.clone(), sink);
    community::install_modrinth_version(
        &state.http,
        &state.paths,
        &instance,
        &input.version_id,
        &reporter,
    )
    .await
    .map_err(|error| {
        let message = error.to_string();
        reporter.fail(&message);
        message
    })
}
