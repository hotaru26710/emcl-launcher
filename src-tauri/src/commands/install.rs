use std::sync::Arc;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::commands::settings::load_settings;
use crate::core::install::{self, InstallSummary, ProgressReporter};
use crate::core::instance as instance_core;
use crate::core::version;
use crate::AppState;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallVersionInput {
    pub version_id: String,
    pub instance_id: Option<String>,
    pub job_id: Option<String>,
    pub force_redownload: Option<bool>,
}

#[tauri::command]
pub async fn install_version(
    app: AppHandle,
    state: State<'_, AppState>,
    input: InstallVersionInput,
) -> Result<InstallSummary, String> {
    let settings = load_settings(&state)?;
    let job_id = input
        .job_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let version_id = input.version_id.clone();

    let event_handle = app.clone();
    let sink: install::ProgressSink = Arc::new(move |progress| {
        let _ = event_handle.emit("install-progress", progress);
    });
    let reporter = ProgressReporter::new(job_id, version_id.clone(), sink);

    let details = match version::fetch_version_details(
        &state.http,
        &state.paths,
        &version_id,
        &settings.download_mirror,
    )
    .await
    {
        Ok(details) => details,
        Err(error) => {
            let message = error.to_string();
            reporter.fail(&message);
            return Err(message);
        }
    };

    let summary = match install::install_version(
        &state.http,
        &state.paths,
        &details,
        &settings.download_mirror,
        settings.download_concurrency,
        input.force_redownload.unwrap_or(false),
        &reporter,
    )
    .await
    {
        Ok(summary) => summary,
        Err(error) => {
            let message = error.to_string();
            reporter.fail(&message);
            return Err(message);
        }
    };

    if let Some(instance_id) = input.instance_id.as_deref() {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        if let Err(error) = instance_core::record_installation(
            &connection,
            instance_id,
            &summary.version_id,
            &summary.status_path,
        ) {
            let message = error.to_string();
            reporter.fail(&message);
            return Err(message);
        }
    }

    Ok(summary)
}
