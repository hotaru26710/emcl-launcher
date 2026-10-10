use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::db::models::{Instance, ModMetadata};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedModMetadata {
    pub mod_id: Option<String>,
    pub mod_name: Option<String>,
    pub version: Option<String>,
    pub loader: Option<String>,
    pub description: Option<String>,
    pub depends: HashMap<String, String>,
    pub breaks: HashMap<String, String>,
    pub conflicts: HashMap<String, String>,
}

pub fn refresh_instance_mod_metadata(
    connection: &Connection,
    instance: &Instance,
) -> Result<Vec<ModMetadata>> {
    let mods_dir = std::path::PathBuf::from(&instance.directory).join("mods");
    std::fs::create_dir_all(&mods_dir)?;
    let mut metadata_list = Vec::new();

    for entry in std::fs::read_dir(&mods_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if file_name.ends_with(".disabled") || !file_name.to_ascii_lowercase().ends_with(".jar") {
            continue;
        }
        let Some(parsed) = parse_jar_metadata(&entry.path())? else {
            continue;
        };
        let sha256 = sha256_file(&entry.path())?;
        let metadata = ModMetadata {
            instance_id: instance.id.clone(),
            mod_id: parsed.mod_id,
            mod_name: parsed.mod_name,
            version: parsed.version,
            loader: parsed.loader,
            file_name,
            sha256,
            source_type: "local".to_string(),
            depends_json: serde_json::to_string(&parsed.depends)?,
            breaks_json: serde_json::to_string(&parsed.breaks)?,
            conflicts_json: serde_json::to_string(&parsed.conflicts)?,
        };
        upsert_mod_metadata(connection, &metadata)?;
        metadata_list.push(metadata);
    }

    Ok(metadata_list)
}

pub fn list_mod_metadata(connection: &Connection, instance_id: &str) -> Result<Vec<ModMetadata>> {
    let mut statement = connection.prepare(
        "SELECT instance_id, mod_id, mod_name, version, loader, file_name, sha256,
                source_type, depends_json, breaks_json, conflicts_json
         FROM mod_metadata WHERE instance_id = ?1 ORDER BY mod_name COLLATE NOCASE",
    )?;
    let rows = statement.query_map(params![instance_id], |row| {
        Ok(ModMetadata {
            instance_id: row.get(0)?,
            mod_id: row.get(1)?,
            mod_name: row.get(2)?,
            version: row.get(3)?,
            loader: row.get(4)?,
            file_name: row.get(5)?,
            sha256: row.get(6)?,
            source_type: row.get(7)?,
            depends_json: row.get(8)?,
            breaks_json: row.get(9)?,
            conflicts_json: row.get(10)?,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn upsert_mod_metadata(connection: &Connection, metadata: &ModMetadata) -> Result<()> {
    connection.execute(
        "INSERT INTO mod_metadata (
            instance_id, mod_id, mod_name, version, loader, file_name, sha256,
            source_type, depends_json, breaks_json, conflicts_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(instance_id, file_name) DO UPDATE SET
            mod_id = excluded.mod_id,
            mod_name = excluded.mod_name,
            version = excluded.version,
            loader = excluded.loader,
            sha256 = excluded.sha256,
            source_type = excluded.source_type,
            depends_json = excluded.depends_json,
            breaks_json = excluded.breaks_json,
            conflicts_json = excluded.conflicts_json",
        params![
            metadata.instance_id,
            metadata.mod_id,
            metadata.mod_name,
            metadata.version,
            metadata.loader,
            metadata.file_name,
            metadata.sha256,
            metadata.source_type,
            metadata.depends_json,
            metadata.breaks_json,
            metadata.conflicts_json,
        ],
    )?;
    Ok(())
}

pub fn parse_jar_metadata(path: &Path) -> Result<Option<ParsedModMetadata>> {
    let file =
        std::fs::File::open(path).with_context(|| format!("无法打开 jar {}", path.display()))?;
    let mut archive = ZipArchive::new(file)?;

    if let Some(value) = read_json_entry(&mut archive, "fabric.mod.json")? {
        return Ok(Some(parse_fabric(value)));
    }
    if let Some(value) = read_json_entry(&mut archive, "quilt.mod.json")? {
        return Ok(Some(parse_quilt(value)));
    }
    for entry in ["META-INF/neoforge.mods.toml", "META-INF/mods.toml"] {
        if let Some(text) = read_text_entry(&mut archive, entry)? {
            let loader = if entry.contains("neoforge") {
                "neoforge"
            } else {
                "forge"
            };
            return Ok(Some(parse_toml(&text, loader)?));
        }
    }

    Ok(None)
}

fn parse_fabric(value: Value) -> ParsedModMetadata {
    ParsedModMetadata {
        mod_id: value
            .get("id")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        mod_name: value
            .get("name")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        version: value
            .get("version")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        loader: Some("fabric".to_string()),
        description: value
            .get("description")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        depends: parse_string_map(value.get("depends")),
        breaks: parse_string_map(value.get("breaks")),
        conflicts: parse_string_map(value.get("conflicts")),
    }
}

fn parse_quilt(value: Value) -> ParsedModMetadata {
    let loader = value.get("quilt_loader").unwrap_or(&value);
    let metadata = loader.get("metadata").unwrap_or(loader);
    ParsedModMetadata {
        mod_id: loader
            .get("id")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        mod_name: metadata
            .get("name")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        version: loader
            .get("version")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        loader: Some("quilt".to_string()),
        description: metadata
            .get("description")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        depends: parse_string_map(loader.get("depends")),
        breaks: parse_string_map(loader.get("breaks")),
        conflicts: parse_string_map(loader.get("conflicts")),
    }
}

fn parse_toml(text: &str, loader: &str) -> Result<ParsedModMetadata> {
    let value: toml::Value = toml::from_str(text).context("无法解析 mods.toml")?;
    let mod_value = value
        .get("mods")
        .and_then(|mods| mods.as_array())
        .and_then(|mods| mods.first())
        .unwrap_or(&value);

    let mut depends = HashMap::new();
    let mut breaks = HashMap::new();
    let mut conflicts = HashMap::new();
    if let Some(dependencies) = value.get("dependencies").and_then(toml::Value::as_table) {
        for (owner, entries) in dependencies {
            if let Some(entries) = entries.as_array() {
                for entry in entries {
                    let mod_id = string_field(entry, "modId").unwrap_or_else(|| owner.clone());
                    let range =
                        string_field(entry, "versionRange").unwrap_or_else(|| "*".to_string());
                    let mandatory = entry
                        .get("mandatory")
                        .and_then(toml::Value::as_bool)
                        .unwrap_or(true);
                    if mandatory {
                        depends.insert(mod_id.clone(), range.clone());
                    } else if range.contains("!") || range.contains("[") {
                        conflicts.insert(mod_id, range);
                    } else {
                        breaks.insert(mod_id, range);
                    }
                }
            }
        }
    }

    Ok(ParsedModMetadata {
        mod_id: string_field(mod_value, "modId"),
        mod_name: string_field(mod_value, "displayName")
            .or_else(|| string_field(mod_value, "name")),
        version: string_field(mod_value, "version"),
        loader: Some(loader.to_string()),
        description: string_field(mod_value, "description"),
        depends,
        breaks,
        conflicts,
    })
}

fn string_field(value: &toml::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
}

fn parse_string_map(value: Option<&Value>) -> HashMap<String, String> {
    value
        .and_then(Value::as_object)
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| {
                    Some((
                        key.clone(),
                        value.as_str().map(ToString::to_string).or_else(|| {
                            value.as_array().map(|items| {
                                items
                                    .iter()
                                    .filter_map(Value::as_str)
                                    .collect::<Vec<_>>()
                                    .join(" || ")
                            })
                        })?,
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fabric_dependency_metadata() {
        let value = serde_json::json!({
            "id": "example",
            "name": "Example",
            "version": "1.0.0",
            "depends": {"fabric-api": ">=0.92.0"}
        });
        let parsed = parse_fabric(value);
        assert_eq!(parsed.mod_id.as_deref(), Some("example"));
        assert_eq!(
            parsed.depends.get("fabric-api").map(String::as_str),
            Some(">=0.92.0")
        );
    }
}
