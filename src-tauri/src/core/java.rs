#![allow(dead_code)]

use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result};
use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use walkdir::WalkDir;

use crate::core::version::VersionDetails;
use crate::db::models::{Instance, Settings};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JavaInstallation {
    pub path: String,
    pub java_home: Option<String>,
    pub version: Option<String>,
    pub major_version: Option<u32>,
    pub vendor: Option<String>,
    pub architecture: Option<String>,
    pub is_64_bit: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRecommendation {
    pub required_major: Option<u32>,
    pub installation: Option<JavaInstallation>,
    pub reason: String,
}

pub async fn detect_java_installations() -> Result<Vec<JavaInstallation>> {
    let candidates = candidate_java_paths();
    let mut installations = stream::iter(candidates)
        .map(|path| async move { inspect_java(&path).await })
        .buffer_unordered(8)
        .filter_map(|result| async move { result.ok() })
        .collect::<Vec<_>>()
        .await;

    installations.sort_by(|left, right| {
        right
            .major_version
            .unwrap_or(0)
            .cmp(&left.major_version.unwrap_or(0))
            .then_with(|| left.path.cmp(&right.path))
    });
    installations.dedup_by(|left, right| {
        normalize_path(Path::new(&left.path)) == normalize_path(Path::new(&right.path))
    });
    Ok(installations)
}

pub async fn inspect_java(path: &Path) -> Result<JavaInstallation> {
    let mut command = Command::new(path);
    command
        .arg("-XshowSettings:properties")
        .arg("-version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        command.creation_flags(0x0800_0000);
    }

    let output = command
        .output()
        .await
        .with_context(|| format!("failed to run {}", path.display()))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = format!("{stdout}\n{stderr}");

    let java_home = property(&text, "java.home").map(PathBuf::from);
    let version = property(&text, "java.version").or_else(|| parse_version_line(&text));
    let vendor = property(&text, "java.vendor");
    let architecture = property(&text, "os.arch");
    let major_version = version.as_deref().and_then(parse_major_version);
    let is_64_bit = architecture
        .as_deref()
        .map(|arch| {
            let arch = arch.to_ascii_lowercase();
            arch.contains("64") || arch.contains("amd64") || arch.contains("aarch64")
        })
        .unwrap_or(false);

    Ok(JavaInstallation {
        path: normalize_path(path).to_string_lossy().into_owned(),
        java_home: java_home.map(|path| path.to_string_lossy().into_owned()),
        version,
        major_version,
        vendor,
        architecture,
        is_64_bit,
    })
}

pub fn recommend_java(
    installations: &[JavaInstallation],
    details: &VersionDetails,
) -> JavaRecommendation {
    let required_major = details.java_version.as_ref().map(|java| java.major_version);
    let mut candidates = installations.to_vec();
    candidates.sort_by_key(|installation| {
        (
            installation.is_64_bit,
            installation.major_version.unwrap_or(0),
        )
    });

    let installation = match required_major {
        Some(required) => candidates
            .iter()
            .filter(|candidate| candidate.major_version.unwrap_or(0) >= required)
            .min_by_key(|candidate| candidate.major_version.unwrap_or(u32::MAX))
            .cloned()
            .or_else(|| candidates.last().cloned()),
        None => candidates
            .iter()
            .max_by_key(|candidate| candidate.major_version)
            .cloned(),
    };

    let reason = match (&installation, required_major) {
        (Some(installation), Some(required)) => format!(
            "Minecraft {} 需要 Java {}，已选择 Java {}",
            details.id,
            required,
            installation.major_version.unwrap_or(0)
        ),
        (Some(installation), None) => format!(
            "该版本未声明最低 Java 版本，已选择检测到的 Java {}",
            installation.major_version.unwrap_or(0)
        ),
        (None, Some(required)) => format!("未检测到可用的 Java {}，请先安装或手动选择", required),
        (None, None) => "未检测到可用的 Java，请先安装或手动选择".to_string(),
    };

    JavaRecommendation {
        required_major,
        installation,
        reason,
    }
}

fn candidate_java_paths() -> Vec<PathBuf> {
    let executable = if cfg!(windows) { "java.exe" } else { "java" };
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    let mut push = |path: PathBuf| {
        let normalized = normalize_path(&path);
        if seen.insert(normalized.clone()) && normalized.is_file() {
            candidates.push(normalized);
        }
    };

    if let Some(java_home) = env::var_os("JAVA_HOME") {
        push(PathBuf::from(java_home).join("bin").join(executable));
    }

    if let Some(path_value) = env::var_os("PATH") {
        for directory in env::split_paths(&path_value) {
            push(directory.join(executable));
        }
    }

    for root in java_search_roots() {
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&root)
            .max_depth(4)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            if entry.file_type().is_file()
                && entry.file_name() == OsStr::new(executable)
                && entry
                    .path()
                    .parent()
                    .and_then(Path::file_name)
                    .is_some_and(|name| name == "bin")
            {
                push(entry.path().to_path_buf());
            }
        }
    }

    candidates.sort();
    candidates.truncate(64);
    candidates
}

fn java_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    #[cfg(windows)]
    {
        for variable in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
            if let Some(base) = env::var_os(variable).map(PathBuf::from) {
                for vendor in [
                    "Java",
                    "Eclipse Adoptium",
                    "Microsoft",
                    "Zulu",
                    "Amazon Corretto",
                    "BellSoft",
                    "Semeru",
                    "GraalVM",
                    "RedHat",
                    "AdoptOpenJDK",
                    "JetBrains",
                ] {
                    roots.push(base.join(vendor));
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        roots.push(PathBuf::from("/Library/Java/JavaVirtualMachines"));
        roots.push(PathBuf::from("/System/Library/Java/JavaVirtualMachines"));
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join("Library/Java/JavaVirtualMachines"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        for root in [
            "/usr/lib/jvm",
            "/usr/java",
            "/opt/java",
            "/opt/jdk",
            "/snap/jdk/current",
        ] {
            roots.push(PathBuf::from(root));
        }
    }

    roots
}

fn property(output: &str, key: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let line = line.trim();
        let (found_key, value) = line.split_once('=')?;
        (found_key.trim() == key).then(|| value.trim().to_string())
    })
}

fn parse_version_line(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let line = line.trim();
        if !(line.starts_with("java version") || line.starts_with("openjdk version")) {
            return None;
        }
        line.split('"').nth(1).map(ToString::to_string)
    })
}

pub fn parse_major_version(version: &str) -> Option<u32> {
    let version = version.trim();
    if let Some(legacy) = version.strip_prefix("1.") {
        return legacy
            .split(['.', '_', '-'])
            .next()
            .and_then(|part| part.parse().ok());
    }
    version
        .split(['.', '_', '-', '+'])
        .next()
        .and_then(|part| part.parse().ok())
}

fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub async fn resolve_java_path(
    instance: &Instance,
    settings: &Settings,
    details: &VersionDetails,
) -> Result<PathBuf> {
    for candidate in [
        instance.java_path.as_deref(),
        settings.default_java_path.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }

    let installations = detect_java_installations().await?;
    let recommendation = recommend_java(&installations, details);
    recommendation
        .installation
        .map(|installation| PathBuf::from(installation.path))
        .ok_or_else(|| anyhow::anyhow!(recommendation.reason))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modern_and_legacy_java_versions() {
        assert_eq!(parse_major_version("21.0.8"), Some(21));
        assert_eq!(parse_major_version("1.8.0_461"), Some(8));
        assert_eq!(parse_major_version("17.0.15+6-LTS"), Some(17));
    }

    #[test]
    fn parses_property_output() {
        let output = "    java.home = C:\\Java\\jdk-21\n    java.version = 21.0.8\n";
        assert_eq!(property(output, "java.version").as_deref(), Some("21.0.8"));
    }
}
