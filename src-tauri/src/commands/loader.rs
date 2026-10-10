use std::sync::Arc;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::commands::settings::load_settings;
use crate::core::install::{self, ProgressReporter};
use crate::core::instance as instance_core;
use crate::core::java;
use crate::core::loader::{self, LoaderInstallRequest, LoaderInstallResult, LoaderType};
use crate::core::version;
use crate::db::models::UpdateInstanceInput;
use crate::AppState;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallLoaderInput {
    pub instance_id: String,
    pub loader: LoaderType,
    pub loader_version: Option<String>,
    pub job_id: Option<String>,
}

#[tauri::command]
pub async fn install_loader(
    app: AppHandle,
    state: State<'_, AppState>,
    input: InstallLoaderInput,
) -> Result<LoaderInstallResult, String> {
    let settings = load_settings(&state)?;
    let instance = {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        instance_core::get_instance(&connection, &input.instance_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "实例不存在".to_string())?
    };
    let base_details = version::fetch_version_details(
        &state.http,
        &state.paths,
        &instance.version_id,
        &settings.download_mirror,
    )
    .await
    .map_err(|error| error.to_string())?;

    let job_id = input
        .job_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("loader-{}", Uuid::new_v4()));
    let event_handle = app.clone();
    let sink: install::ProgressSink = Arc::new(move |progress| {
        let _ = event_handle.emit("install-progress", progress);
    });
    let reporter = ProgressReporter::new(job_id, instance.version_id.clone(), sink);

    if !state
        .paths
        .version_dir(&instance.version_id)
        .join("client.jar")
        .is_file()
    {
        install::install_version(
            &state.http,
            &state.paths,
            &base_details,
            &settings.download_mirror,
            settings.download_concurrency,
            false,
            &reporter,
        )
        .await
        .map_err(|error| {
            let message = error.to_string();
            reporter.fail(&message);
            message
        })?;
    }

    let java_path = java::resolve_java_path(&instance, &settings, &base_details)
        .await
        .map_err(|error| error.to_string())?;
    let result = loader::install_loader(LoaderInstallRequest {
        client: &state.http,
        paths: &state.paths,
        base_details: &base_details,
        loader: input.loader,
        requested_version: input.loader_version,
        java_path,
        mirror: &settings.download_mirror,
        concurrency: settings.download_concurrency,
        reporter: &reporter,
    })
    .await
    .map_err(|error| {
        let message = error.to_string();
        reporter.fail(&message);
        message
    })?;

    {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        instance_core::update_instance(
            &connection,
            &instance.id,
            UpdateInstanceInput {
                name: None,
                version_id: None,
                loader: Some(result.loader.clone()),
                loader_version: Some(result.loader_version.clone()),
                java_path: None,
                memory_mb: None,
                jvm_args: None,
            },
        )
        .map_err(|error| error.to_string())?;
    }

    Ok(result)
}
