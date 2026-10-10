use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::core::metadata;
use crate::db::models::{BuildArtifact, Project};

const ARTIFACT_COLUMNS: &str = "id, project_id, file_path, sha256, file_size, built_at,
    git_commit, git_branch, status, deployed_instance_ids";

#[derive(Debug, Clone)]
struct CandidateArtifact {
    path: PathBuf,
    metadata: metadata::ParsedModMetadata,
    modified: std::time::SystemTime,
    sha256: String,
    file_size: u64,
}

pub fn scan_project_artifacts(
    connection: &Connection,
    project: &Project,
) -> Result<Vec<BuildArtifact>> {
    let output_dir = PathBuf::from(&project.output_dir);
    if !output_dir.is_dir() {
        bail!("输出目录不存在：{}", output_dir.display());
    }

    let mut candidates = Vec::new();
    for entry in WalkDir::new(&output_dir)
        .max_depth(3)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_candidate_jar(path, &project.name) {
            continue;
        }
        let Some(parsed) = metadata::parse_jar_metadata(path)? else {
            continue;
        };
        let (sha256, file_size) = sha256_file(path)?;
        candidates.push(CandidateArtifact {
            path: path.to_path_buf(),
            metadata: parsed,
            modified: entry.metadata()?.modified()?,
            sha256,
            file_size,
        });
    }

    if candidates.is_empty() {
        bail!("没有在输出目录找到可识别的主 jar");
    }

    candidates.sort_by(|left, right| {
        candidate_score(right, &project.name).cmp(&candidate_score(left, &project.name))
    });
    let candidate = candidates.remove(0);
    let (git_commit, git_branch) = read_git_info(Path::new(&project.root_path));
    let built_at = DateTime::<Utc>::from(candidate.modified).to_rfc3339();
    let existing = connection
        .query_row(
            "SELECT id, deployed_instance_ids FROM build_artifacts
             WHERE project_id = ?1 AND sha256 = ?2",
            params![project.id, candidate.sha256],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let (id, deployed_json) =
        existing.unwrap_or_else(|| (Uuid::new_v4().to_string(), "[]".to_string()));

    connection.execute(
        "INSERT INTO build_artifacts (
            id, project_id, file_path, sha256, file_size, built_at,
            git_commit, git_branch, status, deployed_instance_ids
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'ready', ?9)
         ON CONFLICT(project_id, sha256) DO UPDATE SET
            file_path = excluded.file_path,
            file_size = excluded.file_size,
            built_at = excluded.built_at,
            git_commit = excluded.git_commit,
            git_branch = excluded.git_branch,
            status = 'ready'",
        params![
            id,
            project.id,
            candidate.path.to_string_lossy(),
            candidate.sha256,
            candidate.file_size as i64,
            built_at,
            git_commit,
            git_branch,
            deployed_json,
        ],
    )?;

    list_build_artifacts(connection, &project.id)
}

pub fn list_build_artifacts(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<BuildArtifact>> {
    let mut statement = connection.prepare(&format!(
        "SELECT {ARTIFACT_COLUMNS} FROM build_artifacts
         WHERE project_id = ?1 ORDER BY built_at DESC"
    ))?;
    let rows = statement.query_map(params![project_id], row_to_artifact)?;
    let mut artifacts = Vec::new();
    for row in rows {
        artifacts.push(row?);
    }
    Ok(artifacts)
}

pub fn get_build_artifact(connection: &Connection, id: &str) -> Result<Option<BuildArtifact>> {
    connection
        .query_row(
            &format!("SELECT {ARTIFACT_COLUMNS} FROM build_artifacts WHERE id = ?1"),
            params![id],
            row_to_artifact,
        )
        .optional()
        .context("读取构建产物失败")
}

fn row_to_artifact(row: &rusqlite::Row<'_>) -> rusqlite::Result<BuildArtifact> {
    let deployed_json: String = row.get(9)?;
    Ok(BuildArtifact {
        id: row.get(0)?,
        project_id: row.get(1)?,
        file_path: row.get(2)?,
        sha256: row.get(3)?,
        file_size: row.get::<_, i64>(4)? as u64,
        built_at: row.get(5)?,
        git_commit: row.get(6)?,
        git_branch: row.get(7)?,
        status: row.get(8)?,
        deployed_instance_ids: serde_json::from_str(&deployed_json).unwrap_or_default(),
    })
}

fn is_candidate_jar(path: &Path, _project_name: &str) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    let lower = name.to_ascii_lowercase();
    if !lower.ends_with(".jar") {
        return false;
    }
    let excluded = [
        "-sources.jar",
        "-javadoc.jar",
        "-dev.jar",
        "-slim.jar",
        "-api.jar",
        "sources.jar",
        "javadoc.jar",
    ];
    if excluded.iter().any(|suffix| lower.ends_with(suffix)) {
        return false;
    }
    !(lower.contains("sources") || lower.contains("javadoc"))
}

fn candidate_score(candidate: &CandidateArtifact, project_name: &str) -> (u8, u8, u64, i64) {
    let name = candidate
        .path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let normalized = normalize_name(project_name);
    (
        if candidate.metadata.mod_id.is_some() {
            1
        } else {
            0
        },
        if !normalized.is_empty() && name.contains(&normalized) {
            1
        } else {
            0
        },
        candidate.file_size,
        candidate
            .modified
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0),
    )
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn sha256_file(path: &Path) -> Result<(String, u64)> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let size = std::fs::metadata(path)?.len();
    Ok((format!("{:x}", hasher.finalize()), size))
}

fn read_git_info(root: &Path) -> (Option<String>, Option<String>) {
    let git_dir = root.join(".git");
    if !git_dir.is_dir() {
        return (None, None);
    }
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok();
    let Some(head) = head.map(|value| value.trim().to_string()) else {
        return (None, None);
    };

    if let Some(reference) = head.strip_prefix("ref: ") {
        let commit = std::fs::read_to_string(git_dir.join(reference))
            .ok()
            .map(|value| value.trim().to_string())
            .or_else(|| read_packed_ref(&git_dir, reference));
        let branch = reference
            .strip_prefix("refs/heads/")
            .map(ToString::to_string);
        (commit, branch)
    } else {
        (Some(head), None)
    }
}

fn read_packed_ref(git_dir: &Path, reference: &str) -> Option<String> {
    let packed = std::fs::read_to_string(git_dir.join("packed-refs")).ok()?;
    packed.lines().find_map(|line| {
        let (commit, name) = line.split_once(' ')?;
        (name == reference).then(|| commit.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_gradle_auxiliary_jars() {
        assert!(is_candidate_jar(
            Path::new("example-mod-1.jar"),
            "example-mod"
        ));
        assert!(!is_candidate_jar(
            Path::new("example-mod-1-sources.jar"),
            "example-mod"
        ));
        assert!(!is_candidate_jar(
            Path::new("example-mod-1-dev.jar"),
            "example-mod"
        ));
        assert!(is_candidate_jar(Path::new("other-1.jar"), "example-mod"));
    }

    #[test]
    fn reads_git_head_without_git_command() {
        let root = std::env::temp_dir().join(format!("emcl-git-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
        std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(root.join(".git/refs/heads/main"), "abc123\n").unwrap();
        assert_eq!(
            read_git_info(&root),
            (Some("abc123".to_string()), Some("main".to_string()))
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
