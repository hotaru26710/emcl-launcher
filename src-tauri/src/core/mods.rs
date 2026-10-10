use std::collections::HashSet;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zip::ZipArchive;

use crate::core::paths::LauncherPaths;
use crate::db::models::Instance;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModInfo {
    pub file_name: String,
    pub name: String,
    pub mod_id: Option<String>,
    pub version: Option<String>,
    pub loader: Option<String>,
    pub description: Option<String>,
    pub enabled: bool,
}

pub fn list_mods(_paths: &LauncherPaths, instance: &Instance) -> Result<Vec<ModInfo>> {
    let mods_dir = PathBuf::from(&instance.directory).join("mods");
    std::fs::create_dir_all(&mods_dir)?;
    let mut mods = Vec::new();
    for entry in std::fs::read_dir(&mods_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if !is_mod_file(&file_name) {
            continue;
        }
        let enabled = !file_name.ends_with(".disabled");
        let metadata = parse_mod_metadata(entry.path()).unwrap_or_default();
        mods.push(ModInfo {
            file_name,
            name: metadata.name.unwrap_or_else(|| "未命名模组".to_string()),
            mod_id: metadata.id,
            version: metadata.version,
            loader: metadata.loader,
            description: metadata.description,
            enabled,
        });
    }
    mods.sort_by_key(|left| left.name.to_lowercase());
    Ok(mods)
}

pub fn install_mods(
    paths: &LauncherPaths,
    instance: &Instance,
    source_paths: Vec<String>,
) -> Result<Vec<ModInfo>> {
    let mods_dir = PathBuf::from(&instance.directory).join("mods");
    std::fs::create_dir_all(&mods_dir)?;
    let mut seen = HashSet::new();

    for source in source_paths {
        let source = PathBuf::from(source);
        if !source.is_file() || source.extension().and_then(|value| value.to_str()) != Some("jar") {
            bail!("只能安装 .jar 模组文件: {}", source.display());
        }
        let source_name = source
            .file_name()
            .context("模组文件没有文件名")?
            .to_string_lossy()
            .into_owned();
        let destination = unique_destination(&mods_dir, &source_name, &mut seen)?;
        std::fs::copy(&source, &destination)
            .with_context(|| format!("无法复制模组 {} 到实例目录", source.display()))?;
    }

    list_mods(paths, instance)
}

pub fn toggle_mod(
    paths: &LauncherPaths,
    instance: &Instance,
    file_name: &str,
    enabled: bool,
) -> Result<Vec<ModInfo>> {
    let mods_dir = PathBuf::from(&instance.directory).join("mods");
    let current = safe_file_path(&mods_dir, file_name)?;
    if !current.is_file() {
        bail!("模组不存在");
    }

    let target_name = if enabled {
        file_name
            .strip_suffix(".disabled")
            .context("模组文件名格式不正确")?
            .to_string()
    } else if file_name.ends_with(".disabled") {
        file_name.to_string()
    } else {
        format!("{file_name}.disabled")
    };
    let target = safe_file_path(&mods_dir, &target_name)?;
    if target.exists() {
        bail!("目标模组文件已存在");
    }
    std::fs::rename(current, target)?;
    list_mods(paths, instance)
}

pub fn delete_mod(
    paths: &LauncherPaths,
    instance: &Instance,
    file_name: &str,
) -> Result<Vec<ModInfo>> {
    let mods_dir = PathBuf::from(&instance.directory).join("mods");
    let path = safe_file_path(&mods_dir, file_name)?;
    if path.is_file() {
        std::fs::remove_file(path)?;
    }
    list_mods(paths, instance)
}

#[derive(Default)]
struct ModMetadata {
    name: Option<String>,
    id: Option<String>,
    version: Option<String>,
    loader: Option<String>,
    description: Option<String>,
}

fn parse_mod_metadata(path: PathBuf) -> Result<ModMetadata> {
    let file = std::fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    if let Some(value) = read_json_entry(&mut archive, "fabric.mod.json")? {
        return Ok(metadata_from_json(
            &value,
            &["name", "id", "version", "description"],
            Some("fabric"),
        ));
    }
    if let Some(value) = read_json_entry(&mut archive, "quilt.mod.json")? {
        let loader = value.get("quilt_loader").unwrap_or(&value);
        return Ok(metadata_from_json(
            loader,
            &["name", "id", "version", "description"],
            Some("quilt"),
        ));
    }
    for entry in ["META-INF/neoforge.mods.toml", "META-INF/mods.toml"] {
        if let Some(text) = read_text_entry(&mut archive, entry)? {
            let value: toml::Value = toml::from_str(&text).context("无法解析模组 TOML 元数据")?;
            let loader = if entry.contains("neoforge") {
                "neoforge"
            } else {
                "forge"
            };
            return Ok(metadata_from_toml(&value, loader));
        }
    }

    Ok(ModMetadata::default())
}

fn read_json_entry<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<Option<Value>> {
    let Some(text) = read_text_entry(archive, name)? else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_str(&text)?))
}

fn read_text_entry<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<Option<String>> {
    let Ok(mut entry) = archive.by_name(name) else {
        return Ok(None);
    };
    let mut text = String::new();
    entry.read_to_string(&mut text)?;
    Ok(Some(text))
}

fn metadata_from_json(value: &Value, keys: &[&str], loader: Option<&str>) -> ModMetadata {
    ModMetadata {
        name: value
            .get(keys[0])
            .and_then(Value::as_str)
            .map(ToString::to_string),
        id: value
            .get(keys[1])
            .and_then(Value::as_str)
            .map(ToString::to_string),
        version: value
            .get(keys[2])
            .and_then(Value::as_str)
            .map(ToString::to_string),
        loader: loader.map(ToString::to_string),
        description: value
            .get(keys[3])
            .and_then(Value::as_str)
            .map(ToString::to_string),
    }
}

fn metadata_from_toml(value: &toml::Value, loader: &str) -> ModMetadata {
    let mod_value = value
        .get("mods")
        .and_then(|mods| mods.as_array())
        .and_then(|mods| mods.first())
        .unwrap_or(value);

    ModMetadata {
        name: string_field(mod_value, "displayName").or_else(|| string_field(mod_value, "name")),
        id: string_field(mod_value, "modId"),
        version: string_field(mod_value, "version"),
        loader: Some(loader.to_string()),
        description: string_field(mod_value, "description"),
    }
}

fn string_field(value: &toml::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
}

fn unique_destination(
    directory: &Path,
    file_name: &str,
    seen: &mut HashSet<String>,
) -> Result<PathBuf> {
    let path = Path::new(file_name);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("模组文件名不安全");
    }
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("mod");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("jar");
    let mut candidate = file_name.to_string();
    let mut index = 1;
    while directory.join(&candidate).exists() || !seen.insert(candidate.clone()) {
        candidate = format!("{stem} ({index}).{extension}");
        index += 1;
    }
    Ok(directory.join(candidate))
}

fn safe_file_path(directory: &Path, file_name: &str) -> Result<PathBuf> {
    let path = Path::new(file_name);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("模组文件名不安全");
    }
    Ok(directory.join(path))
}

fn is_mod_file(file_name: &str) -> bool {
    let name = file_name.strip_suffix(".disabled").unwrap_or(file_name);
    name.ends_with(".jar")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_enabled_and_disabled_mods() {
        assert!(is_mod_file("example.jar"));
        assert!(is_mod_file("example.jar.disabled"));
        assert!(!is_mod_file("example.txt"));
    }
}
