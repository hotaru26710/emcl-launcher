use anyhow::Result;

use super::mcim::curseforge_url;
use super::{
    ApiRouter, ModPlatform, ProjectDetails, SearchHit, SearchRequest, VersionDependency,
    VersionFile, VersionInfo, OFFICIAL_CURSEFORGE_BASE,
};

#[derive(Clone)]
pub struct CurseForgeClient {
    router: ApiRouter,
}

impl CurseForgeClient {
    pub fn new(router: ApiRouter) -> Self {
        Self { router }
    }

    pub async fn search(&self, request: &SearchRequest) -> Result<Vec<SearchHit>> {
        let mut query = vec![
            ("gameId", "432".to_string()),
            ("classId", "6".to_string()),
            ("searchFilter", request.query.clone()),
            ("gameVersion", request.game_version.clone()),
            ("index", request.offset.to_string()),
            ("pageSize", request.limit.clamp(1, 50).to_string()),
            ("sortField", "2".to_string()),
            ("sortOrder", "desc".to_string()),
        ];
        if let Some(loader) = request
            .loader
            .as_deref()
            .filter(|value| !value.trim().is_empty() && *value != "vanilla")
        {
            if let Some(loader_type) = loader_type(loader) {
                query.push(("modLoaderType", loader_type.to_string()));
            }
        }
        let response: CurseForgeResponse<Vec<SearchHitResponse>> = self
            .router
            .get_json(
                ModPlatform::CurseForge,
                &format!("{OFFICIAL_CURSEFORGE_BASE}/mods/search"),
                &curseforge_url("/mods/search"),
                &query,
            )
            .await?;
        Ok(response
            .data
            .into_iter()
            .map(|hit| SearchHit {
                platform: ModPlatform::CurseForge,
                project_id: hit.id.to_string(),
                slug: hit.slug,
                title: hit.name,
                description: hit.summary,
                icon_url: hit.logo.and_then(|logo| logo.url),
                downloads: hit.download_count,
                follows: 0,
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
                versions: hit.game_versions.unwrap_or_default(),
            })
            .collect())
    }

    pub async fn project(&self, mod_id: u64) -> Result<ProjectDetails> {
        let response: CurseForgeResponse<ProjectResponse> = self
            .router
            .get_json(
                ModPlatform::CurseForge,
                &format!("{OFFICIAL_CURSEFORGE_BASE}/mods/{mod_id}"),
                &curseforge_url(&format!("/mods/{mod_id}")),
                &[],
            )
            .await?;
        let project = response.data;
        Ok(ProjectDetails {
            platform: ModPlatform::CurseForge,
            project_id: project.id.to_string(),
            slug: project.slug,
            title: project.name,
            description: project.summary,
            body: project.description.unwrap_or_default(),
            icon_url: project.logo.and_then(|logo| logo.url),
            author: project
                .authors
                .first()
                .map(|author| author.name.clone())
                .unwrap_or_default(),
            downloads: project.download_count,
            follows: project.thumbs_up_count.unwrap_or(0),
            categories: project
                .categories
                .into_iter()
                .map(|category| category.name)
                .collect(),
            game_versions: project.game_version_latest_files.unwrap_or_default(),
            loaders: Vec::new(),
            updated_at: project.date_modified,
        })
    }

    pub async fn files(
        &self,
        mod_id: u64,
        game_version: &str,
        loader: Option<&str>,
    ) -> Result<Vec<VersionInfo>> {
        let mut query = vec![
            ("gameVersion", game_version.to_string()),
            ("pageSize", "50".to_string()),
            ("index", "0".to_string()),
        ];
        if let Some(loader) = loader.filter(|value| !value.trim().is_empty() && *value != "vanilla")
        {
            if let Some(loader_type) = loader_type(loader) {
                query.push(("modLoaderType", loader_type.to_string()));
            }
        }
        let response: CurseForgeResponse<Vec<FileResponse>> = self
            .router
            .get_json(
                ModPlatform::CurseForge,
                &format!("{OFFICIAL_CURSEFORGE_BASE}/mods/{mod_id}/files"),
                &curseforge_url(&format!("/mods/{mod_id}/files")),
                &query,
            )
            .await?;
        let mut files = response
            .data
            .into_iter()
            .map(|file| map_file(mod_id, file))
            .collect::<Vec<_>>();
        files.sort_by_key(|file| std::cmp::Reverse(file.published_at.clone()));
        Ok(files)
    }
}

fn map_file(mod_id: u64, file: FileResponse) -> VersionInfo {
    let sha1 = file
        .hashes
        .iter()
        .find(|hash| hash.algo == 1)
        .map(|hash| hash.value.clone());
    let download_url = file
        .download_url
        .as_deref()
        .map(rewrite_forgecdn_url)
        .unwrap_or_default();
    VersionInfo {
        platform: ModPlatform::CurseForge,
        project_id: mod_id.to_string(),
        version_id: file.id.to_string(),
        name: file.display_name.clone(),
        version_number: file.display_name,
        game_versions: file.game_versions,
        loaders: Vec::new(),
        release_type: match file.release_type {
            1 => "release",
            2 => "beta",
            3 => "alpha",
            _ => "release",
        }
        .to_string(),
        published_at: file.file_date,
        downloads: 0,
        files: vec![VersionFile {
            url: download_url,
            file_name: file.file_name,
            primary: true,
            size: file.file_length,
            sha1,
            sha512: None,
        }],
        dependencies: file
            .dependencies
            .into_iter()
            .filter(|dependency| dependency.relation_type == 3)
            .map(|dependency| VersionDependency {
                project_id: dependency.project_id.map(|value| value.to_string()),
                version_id: dependency.file_id.map(|value| value.to_string()),
                dependency_type: "required".to_string(),
            })
            .collect(),
    }
}

fn rewrite_forgecdn_url(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("https://edge.forgecdn.net/") {
        return format!("https://mod.mcimirror.top/{rest}");
    }
    url.to_string()
}

fn loader_type(loader: &str) -> Option<u32> {
    match loader.to_ascii_lowercase().as_str() {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}

#[derive(Debug, serde::Deserialize)]
struct CurseForgeResponse<T> {
    data: T,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchHitResponse {
    id: u64,
    slug: String,
    name: String,
    summary: String,
    logo: Option<LogoResponse>,
    download_count: u64,
    #[serde(default)]
    authors: Vec<AuthorResponse>,
    #[serde(default)]
    categories: Vec<CategoryResponse>,
    #[serde(default)]
    game_versions: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
struct LogoResponse {
    url: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct AuthorResponse {
    name: String,
}

#[derive(Debug, serde::Deserialize)]
struct CategoryResponse {
    name: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectResponse {
    id: u64,
    slug: String,
    name: String,
    summary: String,
    description: Option<String>,
    logo: Option<LogoResponse>,
    #[serde(default)]
    authors: Vec<AuthorResponse>,
    #[serde(default)]
    categories: Vec<CategoryResponse>,
    download_count: u64,
    thumbs_up_count: Option<u64>,
    date_modified: String,
    #[serde(default)]
    game_version_latest_files: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileResponse {
    id: u64,
    display_name: String,
    file_name: String,
    file_date: String,
    file_length: u64,
    download_url: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    release_type: u32,
    #[serde(default)]
    hashes: Vec<HashResponse>,
    #[serde(default)]
    dependencies: Vec<DependencyResponse>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct HashResponse {
    algo: u32,
    value: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DependencyResponse {
    project_id: Option<u64>,
    file_id: Option<u64>,
    relation_type: u32,
}

#[allow(dead_code)]
impl super::ModSourceClient for CurseForgeClient {
    fn platform(&self) -> ModPlatform {
        ModPlatform::CurseForge
    }

    fn search<'a>(
        &'a self,
        request: &'a SearchRequest,
    ) -> super::BoxFuture<'a, Result<Vec<SearchHit>>> {
        Box::pin(async move { CurseForgeClient::search(self, request).await })
    }

    fn project<'a>(&'a self, project_id: &'a str) -> super::BoxFuture<'a, Result<ProjectDetails>> {
        Box::pin(async move {
            let mod_id = project_id
                .parse::<u64>()
                .map_err(|error| anyhow::anyhow!("无效的 CurseForge mod id: {error}"))?;
            CurseForgeClient::project(self, mod_id).await
        })
    }

    fn versions<'a>(
        &'a self,
        project_id: &'a str,
        game_version: &'a str,
        loader: Option<&'a str>,
    ) -> super::BoxFuture<'a, Result<Vec<VersionInfo>>> {
        Box::pin(async move {
            let mod_id = project_id
                .parse::<u64>()
                .map_err(|error| anyhow::anyhow!("无效的 CurseForge mod id: {error}"))?;
            CurseForgeClient::files(self, mod_id, game_version, loader).await
        })
    }

    fn version<'a>(&'a self, _version_id: &'a str) -> super::BoxFuture<'a, Result<VersionInfo>> {
        Box::pin(async {
            anyhow::bail!("CurseForge 单文件详情请使用 get_mcim_curseforge_files")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mod_source::ModApiConfig;
    use reqwest::Client;

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn searches_and_loads_files_through_mcim() {
        let client = Client::builder()
            .user_agent("EmberLauncher/test")
            .build()
            .unwrap();
        let source = CurseForgeClient::new(ApiRouter::new(client, ModApiConfig::default()));
        let mut request = SearchRequest::new("jei", "1.20.1");
        request.loader = Some("forge".to_string());
        let hits = source.search(&request).await.unwrap();
        assert!(!hits.is_empty());
        let mod_id = hits[0].project_id.parse::<u64>().unwrap();
        let files = source.files(mod_id, "1.20.1", Some("forge")).await.unwrap();
        assert!(!files.is_empty());
        assert!(files[0].files[0].file_name.ends_with(".jar"));
    }
}
