use tauri::{AppHandle, Emitter, State};

use crate::core::crash::{self, AnalysisContext, CrashAnalysisProgress};
use crate::core::instance as instance_core;
use crate::AppState;

#[tauri::command]
pub async fn analyze_crash_archive(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    job_id: String,
    instance_id: Option<String>,
) -> Result<crash::CrashAnalysisReport, String> {
    let mut context = AnalysisContext::default();
    if let Some(instance_id) = instance_id.as_deref() {
        let connection = state.db.lock().map_err(|error| error.to_string())?;
        if let Some(instance) = instance_core::get_instance(&connection, instance_id)
            .map_err(|error| error.to_string())?
        {
            context = AnalysisContext::from_instance(&instance, "manual");
        }
    }
    if context.source.is_empty() {
        context.source = "manual".to_string();
    }

    let archive_path = std::path::PathBuf::from(&path);
    let paths = state.paths.clone();
    let event_handle = app.clone();
    let event_instance_id = context.instance_id.clone();
    let job_id_for_report = job_id.clone();
    let event_handle_for_error = app.clone();
    let error_instance_id = context.instance_id.clone();
    let error_job_id = job_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let emit = |stage: &str, message: &str, percent: f64| {
            let _ = event_handle.emit(
                "crash-analysis-progress",
                CrashAnalysisProgress {
                    job_id: job_id_for_report.clone(),
                    status: if stage == "complete" {
                        "complete".to_string()
                    } else {
                        "running".to_string()
                    },
                    stage: stage.to_string(),
                    message: message.to_string(),
                    percent,
                    instance_id: event_instance_id.clone(),
                },
            );
        };
        emit("starting", "正在准备错误报告分析", 4.0);
        let report = crash::analyze_crash_archive(&archive_path, context, &emit)?;
        crash::save_crash_report(&paths, &report)?;
        emit("complete", "错误报告分析完成", 100.0);
        anyhow::Ok(report)
    })
    .await;

    let report = match result {
        Ok(Ok(report)) => report,
        Ok(Err(error)) => {
            let _ = event_handle_for_error.emit(
                "crash-analysis-progress",
                CrashAnalysisProgress {
                    job_id: error_job_id,
                    status: "error".to_string(),
                    stage: "error".to_string(),
                    message: error.to_string(),
                    percent: 0.0,
                    instance_id: error_instance_id,
                },
            );
            return Err(error.to_string());
        }
        Err(error) => {
            let _ = event_handle_for_error.emit(
                "crash-analysis-progress",
                CrashAnalysisProgress {
                    job_id: error_job_id,
                    status: "error".to_string(),
                    stage: "error".to_string(),
                    message: error.to_string(),
                    percent: 0.0,
                    instance_id: error_instance_id,
                },
            );
            return Err(error.to_string());
        }
    };
    Ok(report)
}

#[tauri::command]
pub fn list_crash_reports(
    state: State<'_, AppState>,
) -> Result<Vec<crash::CrashAnalysisReport>, String> {
    crash::list_crash_reports(&state.paths).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_crash_report(
    state: State<'_, AppState>,
    id: String,
) -> Result<crash::CrashAnalysisReport, String> {
    crash::get_crash_report(&state.paths, &id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_crash_report(state: State<'_, AppState>, id: String) -> Result<(), String> {
    crash::delete_crash_report(&state.paths, &id).map_err(|error| error.to_string())
}
