use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::fs;

use crate::core::paths::{is_safe_segment, LauncherPaths};

const OFFICIAL_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const BMCL_MANIFEST_URL: &str = "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json";
const MANIFEST_CACHE_AGE: Duration = Duration::from_secs(15 * 60);
const VERSION_CACHE_AGE: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub id: String,
    #[serde(rename(deserialize = "type", serialize = "versionType"))]
    pub version_type: String,
    pub url: String,
    pub time: String,
    pub release_time: String,
    pub sha1: Option<String>,
    pub compliance_level: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDetails {
    pub id: String,
    #[serde(rename(deserialize = "type", serialize = "versionType"))]
    pub version_type: String,
    pub time: String,
    pub release_time: String,
    pub main_class: Option<String>,
    pub assets: Option<String>,
    pub asset_index: Option<AssetIndex>,
    pub downloads: VersionDownloads,
    #[serde(default)]
    pub libraries: Vec<Library>,
    pub java_version: Option<JavaVersion>,
    pub logging: Option<Logging>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDownloads {
    pub client: Option<DownloadArtifact>,
    pub server: Option<DownloadArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub total_size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadArtifact {
    pub path: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub name: String,
    pub downloads: LibraryDownloads,
    pub natives: Option<HashMap<String, String>>,
    pub rules: Option<Vec<Rule>>,
    pub extract: Option<LibraryExtract>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDownloads {
    pub artifact: Option<DownloadArtifact>,
    pub classifiers: Option<HashMap<String, DownloadArtifact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryExtract {
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    pub major_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logging {
    pub client: Option<LoggingClient>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingClient {
    pub argument: String,
    pub file: DownloadArtifact,
    #[serde(rename = "type")]
    pub log_type: String,
}

pub async fn fetch_version_manifest(
    client: &Client,
    paths: &LauncherPaths,
    mirror: &str,
    force_refresh: bool,
) -> Result<VersionManifest> {
    let cache_path = paths.shared_dir().join("meta/version_manifest_v2.json");

    if !force_refresh {
        if let Some(manifest) = read_fresh_json(&cache_path, MANIFEST_CACHE_AGE).await {
            return Ok(manifest);
        }
    }

    let mut urls = Vec::new();
    if mirror == "bmclapi" {
        urls.push(BMCL_MANIFEST_URL);
    }
    urls.push(OFFICIAL_MANIFEST_URL);

    let mut last_error = None;
    for url in urls {
        match fetch_json::<VersionManifest>(client, url).await {
            Ok((manifest, bytes)) => {
                write_bytes_atomic(&cache_path, &bytes).await?;
                return Ok(manifest);
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("no version manifest source is available")))
}

pub async fn fetch_version_details(
    client: &Client,
    paths: &LauncherPaths,
    version_id: &str,
    mirror: &str,
) -> Result<VersionDetails> {
    if !is_safe_segment(version_id) {
        anyhow::bail!("invalid version id");
    }

    let cache_path = paths.version_dir(version_id).join("version.json");

    if let Some(details) = read_fresh_json(&cache_path, VERSION_CACHE_AGE).await {
        return Ok(details);
    }

    let manifest = fetch_version_manifest(client, paths, mirror, false).await?;
    let summary = manifest
        .versions
        .iter()
        .find(|version| version.id == version_id)
        .with_context(|| format!("version {version_id} was not found in the manifest"))?;

    let mut urls = Vec::new();
    if mirror == "bmclapi" {
        urls.push(format!(
            "https://bmclapi2.bangbang93.com/version/{version_id}/json"
        ));
    }
    urls.push(summary.url.clone());

    let mut last_error = None;
    for url in urls {
        match fetch_json::<VersionDetails>(client, &url).await {
            Ok((details, bytes)) if details.id == version_id => {
                write_bytes_atomic(&cache_path, &bytes).await?;
                return Ok(details);
            }
            Ok((details, _)) => {
                last_error = Some(anyhow::anyhow!(
                    "metadata id {} did not match requested version {}",
                    details.id,
                    version_id
                ));
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error
        .unwrap_or_else(|| anyhow::anyhow!("version metadata for {version_id} is unavailable")))
}

async fn fetch_json<T>(client: &Client, url: &str) -> Result<(T, Vec<u8>)>
where
    T: DeserializeOwned,
{
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to request {url}"))?
        .error_for_status()
        .with_context(|| format!("request failed for {url}"))?;
    let bytes = response.bytes().await?.to_vec();
    let parsed = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse JSON from {url}"))?;
    Ok((parsed, bytes))
}

async fn read_fresh_json<T>(path: &Path, max_age: Duration) -> Option<T>
where
    T: DeserializeOwned,
{
    let metadata = fs::metadata(path).await.ok()?;
    let modified = metadata.modified().ok()?;
    let age = SystemTime::now().duration_since(modified).ok()?;
    if age > max_age {
        return None;
    }

    let bytes = fs::read(path).await.ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub async fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("cache path has no valid file name")?;
    let temporary = path.with_file_name(format!(".{file_name}.tmp"));
    fs::write(&temporary, bytes).await.with_context(|| {
        format!(
            "failed to write temporary cache file {}",
            temporary.display()
        )
    })?;

    if fs::try_exists(path).await.unwrap_or(false) {
        fs::remove_file(path).await?;
    }
    fs::rename(&temporary, path)
        .await
        .with_context(|| format!("failed to publish cache file {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_mojang_type_as_version_type() {
        let version: VersionSummary = serde_json::from_value(serde_json::json!({
            "id": "1.21.1",
            "type": "release",
            "url": "https://example.invalid/1.21.1.json",
            "time": "2024-08-08T14:05:17+00:00",
            "releaseTime": "2024-08-08T14:05:17+00:00",
            "sha1": "abc",
            "complianceLevel": 1
        }))
        .unwrap();
        let serialized = serde_json::to_value(version).unwrap();
        assert_eq!(serialized["versionType"], "release");
        assert!(serialized.get("type").is_none());
    }

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn parses_1_21_1_metadata() {
        let client = Client::builder()
            .user_agent("EMCL-Launcher-test/0.1.0")
            .build()
            .expect("build HTTP client");
        let root = std::env::temp_dir().join(format!("emcl-version-test-{}", uuid::Uuid::new_v4()));
        let paths = LauncherPaths::from_root(root.clone());
        paths.ensure().expect("create temporary launcher paths");

        let details = fetch_version_details(&client, &paths, "1.21.1", "official")
            .await
            .expect("fetch 1.21.1 metadata");

        assert_eq!(details.id, "1.21.1");
        assert!(details.downloads.client.is_some());
        assert!(details.libraries.len() > 10);
        assert!(details.asset_index.is_some());

        let _ = std::fs::remove_dir_all(root);
    }
}
