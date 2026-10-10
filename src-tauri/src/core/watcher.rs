use std::time::Duration;

use anyhow::{Context, Result};
use notify::{EventKind, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Manager};

use crate::core::{deploy, project};
use crate::db::models::Project;
use crate::AppState;

pub fn start_registered_watchers(app: AppHandle) -> Result<()> {
    let projects = {
        let state = app.state::<AppState>();
        let connection = state
            .db
            .lock()
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        project::list_projects(&connection)?
    };
    for project in projects.into_iter().filter(|project| project.auto_deploy) {
        let _ = start_project_watcher(app.clone(), project);
    }
    Ok(())
}

pub fn start_project_watcher(app: AppHandle, project: Project) -> Result<()> {
    let project_id = project.id.clone();
    let output_dir = std::path::PathBuf::from(&project.output_dir);
    std::fs::create_dir_all(&output_dir).ok();

    let callback_app = app.clone();
    let callback_project_id = project_id.clone();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        let Ok(event) = result else {
            return;
        };
        if !matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
        ) {
            return;
        }
        let interesting = event.paths.iter().any(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"))
        });
        if !interesting {
            return;
        }

        let app = callback_app.clone();
        let project_id = callback_project_id.clone();
        let _ = app.emit("project-output-changed", project_id.clone());
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(900)).await;
            let _ = deploy_latest_for_project(&app, &project_id).await;
        });
    })?;
    watcher.watch(&output_dir, RecursiveMode::Recursive)?;

    let state = app.state::<AppState>();
    state
        .watchers
        .lock()
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .insert(project_id, watcher);
    Ok(())
}

pub fn stop_project_watcher(app: &AppHandle, project_id: &str) -> Result<()> {
    let state = app.state::<AppState>();
    state
        .watchers
        .lock()
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .remove(project_id);
    Ok(())
}

async fn deploy_latest_for_project(app: &AppHandle, project_id: &str) -> Result<()> {
    let state = app.state::<AppState>();
    let (project, links) = {
        let connection = state
            .db
            .lock()
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        let project = project::get_project(&connection, project_id)?
            .context("watcher project disappeared")?;
        let links = project::list_project_instances(&connection, project_id)?;
        (project, links)
    };

    for link in links.into_iter().filter(|link| link.auto_deploy) {
        let instance = {
            let connection = state
                .db
                .lock()
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            crate::core::instance::get_instance(&connection, &link.instance_id)?
                .context("linked instance disappeared")?
        };
        let connection = state
            .db
            .lock()
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        deploy::deploy_latest(&connection, &instance, &project.id)?;
        let _ = app.emit(
            "project-deployed",
            (project.id.clone(), instance.id.clone()),
        );
    }
    Ok(())
}
