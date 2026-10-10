use anyhow::Context;
use tauri::{AppHandle, State};

use crate::commands::settings::load_settings;
use crate::core::auth;
use crate::core::instance as instance_core;
use crate::core::java;
use crate::core::launch::{self, LaunchRequest, LaunchResult};
use crate::core::loader;
use crate::core::version;
use crate::db::models::now_rfc3339;
use crate::AppState;

#[tauri::command]
pub async fn launch_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<LaunchResult, String> {
    let settings = load_settings(&state)?;
    let (instance, account) = {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        let instance = instance_core::get_instance(&connection, &instance_id)
            .map_err(|error| error.to_string())?
            .context("实例不存在")
            .map_err(|error| error.to_string())?;
        let account = auth::get_active_account(&connection)
            .map_err(|error| error.to_string())?
            .context("请先在账户页创建并选择一个账户")
            .map_err(|error| error.to_string())?;
        (instance, account)
    };

    let details = if let Some(loader_name) = instance.loader.as_deref() {
        let loader_version = instance
            .loader_version
            .as_deref()
            .context("实例缺少加载器版本")
            .map_err(|error| error.to_string())?;
        loader::resolve_effective_version_details(
            &state.http,
            &state.paths,
            &instance.version_id,
            loader_name,
            loader_version,
            &settings.download_mirror,
        )
        .await
        .map_err(|error| error.to_string())?
    } else {
        version::fetch_version_details(
            &state.http,
            &state.paths,
            &instance.version_id,
            &settings.download_mirror,
        )
        .await
        .map_err(|error| error.to_string())?
    };

    let java_path = java::resolve_java_path(&instance, &settings, &details)
        .await
        .map_err(|error| error.to_string())?;
    let result = launch::launch_instance(LaunchRequest {
        app: app.clone(),
        registry: state.processes.clone(),
        instance: instance.clone(),
        details,
        paths: &state.paths,
        settings: &settings,
        account,
        java_path,
    })
    .await
    .map_err(|error| error.to_string())?;

    {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        let now = now_rfc3339();
        connection
            .execute(
                "UPDATE instances SET last_played_at = ?1, updated_at = ?1 WHERE id = ?2",
                rusqlite::params![now, instance_id],
            )
            .map_err(|error| error.to_string())?;
    }

    Ok(result)
}

#[tauri::command]
pub fn stop_instance(state: State<'_, AppState>, instance_id: String) -> Result<(), String> {
    launch::stop_instance(&state.processes, &instance_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn is_instance_running(state: State<'_, AppState>, instance_id: String) -> bool {
    launch::is_running(&state.processes, &instance_id)
}
