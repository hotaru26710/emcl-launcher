use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::core::github;
use crate::core::install::{self, ProgressReporter};
use crate::core::instance as instance_core;
use crate::core::metadata;
use crate::AppState;

#[tauri::command]
pub async fn search_github_mods(
    state: State<'_, AppState>,
    query: String,
    offset: Option<u32>,
) -> Result<Vec<github::GitHubRepository>, String> {
    github::search_repositories(&state.http, &query, offset.unwrap_or(0))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_github_releases(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
) -> Result<Vec<github::GitHubRelease>, String> {
    github::get_releases(&state.http, &owner, &repo)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn install_github_release_asset(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    asset: github::GitHubAsset,
) -> Result<Vec<crate::core::mods::ModInfo>, String> {
    let instance = {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        instance_core::get_instance(&connection, &instance_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "实例不存在".to_string())?
    };
    let job_id = format!("github-{}", Uuid::new_v4());
    let event_handle = app.clone();
    let sink: install::ProgressSink = Arc::new(move |progress| {
        let _ = event_handle.emit("install-progress", progress);
    });
    let reporter = ProgressReporter::new(job_id, instance.version_id.clone(), sink);
    let mods = github::install_asset(&state.http, &instance, &asset, &reporter)
        .await
        .map_err(|error| {
            let message = error.to_string();
            reporter.fail(&message);
            message
        })?;
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    metadata::refresh_instance_mod_metadata(&connection, &instance)
        .map_err(|error| error.to_string())?;
    Ok(mods)
}
