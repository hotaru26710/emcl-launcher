use std::path::{Component, Path, PathBuf};
use std::process::Stdio;

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::fs;
use tokio::process::Command;
use walkdir::WalkDir;

use crate::core::install::{self, ArtifactDownload, ArtifactDownloadRequest, ProgressReporter};
use crate::core::paths::{is_safe_segment, LauncherPaths};
use crate::core::version::VersionDetails;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LoaderType {
    Fabric,
    Quilt,
    Neoforge,
    Forge,
}

impl LoaderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fabric => "fabric",
            Self::Quilt => "quilt",
            Self::Neoforge => "neoforge",
            Self::Forge => "forge",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderInstallResult {
    pub version_id: String,
    pub loader: String,
    pub loader_version: String,
    pub profile_path: String,
}

pub struct LoaderInstallRequest<'a> {
    pub client: &'a Client,
    pub paths: &'a LauncherPaths,
    pub base_details: &'a VersionDetails,
    pub loader: LoaderType,
    pub requested_version: Option<String>,
    pub java_path: PathBuf,
    pub mirror: &'a str,
    pub concurrency: u32,
    pub reporter: &'a ProgressReporter,
}

pub async fn install_loader(request: LoaderInstallRequest<'_>) -> Result<LoaderInstallResult> {
    let loader_version = match request.loader {
        LoaderType::Fabric => {
            resolve_fabric_version(
                request.client,
                &request.base_details.id,
                request.requested_version.as_deref(),
            )
            .await?
        }
        LoaderType::Quilt => {
            resolve_quilt_version(
                request.client,
                &request.base_details.id,
                request.requested_version.as_deref(),
            )
            .await?
        }
        LoaderType::Neoforge => {
            resolve_neoforge_version(
                request.client,
                &request.base_details.id,
                request.requested_version.as_deref(),
            )
            .await?
        }
        LoaderType::Forge => {
            resolve_forge_version(
                request.client,
                &request.base_details.id,
                request.requested_version.as_deref(),
            )
            .await?
        }
    };

    let version_id =
        effective_version_id(&request.base_details.id, &request.loader, &loader_version)?;
    let version_dir = request.paths.version_dir(&version_id);
    fs::create_dir_all(&version_dir).await?;

    match request.loader {
        LoaderType::Fabric => {
            install_profile_loader(
                request.client,
                request.paths,
                request.reporter,
                request.mirror,
                request.concurrency,
                request.base_details,
                &version_id,
                &format!(
                    "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
                    request.base_details.id, loader_version
                ),
            )
            .await?;
        }
        LoaderType::Quilt => {
            install_profile_loader(
                request.client,
                request.paths,
                request.reporter,
                request.mirror,
                request.concurrency,
                request.base_details,
                &version_id,
                &format!(
                    "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
                    request.base_details.id, loader_version
                ),
            )
            .await?;
        }
        LoaderType::Neoforge | LoaderType::Forge => {
            install_installer_loader(
                request.client,
                request.paths,
                request.reporter,
                request.mirror,
                request.concurrency,
                request.base_details,
                &version_id,
                request.loader.clone(),
                &loader_version,
                &request.java_path,
            )
            .await?;
        }
    }

    Ok(LoaderInstallResult {
        version_id,
        loader: request.loader.as_str().to_string(),
        loader_version,
        profile_path: version_dir
            .join("version.json")
            .to_string_lossy()
            .into_owned(),
    })
}

pub async fn resolve_effective_version_details(
    client: &Client,
    paths: &LauncherPaths,
    base_version_id: &str,
    loader: &str,
    loader_version: &str,
    mirror: &str,
) -> Result<VersionDetails> {
    let loader_type = parse_loader(loader)?;
    let version_id = effective_version_id(base_version_id, &loader_type, loader_version)?;
    let profile_path = paths.version_dir(&version_id).join("version.json");
    if !profile_path.is_file() {
        bail!("加载器 {loader} {loader_version} 尚未安装");
    }

    let profile: Value = serde_json::from_slice(&fs::read(&profile_path).await?)?;
    let base =
        crate::core::version::fetch_version_details(client, paths, base_version_id, mirror).await?;
    let base_value = serde_json::to_value(base)?;
    let merged = merge_profile(base_value, profile, &version_id, base_version_id);
    serde_json::from_value(merged).context("无法合并加载器版本清单")
}

#[allow(clippy::too_many_arguments)]
async fn install_profile_loader(
    client: &Client,
    paths: &LauncherPaths,
    reporter: &ProgressReporter,
    mirror: &str,
    concurrency: u32,
    base_details: &VersionDetails,
    version_id: &str,
    profile_url: &str,
) -> Result<()> {
    let text = get_text(client, profile_url).await?;
    let mut profile: Value = serde_json::from_str(&text).context("无法解析加载器 profile")?;
    profile["id"] = json!(version_id);
    profile["inheritsFrom"] = json!(base_details.id);
    let profile_bytes = serde_json::to_vec_pretty(&profile)?;
    crate::core::version::write_bytes_atomic(
        &paths.version_dir(version_id).join("version.json"),
        &profile_bytes,
    )
    .await?;

    let artifacts = profile_artifacts(&profile, paths, version_id)?;
    if !artifacts.is_empty() {
        install::download_artifacts(ArtifactDownloadRequest {
            client,
            mirror,
            concurrency,
            force_redownload: false,
            stage: "loader",
            message: "正在下载加载器依赖",
            artifacts,
            reporter,
        })
        .await?;
    }
    reporter.finish("加载器安装完成");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn install_installer_loader(
    client: &Client,
    paths: &LauncherPaths,
    reporter: &ProgressReporter,
    mirror: &str,
    concurrency: u32,
    base_details: &VersionDetails,
    version_id: &str,
    loader: LoaderType,
    loader_version: &str,
    java_path: &Path,
) -> Result<()> {
    let artifact = match loader {
        LoaderType::Neoforge => format!(
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/{loader_version}/neoforge-{loader_version}-installer.jar"
        ),
        LoaderType::Forge => format!(
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{loader_version}/forge-{loader_version}-installer.jar"
        ),
        _ => bail!("当前加载器不需要 installer"),
    };

    let installer_dir = paths.version_dir(version_id);
    fs::create_dir_all(&installer_dir).await?;
    let installer_path = installer_dir.join("installer.jar");
    install::download_artifacts(ArtifactDownloadRequest {
        client,
        mirror,
        concurrency,
        force_redownload: false,
        stage: "loader-installer",
        message: "正在下载加载器安装器",
        artifacts: vec![ArtifactDownload {
            url: artifact,
            destination: installer_path.clone(),
            sha1: None,
            size: None,
            label: format!("{} installer", loader.as_str()),
            version_id: version_id.to_string(),
        }],
        reporter,
    })
    .await?;

    let install_root = paths
        .shared_dir()
        .join("loaders")
        .join(loader.as_str())
        .join(version_id);
    fs::create_dir_all(&install_root).await?;
    fs::write(
        install_root.join("launcher_profiles.json"),
        br#"{"profiles":{},"settings":{},"version":3}"#,
    )
    .await?;

    let mut last_error = String::new();
    let mut installed = false;
    for arguments in [
        vec!["--installClient".to_string()],
        vec![
            "--installClient".to_string(),
            install_root.to_string_lossy().into_owned(),
        ],
    ] {
        let output = Command::new(java_path)
            .arg("-jar")
            .arg(&installer_path)
            .args(&arguments)
            .current_dir(&install_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .with_context(|| format!("无法运行 {} 安装器", loader.as_str()))?;
        if output.status.success() {
            installed = true;
            break;
        }
        last_error = String::from_utf8_lossy(&output.stderr).into_owned();
    }
    if !installed {
        bail!("{} 安装器执行失败: {}", loader.as_str(), last_error.trim());
    }

    let generated = find_generated_profile(&install_root)?;
    let generated_path = generated.0.to_string_lossy().into_owned();
    let mut generated_json: Value = serde_json::from_slice(&fs::read(&generated.0).await?)?;
    generated_json["id"] = json!(version_id);
    generated_json["inheritsFrom"] = json!(base_details.id);
    crate::core::version::write_bytes_atomic(
        &paths.version_dir(version_id).join("version.json"),
        &serde_json::to_vec_pretty(&generated_json)?,
    )
    .await?;
    if let Some(source_libraries) = generated.1 {
        copy_tree(&source_libraries, &paths.libraries_dir())?;
    }

    reporter.finish("加载器安装完成");
    let _ = generated_path;
    Ok(())
}

async fn resolve_fabric_version(
    client: &Client,
    game_version: &str,
    requested: Option<&str>,
) -> Result<String> {
    if let Some(requested) = requested.filter(|value| !value.trim().is_empty()) {
        return Ok(requested.trim().to_string());
    }
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{game_version}");
    let value = get_json(client, &url).await?;
    latest_loader(&value, "loader")
}

async fn resolve_quilt_version(
    client: &Client,
    game_version: &str,
    requested: Option<&str>,
) -> Result<String> {
    if let Some(requested) = requested.filter(|value| !value.trim().is_empty()) {
        return Ok(requested.trim().to_string());
    }
    let url = format!("https://meta.quiltmc.org/v3/versions/loader/{game_version}");
    let value = get_json(client, &url).await?;
    latest_loader(&value, "loader")
}

async fn resolve_neoforge_version(
    client: &Client,
    game_version: &str,
    requested: Option<&str>,
) -> Result<String> {
    if let Some(requested) = requested.filter(|value| !value.trim().is_empty()) {
        return Ok(requested.trim().to_string());
    }
    let url = "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
    let xml = get_text(client, url).await?;
    let prefix = game_version
        .strip_prefix("1.")
        .unwrap_or(game_version)
        .to_string();
    let versions = xml_versions(&xml)?;
    versions
        .into_iter()
        .filter(|version| {
            version.starts_with(&format!("{prefix}.")) || version.starts_with(&prefix)
        })
        .max_by(|left, right| version_cmp(left, right))
        .context("没有找到匹配的 NeoForge 版本")
}

async fn resolve_forge_version(
    client: &Client,
    game_version: &str,
    requested: Option<&str>,
) -> Result<String> {
    if let Some(requested) = requested.filter(|value| !value.trim().is_empty()) {
        return Ok(requested.trim().to_string());
    }
    let url = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
    let value = get_json(client, url).await?;
    for key in [
        format!("{game_version}-recommended"),
        format!("{game_version}-latest"),
    ] {
        if let Some(version) = value
            .get("promos")
            .and_then(|promos| promos.get(&key))
            .and_then(Value::as_str)
        {
            return Ok(format!("{game_version}-{version}"));
        }
    }
    bail!("没有找到匹配的 Forge 版本")
}

fn latest_loader(value: &Value, field: &str) -> Result<String> {
    let versions = value.as_array().context("加载器元数据不是数组")?;
    for item in versions {
        let loader = item.get(field).unwrap_or(item);
        if loader.get("stable").and_then(Value::as_bool) == Some(true) {
            if let Some(version) = loader.get("version").and_then(Value::as_str) {
                return Ok(version.to_string());
            }
        }
    }
    versions
        .iter()
        .find_map(|item| {
            item.get(field)
                .unwrap_or(item)
                .get("version")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .context("加载器元数据没有版本号")
}

fn xml_versions(xml: &str) -> Result<Vec<String>> {
    let mut versions = Vec::new();
    for fragment in xml.split("<version>").skip(1) {
        let Some(end) = fragment.find("</version>") else {
            continue;
        };
        let version = fragment[..end].trim();
        if !version.is_empty() {
            versions.push(version.to_string());
        }
    }
    Ok(versions)
}

fn version_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    let parse = |value: &str| {
        value
            .split('.')
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    parse(left).cmp(&parse(right))
}

fn profile_artifacts(
    profile: &Value,
    paths: &LauncherPaths,
    version_id: &str,
) -> Result<Vec<ArtifactDownload>> {
    let Some(libraries) = profile.get("libraries").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut artifacts = Vec::new();
    for library in libraries {
        let name = library
            .get("name")
            .and_then(Value::as_str)
            .context("加载器依赖缺少 name")?;
        if let Some(artifact) = library
            .get("downloads")
            .and_then(|downloads| downloads.get("artifact"))
            .filter(|artifact| artifact.get("url").and_then(Value::as_str).is_some())
        {
            let relative = artifact
                .get("path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| maven_path(name));
            artifacts.push(ArtifactDownload {
                url: artifact
                    .get("url")
                    .and_then(Value::as_str)
                    .unwrap()
                    .to_string(),
                destination: safe_join(&paths.libraries_dir(), &relative)?,
                sha1: artifact
                    .get("sha1")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                size: artifact.get("size").and_then(Value::as_u64),
                label: name.to_string(),
                version_id: version_id.to_string(),
            });
        } else if let Some(base) = library.get("url").and_then(Value::as_str) {
            let relative = maven_path(name);
            artifacts.push(ArtifactDownload {
                url: format!("{}{relative}", base.trim_end_matches('/').to_string() + "/"),
                destination: safe_join(&paths.libraries_dir(), &relative)?,
                sha1: None,
                size: None,
                label: name.to_string(),
                version_id: version_id.to_string(),
            });
        }
    }
    Ok(artifacts)
}

fn maven_path(coordinate: &str) -> String {
    let (coordinate, extension) = coordinate.split_once('@').unwrap_or((coordinate, "jar"));
    let parts = coordinate.split(':').collect::<Vec<_>>();
    if parts.len() < 3 {
        return coordinate.replace(':', "/");
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = parts
        .get(3)
        .map(|value| format!("-{value}"))
        .unwrap_or_default();
    format!("{group}/{artifact}/{version}/{artifact}-{version}{classifier}.{extension}")
}

fn find_generated_profile(root: &Path) -> Result<(PathBuf, Option<PathBuf>)> {
    let versions_dir = root.join("versions");
    let mut found = None;
    for entry in WalkDir::new(&versions_dir)
        .max_depth(3)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "json")
            && entry.path() != root.join("launcher_profiles.json")
        {
            if let Ok(bytes) = std::fs::read(entry.path()) {
                if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                    if value.get("mainClass").is_some() {
                        found = Some(entry.path().to_path_buf());
                        break;
                    }
                }
            }
        }
    }
    let profile = found.context("加载器安装器没有生成版本 profile")?;
    let libraries = root.join("libraries");
    Ok((profile, libraries.is_dir().then_some(libraries)))
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(source)?;
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("加载器依赖路径不安全: {relative}");
    }
    Ok(root.join(path))
}

fn effective_version_id(
    base_version: &str,
    loader: &LoaderType,
    loader_version: &str,
) -> Result<String> {
    let id = format!(
        "emcl-{}-{}-{}",
        loader.as_str(),
        base_version.replace(
            |character: char| !character.is_ascii_alphanumeric()
                && character != '.'
                && character != '-'
                && character != '_',
            "-"
        ),
        loader_version.replace(
            |character: char| !character.is_ascii_alphanumeric()
                && character != '.'
                && character != '-'
                && character != '_',
            "-"
        )
    );
    if !is_safe_segment(&id) {
        bail!("加载器版本 id 不安全");
    }
    Ok(id)
}

fn parse_loader(loader: &str) -> Result<LoaderType> {
    match loader {
        "fabric" => Ok(LoaderType::Fabric),
        "quilt" => Ok(LoaderType::Quilt),
        "neoforge" => Ok(LoaderType::Neoforge),
        "forge" => Ok(LoaderType::Forge),
        _ => bail!("不支持的加载器: {loader}"),
    }
}

fn merge_profile(base: Value, profile: Value, version_id: &str, base_id: &str) -> Value {
    let mut merged = base;
    let profile_obj = profile.as_object().cloned().unwrap_or_default();
    let merged_obj = merged
        .as_object_mut()
        .expect("base version must be an object");
    merged_obj.insert("id".to_string(), json!(version_id));
    merged_obj.remove("inheritsFrom");
    for key in ["libraries", "arguments"] {
        if let Some(profile_value) = profile_obj.get(key) {
            if merged_obj.contains_key(key) {
                merge_value(merged_obj.get_mut(key).unwrap(), profile_value.clone());
            } else {
                merged_obj.insert(key.to_string(), profile_value.clone());
            }
        }
    }
    for key in [
        "mainClass",
        "assets",
        "assetIndex",
        "downloads",
        "javaVersion",
        "logging",
        "minecraftArguments",
        "time",
        "releaseTime",
        "type",
    ] {
        if let Some(value) = profile_obj.get(key) {
            if *value != Value::Null {
                merged_obj.insert(key.to_string(), value.clone());
            }
        }
    }
    merged_obj.insert("inheritsFromBase".to_string(), json!(base_id));
    merged
}

fn merge_value(base: &mut Value, incoming: Value) {
    match (base, incoming) {
        (Value::Array(base_values), Value::Array(incoming_values)) => {
            base_values.extend(incoming_values);
        }
        (Value::Object(base_map), Value::Object(incoming_map)) => {
            for (key, value) in incoming_map {
                if let Some(base_value) = base_map.get_mut(&key) {
                    merge_value(base_value, value);
                } else {
                    base_map.insert(key, value);
                }
            }
        }
        (base_value, incoming_value) => {
            *base_value = incoming_value;
        }
    }
}

async fn get_json(client: &Client, url: &str) -> Result<Value> {
    let text = get_text(client, url).await?;
    serde_json::from_str(&text).with_context(|| format!("无法解析 {url}"))
}

async fn get_text(client: &Client, url: &str) -> Result<String> {
    client
        .get(url)
        .send()
        .await
        .with_context(|| format!("请求失败: {url}"))?
        .error_for_status()
        .with_context(|| format!("请求被拒绝: {url}"))?
        .text()
        .await
        .with_context(|| format!("无法读取响应: {url}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_maven_paths() {
        assert_eq!(
            maven_path("net.fabricmc:fabric-loader:0.16.10"),
            "net/fabricmc/fabric-loader/0.16.10/fabric-loader-0.16.10.jar"
        );
    }

    #[test]
    fn merges_profile_arrays() {
        let base = json!({"libraries": [{"name":"base"}], "arguments":{"jvm":["-cp"]}});
        let profile = json!({"mainClass":"Example", "libraries": [{"name":"loader"}], "arguments":{"jvm":["-Dloader"]}});
        let merged = merge_profile(base, profile, "emcl-fabric", "1.21.1");
        assert_eq!(merged["libraries"].as_array().unwrap().len(), 2);
        assert_eq!(merged["mainClass"], "Example");
    }
}
