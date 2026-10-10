use tauri::State;

use crate::core::instance as instance_core;
use crate::core::mods;
use crate::db::models::Instance;
use crate::AppState;

#[tauri::command]
pub fn list_mods(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<Vec<mods::ModInfo>, String> {
    with_instance(&state, &instance_id, |paths, instance| {
        mods::list_mods(paths, instance)
    })
}

#[tauri::command]
pub fn install_mods(
    state: State<'_, AppState>,
    instance_id: String,
    paths: Vec<String>,
) -> Result<Vec<mods::ModInfo>, String> {
    with_instance(&state, &instance_id, |launcher_paths, instance| {
        mods::install_mods(launcher_paths, instance, paths)
    })
}

#[tauri::command]
pub fn toggle_mod(
    state: State<'_, AppState>,
    instance_id: String,
    file_name: String,
    enabled: bool,
) -> Result<Vec<mods::ModInfo>, String> {
    with_instance(&state, &instance_id, |paths, instance| {
        mods::toggle_mod(paths, instance, &file_name, enabled)
    })
}

#[tauri::command]
pub fn delete_mod(
    state: State<'_, AppState>,
    instance_id: String,
    file_name: String,
) -> Result<Vec<mods::ModInfo>, String> {
    with_instance(&state, &instance_id, |paths, instance| {
        mods::delete_mod(paths, instance, &file_name)
    })
}

fn with_instance<F>(
    state: &AppState,
    instance_id: &str,
    operation: F,
) -> Result<Vec<mods::ModInfo>, String>
where
    F: FnOnce(&crate::core::paths::LauncherPaths, &Instance) -> anyhow::Result<Vec<mods::ModInfo>>,
{
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    let instance = instance_core::get_instance(&connection, instance_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "实例不存在".to_string())?;
    operation(&state.paths, &instance).map_err(|error| error.to_string())
}
