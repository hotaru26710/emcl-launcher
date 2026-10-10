use anyhow::Result;
use serde::Deserialize;
use serde_json::json;

use super::mcim::modrinth_url;
use super::{
    ApiRouter, ModPlatform, ProjectDetails, SearchHit, SearchRequest, VersionDependency,
    VersionFile, VersionInfo, OFFICIAL_MODRINTH_BASE,
};

#[derive(Clone)]
pub struct ModrinthClient {
    router: ApiRouter,
}

impl ModrinthClient {
    pub fn new(router: ApiRouter) -> Self {
        Self { router }
    }

    pub async fn search(&self, request: &SearchRequest) -> Result<Vec<SearchHit>> {
        let mut facets = vec![json!(["project_type:mod"])];
        if !request.game_version.trim().is_empty() {
            facets.push(json!([format!("versions:{}", request.game_version)]));
        }
        if let Some(loader) = request
            .loader
            .as_deref()
            .filter(|value| !value.trim().is_empty() && *value != "vanilla")
        {
            facets.push(json!([format!("categories:{}", normalize_loader(loader))]));
        }
        let query = [
            ("query", request.query.clone()),
            ("limit", request.limit.clamp(1, 100).to_string()),
            ("offset", request.offset.to_string()),
            ("facets", serde_json::to_string(&facets)?),
        ];
        let response: SearchResponse = self
            .router
            .get_json(
                ModPlatform::Modrinth,
                &format!("{OFFICIAL_MODRINTH_BASE}/search"),
                &modrinth_url("/search"),
                &query,
            )
            .await?;
        Ok(response
            .hits
            .into_iter()
            .map(|hit| SearchHit {
                platform: ModPlatform::Modrinth,
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

    pub async fn project(&self, project_id: &str) -> Result<ProjectDetails> {
        let project: ProjectResponse = self
            .router
            .get_json(
                ModPlatform::Modrinth,
                &format!("{OFFICIAL_MODRINTH_BASE}/project/{project_id}"),
                &modrinth_url(&format!("/project/{project_id}")),
                &[],
            )
            .await?;
        Ok(ProjectDetails {
            platform: ModPlatform::Modrinth,
            project_id: project.id,
            slug: project.slug,
            title: project.title,
            description: project.description,
            body: project.body,
            icon_url: project.icon_url,
            author: project.team,
            downloads: project.downloads,
            follows: project.followers,
            categories: project.categories,
            game_versions: project.game_versions,
            loaders: project.loaders,
            updated_at: project.updated,
        })
    }

    pub async fn versions(
        &self,
        project_id: &str,
        game_version: &str,
        loader: Option<&str>,
    ) -> Result<Vec<VersionInfo>> {
        let mut query = vec![("game_versions", serde_json::to_string(&vec![game_version])?)];
        if let Some(loader) = loader.filter(|value| !value.trim().is_empty() && *value != "vanilla")
        {
            query.push((
                "loaders",
                serde_json::to_string(&vec![normalize_loader(loader)])?,
            ));
        }
        let versions: Vec<VersionResponse> = self
            .router
            .get_json(
                ModPlatform::Modrinth,
                &format!("{OFFICIAL_MODRINTH_BASE}/project/{project_id}/version"),
                &modrinth_url(&format!("/project/{project_id}/version")),
                &query,
            )
            .await?;
        let mut versions = versions
            .into_iter()
            .map(map_version)
            .collect::<Result<Vec<_>>>()?;
        versions.sort_by_key(|version| std::cmp::Reverse(version.published_at.clone()));
        Ok(versions)
    }

    pub async fn version(&self, version_id: &str) -> Result<VersionInfo> {
        let version: VersionResponse = self
            .router
            .get_json(
                ModPlatform::Modrinth,
                &format!("{OFFICIAL_MODRINTH_BASE}/version/{version_id}"),
                &modrinth_url(&format!("/version/{version_id}")),
                &[],
            )
            .await?;
        map_version(version)
    }
}

fn map_version(version: VersionResponse) -> Result<VersionInfo> {
    let files = version
        .files
        .into_iter()
        .map(|file| VersionFile {
            url: file.url,
            file_name: file.filename,
            primary: file.primary,
            size: file.size,
            sha1: file.hashes.sha1,
            sha512: file.hashes.sha512,
        })
        .collect();
    Ok(VersionInfo {
        platform: ModPlatform::Modrinth,
        project_id: version.project_id,
        version_id: version.id,
        name: version.name,
        version_number: version.version_number,
        game_versions: version.game_versions,
        loaders: version.loaders,
        release_type: version.version_type,
        published_at: version.date_published,
        downloads: version.downloads,
        files,
        dependencies: version
            .dependencies
            .into_iter()
            .map(|dependency| VersionDependency {
                project_id: dependency.project_id,
                version_id: dependency.version_id,
                dependency_type: dependency.dependency_type,
            })
            .collect(),
    })
}

fn normalize_loader(loader: &str) -> String {
    match loader.to_ascii_lowercase().as_str() {
        "neoforge" => "neoforge".to_string(),
        "forge" => "forge".to_string(),
        "fabric" => "fabric".to_string(),
        "quilt" => "quilt".to_string(),
        other => other.to_string(),
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: Vec<SearchHitResponse>,
}

#[derive(Debug, Deserialize)]
struct SearchHitResponse {
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
struct ProjectResponse {
    id: String,
    slug: String,
    title: String,
    description: String,
    #[serde(default)]
    body: String,
    icon_url: Option<String>,
    #[serde(default)]
    team: String,
    downloads: u64,
    #[serde(default)]
    followers: u64,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    #[serde(default)]
    updated: String,
}

#[derive(Debug, Deserialize)]
struct VersionResponse {
    id: String,
    project_id: String,
    name: String,
    version_number: String,
    version_type: String,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    date_published: String,
    downloads: u64,
    #[serde(default)]
    files: Vec<FileResponse>,
    #[serde(default)]
    dependencies: Vec<DependencyResponse>,
}

#[derive(Debug, Deserialize)]
struct FileResponse {
    url: String,
    filename: String,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    size: u64,
    hashes: HashResponse,
}

#[derive(Debug, Deserialize)]
struct HashResponse {
    sha1: Option<String>,
    sha512: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DependencyResponse {
    project_id: Option<String>,
    version_id: Option<String>,
    dependency_type: String,
}

#[allow(dead_code)]
impl super::ModSourceClient for ModrinthClient {
    fn platform(&self) -> ModPlatform {
        ModPlatform::Modrinth
    }

    fn search<'a>(
        &'a self,
        request: &'a SearchRequest,
    ) -> super::BoxFuture<'a, Result<Vec<SearchHit>>> {
        Box::pin(async move { ModrinthClient::search(self, request).await })
    }

    fn project<'a>(&'a self, project_id: &'a str) -> super::BoxFuture<'a, Result<ProjectDetails>> {
        Box::pin(async move { ModrinthClient::project(self, project_id).await })
    }

    fn versions<'a>(
        &'a self,
        project_id: &'a str,
        game_version: &'a str,
        loader: Option<&'a str>,
    ) -> super::BoxFuture<'a, Result<Vec<VersionInfo>>> {
        Box::pin(
            async move { ModrinthClient::versions(self, project_id, game_version, loader).await },
        )
    }

    fn version<'a>(&'a self, version_id: &'a str) -> super::BoxFuture<'a, Result<VersionInfo>> {
        Box::pin(async move { ModrinthClient::version(self, version_id).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mod_source::ModApiConfig;
    use reqwest::Client;

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn searches_and_loads_versions_through_mcim() {
        let client = Client::builder()
            .user_agent("EmberLauncher/test")
            .build()
            .unwrap();
        let source = ModrinthClient::new(ApiRouter::new(client, ModApiConfig::default()));
        let mut request = SearchRequest::new("sodium", "1.21.1");
        request.loader = Some("fabric".to_string());
        let hits = source.search(&request).await.unwrap();
        assert!(!hits.is_empty());
        let versions = source
            .versions(&hits[0].project_id, "1.21.1", Some("fabric"))
            .await
            .unwrap();
        assert!(!versions.is_empty());
        assert!(versions[0]
            .files
            .iter()
            .any(|file| file.url.starts_with("http")));
    }
}
