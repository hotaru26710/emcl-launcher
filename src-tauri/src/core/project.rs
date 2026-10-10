use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::models::{
    now_rfc3339, CreateProjectInput, LinkedProjectInstance, Project, UpdateProjectInput,
};

const PROJECT_COLUMNS: &str = "id, name, root_path, build_tool, output_dir, gradle_task,
    auto_build, auto_deploy, created_at, updated_at";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInspection {
    pub name: String,
    pub root_path: String,
    pub build_tool: String,
    pub output_dir: String,
    pub gradle_task: String,
}

pub fn inspect_project_directory(path: &Path) -> Result<ProjectInspection> {
    if !path.is_dir() {
        bail!("请选择有效的工程目录");
    }
    let root_path = path
        .canonicalize()
        .with_context(|| format!("无法解析工程目录 {}", path.display()))?;

    let has_gradle =
        root_path.join("build.gradle").is_file() || root_path.join("build.gradle.kts").is_file();
    if !has_gradle {
        bail!("当前只支持包含 build.gradle 或 build.gradle.kts 的 Gradle 工程");
    }

    let name = detect_project_name(&root_path).unwrap_or_else(|| {
        root_path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Minecraft Mod".to_string())
    });
    let output_dir = detect_output_dir(&root_path)?;

    Ok(ProjectInspection {
        name,
        root_path: root_path.to_string_lossy().into_owned(),
        build_tool: "gradle".to_string(),
        output_dir: output_dir.to_string_lossy().into_owned(),
        gradle_task: "build".to_string(),
    })
}

pub fn create_project(connection: &Connection, input: CreateProjectInput) -> Result<Project> {
    let inspection = inspect_project_directory(Path::new(&input.root_path))?;
    let output_dir = input
        .output_dir
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            let path = PathBuf::from(value);
            if path.is_absolute() {
                path
            } else {
                PathBuf::from(&inspection.root_path).join(path)
            }
        })
        .unwrap_or_else(|| PathBuf::from(&inspection.output_dir));

    let id = Uuid::new_v4().to_string();
    let now = now_rfc3339();
    connection.execute(
        "INSERT INTO projects (
            id, name, root_path, build_tool, output_dir, gradle_task,
            auto_build, auto_deploy, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![
            id,
            input.name.trim().if_empty_then(&inspection.name),
            inspection.root_path,
            inspection.build_tool,
            output_dir.to_string_lossy(),
            input.gradle_task.unwrap_or(inspection.gradle_task),
            bool_to_int(input.auto_build.unwrap_or(false)),
            bool_to_int(input.auto_deploy.unwrap_or(true)),
            now,
        ],
    )?;

    if let Some(instance_ids) = input.instance_ids {
        for instance_id in instance_ids {
            link_project_instance(connection, &id, &instance_id, true)?;
        }
    }

    get_project(connection, &id)?.context("工程创建后无法读取")
}

pub fn list_projects(connection: &Connection) -> Result<Vec<Project>> {
    let mut statement = connection.prepare(&format!(
        "SELECT {PROJECT_COLUMNS} FROM projects ORDER BY updated_at DESC, name COLLATE NOCASE"
    ))?;
    let rows = statement.query_map([], row_to_project)?;
    let mut projects = Vec::new();
    for row in rows {
        projects.push(row?);
    }
    Ok(projects)
}

pub fn get_project(connection: &Connection, id: &str) -> Result<Option<Project>> {
    connection
        .query_row(
            &format!("SELECT {PROJECT_COLUMNS} FROM projects WHERE id = ?1"),
            params![id],
            row_to_project,
        )
        .optional()
        .context("读取工程失败")
}

pub fn update_project(
    connection: &Connection,
    id: &str,
    input: UpdateProjectInput,
) -> Result<Project> {
    let existing = get_project(connection, id)?.context("工程不存在")?;
    connection.execute(
        "UPDATE projects
         SET name = ?1, output_dir = ?2, gradle_task = ?3,
             auto_build = ?4, auto_deploy = ?5, updated_at = ?6
         WHERE id = ?7",
        params![
            input.name.unwrap_or(existing.name),
            input.output_dir.unwrap_or(existing.output_dir),
            input.gradle_task.unwrap_or(existing.gradle_task),
            bool_to_int(input.auto_build.unwrap_or(existing.auto_build)),
            bool_to_int(input.auto_deploy.unwrap_or(existing.auto_deploy)),
            now_rfc3339(),
            id,
        ],
    )?;
    get_project(connection, id)?.context("工程更新后无法读取")
}

pub fn delete_project(connection: &Connection, id: &str) -> Result<()> {
    connection.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn link_project_instance(
    connection: &Connection,
    project_id: &str,
    instance_id: &str,
    auto_deploy: bool,
) -> Result<()> {
    if get_project(connection, project_id)?.is_none() {
        bail!("工程不存在");
    }
    let instance_exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM instances WHERE id = ?1)",
        params![instance_id],
        |row| row.get(0),
    )?;
    if !instance_exists {
        bail!("实例不存在");
    }

    connection.execute(
        "INSERT INTO project_instance_links (project_id, instance_id, auto_deploy)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(project_id, instance_id)
         DO UPDATE SET auto_deploy = excluded.auto_deploy",
        params![project_id, instance_id, bool_to_int(auto_deploy)],
    )?;
    Ok(())
}

pub fn unlink_project_instance(
    connection: &Connection,
    project_id: &str,
    instance_id: &str,
) -> Result<()> {
    connection.execute(
        "DELETE FROM project_instance_links WHERE project_id = ?1 AND instance_id = ?2",
        params![project_id, instance_id],
    )?;
    Ok(())
}

pub fn list_project_instances(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<LinkedProjectInstance>> {
    let mut statement = connection.prepare(
        "SELECT l.project_id, l.instance_id, i.name, l.auto_deploy
         FROM project_instance_links l
         JOIN instances i ON i.id = l.instance_id
         WHERE l.project_id = ?1
         ORDER BY i.name COLLATE NOCASE",
    )?;
    let rows = statement.query_map(params![project_id], |row| {
        Ok(LinkedProjectInstance {
            project_id: row.get(0)?,
            instance_id: row.get(1)?,
            instance_name: row.get(2)?,
            auto_deploy: row.get::<_, i64>(3)? != 0,
        })
    })?;
    let mut links = Vec::new();
    for row in rows {
        links.push(row?);
    }
    Ok(links)
}

fn detect_project_name(root: &Path) -> Option<String> {
    let regex = Regex::new(r#"rootProject\.name\s*=\s*["']([^"']+)["']"#).ok()?;
    for file in ["settings.gradle", "settings.gradle.kts"] {
        let Ok(text) = std::fs::read_to_string(root.join(file)) else {
            continue;
        };
        if let Some(captures) = regex.captures(&text) {
            return captures.get(1).map(|value| value.as_str().to_string());
        }
    }
    None
}

fn detect_output_dir(root: &Path) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    for file in ["build.gradle", "build.gradle.kts"] {
        let path = root.join(file);
        if let Ok(text) = std::fs::read_to_string(path) {
            candidates.push(text);
        }
    }
    let text = candidates.join("\n");

    for pattern in [
        r#"buildDir\s*=\s*(?:file\()?["']([^"']+)["']"#,
        r#"layout\.buildDirectory\.set\((?:layout\.projectDirectory\.dir\()?["']([^"']+)["']"#,
    ] {
        let regex = Regex::new(pattern)?;
        if let Some(captures) = regex.captures(&text) {
            if let Some(value) = captures.get(1).map(|value| value.as_str()) {
                let path = PathBuf::from(value);
                let path = if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                };
                return Ok(path);
            }
        }
    }

    Ok(root.join("build").join("libs"))
}

fn row_to_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        root_path: row.get(2)?,
        build_tool: row.get(3)?,
        output_dir: row.get(4)?,
        gradle_task: row.get(5)?,
        auto_build: row.get::<_, i64>(6)? != 0,
        auto_deploy: row.get::<_, i64>(7)? != 0,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

trait EmptyThen {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl EmptyThen for &str {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.trim().is_empty() {
            fallback.to_string()
        } else {
            self.trim().to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_default_output_directory() {
        let root = std::env::temp_dir().join(format!("emcl-project-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("build.gradle"), "plugins { id 'fabric-loom' }").unwrap();
        assert_eq!(
            detect_output_dir(&root).unwrap(),
            root.join("build").join("libs")
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
