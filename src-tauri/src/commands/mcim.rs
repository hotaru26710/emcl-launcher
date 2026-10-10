use std::sync::Arc;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::commands::settings::load_settings;
use crate::core::install::{self, ProgressReporter};
use crate::core::instance as instance_core;
use crate::core::mcim;
use crate::core::mod_source::curseforge::CurseForgeClient;
use crate::core::mod_source::{ApiRouter, ModApiConfig, ProjectDetails, SearchRequest};
use crate::AppState;

fn curseforge_client(state: &AppState) -> Result<CurseForgeClient, String> {
    let settings = load_settings(state)?;
    Ok(CurseForgeClient::new(ApiRouter::new(
        state.http.clone(),
        ModApiConfig {
            force_official: settings.mod_api_mode == "official",
            curseforge_api_key: settings.curseforge_api_key,
        },
    )))
}

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
    let hits = curseforge_client(&state)?
        .search(&{
            let mut request = SearchRequest::new(input.query, input.game_version);
            request.loader = input.loader;
            request.offset = input.offset.unwrap_or(0);
            request
        })
        .await
        .map_err(|error| error.to_string())?;
    Ok(hits
        .into_iter()
        .filter_map(|hit| {
            let id = hit.project_id.parse::<u64>().ok()?;
            Some(mcim::McimCurseForgeSearchResult {
                id,
                project_id: hit.project_id,
                slug: hit.slug,
                title: hit.title,
                description: hit.description,
                icon_url: hit.icon_url,
                downloads: hit.downloads,
                author: hit.author,
                categories: hit.categories,
            })
        })
        .collect())
}

#[tauri::command]
pub async fn get_mcim_curseforge_project(
    state: State<'_, AppState>,
    mod_id: u64,
) -> Result<ProjectDetails, String> {
    curseforge_client(&state)?
        .project(mod_id)
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
    let versions = curseforge_client(&state)?
        .files(mod_id, &game_version, loader.as_deref())
        .await
        .map_err(|error| error.to_string())?;
    Ok(versions
        .into_iter()
        .filter_map(|version| {
            let id = version.version_id.parse::<u64>().ok()?;
            let file = version.files.into_iter().next()?;
            Some(mcim::McimCurseForgeFile {
                id,
                mod_id,
                display_name: version.name,
                file_name: file.file_name,
                file_date: version.published_at,
                file_length: file.size,
                download_url: Some(file.url),
                game_versions: version.game_versions,
                release_type: match version.release_type.as_str() {
                    "beta" => 2,
                    "alpha" => 3,
                    _ => 1,
                },
                sha1: file.sha1,
            })
        })
        .collect())
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
