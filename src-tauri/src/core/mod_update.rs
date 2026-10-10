use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::core::mod_install::InstalledMod;
use crate::core::mod_source::curseforge::CurseForgeClient;
use crate::core::mod_source::modrinth::ModrinthClient;
use crate::core::mod_source::ModApiConfig;
use crate::db::models::Instance;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedModSource {
    pub mod_id: String,
    pub platform: String,
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModUpdateResult {
    pub updates: Vec<ModUpdate>,
    pub resolved_sources: Vec<ResolvedModSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModUpdate {
    pub mod_id: String,
    pub mod_name: Option<String>,
    pub file_name: String,
    pub platform: String,
    pub project_id: String,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub latest_version_id: String,
    pub update_available: bool,
}

pub async fn check_updates(
    http: &reqwest::Client,
    instance: &Instance,
    installed: Vec<InstalledMod>,
    config: ModApiConfig,
) -> Result<ModUpdateResult> {
    let modrinth = ModrinthClient::new(crate::core::mod_source::ApiRouter::new(
        http.clone(),
        config.clone(),
    ));
    let curseforge = CurseForgeClient::new(crate::core::mod_source::ApiRouter::new(
        http.clone(),
        config,
    ));
    let mut updates = Vec::new();
    let mut resolved_sources = Vec::new();

    for mut item in installed {
        if item.platform == "local" {
            if let Some((platform, project_id)) =
                resolve_local_source(&item, instance, &modrinth, &curseforge).await
            {
                resolved_sources.push(ResolvedModSource {
                    mod_id: item.mod_id.clone(),
                    platform: platform.to_string(),
                    project_id: project_id.clone(),
                });
                item.platform = platform.to_string();
                item.project_id = project_id;
            }
        }
        match item.platform.as_str() {
            "modrinth" if !item.project_id.is_empty() => {
                if let Ok(versions) = modrinth
                    .versions(
                        &item.project_id,
                        &instance.version_id,
                        instance.loader.as_deref(),
                    )
                    .await
                {
                    if let Some(latest) = versions.first() {
                        let available =
                            item.version_id.as_deref() != Some(latest.version_id.as_str());
                        if available {
                            updates.push(to_update(
                                &item,
                                latest.version_id.clone(),
                                Some(latest.version_number.clone()),
                            ));
                        }
                    }
                }
            }
            "curseforge" => {
                if let Ok(mod_id) = item.project_id.parse::<u64>() {
                    if let Ok(files) = curseforge
                        .files(mod_id, &instance.version_id, instance.loader.as_deref())
                        .await
                    {
                        if let Some(latest) = files.first() {
                            let available =
                                item.version_id.as_deref() != Some(latest.version_id.as_str());
                            if available {
                                updates.push(to_update(
                                    &item,
                                    latest.version_id.clone(),
                                    Some(latest.version_number.clone()),
                                ));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(ModUpdateResult {
        updates,
        resolved_sources,
    })
}

async fn resolve_local_source(
    item: &InstalledMod,
    instance: &Instance,
    modrinth: &ModrinthClient,
    curseforge: &CurseForgeClient,
) -> Option<(&'static str, String)> {
    let target = normalize(&item.mod_id);
    if target.is_empty() {
        return None;
    }

    let mut request =
        crate::core::mod_source::SearchRequest::new(&item.mod_id, &instance.version_id);
    request.loader = instance.loader.clone();
    if let Ok(hits) = modrinth.search(&request).await {
        if let Some(hit) = hits.into_iter().find(|hit| {
            normalize(&hit.slug) == target
                || normalize(&hit.title) == target
                || normalize(&hit.project_id) == target
        }) {
            return Some(("modrinth", hit.project_id));
        }
    }

    if let Ok(hits) = curseforge.search(&request).await {
        if let Some(hit) = hits.into_iter().find(|hit| {
            normalize(&hit.slug) == target
                || normalize(&hit.title) == target
                || normalize(&hit.project_id) == target
        }) {
            return Some(("curseforge", hit.project_id));
        }
    }
    None
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn to_update(
    item: &InstalledMod,
    latest_version_id: String,
    latest_version: Option<String>,
) -> ModUpdate {
    ModUpdate {
        mod_id: item.mod_id.clone(),
        mod_name: item.mod_name.clone(),
        file_name: item.file_name.clone(),
        platform: item.platform.clone(),
        project_id: item.project_id.clone(),
        current_version: item.version.clone(),
        latest_version,
        latest_version_id,
        update_available: true,
    }
}
