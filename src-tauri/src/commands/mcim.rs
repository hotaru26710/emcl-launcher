use std::sync::Arc;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::core::install::{self, ProgressReporter};
use crate::core::instance as instance_core;
use crate::core::mcim;
use crate::AppState;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMcimInput {
    pub query: String,
    pub game_version: String,
    pub loader: Option<String>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallMcimInput {
    pub instance_id: String,
    pub mod_id: u64,
    pub file_id: u64,
}

#[tauri::command]
pub async fn search_mcim_curseforge_mods(
    state: State<'_, AppState>,
    input: SearchMcimInput,
) -> Result<Vec<mcim::McimCurseForgeSearchResult>, String> {
    mcim::search_curseforge(
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
pub async fn get_mcim_curseforge_files(
    state: State<'_, AppState>,
    mod_id: u64,
    game_version: String,
    loader: Option<String>,
) -> Result<Vec<mcim::McimCurseForgeFile>, String> {
    mcim::get_curseforge_files(&state.http, mod_id, &game_version, loader.as_deref())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn install_mcim_curseforge_file(
    app: AppHandle,
    state: State<'_, AppState>,
    input: InstallMcimInput,
) -> Result<crate::core::community::CommunityInstallResult, String> {
    let instance = {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        instance_core::get_instance(&connection, &input.instance_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "实例不存在".to_string())?
    };
    let job_id = format!("mcim-{}", Uuid::new_v4());
    let event_handle = app.clone();
    let sink: install::ProgressSink = Arc::new(move |progress| {
        let _ = event_handle.emit("install-progress", progress);
    });
    let reporter = ProgressReporter::new(job_id, instance.version_id.clone(), sink);
    mcim::install_curseforge_file(
        &state.http,
        &instance,
        input.mod_id,
        input.file_id,
        &reporter,
    )
    .await
    .map_err(|error| {
        let message = error.to_string();
        reporter.fail(&message);
        message
    })
}
