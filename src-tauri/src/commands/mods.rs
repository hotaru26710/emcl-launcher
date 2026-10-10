use tauri::State;

use crate::commands::settings::load_settings;
use crate::core::instance as instance_core;
use crate::core::mod_install;
use crate::core::mod_source::ModApiConfig;
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

#[tauri::command]
pub fn list_installed_mods(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<Vec<mod_install::InstalledMod>, String> {
    let connection = state.db.lock().map_err(|error| error.to_string())?;
    let instance = instance_core::get_instance(&connection, &instance_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "实例不存在".to_string())?;
    mod_install::sync_from_mods(&connection, &instance).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn check_mod_updates(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<Vec<crate::core::mod_update::ModUpdate>, String> {
    let settings = load_settings(&state)?;
    let config = ModApiConfig {
        force_official: settings.mod_api_mode == "official",
        curseforge_api_key: settings.curseforge_api_key,
    };
    let (instance, installed) = {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        let instance = instance_core::get_instance(&connection, &instance_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "实例不存在".to_string())?;
        let installed = mod_install::sync_from_mods(&connection, &instance)
            .map_err(|error| error.to_string())?;
        (instance, installed)
    };
    let installed_ids = installed
        .iter()
        .map(|item| item.mod_id.clone())
        .collect::<Vec<_>>();
    let updates = crate::core::mod_update::check_updates(&state.http, &instance, installed, config)
        .await
        .map_err(|error| error.to_string())?;
    {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        for mod_id in installed_ids {
            mod_install::set_update(&connection, &instance.id, &mod_id, None)
                .map_err(|error| error.to_string())?;
        }
        for update in &updates {
            mod_install::set_update(
                &connection,
                &instance.id,
                &update.mod_id,
                Some(&update.latest_version_id),
            )
            .map_err(|error| error.to_string())?;
        }
    }
    Ok(updates)
}
