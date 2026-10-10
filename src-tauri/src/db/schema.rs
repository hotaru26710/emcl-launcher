use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

pub const DEFAULT_DOWNLOAD_MIRROR: &str = "official";
pub const DEFAULT_DOWNLOAD_CONCURRENCY: u32 = 8;

pub fn open_database(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let connection = Connection::open(path)
        .with_context(|| format!("failed to open SQLite database {}", path.display()))?;
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;",
        )
        .context("failed to configure SQLite")?;

    migrate(&connection)?;
    seed_settings(&connection)?;
    Ok(connection)
}

fn migrate(connection: &Connection) -> Result<()> {
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if version < 1 {
        connection
            .execute_batch(
                "BEGIN;
                 CREATE TABLE IF NOT EXISTS instances (
                     id TEXT PRIMARY KEY NOT NULL,
                     name TEXT NOT NULL,
                     version_id TEXT NOT NULL,
                     loader TEXT,
                     loader_version TEXT,
                     java_path TEXT,
                     jvm_args TEXT NOT NULL DEFAULT '',
                     memory_mb INTEGER NOT NULL DEFAULT 4096,
                     directory TEXT NOT NULL,
                     created_at TEXT NOT NULL,
                     updated_at TEXT NOT NULL,
                     last_played_at TEXT
                 );

                 CREATE TABLE IF NOT EXISTS accounts (
                     id TEXT PRIMARY KEY NOT NULL,
                     username TEXT NOT NULL,
                     uuid TEXT NOT NULL,
                     account_type TEXT NOT NULL,
                     access_token TEXT,
                     refresh_token TEXT,
                     expires_at TEXT,
                     created_at TEXT NOT NULL
                 );

                 CREATE TABLE IF NOT EXISTS settings (
                     key TEXT PRIMARY KEY NOT NULL,
                     value TEXT NOT NULL
                 );

                 CREATE TABLE IF NOT EXISTS installed_versions (
                     id INTEGER PRIMARY KEY AUTOINCREMENT,
                     instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
                     version_id TEXT NOT NULL,
                     loader TEXT NOT NULL DEFAULT '',
                     loader_version TEXT NOT NULL DEFAULT '',
                     status TEXT NOT NULL,
                     path TEXT NOT NULL,
                     installed_at TEXT NOT NULL,
                     UNIQUE(instance_id, version_id, loader, loader_version)
                 );

                 CREATE INDEX IF NOT EXISTS idx_instances_updated_at
                     ON instances(updated_at DESC);
                 CREATE INDEX IF NOT EXISTS idx_installed_versions_instance
                     ON installed_versions(instance_id);

                 PRAGMA user_version = 1;
                 COMMIT;",
            )
            .context("failed to run database migration 1")?;
    }

    Ok(())
}

fn seed_settings(connection: &Connection) -> Result<()> {
    connection.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
        params!["download_mirror", DEFAULT_DOWNLOAD_MIRROR],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
        params![
            "download_concurrency",
            DEFAULT_DOWNLOAD_CONCURRENCY.to_string()
        ],
    )?;
    Ok(())
}

pub fn get_setting(connection: &Connection, key: &str) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .context("failed to read setting")
}

pub fn set_setting(connection: &Connection, key: &str, value: &str) -> Result<()> {
    connection
        .execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .context("failed to save setting")?;
    Ok(())
}
