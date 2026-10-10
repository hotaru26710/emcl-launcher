use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::core::mod_source::{ModPlatform, SearchHit, SearchRequest};

pub const SEARCH_CACHE_TTL_MINUTES: i64 = 10;

pub fn query_key(platform: ModPlatform, request: &SearchRequest) -> String {
    let mut hasher = Sha256::new();
    hasher.update(
        format!(
            "{}|{}|{}|{}|{}",
            platform.as_str(),
            request.query.trim().to_ascii_lowercase(),
            request.game_version.trim(),
            request.loader.as_deref().unwrap_or(""),
            request.offset
        )
        .as_bytes(),
    );
    format!("{:x}", hasher.finalize())
}

pub fn read_cache<T: DeserializeOwned>(
    connection: &Connection,
    platform: ModPlatform,
    key: &str,
) -> Result<Option<T>> {
    let row = connection
        .query_row(
            "SELECT response_json, cached_at FROM search_cache
             WHERE query_hash = ?1 AND platform = ?2",
            params![key, platform.as_str()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((json, cached_at)) = row else {
        return Ok(None);
    };
    let Ok(time) = DateTime::parse_from_rfc3339(&cached_at) else {
        return Ok(None);
    };
    if Utc::now()
        .signed_duration_since(time.with_timezone(&Utc))
        .num_minutes()
        > SEARCH_CACHE_TTL_MINUTES
    {
        return Ok(None);
    }
    Ok(serde_json::from_str(&json).ok())
}

pub fn write_cache<T: Serialize>(
    connection: &Connection,
    platform: ModPlatform,
    key: &str,
    value: &T,
) -> Result<()> {
    let json = serde_json::to_string(value)?;
    connection.execute(
        "INSERT INTO search_cache (query_hash, platform, response_json, cached_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(query_hash, platform) DO UPDATE SET
            response_json = excluded.response_json,
            cached_at = excluded.cached_at",
        params![key, platform.as_str(), json, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn rank_hits(mut hits: Vec<SearchHit>) -> Vec<SearchHit> {
    hits.sort_by_key(|hit| {
        std::cmp::Reverse(hit.downloads.saturating_add(hit.follows.saturating_mul(20)))
    });
    hits
}

pub fn cache_project(
    connection: &Connection,
    project: &crate::core::mod_source::ProjectDetails,
) -> Result<()> {
    connection.execute(
        "INSERT INTO mod_sources (
            platform, project_id, slug, title, description, icon_url, author,
            downloads, categories_json, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(platform, project_id) DO UPDATE SET
            slug = excluded.slug,
            title = excluded.title,
            description = excluded.description,
            icon_url = excluded.icon_url,
            author = excluded.author,
            downloads = excluded.downloads,
            categories_json = excluded.categories_json,
            updated_at = excluded.updated_at",
        params![
            project.platform.as_str(),
            project.project_id,
            project.slug,
            project.title,
            project.description,
            project.icon_url,
            project.author,
            project.downloads as i64,
            serde_json::to_string(&project.categories)?,
            if project.updated_at.is_empty() {
                Utc::now().to_rfc3339()
            } else {
                project.updated_at.clone()
            },
        ],
    )?;
    Ok(())
}

pub fn cache_versions(
    connection: &Connection,
    versions: &[crate::core::mod_source::VersionInfo],
) -> Result<()> {
    for version in versions {
        let file = version
            .files
            .iter()
            .find(|file| file.primary)
            .or_else(|| version.files.first());
        connection.execute(
            "INSERT INTO mod_versions (
                platform, project_id, version_id, version_number,
                game_versions_json, loaders_json, download_url, file_name,
                file_size, sha1, dependencies_json, release_type, published_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(platform, project_id, version_id) DO UPDATE SET
                version_number = excluded.version_number,
                game_versions_json = excluded.game_versions_json,
                loaders_json = excluded.loaders_json,
                download_url = excluded.download_url,
                file_name = excluded.file_name,
                file_size = excluded.file_size,
                sha1 = excluded.sha1,
                dependencies_json = excluded.dependencies_json,
                release_type = excluded.release_type,
                published_at = excluded.published_at",
            params![
                version.platform.as_str(),
                version.project_id,
                version.version_id,
                version.version_number,
                serde_json::to_string(&version.game_versions)?,
                serde_json::to_string(&version.loaders)?,
                file.map(|file| file.url.as_str()).unwrap_or(""),
                file.map(|file| file.file_name.as_str()).unwrap_or(""),
                file.map(|file| file.size as i64).unwrap_or(0),
                file.and_then(|file| file.sha1.as_deref()),
                serde_json::to_string(&version.dependencies)?,
                version.release_type,
                version.published_at,
            ],
        )?;
    }
    Ok(())
}
