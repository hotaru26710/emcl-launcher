use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::core::metadata::{self, ParsedModMetadata};
use crate::db::models::{now_rfc3339, ConflictReport, Instance, ModMetadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictFinding {
    pub severity: String,
    pub conflict_type: String,
    pub source_mod_id: Option<String>,
    pub target_mod_id: Option<String>,
    pub detail: String,
    pub suggestion: String,
}

const PLATFORM_DEPENDENCIES: &[&str] = &[
    "minecraft",
    "forge",
    "neoforge",
    "fabric",
    "fabricloader",
    "fabric-loader",
    "quilt",
    "quilt_loader",
    "quilt-loader",
    "java",
    "javafml",
    "fml",
    "minecraftforge",
];

pub fn check_instance_conflicts(
    connection: &Connection,
    instance: &Instance,
    candidate: Option<&Path>,
) -> Result<Vec<ConflictFinding>> {
    metadata::refresh_instance_mod_metadata(connection, instance)?;
    let mut mods = metadata::list_mod_metadata(connection, &instance.id)?;

    if let Some(path) = candidate {
        if let Some(parsed) = metadata::parse_jar_metadata(path)? {
            mods.push(metadata_from_candidate(instance, path, parsed));
        }
    }

    let mut findings = detect_conflicts(&mods);
    findings.extend(detect_known_conflicts(&mods));
    persist_reports(connection, &instance.id, &findings)?;
    Ok(findings)
}

pub fn list_conflict_reports(
    connection: &Connection,
    instance_id: &str,
) -> Result<Vec<ConflictReport>> {
    let mut statement = connection.prepare(
        "SELECT id, instance_id, severity, type, source_mod_id, target_mod_id,
                detail, suggestion, created_at
         FROM conflict_reports WHERE instance_id = ?1
         ORDER BY created_at DESC, id DESC LIMIT 200",
    )?;
    let rows = statement.query_map(params![instance_id], |row| {
        Ok(ConflictReport {
            id: row.get(0)?,
            instance_id: row.get(1)?,
            severity: row.get(2)?,
            conflict_type: row.get(3)?,
            source_mod_id: row.get(4)?,
            target_mod_id: row.get(5)?,
            detail: row.get(6)?,
            suggestion: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?;
    let mut reports = Vec::new();
    for row in rows {
        reports.push(row?);
    }
    Ok(reports)
}

fn detect_conflicts(mods: &[ModMetadata]) -> Vec<ConflictFinding> {
    let mut findings = Vec::new();
    let by_id = mods
        .iter()
        .filter_map(|metadata| {
            metadata
                .mod_id
                .as_ref()
                .map(|id| (id.to_ascii_lowercase(), metadata))
        })
        .collect::<HashMap<_, _>>();

    for metadata in mods {
        let source_id = metadata
            .mod_id
            .clone()
            .unwrap_or_else(|| metadata.file_name.clone());
        for (target, range) in parse_map(&metadata.depends_json) {
            if is_platform_dependency(&target) {
                continue;
            }
            let Some(dependency) = by_id.get(&target.to_ascii_lowercase()) else {
                findings.push(ConflictFinding {
                    severity: "error".to_string(),
                    conflict_type: "missing_dependency".to_string(),
                    source_mod_id: Some(source_id.clone()),
                    target_mod_id: Some(target.clone()),
                    detail: format!("{} 依赖 {}，但实例中未安装", source_id, target),
                    suggestion: format!("安装或启用 {} {}", target, range),
                });
                continue;
            };
            if !version_matches(&range, dependency.version.as_deref().unwrap_or("")) {
                findings.push(ConflictFinding {
                    severity: "error".to_string(),
                    conflict_type: "dependency_version".to_string(),
                    source_mod_id: Some(source_id.clone()),
                    target_mod_id: Some(target.clone()),
                    detail: format!(
                        "{} 要求 {} {}，当前版本为 {}",
                        source_id,
                        target,
                        range,
                        dependency.version.as_deref().unwrap_or("未知")
                    ),
                    suggestion: format!("升级 {} 到满足 {} 的版本", target, range),
                });
            }
        }

        for (url_type, json) in [
            ("explicit_conflict", &metadata.conflicts_json),
            ("explicit_break", &metadata.breaks_json),
        ] {
            for (target, _) in parse_map(json) {
                if by_id.contains_key(&target.to_ascii_lowercase()) {
                    findings.push(ConflictFinding {
                        severity: "error".to_string(),
                        conflict_type: url_type.to_string(),
                        source_mod_id: Some(source_id.clone()),
                        target_mod_id: Some(target.clone()),
                        detail: format!("{} 声明与 {} 冲突", source_id, target),
                        suggestion: "禁用其中一个模组，或按模组作者说明调整版本".to_string(),
                    });
                }
            }
        }
    }

    findings
}

fn detect_known_conflicts(mods: &[ModMetadata]) -> Vec<ConflictFinding> {
    let ids = mods
        .iter()
        .filter_map(|metadata| metadata.mod_id.as_ref())
        .map(|id| id.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut findings = Vec::new();
    for (left, right) in [
        ("jei", "rei"),
        ("journeymap", "xaero-waypoint"),
        ("optifine", "sodium"),
        ("optifine", "iris"),
    ] {
        if ids.iter().any(|id| id == left) && ids.iter().any(|id| id == right) {
            findings.push(ConflictFinding {
                severity: "warning".to_string(),
                conflict_type: "known_conflict".to_string(),
                source_mod_id: Some(left.to_string()),
                target_mod_id: Some(right.to_string()),
                detail: format!("{} 与 {} 同时安装时可能产生功能冲突", left, right),
                suggestion: "只保留其中一个，或查阅兼容补丁".to_string(),
            });
        }
    }
    findings
}

fn metadata_from_candidate(
    instance: &Instance,
    path: &Path,
    parsed: ParsedModMetadata,
) -> ModMetadata {
    ModMetadata {
        instance_id: instance.id.clone(),
        mod_id: parsed.mod_id,
        mod_name: parsed.mod_name,
        version: parsed.version,
        loader: parsed.loader,
        file_name: path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| "candidate.jar".to_string()),
        sha256: metadata::sha256_file(path).unwrap_or_default(),
        source_type: "build".to_string(),
        depends_json: serde_json::to_string(&parsed.depends).unwrap_or_else(|_| "{}".to_string()),
        breaks_json: serde_json::to_string(&parsed.breaks).unwrap_or_else(|_| "{}".to_string()),
        conflicts_json: serde_json::to_string(&parsed.conflicts)
            .unwrap_or_else(|_| "{}".to_string()),
    }
}

fn persist_reports(
    connection: &Connection,
    instance_id: &str,
    findings: &[ConflictFinding],
) -> Result<()> {
    for finding in findings {
        let existing = connection
            .query_row(
                "SELECT id FROM conflict_reports
                 WHERE instance_id = ?1 AND severity = ?2 AND type = ?3
                   AND source_mod_id IS ?4 AND target_mod_id IS ?5
                 LIMIT 1",
                params![
                    instance_id,
                    finding.severity,
                    finding.conflict_type,
                    finding.source_mod_id.as_deref(),
                    finding.target_mod_id.as_deref()
                ],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        let now = now_rfc3339();
        if let Some(id) = existing {
            connection.execute(
                "UPDATE conflict_reports
                 SET detail = ?1, suggestion = ?2, created_at = ?3
                 WHERE id = ?4",
                params![finding.detail, finding.suggestion, now, id],
            )?;
        } else {
            connection.execute(
                "INSERT INTO conflict_reports (
                    instance_id, severity, type, source_mod_id, target_mod_id,
                    detail, suggestion, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    instance_id,
                    finding.severity,
                    finding.conflict_type,
                    finding.source_mod_id,
                    finding.target_mod_id,
                    finding.detail,
                    finding.suggestion,
                    now
                ],
            )?;
        }
    }
    Ok(())
}

fn is_platform_dependency(value: &str) -> bool {
    PLATFORM_DEPENDENCIES
        .iter()
        .any(|platform| platform.eq_ignore_ascii_case(value.trim()))
}

fn parse_map(json: &str) -> Vec<(String, String)> {
    serde_json::from_str::<HashMap<String, String>>(json)
        .map(|map| map.into_iter().collect())
        .unwrap_or_default()
}

pub fn version_matches(requirement: &str, actual: &str) -> bool {
    let requirement = requirement.trim();
    if requirement.is_empty() || requirement == "*" {
        return true;
    }
    requirement
        .split("||")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .any(|part| single_requirement_matches(part, actual))
}

fn single_requirement_matches(requirement: &str, actual: &str) -> bool {
    let requirement = requirement.trim();
    if requirement.contains(',') && requirement.starts_with(['[', '(']) {
        let (lower, upper) = parse_range(requirement);
        return lower.as_ref().is_none_or(|(bound, inclusive)| {
            let ordering = version_cmp(actual, bound);
            if *inclusive {
                ordering != Ordering::Less
            } else {
                ordering == Ordering::Greater
            }
        }) && upper.as_ref().is_none_or(|(bound, inclusive)| {
            let ordering = version_cmp(actual, bound);
            if *inclusive {
                ordering != Ordering::Greater
            } else {
                ordering == Ordering::Less
            }
        });
    }
    if let Some(bound) = requirement.strip_prefix(">=") {
        return version_cmp(actual, bound.trim()) != Ordering::Less;
    }
    if let Some(bound) = requirement.strip_prefix("<=") {
        return version_cmp(actual, bound.trim()) != Ordering::Greater;
    }
    if let Some(bound) = requirement.strip_prefix('>') {
        return version_cmp(actual, bound.trim()) == Ordering::Greater;
    }
    if let Some(bound) = requirement.strip_prefix('<') {
        return version_cmp(actual, bound.trim()) == Ordering::Less;
    }
    if let Some(bound) = requirement.strip_prefix('=') {
        return version_cmp(actual, bound.trim()) == Ordering::Equal;
    }
    if requirement.contains('x') || requirement.contains('X') || requirement.contains('*') {
        let prefix = requirement
            .split(['x', 'X', '*'])
            .next()
            .unwrap_or("")
            .trim_end_matches('.');
        return actual == prefix || actual.starts_with(&format!("{prefix}."));
    }
    version_cmp(actual, requirement) == Ordering::Equal
}

type RangeBound = Option<(String, bool)>;

fn parse_range(value: &str) -> (RangeBound, RangeBound) {
    let start_inclusive = value.starts_with('[');
    let end_inclusive = value.ends_with(']');
    let inner = value.trim_matches(['[', ']', '(', ')']);
    let mut parts = inner.split(',');
    let lower = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| (value.to_string(), start_inclusive));
    let upper = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| (value.to_string(), end_inclusive));
    (lower, upper)
}

fn version_cmp(left: &str, right: &str) -> Ordering {
    parse_version(left).cmp(&parse_version(right))
}

fn parse_version(value: &str) -> Vec<u64> {
    value
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_platform_dependencies() {
        let metadata = ModMetadata {
            instance_id: "instance".to_string(),
            mod_id: Some("re_mc".to_string()),
            mod_name: Some("Re MC".to_string()),
            version: Some("1.0.0".to_string()),
            loader: Some("forge".to_string()),
            file_name: "re_mc.jar".to_string(),
            sha256: String::new(),
            source_type: "build".to_string(),
            depends_json: r#"{"minecraft":">=1.20.1","forge":"[47,)","tectonic":"*"}"#.to_string(),
            breaks_json: "{}".to_string(),
            conflicts_json: "{}".to_string(),
        };
        let findings = detect_conflicts(&[metadata]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].target_mod_id.as_deref(), Some("tectonic"));
    }

    #[test]
    fn compares_common_dependency_ranges() {
        assert!(version_matches(">=0.92.0", "0.92.1"));
        assert!(!version_matches(">=0.92.0", "0.90.0"));
        assert!(version_matches("[1.0,2.0)", "1.5.0"));
        assert!(!version_matches("[1.0,2.0)", "2.0.0"));
    }
}
