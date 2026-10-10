use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection};

use crate::core::mods::{self, ModInfo};
use crate::core::{artifact, conflict, metadata};
use crate::db::models::{BuildArtifact, Instance};

pub fn deploy_artifact(
    connection: &Connection,
    instance: &Instance,
    artifact: &BuildArtifact,
) -> Result<Vec<ModInfo>> {
    let source = PathBuf::from(&artifact.file_path);
    if !source.is_file() {
        bail!("构建产物不存在：{}", source.display());
    }

    let findings = conflict::check_instance_conflicts(connection, instance, Some(&source))?;
    let blockers = findings
        .iter()
        .filter(|finding| finding.severity == "error")
        .map(|finding| finding.detail.as_str())
        .collect::<Vec<_>>();
    if !blockers.is_empty() {
        bail!("部署被冲突检测拦截：{}", blockers.join("；"));
    }

    let mods_dir = PathBuf::from(&instance.directory).join("mods");
    std::fs::create_dir_all(&mods_dir)?;
    let file_name = source.file_name().context("构建产物没有文件名")?;
    let destination = mods_dir.join(file_name);
    std::fs::copy(&source, &destination)
        .with_context(|| format!("无法部署构建产物到 {}", destination.display()))?;

    remove_old_project_artifacts(connection, instance, artifact, &mods_dir)?;
    let mut deployed = artifact.deployed_instance_ids.clone();
    if !deployed.contains(&instance.id) {
        deployed.push(instance.id.clone());
    }
    set_deployed_instances(connection, &artifact.id, &deployed, "deployed")?;
    metadata::refresh_instance_mod_metadata(connection, instance)?;

    mods::list_mods(
        &crate::core::paths::LauncherPaths::from_root(PathBuf::from(".")),
        instance,
    )
}

pub fn deploy_latest(
    connection: &Connection,
    instance: &Instance,
    project_id: &str,
) -> Result<Vec<ModInfo>> {
    let artifacts = artifact::list_build_artifacts(connection, project_id)?;
    let latest = artifacts.first().context("工程还没有可部署的构建产物")?;
    deploy_artifact(connection, instance, latest)
}

fn remove_old_project_artifacts(
    connection: &Connection,
    instance: &Instance,
    current: &BuildArtifact,
    mods_dir: &Path,
) -> Result<()> {
    for old in artifact::list_build_artifacts(connection, &current.project_id)? {
        if old.id == current.id || !old.deployed_instance_ids.contains(&instance.id) {
            continue;
        }
        let old_path = PathBuf::from(&old.file_path);
        if let Some(file_name) = old_path.file_name() {
            let target = mods_dir.join(file_name);
            if target.is_file() {
                std::fs::remove_file(&target)?;
            }
        }
        let deployed = old
            .deployed_instance_ids
            .into_iter()
            .filter(|id| id != &instance.id)
            .collect::<Vec<_>>();
        set_deployed_instances(connection, &old.id, &deployed, &old.status)?;
    }
    Ok(())
}

fn set_deployed_instances(
    connection: &Connection,
    artifact_id: &str,
    deployed: &[String],
    status: &str,
) -> Result<()> {
    connection.execute(
        "UPDATE build_artifacts SET deployed_instance_ids = ?1, status = ?2 WHERE id = ?3",
        params![serde_json::to_string(deployed)?, status, artifact_id],
    )?;
    Ok(())
}
