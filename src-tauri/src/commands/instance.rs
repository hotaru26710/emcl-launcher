use tauri::State;

use crate::core::instance as instance_core;
use crate::db::models::{CreateInstanceInput, Instance, UpdateInstanceInput};
use crate::AppState;

#[tauri::command]
pub fn list_instances(state: State<'_, AppState>) -> Result<Vec<Instance>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    instance_core::list_instances(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_instance(state: State<'_, AppState>, id: String) -> Result<Option<Instance>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    instance_core::get_instance(&connection, &id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_instance(
    state: State<'_, AppState>,
    input: CreateInstanceInput,
) -> Result<Instance, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    instance_core::create_instance(&state.paths, &connection, input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_instance(
    state: State<'_, AppState>,
    id: String,
    input: UpdateInstanceInput,
) -> Result<Instance, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    instance_core::update_instance(&connection, &id, input).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_instance(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    instance_core::delete_instance(&state.paths, &connection, &id)
        .map_err(|error| error.to_string())
}
