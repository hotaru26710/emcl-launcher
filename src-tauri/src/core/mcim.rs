use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::core::community::{self, CommunityInstallResult};
use crate::core::install::ProgressReporter;
use crate::db::models::Instance;

const MCIM_BASE: &str = "https://mod.mcimirror.top/curseforge/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McimCurseForgeSearchResult {
    pub id: u64,
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub author: String,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McimCurseForgeFile {
    pub id: u64,
    pub mod_id: u64,
    pub display_name: String,
    pub file_name: String,
    pub file_date: String,
    pub file_length: u64,
    pub download_url: Option<String>,
    pub game_versions: Vec<String>,
    pub release_type: u32,
    pub sha1: Option<String>,
}

pub async fn search_curseforge(
    client: &Client,
    query: &str,
    game_version: &str,
    loader: Option<&str>,
    offset: u32,
) -> Result<Vec<McimCurseForgeSearchResult>> {
    let mut params = vec![
        ("gameId", "432".to_string()),
        ("classId", "6".to_string()),
        ("searchFilter", query.to_string()),
        ("gameVersion", game_version.to_string()),
        ("index", offset.to_string()),
        ("pageSize", "20".to_string()),
        ("sortField", "2".to_string()),
        ("sortOrder", "desc".to_string()),
    ];
    if let Some(loader_type) = loader_type(loader) {
        params.push(("modLoaderType", loader_type.to_string()));
    }
    let response: McimResponse<Vec<McimSearchHit>> =
        mcim_get(client, "/mods/search", &params).await?;
    Ok(response
        .data
        .into_iter()
        .map(|hit| McimCurseForgeSearchResult {
            project_id: hit.id.to_string(),
            id: hit.id,
            slug: hit.slug,
            title: hit.name,
            description: hit.summary,
            icon_url: hit.logo.and_then(|logo| logo.url),
            downloads: hit.download_count,
            author: hit
                .authors
                .first()
                .map(|author| author.name.clone())
                .unwrap_or_default(),
            categories: hit
                .categories
                .into_iter()
                .map(|category| category.name)
                .collect(),
        })
        .collect())
}

pub async fn get_curseforge_files(
    client: &Client,
    mod_id: u64,
    game_version: &str,
    loader: Option<&str>,
) -> Result<Vec<McimCurseForgeFile>> {
    let mut params = vec![
        ("gameVersion", game_version.to_string()),
        ("pageSize", "50".to_string()),
        ("index", "0".to_string()),
    ];
    if let Some(loader_type) = loader_type(loader) {
        params.push(("modLoaderType", loader_type.to_string()));
    }
    let response: McimResponse<Vec<McimRawFile>> =
        mcim_get(client, &format!("/mods/{mod_id}/files"), &params).await?;
    let mut files = response
        .data
        .into_iter()
        .map(|file| McimCurseForgeFile {
            id: file.id,
            mod_id: file.mod_id,
            display_name: file.display_name,
            file_name: file.file_name,
            file_date: file.file_date,
            file_length: file.file_length,
            download_url: file.download_url.map(|url| rewrite_download_url(&url)),
            game_versions: file.game_versions,
            release_type: file.release_type,
            sha1: file
                .hashes
                .iter()
                .find(|hash| hash.algo == 1)
                .map(|hash| hash.value.clone()),
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|file| std::cmp::Reverse(file.file_date.clone()));
    Ok(files)
}

pub async fn install_curseforge_file(
    client: &Client,
    instance: &Instance,
    mod_id: u64,
    file_id: u64,
    reporter: &ProgressReporter,
) -> Result<CommunityInstallResult> {
    let mut result = CommunityInstallResult::default();
    let mut visited = HashSet::new();
    install_recursive(
        client,
        instance,
        mod_id,
        file_id,
        reporter,
        0,
        &mut visited,
        &mut result,
    )
    .await?;
    result.mods = crate::core::mods::list_mods(
        &crate::core::paths::LauncherPaths::from_root(std::path::PathBuf::from(".")),
        instance,
    )?;
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
async fn install_recursive(
    client: &Client,
    instance: &Instance,
    mod_id: u64,
    file_id: u64,
    reporter: &ProgressReporter,
    depth: usize,
    visited: &mut HashSet<u64>,
    result: &mut CommunityInstallResult,
) -> Result<()> {
    if depth > 8 {
        bail!("MCIM CurseForge 依赖层级过深");
    }
    if !visited.insert(file_id) {
        return Ok(());
    }
    let response: McimResponse<McimRawFile> =
        mcim_get(client, &format!("/mods/{mod_id}/files/{file_id}"), &[]).await?;
    let file = response.data;
    let download_url = file
        .download_url
        .as_deref()
        .map(rewrite_download_url)
        .filter(|url| !url.is_empty())
        .context("MCIM CurseForge 文件没有下载地址")?;
    let sha1 = file
        .hashes
        .iter()
        .find(|hash| hash.algo == 1)
        .map(|hash| hash.value.clone());
    let skipped = community::install_mod_url_file(
        client,
        instance,
        &download_url,
        &file.file_name,
        sha1.as_deref(),
        Some(file.file_length),
        reporter,
    )
    .await?;
    if skipped {
        result.skipped += 1;
    } else {
        result.installed += 1;
    }
    result.dependencies.push(file.file_name.clone());

    for dependency in file
        .dependencies
        .into_iter()
        .filter(|dependency| dependency.relation_type == 3)
    {
        let files = get_curseforge_files(
            client,
            dependency.mod_id,
            &instance.version_id,
            instance.loader.as_deref(),
        )
        .await
        .with_context(|| format!("无法解析 CurseForge 依赖 {}", dependency.mod_id))?;
        let dependency_file = files.first().with_context(|| {
            format!(
                "CurseForge 依赖 {} 没有适用于 {} / {} 的文件",
                dependency.mod_id,
                instance.version_id,
                instance.loader.as_deref().unwrap_or("vanilla")
            )
        })?;
        Box::pin(install_recursive(
            client,
            instance,
            dependency.mod_id,
            dependency_file.id,
            reporter,
            depth + 1,
            visited,
            result,
        ))
        .await?;
    }
    Ok(())
}

fn rewrite_download_url(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("https://edge.forgecdn.net/") {
        return format!("https://mod.mcimirror.top/{rest}");
    }
    url.to_string()
}

fn loader_type(loader: Option<&str>) -> Option<u32> {
    match loader.unwrap_or_default().to_ascii_lowercase().as_str() {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}

async fn mcim_get<T: for<'de> Deserialize<'de>>(
    client: &Client,
    path: &str,
    query: &[(&str, String)],
) -> Result<McimResponse<T>> {
    let response = client
        .get(format!("{MCIM_BASE}{path}"))
        .query(query)
        .header(
            "User-Agent",
            "EmberMCL/0.1.0 (+https://github.com/hotaru26710/emcl-launcher)",
        )
        .send()
        .await
        .context("MCIM 请求失败")?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        bail!(
            "MCIM 返回 {status}: {}",
            text.chars().take(300).collect::<String>()
        );
    }
    serde_json::from_str(&text).context("无法解析 MCIM 响应")
}

#[derive(Debug, Deserialize)]
struct McimResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McimSearchHit {
    id: u64,
    name: String,
    slug: String,
    summary: String,
    download_count: u64,
    logo: Option<McimLogo>,
    categories: Vec<McimCategory>,
    authors: Vec<McimAuthor>,
}

#[derive(Debug, Deserialize)]
struct McimLogo {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct McimCategory {
    name: String,
}

#[derive(Debug, Deserialize)]
struct McimAuthor {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McimRawFile {
    id: u64,
    mod_id: u64,
    display_name: String,
    file_name: String,
    file_date: String,
    file_length: u64,
    download_url: Option<String>,
    game_versions: Vec<String>,
    release_type: u32,
    dependencies: Vec<McimDependency>,
    hashes: Vec<McimHash>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McimDependency {
    mod_id: u64,
    relation_type: u32,
}

#[derive(Debug, Deserialize)]
struct McimHash {
    value: String,
    algo: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_forgecdn_urls() {
        assert_eq!(
            rewrite_download_url("https://edge.forgecdn.net/files/1/2/test.jar"),
            "https://mod.mcimirror.top/files/1/2/test.jar"
        );
    }
}
