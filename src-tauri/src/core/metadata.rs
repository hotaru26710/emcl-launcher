use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zip::ZipArchive;

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
