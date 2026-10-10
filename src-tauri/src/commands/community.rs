use std::sync::Arc;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::commands::settings::load_settings;
use crate::core::community;
use crate::core::conflict::ConflictFinding;
use crate::core::install::{self, ProgressReporter};
use crate::core::instance as instance_core;
use crate::core::mod_source::modrinth::ModrinthClient;
use crate::core::mod_source::{ApiRouter, ModApiConfig, ProjectDetails, VersionInfo};
use crate::AppState;

fn mod_api_config(state: &AppState) -> Result<ModApiConfig, String> {
    let settings = load_settings(state)?;
    Ok(ModApiConfig {
        force_official: settings.mod_api_mode == "official",
        curseforge_api_key: settings.curseforge_api_key,
    })
}

fn modrinth_client(state: &AppState) -> Result<ModrinthClient, String> {
    Ok(ModrinthClient::new(ApiRouter::new(
        state.http.clone(),
        mod_api_config(state)?,
    )))
}

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
    let client = modrinth_client(&state)?;
    let hits = client
        .search(&{
            let mut request =
                crate::core::mod_source::SearchRequest::new(input.query, input.game_version);
            request.loader = input.loader;
            request.offset = input.offset.unwrap_or(0);
            request
        })
        .await
        .map_err(|error| error.to_string())?;
    Ok(hits
        .into_iter()
        .map(|hit| community::ModrinthSearchResult {
            project_id: hit.project_id,
            slug: hit.slug,
            title: hit.title,
            description: hit.description,
            icon_url: hit.icon_url,
            downloads: hit.downloads,
            follows: hit.follows,
            author: hit.author,
            categories: hit.categories,
            versions: hit.versions,
        })
        .collect())
}

#[tauri::command]
pub async fn get_modrinth_versions(
    state: State<'_, AppState>,
    project_id: String,
    game_version: String,
    loader: Option<String>,
) -> Result<Vec<community::ModrinthVersionInfo>, String> {
    let client = modrinth_client(&state)?;
    let versions = client
        .versions(&project_id, &game_version, loader.as_deref())
        .await
        .map_err(|error| error.to_string())?;
    Ok(map_modrinth_versions(versions))
}

#[tauri::command]
pub async fn get_modrinth_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ProjectDetails, String> {
    modrinth_client(&state)?
        .project(&project_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_modrinth_version(
    state: State<'_, AppState>,
    version_id: String,
) -> Result<VersionInfo, String> {
    modrinth_client(&state)?
        .version(&version_id)
        .await
        .map_err(|error| error.to_string())
}

fn map_modrinth_versions(versions: Vec<VersionInfo>) -> Vec<community::ModrinthVersionInfo> {
    versions
        .into_iter()
        .enumerate()
        .map(|(index, version)| community::ModrinthVersionInfo {
            id: version.version_id,
            project_id: version.project_id,
            name: version.name,
            version_number: version.version_number,
            version_type: version.release_type,
            game_versions: version.game_versions,
            loaders: version.loaders,
            date_published: version.published_at,
            downloads: version.downloads,
            recommended: index == 0,
            files: version
                .files
                .into_iter()
                .filter(|file| file.url.starts_with("http"))
                .map(|file| community::ModrinthFile {
                    url: file.url,
                    filename: file.file_name,
                    primary: file.primary,
                    size: file.size,
                    hashes: community::ModrinthHashes {
                        sha1: file.sha1,
                        sha512: file.sha512,
                    },
                })
                .collect(),
        })
        .collect()
}

#[tauri::command]
pub async fn install_modrinth_version(
    app: AppHandle,
    state: State<'_, AppState>,
    input: InstallModrinthInput,
) -> Result<community::CommunityInstallResult, String> {
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

#[tauri::command]
pub async fn repair_instance_dependencies(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    job_id: String,
) -> Result<community::DependencyRepairResult, String> {
    let (instance, findings) = {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        let instance = instance_core::get_instance(&connection, &instance_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "实例不存在".to_string())?;
        let mut findings =
            crate::core::conflict::check_instance_conflicts(&connection, &instance, None)
                .map_err(|error| error.to_string())?;
        let reports = crate::core::conflict::list_conflict_reports(&connection, &instance_id)
            .map_err(|error| error.to_string())?;
        let mut seen_targets = findings
            .iter()
            .filter(|finding| finding.conflict_type == "missing_dependency")
            .filter_map(|finding| finding.target_mod_id.as_deref())
            .map(str::to_ascii_lowercase)
            .collect::<std::collections::HashSet<_>>();
        for report in reports.into_iter().filter(|report| {
            report.conflict_type == "missing_dependency" && report.target_mod_id.is_some()
        }) {
            let target = report.target_mod_id.clone().unwrap_or_default();
            if seen_targets.insert(target.to_ascii_lowercase()) {
                findings.push(ConflictFinding {
                    severity: report.severity,
                    conflict_type: report.conflict_type,
                    source_mod_id: report.source_mod_id,
                    target_mod_id: report.target_mod_id,
                    detail: report.detail,
                    suggestion: report.suggestion,
                });
            }
        }
        (instance, findings)
    };

    let event_handle = app.clone();
    let sink: install::ProgressSink = Arc::new(move |progress| {
        let _ = event_handle.emit("install-progress", progress);
    });
    let reporter = ProgressReporter::new(job_id, instance.version_id.clone(), sink);
    reporter.stage("dependency-repair", "正在查找缺失的必需依赖", 0, 0);
    let mut result = match community::repair_missing_dependencies(
        &state.http,
        &state.paths,
        &instance,
        &findings,
        &reporter,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            reporter.fail(&error.to_string());
            return Err(error.to_string());
        }
    };

    let connection = state.db.lock().map_err(|error| error.to_string())?;
    result.remaining =
        crate::core::conflict::check_instance_conflicts(&connection, &instance, None)
            .map_err(|error| error.to_string())?;
    if result.remaining.is_empty() {
        crate::core::conflict::clear_conflict_reports(&connection, &instance.id)
            .map_err(|error| error.to_string())?;
    }
    reporter.finish(if result.failed.is_empty() {
        "依赖补全完成"
    } else {
        "依赖补全完成，部分依赖未能安装"
    });
    Ok(result)
}
