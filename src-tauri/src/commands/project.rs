use std::path::Path;

use tauri::{AppHandle, State};

use crate::core::mods::ModInfo;
use crate::core::{
    artifact, build, conflict, crash, deploy, instance as instance_core, metadata, project, watcher,
};
use crate::db::models::{
    BuildArtifact, ConflictReport, CreateProjectInput, LinkedProjectInstance, ModMetadata, Project,
    UpdateProjectInput,
};
use crate::AppState;

#[tauri::command]
pub fn inspect_project_directory(path: String) -> Result<project::ProjectInspection, String> {
    project::inspect_project_directory(Path::new(&path)).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    project::list_projects(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_project(state: State<'_, AppState>, id: String) -> Result<Option<Project>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    project::get_project(&connection, &id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_project(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CreateProjectInput,
) -> Result<Project, String> {
    let project = {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        project::create_project(&connection, input).map_err(|error| error.to_string())?
    };
    if project.auto_deploy {
        let _ = watcher::start_project_watcher(app, project.clone());
    }
    Ok(project)
}

#[tauri::command]
pub fn update_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    input: UpdateProjectInput,
) -> Result<Project, String> {
    let project = {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        project::update_project(&connection, &id, input).map_err(|error| error.to_string())?
    };
    if project.auto_deploy {
        let _ = watcher::start_project_watcher(app, project.clone());
    } else {
        let _ = watcher::stop_project_watcher(&app, &id);
    }
    Ok(project)
}

#[tauri::command]
pub fn delete_project(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    project::delete_project(&connection, &id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn link_project_instance(
    state: State<'_, AppState>,
    project_id: String,
    instance_id: String,
    auto_deploy: Option<bool>,
) -> Result<(), String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    project::link_project_instance(
        &connection,
        &project_id,
        &instance_id,
        auto_deploy.unwrap_or(true),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn unlink_project_instance(
    state: State<'_, AppState>,
    project_id: String,
    instance_id: String,
) -> Result<(), String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    project::unlink_project_instance(&connection, &project_id, &instance_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_project_instances(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<LinkedProjectInstance>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    project::list_project_instances(&connection, &project_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn scan_project_artifacts(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<BuildArtifact>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    let project = project::get_project(&connection, &project_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "工程不存在".to_string())?;
    artifact::scan_project_artifacts(&connection, &project).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_build_artifacts(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<BuildArtifact>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    artifact::list_build_artifacts(&connection, &project_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_build_artifact(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<BuildArtifact>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    artifact::get_build_artifact(&connection, &id).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn run_project_build(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<BuildArtifact>, String> {
    let (project, _) = {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        let project = project::get_project(&connection, &project_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "工程不存在".to_string())?;
        (project, ())
    };
    build::build_project(app, &project)
        .await
        .map_err(|error| error.to_string())?;
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    artifact::scan_project_artifacts(&connection, &project).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn deploy_build_artifact(
    state: State<'_, AppState>,
    instance_id: String,
    artifact_id: String,
) -> Result<Vec<ModInfo>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    let instance = instance_core::get_instance(&connection, &instance_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "实例不存在".to_string())?;
    let artifact = artifact::get_build_artifact(&connection, &artifact_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "构建产物不存在".to_string())?;
    deploy::deploy_artifact(&connection, &instance, &artifact).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn check_instance_conflicts(
    state: State<'_, AppState>,
    instance_id: String,
    artifact_id: Option<String>,
) -> Result<Vec<conflict::ConflictFinding>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    let instance = instance_core::get_instance(&connection, &instance_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "实例不存在".to_string())?;
    let candidate = artifact_id
        .as_deref()
        .map(|id| artifact::get_build_artifact(&connection, id))
        .transpose()
        .map_err(|error| error.to_string())?
        .flatten()
        .map(|artifact| std::path::PathBuf::from(artifact.file_path));
    conflict::check_instance_conflicts(&connection, &instance, candidate.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_conflict_reports(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<Vec<ConflictReport>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    conflict::list_conflict_reports(&connection, &instance_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn refresh_instance_mod_metadata(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<Vec<ModMetadata>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    let instance = instance_core::get_instance(&connection, &instance_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "实例不存在".to_string())?;
    metadata::refresh_instance_mod_metadata(&connection, &instance)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn start_project_watcher(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
) -> Result<(), String> {
    let project = {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        project::get_project(&connection, &project_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "工程不存在".to_string())?
    };
    watcher::start_project_watcher(app, project).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn stop_project_watcher(app: AppHandle, project_id: String) -> Result<(), String> {
    watcher::stop_project_watcher(&app, &project_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analyze_crash_log(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<crash::CrashAnalysis, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    let instance = instance_core::get_instance(&connection, &instance_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "实例不存在".to_string())?;
    crash::analyze_latest_log(&instance).map_err(|error| error.to_string())
}
