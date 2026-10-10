#![allow(dead_code)]

use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub version_id: String,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub java_path: Option<String>,
    pub jvm_args: String,
    pub memory_mb: u32,
    pub directory: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_played_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInstanceInput {
    pub name: String,
    pub version_id: String,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub memory_mb: Option<u32>,
    pub jvm_args: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstanceInput {
    pub name: Option<String>,
    pub version_id: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub java_path: Option<String>,
    pub memory_mb: Option<u32>,
    pub jvm_args: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub id: String,
    pub username: String,
    pub uuid: String,
    pub account_type: String,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredAccount {
    pub id: String,
    pub username: String,
    pub uuid: String,
    pub account_type: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub download_mirror: String,
    pub download_concurrency: u32,
    pub mod_api_mode: String,
    pub curseforge_api_key: Option<String>,
    pub default_java_path: Option<String>,
    pub microsoft_client_id: Option<String>,
    pub theme: String,
    pub custom_theme: Option<String>,
    pub default_memory_mb: u32,
    pub game_width: u32,
    pub game_height: u32,
    pub fullscreen: bool,
    pub shared_directory: String,
    pub data_directory: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsInput {
    pub download_mirror: Option<String>,
    pub download_concurrency: Option<u32>,
    pub mod_api_mode: Option<String>,
    pub curseforge_api_key: Option<String>,
    pub microsoft_client_id: Option<String>,
    pub theme: Option<String>,
    pub custom_theme: Option<String>,
    pub default_memory_mb: Option<u32>,
    pub game_width: Option<u32>,
    pub game_height: Option<u32>,
    pub fullscreen: Option<bool>,
}

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub build_tool: String,
    pub output_dir: String,
    pub gradle_task: String,
    pub auto_build: bool,
    pub auto_deploy: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectInput {
    pub name: String,
    pub root_path: String,
    pub output_dir: Option<String>,
    pub gradle_task: Option<String>,
    pub auto_build: Option<bool>,
    pub auto_deploy: Option<bool>,
    pub instance_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedProjectInstance {
    pub project_id: String,
    pub instance_id: String,
    pub instance_name: String,
    pub auto_deploy: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectInput {
    pub name: Option<String>,
    pub output_dir: Option<String>,
    pub gradle_task: Option<String>,
    pub auto_build: Option<bool>,
    pub auto_deploy: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildArtifact {
    pub id: String,
    pub project_id: String,
    pub file_path: String,
    pub sha256: String,
    pub file_size: u64,
    pub built_at: String,
    pub git_commit: Option<String>,
    pub git_branch: Option<String>,
    pub status: String,
    pub deployed_instance_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModMetadata {
    pub instance_id: String,
    pub mod_id: Option<String>,
    pub mod_name: Option<String>,
    pub version: Option<String>,
    pub loader: Option<String>,
    pub file_name: String,
    pub sha256: String,
    pub source_type: String,
    pub depends_json: String,
    pub breaks_json: String,
    pub conflicts_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictReport {
    pub id: i64,
    pub instance_id: String,
    pub severity: String,
    pub conflict_type: String,
    pub source_mod_id: Option<String>,
    pub target_mod_id: Option<String>,
    pub detail: String,
    pub suggestion: String,
    pub created_at: String,
}
