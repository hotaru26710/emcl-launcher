use std::path::Path;

use tauri::State;

use crate::core::project;
use crate::db::models::{CreateProjectInput, LinkedProjectInstance, Project, UpdateProjectInput};
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
    state: State<'_, AppState>,
    input: CreateProjectInput,
) -> Result<Project, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    project::create_project(&connection, input).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_project(
    state: State<'_, AppState>,
    id: String,
    input: UpdateProjectInput,
) -> Result<Project, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    project::update_project(&connection, &id, input).map_err(|error| error.to_string())
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
