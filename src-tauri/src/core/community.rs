use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::conflict::ConflictFinding;
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DependencyRepairResult {
    pub installed: Vec<String>,
    pub failed: Vec<String>,
    pub remaining: Vec<ConflictFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CommunityInstallResult {
    pub installed: usize,
    pub skipped: usize,
    pub dependencies: Vec<String>,
    pub mods: Vec<ModInfo>,
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
    pub recommended: bool,
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

    let mut versions = versions.into_iter().collect::<Vec<_>>();
    versions.sort_by_key(|version| std::cmp::Reverse(version_score(version)));
    Ok(versions
        .into_iter()
        .enumerate()
        .map(|(index, version)| version_info(version, index == 0))
        .collect())
}

fn version_score(version: &ModrinthVersion) -> (u8, u64, String) {
    (
        match version.version_type.as_str() {
            "release" => 3,
            "beta" => 2,
            "alpha" => 1,
            _ => 0,
        },
        version.downloads,
        version.date_published.clone(),
    )
}

pub async fn install_modrinth_version(
    client: &Client,
    paths: &LauncherPaths,
    instance: &Instance,
    version_id: &str,
    reporter: &ProgressReporter,
) -> Result<CommunityInstallResult> {
    let mut visited = HashSet::new();
    let mut result = install_version_recursive(
        client,
        paths,
        instance,
        version_id,
        reporter,
        0,
        &mut visited,
    )
    .await?;
    result.mods = mods::list_mods(paths, instance)?;
    Ok(result)
}

pub async fn repair_missing_dependencies(
    client: &Client,
    paths: &LauncherPaths,
    instance: &Instance,
    findings: &[ConflictFinding],
    reporter: &ProgressReporter,
) -> Result<DependencyRepairResult> {
    let mut targets = findings
        .iter()
        .filter(|finding| finding.conflict_type == "missing_dependency")
        .filter_map(|finding| finding.target_mod_id.clone())
        .collect::<Vec<_>>();
    targets.sort();
    targets.dedup();

    let mut result = DependencyRepairResult::default();
    for target in targets {
        match find_and_install_dependency(client, paths, instance, reporter, &target).await {
            Ok(project_title) => result.installed.push(project_title),
            Err(error) => result.failed.push(format!("{target}: {error}")),
        }
    }

    Ok(result)
}

async fn find_and_install_dependency(
    client: &Client,
    paths: &LauncherPaths,
    instance: &Instance,
    reporter: &ProgressReporter,
    target: &str,
) -> Result<String> {
    let normalized = normalize_mod_id(target);
    let results = search_modrinth(
        client,
        target,
        &instance.version_id,
        instance.loader.as_deref(),
        0,
    )
    .await?;
    let project = results
        .into_iter()
        .find(|project| {
            normalize_mod_id(&project.slug) == normalized
                || normalize_mod_id(&project.title) == normalized
                || normalize_mod_id(&project.project_id) == normalized
        })
        .with_context(|| format!("Modrinth 没有找到匹配项目：{target}"))?;
    let versions = get_modrinth_versions(
        client,
        &project.project_id,
        &instance.version_id,
        instance.loader.as_deref(),
    )
    .await?;
    let version = versions
        .first()
        .with_context(|| format!("Modrinth 项目 {} 没有兼容版本", project.title))?;
    install_modrinth_version(client, paths, instance, &version.id, reporter).await?;
    Ok(project.title)
}

fn normalize_mod_id(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

async fn install_version_recursive(
    client: &Client,
    paths: &LauncherPaths,
    instance: &Instance,
    version_id: &str,
    reporter: &ProgressReporter,
    depth: usize,
    visited: &mut HashSet<String>,
) -> Result<CommunityInstallResult> {
    if depth > 8 {
        bail!("Modrinth 依赖层级过深");
    }
    if !visited.insert(version_id.to_string()) {
        return Ok(CommunityInstallResult::default());
    }

    let version: ModrinthVersion =
        api_get(client, &format!("{MODRINTH_API}/version/{version_id}"), &[]).await?;
    let file = version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .context("Modrinth 版本没有可下载文件")?;
    let mut result = CommunityInstallResult::default();
    let skipped = install_mod_file(client, paths, instance, file, reporter).await?;
    if skipped {
        result.skipped += 1;
    } else {
        result.installed += 1;
    }
    result.dependencies.push(file.filename.clone());

    for dependency in version
        .dependencies
        .iter()
        .filter(|dependency| dependency.dependency_type == "required")
    {
        if let Some(version_id) = dependency.version_id.as_deref() {
            let child = Box::pin(install_version_recursive(
                client,
                paths,
                instance,
                version_id,
                reporter,
                depth + 1,
                visited,
            ))
            .await?;
            merge_install_result(&mut result, child);
        } else if let Some(project_id) = dependency.project_id.as_deref() {
            let versions = get_modrinth_versions(
                client,
                project_id,
                &instance.version_id,
                instance.loader.as_deref(),
            )
            .await?;
            if let Some(version) = versions.first() {
                let child = Box::pin(install_version_recursive(
                    client,
                    paths,
                    instance,
                    &version.id,
                    reporter,
                    depth + 1,
                    visited,
                ))
                .await?;
                merge_install_result(&mut result, child);
            }
        }
    }

    Ok(result)
}

fn merge_install_result(target: &mut CommunityInstallResult, incoming: CommunityInstallResult) {
    target.installed += incoming.installed;
    target.skipped += incoming.skipped;
    target.dependencies.extend(incoming.dependencies);
}

async fn install_mod_file(
    client: &Client,
    _paths: &LauncherPaths,
    instance: &Instance,
    file: &ModrinthFile,
    reporter: &ProgressReporter,
) -> Result<bool> {
    if !file.filename.to_ascii_lowercase().ends_with(".jar") {
        bail!("社区模组文件不是 .jar: {}", file.filename);
    }
    let destination = safe_mod_path(
        &PathBuf::from(&instance.directory).join("mods"),
        &file.filename,
    )?;
    if install::artifact_file_is_valid(&destination, file.hashes.sha1.as_deref(), Some(file.size))
        .await?
    {
        return Ok(true);
    }
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
    .await?;
    Ok(false)
}

fn version_info(version: ModrinthVersion, recommended: bool) -> ModrinthVersionInfo {
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
        recommended,
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
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/124 Safari/537.36",
        )
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
