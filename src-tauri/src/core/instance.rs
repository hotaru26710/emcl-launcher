use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::core::paths::LauncherPaths;
use crate::db::models::{now_rfc3339, CreateInstanceInput, Instance, UpdateInstanceInput};

const INSTANCE_COLUMNS: &str = "id, name, version_id, loader, loader_version, java_path,
    jvm_args, memory_mb, directory, created_at, updated_at, last_played_at";

fn row_to_instance(row: &rusqlite::Row<'_>) -> rusqlite::Result<Instance> {
    Ok(Instance {
        id: row.get(0)?,
        name: row.get(1)?,
        version_id: row.get(2)?,
        loader: row.get(3)?,
        loader_version: row.get(4)?,
        java_path: row.get(5)?,
        jvm_args: row.get(6)?,
        memory_mb: row.get(7)?,
        directory: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        last_played_at: row.get(11)?,
    })
}

pub fn list_instances(connection: &Connection) -> Result<Vec<Instance>> {
    let sql = format!(
        "SELECT {INSTANCE_COLUMNS} FROM instances ORDER BY updated_at DESC, name COLLATE NOCASE"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map([], row_to_instance)?;
    let mut instances = Vec::new();
    for row in rows {
        instances.push(row?);
    }
    Ok(instances)
}

pub fn get_instance(connection: &Connection, id: &str) -> Result<Option<Instance>> {
    let sql = format!("SELECT {INSTANCE_COLUMNS} FROM instances WHERE id = ?1");
    connection
        .query_row(&sql, params![id], row_to_instance)
        .optional()
        .context("failed to read instance")
}

pub fn create_instance(
    paths: &LauncherPaths,
    connection: &Connection,
    input: CreateInstanceInput,
) -> Result<Instance> {
    let name = input.name.trim();
    if name.is_empty() {
        bail!("instance name cannot be empty");
    }
    if input.version_id.trim().is_empty() {
        bail!("version id cannot be empty");
    }

    let id = Uuid::new_v4().to_string();
    let directory = paths.instance_dir(&id)?;
    for child in [
        "mods",
        "config",
        "saves",
        "logs",
        "crash-reports",
        "resourcepacks",
        "shaderpacks",
    ] {
        std::fs::create_dir_all(directory.join(child)).with_context(|| {
            format!(
                "failed to create instance directory {}",
                directory.display()
            )
        })?;
    }

    let now = now_rfc3339();
    connection.execute(
        "INSERT INTO instances (
            id, name, version_id, loader, loader_version, java_path, jvm_args,
            memory_mb, directory, created_at, updated_at, last_played_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?9, ?9, NULL)",
        params![
            id,
            name,
            input.version_id.trim(),
            input.loader,
            input.loader_version,
            input.jvm_args.unwrap_or_default(),
            input.memory_mb.unwrap_or(4096).clamp(512, 65536),
            directory.to_string_lossy(),
            now,
        ],
    )?;

    get_instance(connection, &id)?.context("instance was not found after creation")
}

pub fn update_instance(
    connection: &Connection,
    id: &str,
    input: UpdateInstanceInput,
) -> Result<Instance> {
    let existing = get_instance(connection, id)?.context("instance was not found")?;
    let now = now_rfc3339();
    let name = input.name.unwrap_or(existing.name);
    let version_id = input.version_id.unwrap_or(existing.version_id);

    connection.execute(
        "UPDATE instances
         SET name = ?1,
             version_id = ?2,
             loader = COALESCE(?3, loader),
             loader_version = COALESCE(?4, loader_version),
             java_path = COALESCE(?5, java_path),
             memory_mb = ?6,
             jvm_args = ?7,
             updated_at = ?8
         WHERE id = ?9",
        params![
            name.trim(),
            version_id.trim(),
            input.loader,
            input.loader_version,
            input.java_path,
            input
                .memory_mb
                .unwrap_or(existing.memory_mb)
                .clamp(512, 65536),
            input.jvm_args.unwrap_or(existing.jvm_args),
            now,
            id,
        ],
    )?;

    get_instance(connection, id)?.context("instance was not found after update")
}

pub fn delete_instance(paths: &LauncherPaths, connection: &Connection, id: &str) -> Result<()> {
    let instance = get_instance(connection, id)?.context("instance was not found")?;
    let directory = PathBuf::from(&instance.directory);

    if directory.exists() {
        let instances_root = paths
            .instances_dir()
            .canonicalize()
            .unwrap_or_else(|_| paths.instances_dir());
        let resolved = directory
            .canonicalize()
            .with_context(|| format!("failed to resolve {}", directory.display()))?;
        if !resolved.starts_with(&instances_root) {
            bail!("refusing to delete a directory outside the instances root");
        }
        std::fs::remove_dir_all(&resolved)
            .with_context(|| format!("failed to remove {}", resolved.display()))?;
    }

    connection.execute("DELETE FROM instances WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn record_installation(
    connection: &Connection,
    instance_id: &str,
    version_id: &str,
    path: &str,
) -> Result<()> {
    let now = now_rfc3339();
    connection.execute(
        "INSERT INTO installed_versions (
            instance_id, version_id, loader, loader_version, status, path, installed_at
         ) VALUES (?1, ?2, '', '', 'ready', ?3, ?4)
         ON CONFLICT(instance_id, version_id, loader, loader_version)
         DO UPDATE SET status = 'ready', path = excluded.path, installed_at = excluded.installed_at",
        params![instance_id, version_id, path, now],
    )?;
    connection.execute(
        "UPDATE instances SET version_id = ?1, updated_at = ?2 WHERE id = ?3",
        params![version_id, now, instance_id],
    )?;
    Ok(())
}
