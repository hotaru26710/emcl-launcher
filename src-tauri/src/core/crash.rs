use std::path::PathBuf;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::db::models::Instance;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashAnalysis {
    pub category: String,
    pub summary: String,
    pub severity: String,
    pub matched_lines: Vec<String>,
    pub suggestion: String,
}

pub fn analyze_latest_log(instance: &Instance) -> Result<CrashAnalysis> {
    let log_path = PathBuf::from(&instance.directory)
        .join("logs")
        .join("latest.log");
    if !log_path.is_file() {
        bail!("实例还没有 latest.log");
    }
    let text = std::fs::read_to_string(&log_path)?;
    Ok(analyze_text(&text))
}

fn analyze_text(text: &str) -> CrashAnalysis {
    let rules = [
        (
            "mixin",
            "Mixin 注入失败",
            "error",
            &[
                "Mixin apply failed",
                "InjectionError",
                "Mixin injection",
                "mixin.injection",
            ][..],
            "检查 Mixin 配置、目标版本和是否有多个模组修改同一个类。",
        ),
        (
            "missing_dependency",
            "缺失依赖或依赖版本不满足",
            "error",
            &[
                "Missing or unsupported mandatory dependencies",
                "Missing dependency",
                "requires",
                "NoClassDefFoundError",
            ][..],
            "检查冲突报告，安装缺失的前置模组并确认版本范围。",
        ),
        (
            "java_version",
            "Java 版本不兼容",
            "error",
            &[
                "UnsupportedClassVersionError",
                "requires Java",
                "java.lang.UnsupportedClassVersionError",
            ][..],
            "在实例设置中选择与 Minecraft/加载器匹配的 Java 版本。",
        ),
        (
            "memory",
            "内存不足或 JVM 内存配置异常",
            "error",
            &[
                "OutOfMemoryError",
                "Could not reserve enough space",
                "insufficient memory",
            ][..],
            "降低模组数量或在实例设置中提高内存，确认系统物理内存足够。",
        ),
        (
            "duplicate_mod",
            "存在重复模组或重复类",
            "warning",
            &[
                "Duplicate mods",
                "DuplicateModsFoundException",
                "duplicate class",
            ][..],
            "移除重复或版本不同的同一个模组。",
        ),
    ];

    for (category, summary, severity, needles, suggestion) in rules {
        let matched_lines = text
            .lines()
            .filter(|line| needles.iter().any(|needle| line.contains(needle)))
            .take(30)
            .map(|line| line.trim().to_string())
            .collect::<Vec<_>>();
        if !matched_lines.is_empty() {
            return CrashAnalysis {
                category: category.to_string(),
                summary: summary.to_string(),
                severity: severity.to_string(),
                matched_lines,
                suggestion: suggestion.to_string(),
            };
        }
    }

    let matched_lines = text
        .lines()
        .filter(|line| {
            line.contains("ERROR") || line.contains("Exception") || line.contains("Caused by:")
        })
        .take(30)
        .map(|line| line.trim().to_string())
        .collect::<Vec<_>>();
    CrashAnalysis {
        category: "unknown".to_string(),
        summary: if matched_lines.is_empty() {
            "没有发现明确错误".to_string()
        } else {
            "发现未分类异常".to_string()
        },
        severity: "info".to_string(),
        matched_lines,
        suggestion: "查看完整 latest.log，并确认加载器和模组版本匹配。".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_mixin_failure() {
        let analysis = analyze_text("Mixin apply failed example.mixins.json");
        assert_eq!(analysis.category, "mixin");
    }
}
