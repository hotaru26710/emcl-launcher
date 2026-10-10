use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

pub mod curseforge;
pub mod mcim;
pub mod modrinth;

pub const OFFICIAL_MODRINTH_BASE: &str = "https://api.modrinth.com/v2";
pub const MCIM_MODRINTH_BASE: &str = "https://mod.mcimirror.top/modrinth/v2";
pub const OFFICIAL_CURSEFORGE_BASE: &str = "https://api.curseforge.com/v1";
pub const MCIM_CURSEFORGE_BASE: &str = "https://mod.mcimirror.top/curseforge/v1";
pub const SYNC_MAX_AGE_DAYS: i64 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModPlatform {
    Modrinth,
    CurseForge,
}

impl ModPlatform {}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[allow(dead_code)]
pub trait ModSourceClient: Send + Sync {
    fn platform(&self) -> ModPlatform;

    fn search<'a>(&'a self, request: &'a SearchRequest) -> BoxFuture<'a, Result<Vec<SearchHit>>>;

    fn project<'a>(&'a self, project_id: &'a str) -> BoxFuture<'a, Result<ProjectDetails>>;

    fn versions<'a>(
        &'a self,
        project_id: &'a str,
        game_version: &'a str,
        loader: Option<&'a str>,
    ) -> BoxFuture<'a, Result<Vec<VersionInfo>>>;

    fn version<'a>(&'a self, version_id: &'a str) -> BoxFuture<'a, Result<VersionInfo>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
    pub game_version: String,
    pub loader: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

impl SearchRequest {
    pub fn new(query: impl Into<String>, game_version: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            game_version: game_version.into(),
            loader: None,
            offset: 0,
            limit: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub platform: ModPlatform,
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
pub struct ProjectDetails {
    pub platform: ModPlatform,
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub icon_url: Option<String>,
    pub author: String,
    pub downloads: u64,
    pub follows: u64,
    pub categories: Vec<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub platform: ModPlatform,
    pub project_id: String,
    pub version_id: String,
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub release_type: String,
    pub published_at: String,
    pub downloads: u64,
    pub files: Vec<VersionFile>,
    pub dependencies: Vec<VersionDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionFile {
    pub url: String,
    pub file_name: String,
    pub primary: bool,
    pub size: u64,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDependency {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub dependency_type: String,
}

#[derive(Debug, Clone, Default)]
pub struct ModApiConfig {
    pub force_official: bool,
    pub curseforge_api_key: Option<String>,
}

#[derive(Clone)]
pub struct ApiRouter {
    client: Client,
    config: ModApiConfig,
}

impl ApiRouter {
    pub fn new(client: Client, config: ModApiConfig) -> Self {
        Self { client, config }
    }

    pub async fn get_json<T: DeserializeOwned>(
        &self,
        platform: ModPlatform,
        official_url: &str,
        mirror_url: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        if self.config.force_official {
            let (value, _) = self
                .request_json(platform, official_url, query, true)
                .await?;
            return Ok(value);
        }

        match self.request_json(platform, mirror_url, query, false).await {
            Ok((value, fresh)) if fresh => Ok(value),
            Ok((mirror_value, _)) => {
                match self.request_json(platform, official_url, query, true).await {
                    Ok((official_value, _)) => Ok(official_value),
                    Err(official_error) => {
                        eprintln!("official API fallback failed, using stale MCIM response: {official_error}");
                        Ok(mirror_value)
                    }
                }
            }
            Err(mirror_error) => {
                let (value, _) = self
                    .request_json(platform, official_url, query, true)
                    .await
                    .with_context(|| {
                        format!("MCIM 请求失败，官方 API 也无法使用：{mirror_error}")
                    })?;
                Ok(value)
            }
        }
    }

    #[allow(dead_code)]
    pub async fn post_json<T: DeserializeOwned, B: Serialize + ?Sized>(
        &self,
        platform: ModPlatform,
        official_url: &str,
        body: &B,
    ) -> Result<T> {
        let mut request = self.client.post(official_url).json(body);
        if platform == ModPlatform::CurseForge {
            request = self.with_curseforge_key(request, true)?;
        }
        let response = request.send().await?;
        decode_json(response).await
    }

    async fn request_json<T: DeserializeOwned>(
        &self,
        platform: ModPlatform,
        url: &str,
        query: &[(&str, String)],
        official: bool,
    ) -> Result<(T, bool)> {
        let mut request = self.client.get(url).query(query);
        if official && platform == ModPlatform::CurseForge {
            request = self.with_curseforge_key(request, true)?;
        }
        let response = request.send().await?;
        let fresh = if official {
            true
        } else {
            sync_is_fresh(&response)
        };
        Ok((decode_json(response).await?, fresh))
    }

    fn with_curseforge_key(
        &self,
        request: reqwest::RequestBuilder,
        required: bool,
    ) -> Result<reqwest::RequestBuilder> {
        let Some(key) = self
            .config
            .curseforge_api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            if required {
                bail!("CurseForge 官方 API 需要配置 API Key");
            }
            return Ok(request);
        };
        Ok(request.header("x-api-key", key))
    }
}

async fn decode_json<T: DeserializeOwned>(response: Response) -> Result<T> {
    let status = response.status();
    let bytes = response.bytes().await?;
    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes);
        bail!(
            "API 请求失败 ({status}): {}",
            body.chars().take(300).collect::<String>()
        );
    }
    serde_json::from_slice(&bytes).context("API 返回了无法解析的 JSON")
}

fn sync_is_fresh(response: &Response) -> bool {
    let Some(raw) = ["sync_at", "sync-at", "x-sync-at"]
        .iter()
        .find_map(|key| response.headers().get(*key))
        .and_then(|value| value.to_str().ok())
    else {
        return true;
    };

    if let Ok(timestamp) = raw.parse::<i64>() {
        let seconds = if timestamp > 10_000_000_000 {
            timestamp / 1000
        } else {
            timestamp
        };
        let synced = DateTime::<Utc>::from_timestamp(seconds, 0);
        return synced
            .map(|time| Utc::now().signed_duration_since(time).num_days() <= SYNC_MAX_AGE_DAYS)
            .unwrap_or(true);
    }
    if let Ok(time) = DateTime::parse_from_rfc3339(raw) {
        return Utc::now()
            .signed_duration_since(time.with_timezone(&Utc))
            .num_days()
            <= SYNC_MAX_AGE_DAYS;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn falls_back_to_official_modrinth_when_mirror_fails() {
        let client = Client::builder()
            .user_agent("EmberLauncher/test")
            .build()
            .unwrap();
        let router = ApiRouter::new(client, ModApiConfig::default());
        let project: serde_json::Value = router
            .get_json(
                ModPlatform::Modrinth,
                &format!("{OFFICIAL_MODRINTH_BASE}/project/sodium"),
                "http://127.0.0.1:9/modrinth/v2/project/sodium",
                &[],
            )
            .await
            .unwrap();
        assert_eq!(project["slug"], "sodium");
    }
}
