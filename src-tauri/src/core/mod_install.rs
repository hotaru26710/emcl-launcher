use std::path::PathBuf;

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::core::metadata;
use crate::core::mods::{self, ModInfo};
use crate::db::models::Instance;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledMod {
    pub instance_id: String,
    pub mod_id: String,
    pub mod_name: Option<String>,
    pub version: Option<String>,
    pub platform: String,
    pub project_id: String,
    pub version_id: Option<String>,
    pub file_name: String,
    pub sha256: String,
    pub source_type: String,
    pub update_available: bool,
    pub latest_version_id: Option<String>,
    pub dependency_of: Option<String>,
}

#[allow(dead_code)]
pub fn record_install(
    connection: &Connection,
    instance: &Instance,
    platform: &str,
    project_id: &str,
    version_id: Option<&str>,
    dependency_of: Option<&str>,
    mods: &[ModInfo],
) -> Result<()> {
    record_install_inner(
        connection,
        instance,
        platform,
        project_id,
        version_id,
        dependency_of,
        mods,
        false,
        None,
    )
}

pub fn record_install_rooted(
    connection: &Connection,
    instance: &Instance,
    platform: &str,
    project_id: &str,
    version_id: Option<&str>,
    root_file_name: Option<&str>,
    mods: &[ModInfo],
) -> Result<()> {
    record_install_inner(
        connection,
        instance,
        platform,
        project_id,
        version_id,
        None,
        mods,
        false,
        root_file_name,
    )
}

#[allow(clippy::too_many_arguments)]
fn record_install_inner(
    connection: &Connection,
    instance: &Instance,
    platform: &str,
    project_id: &str,
    version_id: Option<&str>,
    dependency_of: Option<&str>,
    mods: &[ModInfo],
    preserve_existing: bool,
    root_file_name: Option<&str>,
) -> Result<()> {
    let mods_dir = PathBuf::from(&instance.directory).join("mods");
    for item in mods {
        let path = mods_dir.join(&item.file_name);
        if !path.is_file() {
            continue;
        }
        let parsed = metadata::parse_jar_metadata(&path)?.unwrap_or_default();
        let mod_id = item
            .mod_id
            .clone()
            .or(parsed.mod_id)
            .unwrap_or_else(|| item.file_name.clone());
        let is_root = root_file_name
            .map(|name| name == item.file_name)
            .unwrap_or(true);
        let item_project_id = if is_root { project_id } else { "" };
        let item_version_id = if is_root { version_id } else { None };
        let item_dependency_of = if is_root {
            dependency_of
        } else {
            root_file_name
        };
        let sha256 = metadata::sha256_file(&path)?;
        if preserve_existing
            && connection
                .query_row(
                    "SELECT 1 FROM installed_mods WHERE instance_id = ?1 AND mod_id = ?2",
                    params![instance.id, mod_id],
                    |_| Ok(()),
                )
                .optional()?
                .is_some()
        {
            continue;
        }
        connection.execute(
            "INSERT INTO installed_mods (
                instance_id, mod_id, mod_name, version, platform, project_id,
                version_id, file_name, sha256, source_type, update_available,
                latest_version_id, dependency_of
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, NULL, ?11)
             ON CONFLICT(instance_id, mod_id) DO UPDATE SET
                mod_name = excluded.mod_name,
                version = excluded.version,
                platform = excluded.platform,
                project_id = excluded.project_id,
                version_id = excluded.version_id,
                file_name = excluded.file_name,
                sha256 = excluded.sha256,
                source_type = excluded.source_type,
                dependency_of = excluded.dependency_of",
            params![
                instance.id,
                mod_id,
                item.name,
                item.version,
                platform,
                item_project_id,
                item_version_id,
                item.file_name,
                sha256,
                "community",
                item_dependency_of,
            ],
        )?;
    }
    Ok(())
}

pub fn sync_from_mods(connection: &Connection, instance: &Instance) -> Result<Vec<InstalledMod>> {
    let mods = mods::list_mods(
        &crate::core::paths::LauncherPaths::from_root(PathBuf::from(".")),
        instance,
    )?;
    record_install_inner(
        connection, instance, "local", "", None, None, &mods, true, None,
    )?;
    let active_files = mods
        .iter()
        .map(|item| item.file_name.as_str())
        .collect::<std::collections::HashSet<_>>();
    for item in list_installed(connection, &instance.id)? {
        if !active_files.contains(item.file_name.as_str()) {
            remove_installed(connection, &instance.id, &item.file_name)?;
        }
    }
    list_installed(connection, &instance.id)
}

pub fn list_installed(connection: &Connection, instance_id: &str) -> Result<Vec<InstalledMod>> {
    let mut statement = connection.prepare(
        "SELECT instance_id, mod_id, mod_name, version, platform, project_id,
                version_id, file_name, sha256, source_type, update_available,
                latest_version_id, dependency_of
         FROM installed_mods WHERE instance_id = ?1
         ORDER BY mod_name COLLATE NOCASE, file_name COLLATE NOCASE",
    )?;
    let rows = statement.query_map(params![instance_id], |row| {
        Ok(InstalledMod {
            instance_id: row.get(0)?,
            mod_id: row.get(1)?,
            mod_name: row.get(2)?,
            version: row.get(3)?,
            platform: row.get(4)?,
            project_id: row.get(5)?,
            version_id: row.get(6)?,
            file_name: row.get(7)?,
            sha256: row.get(8)?,
            source_type: row.get(9)?,
            update_available: row.get::<_, i64>(10)? != 0,
            latest_version_id: row.get(11)?,
            dependency_of: row.get(12)?,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn set_update(
    connection: &Connection,
    instance_id: &str,
    mod_id: &str,
    latest_version_id: Option<&str>,
) -> Result<()> {
    let update_available = latest_version_id.is_some();
    connection.execute(
        "UPDATE installed_mods SET update_available = ?1, latest_version_id = ?2
         WHERE instance_id = ?3 AND mod_id = ?4",
        params![update_available, latest_version_id, instance_id, mod_id],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn find_installed(
    connection: &Connection,
    instance_id: &str,
    mod_id: &str,
) -> Result<Option<InstalledMod>> {
    Ok(list_installed(connection, instance_id)?
        .into_iter()
        .find(|item| item.mod_id == mod_id))
}

#[allow(dead_code)]
pub fn remove_installed(connection: &Connection, instance_id: &str, file_name: &str) -> Result<()> {
    connection.execute(
        "DELETE FROM installed_mods WHERE instance_id = ?1 AND file_name = ?2",
        params![instance_id, file_name],
    )?;
    Ok(())
}
