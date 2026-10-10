use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::install::{self, ArtifactDownload, ArtifactDownloadRequest, ProgressReporter};
use crate::core::mods::{self, ModInfo};
use crate::core::paths::LauncherPaths;
use crate::db::models::Instance;

const MODRINTH_API: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthSearchResult {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: u64,
    pub author: String,
    pub categories: Vec<String>,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthVersionInfo {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub version_type: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub date_published: String,
    pub downloads: u64,
    pub files: Vec<ModrinthFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub hashes: ModrinthHashes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthHashes {
    pub sha1: Option<String>,
    pub sha512: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModrinthSearchResponse {
    hits: Vec<ModrinthSearchHit>,
}

#[derive(Debug, Deserialize)]
struct ModrinthSearchHit {
    project_id: String,
    slug: String,
    title: String,
    description: String,
    icon_url: Option<String>,
    downloads: u64,
    follows: u64,
    author: String,
    categories: Vec<String>,
    versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ModrinthVersion {
    id: String,
    project_id: String,
    name: String,
    version_number: String,
    version_type: String,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    date_published: String,
    downloads: u64,
    files: Vec<ModrinthFile>,
    dependencies: Vec<ModrinthDependency>,
}

#[derive(Debug, Deserialize)]
struct ModrinthDependency {
    version_id: Option<String>,
    project_id: Option<String>,
    dependency_type: String,
}

pub async fn search_modrinth(
    client: &Client,
    query: &str,
    game_version: &str,
    loader: Option<&str>,
    offset: u32,
) -> Result<Vec<ModrinthSearchResult>> {
    let mut facets = vec![json!(["project_type:mod"])];
    if !game_version.trim().is_empty() {
        facets.push(json!([format!("versions:{game_version}")]));
    }
    if let Some(loader) = loader.filter(|value| !value.trim().is_empty() && *value != "vanilla") {
        facets.push(json!([format!("categories:{}", normalize_loader(loader))]));
    }

    let response: ModrinthSearchResponse = api_get(
        client,
        &format!("{MODRINTH_API}/search"),
        &[
            ("query", query.to_string()),
            ("limit", "20".to_string()),
            ("offset", offset.to_string()),
            ("facets", serde_json::to_string(&facets)?),
        ],
    )
    .await?;

    Ok(response
        .hits
        .into_iter()
        .map(|hit| ModrinthSearchResult {
            project_id: hit.project_id,
            slug: hit.slug,
            title: hit.title,
            description: hit.description,
            icon_url: hit.icon_url,
            downloads: hit.downloads,
            follows: hit.follows,
            author: hit.author,
            categories: hit.categories,
            versions: hit.versions,
        })
        .collect())
}

pub async fn get_modrinth_versions(
    client: &Client,
    project_id: &str,
    game_version: &str,
    loader: Option<&str>,
) -> Result<Vec<ModrinthVersionInfo>> {
    let mut query = vec![("game_versions", serde_json::to_string(&vec![game_version])?)];
    if let Some(loader) = loader.filter(|value| !value.trim().is_empty() && *value != "vanilla") {
        query.push((
            "loaders",
            serde_json::to_string(&vec![normalize_loader(loader)])?,
        ));
    }
    let versions: Vec<ModrinthVersion> = api_get(
        client,
        &format!("{MODRINTH_API}/project/{project_id}/version"),
        &query,
    )
    .await?;

    Ok(versions.into_iter().map(version_info).collect())
}

pub async fn install_modrinth_version(
    client: &Client,
    paths: &LauncherPaths,
    instance: &Instance,
    version_id: &str,
    reporter: &ProgressReporter,
) -> Result<Vec<ModInfo>> {
    let mut visited = HashSet::new();
    install_version_recursive(
        client,
        paths,
        instance,
        version_id,
        reporter,
        0,
        &mut visited,
    )
    .await?;
    mods::list_mods(paths, instance)
}

async fn install_version_recursive(
    client: &Client,
    paths: &LauncherPaths,
    instance: &Instance,
    version_id: &str,
    reporter: &ProgressReporter,
    depth: usize,
    visited: &mut HashSet<String>,
) -> Result<()> {
    if depth > 8 {
        bail!("Modrinth 依赖层级过深");
    }
    if !visited.insert(version_id.to_string()) {
        return Ok(());
    }

    let version: ModrinthVersion =
        api_get(client, &format!("{MODRINTH_API}/version/{version_id}"), &[]).await?;
    let file = version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .context("Modrinth 版本没有可下载文件")?;
    install_mod_file(client, paths, instance, file, reporter).await?;

    for dependency in version
        .dependencies
        .iter()
        .filter(|dependency| dependency.dependency_type == "required")
    {
        if let Some(version_id) = dependency.version_id.as_deref() {
            Box::pin(install_version_recursive(
                client,
                paths,
                instance,
                version_id,
                reporter,
                depth + 1,
                visited,
            ))
            .await?;
        } else if let Some(project_id) = dependency.project_id.as_deref() {
            let versions = get_modrinth_versions(
                client,
                project_id,
                &instance.version_id,
                instance.loader.as_deref(),
            )
            .await?;
            if let Some(version) = versions.first() {
                Box::pin(install_version_recursive(
                    client,
                    paths,
                    instance,
                    &version.id,
                    reporter,
                    depth + 1,
                    visited,
                ))
                .await?;
            }
        }
    }

    Ok(())
}

async fn install_mod_file(
    client: &Client,
    _paths: &LauncherPaths,
    instance: &Instance,
    file: &ModrinthFile,
    reporter: &ProgressReporter,
) -> Result<()> {
    if !file.filename.to_ascii_lowercase().ends_with(".jar") {
        bail!("社区模组文件不是 .jar: {}", file.filename);
    }
    let destination = safe_mod_path(
        &PathBuf::from(&instance.directory).join("mods"),
        &file.filename,
    )?;
    install::download_artifacts(ArtifactDownloadRequest {
        client,
        mirror: "official",
        concurrency: 1,
        force_redownload: false,
        stage: "community-mod",
        message: "正在下载社区模组",
        artifacts: vec![ArtifactDownload {
            url: file.url.clone(),
            destination,
            sha1: file.hashes.sha1.clone(),
            size: Some(file.size),
            label: file.filename.clone(),
            version_id: instance.version_id.clone(),
        }],
        reporter,
    })
    .await
}

fn version_info(version: ModrinthVersion) -> ModrinthVersionInfo {
    ModrinthVersionInfo {
        id: version.id,
        project_id: version.project_id,
        name: version.name,
        version_number: version.version_number,
        version_type: version.version_type,
        game_versions: version.game_versions,
        loaders: version.loaders,
        date_published: version.date_published,
        downloads: version.downloads,
        files: version
            .files
            .into_iter()
            .map(|file| ModrinthFile {
                url: file.url,
                filename: file.filename,
                primary: file.primary,
                size: file.size,
                hashes: file.hashes,
            })
            .collect(),
    }
}

async fn api_get<T: for<'de> Deserialize<'de>>(
    client: &Client,
    url: &str,
    query: &[(&str, String)],
) -> Result<T> {
    let response = client
        .get(url)
        .query(query)
        .header("User-Agent", "EMCL/0.1.0")
        .send()
        .await
        .with_context(|| format!("Modrinth 请求失败: {url}"))?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        bail!(
            "Modrinth 返回 {status}: {}",
            text.chars().take(300).collect::<String>()
        );
    }
    serde_json::from_str(&text).with_context(|| format!("无法解析 Modrinth 响应: {url}"))
}

fn normalize_loader(loader: &str) -> &str {
    match loader {
        "neoForge" | "neoforge" => "neoforge",
        other => other,
    }
}

fn safe_mod_path(directory: &Path, file_name: &str) -> Result<PathBuf> {
    let path = Path::new(file_name);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("社区模组文件名不安全");
    }
    std::fs::create_dir_all(directory)?;
    Ok(directory.join(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn searches_and_resolves_modrinth_versions() {
        let client = Client::builder()
            .user_agent("EMCL-Community-test/0.1.0")
            .build()
            .unwrap();
        let results = search_modrinth(&client, "sodium", "1.21.1", Some("fabric"), 0)
            .await
            .unwrap();
        assert!(!results.is_empty());
        let versions =
            get_modrinth_versions(&client, &results[0].project_id, "1.21.1", Some("fabric"))
                .await
                .unwrap();
        assert!(!versions.is_empty());
        assert!(versions[0]
            .files
            .iter()
            .any(|file| file.filename.ends_with(".jar")));
    }
}
