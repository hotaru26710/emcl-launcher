use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::core::install::{self, ArtifactDownload, ArtifactDownloadRequest, ProgressReporter};
use crate::core::mods::{self, ModInfo};
use crate::db::models::Instance;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubRepository {
    pub id: u64,
    pub full_name: String,
    pub name: String,
    pub owner: String,
    pub description: Option<String>,
    pub html_url: String,
    pub stars: u64,
    pub forks: u64,
    pub updated_at: String,
    pub topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubRelease {
    pub id: u64,
    pub tag_name: String,
    pub name: Option<String>,
    pub html_url: String,
    pub published_at: Option<String>,
    pub prerelease: bool,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubAsset {
    pub id: u64,
    pub name: String,
    pub size: u64,
    pub download_url: String,
    pub content_type: Option<String>,
}

pub async fn search_repositories(
    client: &Client,
    query: &str,
    offset: u32,
) -> Result<Vec<GitHubRepository>> {
    let page = offset / 30 + 1;
    let search = if query.trim().is_empty() {
        "minecraft mod".to_string()
    } else {
        format!("{} minecraft mod", query.trim())
    };
    let response: GitHubSearchResponse = github_get(
        client,
        "https://api.github.com/search/repositories",
        &[
            ("q", search),
            ("sort", "stars".to_string()),
            ("order", "desc".to_string()),
            ("per_page", "30".to_string()),
            ("page", page.to_string()),
        ],
    )
    .await?;

    Ok(response
        .items
        .into_iter()
        .map(|item| GitHubRepository {
            id: item.id,
            full_name: item.full_name,
            name: item.name,
            owner: item.owner.login,
            description: item.description,
            html_url: item.html_url,
            stars: item.stargazers_count,
            forks: item.forks_count,
            updated_at: item.updated_at,
            topics: item.topics,
        })
        .collect())
}

pub async fn get_releases(client: &Client, owner: &str, repo: &str) -> Result<Vec<GitHubRelease>> {
    let releases: Vec<GitHubReleaseRaw> = github_get(
        client,
        &format!("https://api.github.com/repos/{owner}/{repo}/releases"),
        &[("per_page", "30".to_string())],
    )
    .await?;

    Ok(releases
        .into_iter()
        .map(|release| GitHubRelease {
            id: release.id,
            tag_name: release.tag_name,
            name: release.name,
            html_url: release.html_url,
            published_at: release.published_at,
            prerelease: release.prerelease,
            assets: release
                .assets
                .into_iter()
                .filter(|asset| asset.name.to_ascii_lowercase().ends_with(".jar"))
                .map(|asset| GitHubAsset {
                    id: asset.id,
                    name: asset.name,
                    size: asset.size,
                    download_url: asset.browser_download_url,
                    content_type: asset.content_type,
                })
                .collect(),
        })
        .collect())
}

pub async fn install_asset(
    client: &Client,
    instance: &Instance,
    asset: &GitHubAsset,
    reporter: &ProgressReporter,
) -> Result<Vec<ModInfo>> {
    if !asset.name.to_ascii_lowercase().ends_with(".jar") {
        bail!("GitHub Release 资产不是 .jar");
    }
    if !asset.download_url.starts_with("https://github.com/")
        && !asset
            .download_url
            .starts_with("https://objects.githubusercontent.com/")
    {
        bail!("GitHub 下载地址不安全");
    }
    let destination = std::path::PathBuf::from(&instance.directory)
        .join("mods")
        .join(safe_file_name(&asset.name)?);
    install::download_artifacts(ArtifactDownloadRequest {
        client,
        mirror: "official",
        concurrency: 1,
        force_redownload: false,
        stage: "github-release",
        message: "正在下载 GitHub Release",
        artifacts: vec![ArtifactDownload {
            url: asset.download_url.clone(),
            destination,
            sha1: None,
            size: Some(asset.size),
            label: asset.name.clone(),
            version_id: instance.version_id.clone(),
        }],
        reporter,
    })
    .await?;
    mods::list_mods(
        &crate::core::paths::LauncherPaths::from_root(std::path::PathBuf::from(".")),
        instance,
    )
}

async fn github_get<T: for<'de> Deserialize<'de>>(
    client: &Client,
    url: &str,
    query: &[(&str, String)],
) -> Result<T> {
    let response = client
        .get(url)
        .query(query)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "EMCL/0.1.0")
        .send()
        .await
        .context("GitHub 请求失败")?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        bail!(
            "GitHub 返回 {status}: {}",
            text.chars().take(300).collect::<String>()
        );
    }
    serde_json::from_str(&text).context("无法解析 GitHub 响应")
}

fn safe_file_name(file_name: &str) -> Result<String> {
    let path = std::path::Path::new(file_name);
    if path.is_absolute() || path.components().count() != 1 {
        bail!("GitHub Release 文件名不安全");
    }
    Ok(file_name.to_string())
}

#[derive(Debug, Deserialize)]
struct GitHubSearchResponse {
    items: Vec<GitHubRepositoryRaw>,
}

#[derive(Debug, Deserialize)]
struct GitHubRepositoryRaw {
    id: u64,
    name: String,
    full_name: String,
    owner: GitHubOwner,
    description: Option<String>,
    html_url: String,
    stargazers_count: u64,
    forks_count: u64,
    updated_at: String,
    #[serde(default)]
    topics: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubOwner {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseRaw {
    id: u64,
    tag_name: String,
    name: Option<String>,
    html_url: String,
    published_at: Option<String>,
    prerelease: bool,
    assets: Vec<GitHubAssetRaw>,
}

#[derive(Debug, Deserialize)]
struct GitHubAssetRaw {
    id: u64,
    name: String,
    size: u64,
    browser_download_url: String,
    content_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn searches_github_and_reads_releases() {
        let client = Client::builder()
            .user_agent("EMCL-GitHub-test/0.1.0")
            .build()
            .unwrap();
        let repos = search_repositories(&client, "sodium", 0).await.unwrap();
        assert!(!repos.is_empty());
        let repo = repos
            .iter()
            .find(|repo| {
                repo.full_name
                    .eq_ignore_ascii_case("CaffeineMC/sodium-fabric")
            })
            .unwrap_or(&repos[0]);
        let _ = get_releases(&client, &repo.owner, &repo.name)
            .await
            .unwrap();
    }
}
