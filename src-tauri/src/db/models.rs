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
pub struct Settings {
    pub download_mirror: String,
    pub download_concurrency: u32,
    pub default_java_path: Option<String>,
    pub shared_directory: String,
    pub data_directory: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsInput {
    pub download_mirror: Option<String>,
    pub download_concurrency: Option<u32>,
}

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}
