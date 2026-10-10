use std::sync::Arc;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::core::community;
use crate::core::conflict::ConflictFinding;
use crate::core::install::{self, ProgressReporter};
use crate::core::instance as instance_core;
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
