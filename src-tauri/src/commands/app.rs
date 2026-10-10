use serde::Serialize;
use tauri::State;

use crate::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherInfo {
    pub data_directory: String,
    pub shared_directory: String,
    pub instances_directory: String,
    pub database_path: String,
}

#[tauri::command]
pub fn get_launcher_info(state: State<'_, AppState>) -> LauncherInfo {
    LauncherInfo {
        data_directory: state.paths.root().to_string_lossy().into_owned(),
        shared_directory: state.paths.shared_dir().to_string_lossy().into_owned(),
        instances_directory: state.paths.instances_dir().to_string_lossy().into_owned(),
        database_path: state.paths.database_file().to_string_lossy().into_owned(),
    }
}
