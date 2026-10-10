use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::core::metadata;
use crate::core::paths::{is_safe_segment, LauncherPaths};
use crate::db::models::{now_rfc3339, Instance};

const MAX_ARCHIVE_ENTRY_BYTES: u64 = 12 * 1024 * 1024;
const MAX_ARCHIVE_TOTAL_BYTES: u64 = 96 * 1024 * 1024;
const MAX_INSTANCE_FILE_BYTES: u64 = 24 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashAnalysis {
    pub category: String,
    pub summary: String,
    pub severity: String,
    pub matched_lines: Vec<String>,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashAnalysisProgress {
    pub job_id: String,
    pub status: String,
    pub stage: String,
    pub message: String,
    pub percent: f64,
    pub instance_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReportReady {
    pub instance_id: String,
    pub archive_path: String,
    pub report: CrashAnalysisReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashAnalysisReport {
    pub id: String,
    pub source: String,
    pub archive_path: String,
    pub instance_id: Option<String>,
    pub instance_name: Option<String>,
    pub created_at: String,
    pub severity: String,
    pub summary: String,
    pub confidence: u8,
    pub game: CrashGameContext,
    pub findings: Vec<CrashFinding>,
    pub suspected_mods: Vec<SuspectMod>,
    pub recommendations: Vec<String>,
    pub evidence: Vec<String>,
    pub included_files: Vec<String>,
    pub analyzed_lines: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashGameContext {
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub java_version: Option<String>,
    pub operating_system: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashFinding {
    pub category: String,
    pub title: String,
    pub severity: String,
    pub confidence: u8,
    pub evidence: Vec<String>,
    pub explanation: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuspectMod {
    pub mod_id: String,
    pub name: String,
    pub version: Option<String>,
    pub score: u32,
    pub confidence: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AnalysisContext {
    pub source: String,
    pub instance_id: Option<String>,
    pub instance_name: Option<String>,
    pub version_id: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub mods_dir: Option<PathBuf>,
}

impl AnalysisContext {
    pub fn from_instance(instance: &Instance, source: &str) -> Self {
        Self {
            source: source.to_string(),
            instance_id: Some(instance.id.clone()),
            instance_name: Some(instance.name.clone()),
            version_id: Some(instance.version_id.clone()),
            loader: instance.loader.clone(),
            loader_version: instance.loader_version.clone(),
            mods_dir: Some(PathBuf::from(&instance.directory).join("mods")),
        }
    }
}

#[derive(Debug, Clone)]
struct LoadedMod {
    id: String,
    name: String,
    version: Option<String>,
    file_name: Option<String>,
}

#[derive(Debug, Clone)]
struct TextEntry {
    name: String,
    content: String,
}
pub fn capture_crash_archive(
    paths: &LauncherPaths,
    instance: &Instance,
    exit_code: Option<i32>,
    progress: &dyn Fn(&str, &str, f64),
) -> Result<PathBuf> {
    progress("collecting", "正在收集崩溃日志与实例信息", 8.0);
    fs::create_dir_all(paths.crash_reports_dir())?;
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let safe_name = sanitize_component(&instance.name);
    let id_prefix = instance.id.chars().take(8).collect::<String>();
    let archive_path = paths
        .crash_reports_dir()
        .join(format!("{}-{}-{}.zip", safe_name, timestamp, id_prefix));

    let files = collect_instance_text_files(instance)?;
    let mods_list = build_mods_list(instance);
    let context = build_crash_context(instance, exit_code);

    progress("archiving", "正在生成错误报告压缩包", 22.0);
    let file = File::create(&archive_path)
        .with_context(|| format!("无法创建错误报告 {}", archive_path.display()))?;
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    writer.start_file("crash-context.txt", options)?;
    writer.write_all(context.as_bytes())?;
    writer.start_file("mods-list.txt", options)?;
    writer.write_all(mods_list.as_bytes())?;

    for (entry_name, path) in &files {
        let metadata = fs::metadata(path)?;
        if metadata.len() > MAX_INSTANCE_FILE_BYTES {
            continue;
        }
        let Ok(mut source) = File::open(path) else {
            continue;
        };
        let mut content = Vec::new();
        if source.read_to_end(&mut content).is_err() || content.contains(&0) {
            continue;
        }
        writer.start_file(entry_name, options)?;
        writer.write_all(&content)?;
    }
    writer.finish()?;
    progress("archive-ready", "错误报告已导出", 34.0);
    Ok(archive_path)
}

pub fn analyze_crash_archive(
    archive_path: &Path,
    context: AnalysisContext,
    progress: &dyn Fn(&str, &str, f64),
) -> Result<CrashAnalysisReport> {
    if !archive_path.is_file() {
        bail!("错误报告不存在：{}", archive_path.display());
    }

    progress("reading", "正在读取错误报告内容", 42.0);
    let entries = load_text_entries(archive_path)?;
    if entries.is_empty() {
        bail!("压缩包中没有可读取的文本日志");
    }

    progress("parsing", "正在识别游戏、Java 与加载器信息", 56.0);
    let mut combined = String::new();
    let mut included_files = Vec::new();
    for entry in &entries {
        included_files.push(entry.name.clone());
        combined.push_str("\n===== ");
        combined.push_str(&entry.name);
        combined.push_str(" =====\n");
        combined.push_str(&entry.content);
        combined.push('\n');
    }

    let game = parse_game_context(&combined, &context);
    let mods = parse_loaded_mods(&entries, &context);
    progress("rules", "正在运行多维度崩溃规则分析", 68.0);
    let mut findings = detect_findings(&combined);
    findings.extend(detect_known_conflicts(&combined, &mods));
    if findings.is_empty() {
        findings.push(unknown_finding(&combined));
    }

    progress("mods", "正在从堆栈和 Mixin 信息中定位疑似模组", 82.0);
    let suspected_mods = rank_suspect_mods(&combined, &mods, &findings);
    let primary = pick_primary_finding(&findings);
    let mut recommendations = deduplicate(
        findings
            .iter()
            .map(|finding| finding.suggestion.clone())
            .collect::<Vec<_>>(),
    );
    if let Some(suspect) = suspected_mods.first() {
        if suspect.confidence == "high" || suspect.confidence == "medium" {
            recommendations.insert(
                0,
                format!(
                    "先单独禁用 {}，再启动一次；如果崩溃消失，优先检查它与前置模组或游戏版本的兼容性。",
                    display_mod_name(suspect)
                ),
            );
        }
    }
    if recommendations.is_empty() {
        recommendations
            .push("保留本次错误报告，并逐步禁用最近新增的模组进行二分排查。".to_string());
    }

    let confidence = report_confidence(&findings, &suspected_mods);
    let summary = if let Some(suspect) = suspected_mods
        .iter()
        .find(|suspect| suspect.confidence == "high" || suspect.confidence == "medium")
    {
        format!(
            "{}；疑似与 {} 有关",
            primary.title,
            display_mod_name(suspect)
        )
    } else {
        primary.title.clone()
    };
    let evidence = deduplicate(
        findings
            .iter()
            .flat_map(|finding| finding.evidence.clone())
            .take(24)
            .collect::<Vec<_>>(),
    );

    progress("saving", "正在保存分析结果", 94.0);
    let report = CrashAnalysisReport {
        id: uuid::Uuid::new_v4().to_string(),
        source: if context.source.is_empty() {
            "manual".to_string()
        } else {
            context.source.clone()
        },
        archive_path: archive_path.to_string_lossy().into_owned(),
        instance_id: context.instance_id,
        instance_name: context.instance_name,
        created_at: now_rfc3339(),
        severity: highest_severity(&findings).to_string(),
        summary,
        confidence,
        game,
        findings,
        suspected_mods,
        recommendations,
        evidence,
        included_files,
        analyzed_lines: combined.lines().count(),
    };
    Ok(report)
}
pub fn save_crash_report(paths: &LauncherPaths, report: &CrashAnalysisReport) -> Result<()> {
    fs::create_dir_all(paths.crash_reports_dir())?;
    let path = paths
        .crash_reports_dir()
        .join(format!("{}.json", report.id));
    let bytes = serde_json::to_vec_pretty(report)?;
    fs::write(&path, bytes)?;
    Ok(())
}

pub fn list_crash_reports(paths: &LauncherPaths) -> Result<Vec<CrashAnalysisReport>> {
    fs::create_dir_all(paths.crash_reports_dir())?;
    let mut reports = Vec::new();
    for entry in fs::read_dir(paths.crash_reports_dir())? {
        let entry = entry?;
        if !entry.file_type()?.is_file()
            || entry.path().extension().and_then(|value| value.to_str()) != Some("json")
        {
            continue;
        }
        let Ok(bytes) = fs::read(entry.path()) else {
            continue;
        };
        if let Ok(report) = serde_json::from_slice::<CrashAnalysisReport>(&bytes) {
            reports.push(report);
        }
    }
    reports.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    reports.truncate(100);
    Ok(reports)
}

pub fn get_crash_report(paths: &LauncherPaths, id: &str) -> Result<CrashAnalysisReport> {
    if !is_safe_segment(id) {
        bail!("错误报告标识不安全");
    }
    let path = paths.crash_reports_dir().join(format!("{id}.json"));
    let bytes = fs::read(&path).with_context(|| format!("错误报告不存在：{id}"))?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn delete_crash_report(paths: &LauncherPaths, id: &str) -> Result<()> {
    if !is_safe_segment(id) {
        bail!("错误报告标识不安全");
    }
    let path = paths.crash_reports_dir().join(format!("{id}.json"));
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn analyze_latest_log(instance: &Instance) -> Result<CrashAnalysis> {
    let log_path = PathBuf::from(&instance.directory)
        .join("logs")
        .join("latest.log");
    if !log_path.is_file() {
        bail!("实例还没有 latest.log");
    }
    let text = fs::read_to_string(&log_path)?;
    Ok(analyze_text(&text))
}

fn analyze_text(text: &str) -> CrashAnalysis {
    let findings = detect_findings(text);
    let primary = pick_primary_finding(&findings);
    CrashAnalysis {
        category: primary.category.clone(),
        summary: primary.title.clone(),
        severity: primary.severity.clone(),
        matched_lines: primary.evidence.clone(),
        suggestion: primary.suggestion.clone(),
    }
}

fn build_crash_context(instance: &Instance, exit_code: Option<i32>) -> String {
    format!(
        "EMCL Crash Context\ninstance_name={}\ninstance_id={}\nminecraft_version={}\nloader={}\nloader_version={}\njava_path={}\nmemory_mb={}\njvm_args={}\nexit_code={}\nexported_at={}\n",
        instance.name,
        instance.id,
        instance.version_id,
        instance.loader.as_deref().unwrap_or("vanilla"),
        instance.loader_version.as_deref().unwrap_or(""),
        instance.java_path.as_deref().unwrap_or(""),
        instance.memory_mb,
        instance.jvm_args,
        exit_code.map(|value| value.to_string()).unwrap_or_default(),
        now_rfc3339()
    )
}

fn collect_instance_text_files(instance: &Instance) -> Result<Vec<(String, PathBuf)>> {
    let root = PathBuf::from(&instance.directory);
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    for directory in ["logs", "crash-reports"] {
        let path = root.join(directory);
        if !path.exists() {
            continue;
        }
        for entry in WalkDir::new(&path).max_depth(2).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_path_buf();
            if !looks_like_instance_log(&path) || !seen.insert(path.clone()) {
                continue;
            }
            if fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0) > MAX_INSTANCE_FILE_BYTES {
                continue;
            }
            let relative = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            files.push((relative, path));
        }
    }
    for entry in fs::read_dir(&root).into_iter().flatten().flatten() {
        let path = entry.path();
        if !entry
            .file_type()
            .map(|kind| kind.is_file())
            .unwrap_or(false)
        {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if !name.starts_with("hs_err_pid") || !seen.insert(path.clone()) {
            continue;
        }
        files.push((format!("jvm/{name}"), path));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files.truncate(120);
    Ok(files)
}

fn looks_like_instance_log(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("log" | "txt" | "json" | "toml" | "yml" | "yaml" | "properties" | "conf" | "config")
    ) || lower.contains("crash")
        || lower.contains("latest.log")
}
fn build_mods_list(instance: &Instance) -> String {
    let mods_dir = PathBuf::from(&instance.directory).join("mods");
    let mut lines = vec![
        "MOD\tmod_id\tname\tversion\tloader\tfile".to_string(),
        format!(
            "INSTANCE\t{}\t{}\t{}\t{}",
            instance.version_id,
            instance.loader.as_deref().unwrap_or("vanilla"),
            instance.loader_version.as_deref().unwrap_or(""),
            instance.name
        ),
    ];
    let Ok(entries) = fs::read_dir(mods_dir) else {
        return lines.join("\n");
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let lower = file_name.to_ascii_lowercase();
        if !lower.ends_with(".jar") && !lower.ends_with(".jar.disabled") {
            continue;
        }
        let parsed = metadata::parse_jar_metadata(&path).ok().flatten();
        lines.push(format!(
            "MOD\t{}\t{}\t{}\t{}\t{}",
            parsed
                .as_ref()
                .and_then(|value| value.mod_id.clone())
                .unwrap_or_default(),
            parsed
                .as_ref()
                .and_then(|value| value.mod_name.clone())
                .unwrap_or_default(),
            parsed
                .as_ref()
                .and_then(|value| value.version.clone())
                .unwrap_or_default(),
            parsed
                .as_ref()
                .and_then(|value| value.loader.clone())
                .unwrap_or_default(),
            file_name,
        ));
    }
    lines.join("\n")
}

fn load_text_entries(path: &Path) -> Result<Vec<TextEntry>> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension != "zip" {
        let bytes = fs::read(path)?;
        if bytes.len() as u64 > MAX_ARCHIVE_ENTRY_BYTES {
            bail!("错误报告文件过大");
        }
        return Ok(vec![TextEntry {
            name: path
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| "error-report.txt".to_string()),
            content: String::from_utf8_lossy(&bytes).into_owned(),
        }]);
    }

    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file).context("无法读取错误报告压缩包")?;
    let mut entries = Vec::new();
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        if name.starts_with("__MACOSX/") || name.contains("../") {
            continue;
        }
        let size = entry.size();
        if size == 0
            || size > MAX_ARCHIVE_ENTRY_BYTES
            || total.saturating_add(size) > MAX_ARCHIVE_TOTAL_BYTES
        {
            continue;
        }
        let mut bytes = Vec::with_capacity(size.min(MAX_ARCHIVE_ENTRY_BYTES) as usize);
        entry
            .by_ref()
            .take(MAX_ARCHIVE_ENTRY_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_ARCHIVE_ENTRY_BYTES || bytes.contains(&0) {
            continue;
        }
        let Ok(content) = String::from_utf8(bytes) else {
            continue;
        };
        total = total.saturating_add(content.len() as u64);
        entries.push(TextEntry { name, content });
    }
    Ok(entries)
}

fn parse_game_context(text: &str, context: &AnalysisContext) -> CrashGameContext {
    CrashGameContext {
        minecraft_version: context
            .version_id
            .clone()
            .or_else(|| find_value(text, &["Minecraft Version:", "Minecraft version:"])),
        loader: context.loader.clone().or_else(|| detect_loader(text)),
        loader_version: context.loader_version.clone().or_else(|| {
            find_value(
                text,
                &[
                    "Fabric Loader:",
                    "Forge Version:",
                    "NeoForge Version:",
                    "Quilt Loader:",
                ],
            )
        }),
        java_version: find_value(text, &["Java Version:", "Java version:"]),
        operating_system: find_value(text, &["Operating System:", "OS:"]),
    }
}

fn find_value(text: &str, keys: &[&str]) -> Option<String> {
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        keys.iter().find_map(|key| {
            trimmed
                .strip_prefix(key)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
    })
}

fn detect_loader(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("neoforge") {
        Some("neoforge".to_string())
    } else if lower.contains("forge") {
        Some("forge".to_string())
    } else if lower.contains("quilt") {
        Some("quilt".to_string())
    } else if lower.contains("fabric") {
        Some("fabric".to_string())
    } else {
        None
    }
}
fn parse_loaded_mods(entries: &[TextEntry], context: &AnalysisContext) -> Vec<LoadedMod> {
    let mut mods = Vec::new();
    let mut seen = HashSet::new();
    for entry in entries {
        let lower_name = entry.name.to_ascii_lowercase();
        if lower_name.contains("mods-list") || lower_name.contains("mod-list") {
            for line in entry.content.lines() {
                let parts = line.split('\t').collect::<Vec<_>>();
                if parts.len() >= 4 && parts[0] == "MOD" && !parts[1].is_empty() {
                    push_mod(
                        &mut mods,
                        &mut seen,
                        parts[1].to_string(),
                        parts.get(2).unwrap_or(&"").to_string(),
                        non_empty(parts.get(3).copied()),
                        non_empty(parts.get(5).copied()),
                    );
                }
            }
        }
    }

    for entry in entries {
        let mut in_mod_list = false;
        for line in entry.content.lines() {
            let trimmed = line.trim();
            let lower = trimmed.to_ascii_lowercase();
            if lower.contains("mod list")
                || lower == "-- mods --"
                || lower.starts_with("fabric mods")
            {
                in_mod_list = true;
                continue;
            }
            if in_mod_list && (trimmed.is_empty() || trimmed.starts_with("-- ")) {
                if trimmed.starts_with("-- ") && !trimmed.eq_ignore_ascii_case("-- mods --") {
                    in_mod_list = false;
                }
                continue;
            }
            if !in_mod_list {
                continue;
            }
            if let Some((id, name, version)) = parse_mod_line(trimmed) {
                push_mod(&mut mods, &mut seen, id, name, version, None);
            }
        }
    }

    if mods.is_empty() {
        if let Some(mods_dir) = context.mods_dir.as_ref() {
            if let Ok(entries) = fs::read_dir(mods_dir) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().into_owned();
                    let lower = file_name.to_ascii_lowercase();
                    if !lower.ends_with(".jar") && !lower.ends_with(".jar.disabled") {
                        continue;
                    }
                    let parsed = metadata::parse_jar_metadata(&entry.path()).ok().flatten();
                    let id = parsed
                        .as_ref()
                        .and_then(|value| value.mod_id.clone())
                        .unwrap_or_else(|| {
                            file_name
                                .trim_end_matches(".disabled")
                                .trim_end_matches(".jar")
                                .to_string()
                        });
                    push_mod(
                        &mut mods,
                        &mut seen,
                        id,
                        parsed
                            .as_ref()
                            .and_then(|value| value.mod_name.clone())
                            .unwrap_or_else(|| file_name.clone()),
                        parsed.and_then(|value| value.version),
                        Some(file_name),
                    );
                }
            }
        }
    }
    mods
}

fn push_mod(
    mods: &mut Vec<LoadedMod>,
    seen: &mut HashSet<String>,
    id: String,
    name: String,
    version: Option<String>,
    file_name: Option<String>,
) {
    let key = id.to_ascii_lowercase();
    if key.is_empty() || !seen.insert(key) {
        return;
    }
    let display_name = if name.is_empty() { id.clone() } else { name };
    mods.push(LoadedMod {
        id,
        name: display_name,
        version,
        file_name,
    });
}

fn parse_mod_line(line: &str) -> Option<(String, String, Option<String>)> {
    if let Some(rest) = line.strip_prefix("- ") {
        let mut parts = rest.split_whitespace();
        let id = parts.next()?.to_string();
        let version = parts.next().map(ToString::to_string);
        let name = rest.strip_prefix(&id).unwrap_or(rest).trim().to_string();
        return Some((id, name, version));
    }
    let (left, right) = line.split_once(':')?;
    let id = left.trim();
    if id.is_empty() || id.contains(' ') || id.len() > 80 {
        return None;
    }
    let mut parts = right.split_whitespace();
    let name = parts.next().unwrap_or(id).to_string();
    let version = parts.next().map(ToString::to_string);
    Some((id.to_string(), name, version))
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}
fn detect_findings(text: &str) -> Vec<CrashFinding> {
    let rules = vec![
        Rule::new(
            "mixin",
            "Mixin 注入失败",
            "error",
            &[
                "Mixin apply failed",
                "InjectionError",
                "MixinTransformerError",
                "mixin.injection",
            ],
            "某个模组的 Mixin 与当前游戏或其它模组修改了同一个目标类。",
            "保留完整堆栈，定位 Mixin 配置所属模组，并逐个禁用最近更新的模组验证。",
        ),
        Rule::new(
            "missing_dependency",
            "缺失前置依赖",
            "error",
            &[
                "Missing or unsupported mandatory dependencies",
                "NoClassDefFoundError",
                "ClassNotFoundException",
                "Missing dependency",
            ],
            "其中一个模组缺少必需的前置模组，或依赖没有随实例一起加载。",
            "查看缺失的 mod id 与版本，使用自动补全并按报告中的版本范围安装。",
        ),
        Rule::new(
            "version_mismatch",
            "模组与游戏或前置版本不兼容",
            "error",
            &[
                "NoSuchMethodError",
                "NoSuchFieldError",
                "AbstractMethodError",
                "IncompatibleClassChangeError",
            ],
            "模组调用了当前版本中不存在的方法或字段，通常是模组与游戏/加载器/前置版本不匹配。",
            "将崩溃堆栈顶端涉及的模组、前置模组和加载器升级到同一游戏版本分支。",
        ),
        Rule::new(
            "duplicate_mod",
            "重复模组或重复类",
            "error",
            &[
                "Duplicate mods",
                "DuplicateModsFoundException",
                "duplicate class",
                "DuplicateModsFound",
            ],
            "同一个模组存在多个版本，或两个模组打包了冲突的类。",
            "只保留每个模组与前置模组的单一版本，并检查整合包重复文件。",
        ),
        Rule::new(
            "java_version",
            "Java 版本不兼容",
            "error",
            &[
                "UnsupportedClassVersionError",
                "class file version",
                "requires Java",
            ],
            "当前 Java 版本低于游戏或模组要求。",
            "在实例设置中选择与 Minecraft 版本匹配的 Java 版本。",
        ),
        Rule::new(
            "memory",
            "内存不足或 JVM 参数异常",
            "error",
            &[
                "OutOfMemoryError",
                "Could not reserve enough space",
                "insufficient memory",
                "Invalid maximum heap size",
            ],
            "JVM 无法分配所需内存，或内存参数超过系统可用范围。",
            "适当降低实例内存或关闭其它高占用程序，并检查 JVM 参数是否重复设置内存。",
        ),
        Rule::new(
            "graphics",
            "显卡驱动或 OpenGL 初始化失败",
            "error",
            &[
                "EXCEPTION_ACCESS_VIOLATION",
                "GLFW error",
                "OpenGL",
                "Pixel format",
                "nvoglv",
                "atio",
                "ig[0-9].dll",
            ],
            "显卡驱动、OpenGL 状态或图形模组初始化出现问题。",
            "更新显卡驱动，禁用光影和图形优化模组后再测试。",
        ),
        Rule::new(
            "jvm",
            "JVM 启动参数错误",
            "error",
            &[
                "Unrecognized VM option",
                "Could not create the Java Virtual Machine",
                "Error: Could not find or load main class",
            ],
            "启动参数或 Java 安装不完整，游戏主类未能启动。",
            "恢复实例 JVM 参数为默认值，并重新选择有效的 Java 可执行文件。",
        ),
        Rule::new(
            "world_data",
            "世界或存档数据损坏",
            "error",
            &[
                "Chunk file at",
                "corrupted",
                "Failed to load world",
                "DataFixer",
                "Unable to read world",
            ],
            "存档区块或数据修复过程发生错误。",
            "先备份存档；必要时从备份恢复可疑区块，再检查最近新增的世界生成模组。",
        ),
        Rule::new(
            "resource",
            "资源包或着色器加载失败",
            "warning",
            &[
                "ZipException",
                "failed to load resource pack",
                "Shader could not be loaded",
                "Failed to load shader",
            ],
            "资源包、光影包或资源索引存在损坏或版本不兼容。",
            "移除最近加入的资源包或光影，重新下载后验证压缩包是否完整。",
        ),
        Rule::new(
            "loader",
            "加载器或模组加载阶段失败",
            "error",
            &[
                "Failed to load mod",
                "Loading errors encountered",
                "Mod loading error",
            ],
            "加载器在扫描、构造或初始化模组时中止。",
            "查看加载器错误段列出的第一个模组，优先处理它缺失的前置或版本要求。",
        ),
        Rule::new(
            "config",
            "模组配置解析失败",
            "warning",
            &[
                "Failed to parse",
                "JsonParseException",
                "TOML",
                "Config file",
            ],
            "模组配置文件损坏或使用了旧格式。",
            "备份并删除报错中提到的配置文件，让模组重新生成默认配置。",
        ),
    ];

    let mut findings = Vec::new();
    for rule in rules {
        let evidence = text
            .lines()
            .filter(|line| rule.needles.iter().any(|needle| line.contains(needle)))
            .map(compact_line)
            .filter(|line| !line.is_empty())
            .take(6)
            .collect::<Vec<_>>();
        if evidence.is_empty() {
            continue;
        }
        findings.push(CrashFinding {
            category: rule.category.to_string(),
            title: rule.title.to_string(),
            severity: rule.severity.to_string(),
            confidence: (58 + evidence.len().min(4) as u8 * 9).min(94),
            evidence: deduplicate(evidence),
            explanation: rule.explanation.to_string(),
            suggestion: rule.suggestion.to_string(),
        });
    }
    findings
}

struct Rule {
    category: &'static str,
    title: &'static str,
    severity: &'static str,
    needles: &'static [&'static str],
    explanation: &'static str,
    suggestion: &'static str,
}

impl Rule {
    fn new(
        category: &'static str,
        title: &'static str,
        severity: &'static str,
        needles: &'static [&'static str],
        explanation: &'static str,
        suggestion: &'static str,
    ) -> Self {
        Self {
            category,
            title,
            severity,
            needles,
            explanation,
            suggestion,
        }
    }
}
fn detect_known_conflicts(text: &str, mods: &[LoadedMod]) -> Vec<CrashFinding> {
    let lower = text.to_ascii_lowercase();
    let pairs = [
        ("jei", "rei", "JEI 与 REI 同时加载"),
        (
            "optifine",
            "embeddium",
            "OptiFine 与 Embeddium 图形实现冲突",
        ),
        ("sodium", "optifine", "Sodium 与 OptiFine 图形实现冲突"),
    ];
    let ids = mods
        .iter()
        .map(|item| item.id.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    pairs
        .into_iter()
        .filter(|(left, right, _)| {
            (ids.contains(*left) || lower.contains(left))
                && (ids.contains(*right) || lower.contains(right))
        })
        .map(|(left, right, title)| CrashFinding {
            category: "known_conflict".to_string(),
            title: title.to_string(),
            severity: "error".to_string(),
            confidence: 86,
            evidence: vec![format!("检测到同时存在 {left} 与 {right}")],
            explanation: "这两个模组提供重叠功能，同时加载可能导致注册表、渲染或配方阶段崩溃。"
                .to_string(),
            suggestion: "保留其中一个并删除另一个，再启动验证。".to_string(),
        })
        .collect()
}

fn unknown_finding(text: &str) -> CrashFinding {
    let evidence = text
        .lines()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("exception")
                || lower.contains("caused by:")
                || lower.contains("error")
                || lower.contains("fatal")
        })
        .map(compact_line)
        .filter(|line| !line.is_empty())
        .take(8)
        .collect::<Vec<_>>();
    CrashFinding {
        category: "unknown".to_string(),
        title: if evidence.is_empty() {
            "没有识别到明确错误".to_string()
        } else {
            "发现未分类异常".to_string()
        },
        severity: if evidence.is_empty() {
            "info"
        } else {
            "warning"
        }
        .to_string(),
        confidence: if evidence.is_empty() { 20 } else { 45 },
        evidence,
        explanation: "日志中没有命中内置规则，可能仍包含模组私有异常或需要结合完整堆栈判断。"
            .to_string(),
        suggestion: "检查报告中的第一个 Caused by，并逐个禁用最近更新的模组进行二分排查。"
            .to_string(),
    }
}
fn rank_suspect_mods(text: &str, mods: &[LoadedMod], findings: &[CrashFinding]) -> Vec<SuspectMod> {
    let lower = text.to_ascii_lowercase();
    let exception_lines = text
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            let lower = line.to_ascii_lowercase();
            lower.contains("exception")
                || lower.contains("caused by:")
                || lower.contains("error")
                || lower.contains("crash")
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let finding_text = findings
        .iter()
        .flat_map(|finding| finding.evidence.iter())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();

    let mut suspects = Vec::new();
    for item in mods {
        let normalized_id = normalize(&item.id);
        let normalized_name = normalize(&item.name);
        if normalized_id.len() < 2 && normalized_name.len() < 2 {
            continue;
        }
        let mut score = 0_u32;
        let mut reasons = Vec::new();
        let id_hits = if normalized_id.is_empty() {
            0
        } else {
            lower.matches(&normalized_id).count()
        };
        let name_hits = if normalized_name.is_empty() {
            0
        } else {
            lower.matches(&normalized_name).count()
        };
        if id_hits > 0 {
            score += 3 + (id_hits.min(4) as u32 * 2);
            reasons.push(format!(
                "堆栈或日志中出现了 mod id「{}」{} 次",
                item.id, id_hits
            ));
        }
        if name_hits > 0 && normalized_name != normalized_id {
            score += 2 + name_hits.min(3) as u32;
            reasons.push(format!("日志中出现了模组名「{}」", item.name));
        }
        if let Some(file_name) = item.file_name.as_deref() {
            let file_norm = normalize(file_name);
            if !file_norm.is_empty() && lower.contains(&file_norm) {
                score += 4;
                reasons.push("日志直接引用了模组文件名".to_string());
            }
        }
        for line in text.lines() {
            let line_lower = line.to_ascii_lowercase();
            if !line_lower.contains(&item.id.to_ascii_lowercase())
                && !line_lower.contains(&item.name.to_ascii_lowercase())
            {
                continue;
            }
            if line_lower.contains(" at ")
                || line_lower.starts_with("at ")
                || line_lower.contains("caused by:")
            {
                score += 5;
                reasons.push("命中异常堆栈帧".to_string());
                break;
            }
        }
        for line_number in &exception_lines {
            let start = line_number.saturating_sub(42);
            let end = line_number.saturating_add(8);
            if text
                .lines()
                .skip(start)
                .take(end.saturating_sub(start) + 1)
                .any(|line| {
                    let line = line.to_ascii_lowercase();
                    line.contains(&item.id.to_ascii_lowercase())
                        || line.contains(&item.name.to_ascii_lowercase())
                })
            {
                score += 3;
                reasons.push("出现在异常堆栈附近".to_string());
                break;
            }
        }
        if !finding_text.is_empty()
            && (finding_text.contains(&item.id.to_ascii_lowercase())
                || finding_text.contains(&item.name.to_ascii_lowercase()))
        {
            score += 4;
            reasons.push("出现在关键错误证据中".to_string());
        }
        if score == 0 {
            continue;
        }
        let confidence = if score >= 10 {
            "high"
        } else if score >= 4 {
            "medium"
        } else {
            "low"
        };
        suspects.push(SuspectMod {
            mod_id: item.id.clone(),
            name: item.name.clone(),
            version: item.version.clone(),
            score,
            confidence: confidence.to_string(),
            reasons: deduplicate(reasons),
        });
    }
    suspects.sort_by_key(|suspect| std::cmp::Reverse(suspect.score));
    suspects.truncate(6);
    suspects
}

fn pick_primary_finding(findings: &[CrashFinding]) -> &CrashFinding {
    findings
        .iter()
        .max_by_key(|finding| {
            let severity = match finding.severity.as_str() {
                "error" => 3,
                "warning" => 2,
                _ => 1,
            };
            (severity, finding.confidence)
        })
        .unwrap_or_else(|| findings.first().expect("findings must not be empty"))
}
fn highest_severity(findings: &[CrashFinding]) -> &'static str {
    if findings.iter().any(|finding| finding.severity == "error") {
        "error"
    } else if findings.iter().any(|finding| finding.severity == "warning") {
        "warning"
    } else {
        "info"
    }
}

fn report_confidence(findings: &[CrashFinding], suspects: &[SuspectMod]) -> u8 {
    let finding_score = findings
        .iter()
        .map(|finding| finding.confidence as u16)
        .max()
        .unwrap_or(30);
    let suspect_score = suspects
        .first()
        .map(|suspect| match suspect.confidence.as_str() {
            "high" => 90,
            "medium" => 72,
            _ => 52,
        })
        .unwrap_or(35) as u16;
    ((finding_score + suspect_score) / 2).min(95) as u8
}

fn compact_line(line: &str) -> String {
    line.trim().chars().take(320).collect()
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn sanitize_component(value: &str) -> String {
    let result = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let result = result.trim_matches('-');
    if result.is_empty() {
        "instance".to_string()
    } else {
        result.chars().take(48).collect()
    }
}

fn display_mod_name(suspect: &SuspectMod) -> String {
    if suspect.name.trim().is_empty() {
        suspect.mod_id.clone()
    } else {
        suspect.name.clone()
    }
}

fn deduplicate(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|value| !value.trim().is_empty() && seen.insert(value.to_ascii_lowercase()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_mixin_failure() {
        let analysis = analyze_text("Mixin apply failed example.mixins.json");
        assert_eq!(analysis.category, "mixin");
    }

    #[test]
    fn detects_multiple_dimensions() {
        let findings = detect_findings(
            "Missing dependency example\nNoSuchMethodError: example.Api.call\nOutOfMemoryError",
        );
        assert!(findings
            .iter()
            .any(|finding| finding.category == "missing_dependency"));
        assert!(findings
            .iter()
            .any(|finding| finding.category == "version_mismatch"));
        assert!(findings.iter().any(|finding| finding.category == "memory"));
    }

    #[test]
    fn ranks_mod_from_stacktrace() {
        let mods = vec![LoadedMod {
            id: "examplemod".to_string(),
            name: "Example Mod".to_string(),
            version: Some("1.0.0".to_string()),
            file_name: None,
        }];
        let text = "Caused by: java.lang.NoSuchMethodError\n at com.examplemod.Example.call(Example.java:10)";
        let findings = detect_findings(text);
        let suspects = rank_suspect_mods(text, &mods, &findings);
        assert_eq!(suspects[0].mod_id, "examplemod");
    }
}

#[cfg(test)]
mod archive_tests {
    use super::*;

    #[test]
    fn captures_instance_logs_into_archive() {
        let root = std::env::temp_dir().join(format!("emcl-capture-test-{}", uuid::Uuid::new_v4()));
        let instance_dir = root.join("instance");
        std::fs::create_dir_all(instance_dir.join("logs")).unwrap();
        std::fs::create_dir_all(instance_dir.join("crash-reports")).unwrap();
        std::fs::write(
            instance_dir.join("logs/latest.log"),
            "Mixin apply failed badmod.mixins.json\n",
        )
        .unwrap();
        let instance = Instance {
            id: "capture-test-instance".to_string(),
            name: "Capture Test".to_string(),
            version_id: "1.20.1".to_string(),
            loader: Some("forge".to_string()),
            loader_version: Some("47.4.0".to_string()),
            java_path: None,
            jvm_args: String::new(),
            memory_mb: 4096,
            directory: instance_dir.to_string_lossy().into_owned(),
            created_at: String::new(),
            updated_at: String::new(),
            last_played_at: None,
        };
        let paths = LauncherPaths::from_root(root.clone());
        let archive = capture_crash_archive(&paths, &instance, Some(1), &|_, _, _| {}).unwrap();
        assert!(archive.is_file());
        let report = analyze_crash_archive(
            &archive,
            AnalysisContext::from_instance(&instance, "auto"),
            &|_, _, _| {},
        )
        .unwrap();
        assert_eq!(report.severity, "error");
        assert!(report
            .included_files
            .iter()
            .any(|name| name.contains("latest.log")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn analyzes_zip_with_mods_list_and_stacktrace() {
        let root = std::env::temp_dir().join(format!("emcl-crash-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("report.zip");
        let file = File::create(&archive_path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer.start_file("latest.log", options).unwrap();
        writer
            .write_all(
                b"Caused by: java.lang.NoSuchMethodError\n at com.badmod.Bad.call(Bad.java:1)\n",
            )
            .unwrap();
        writer.start_file("mods-list.txt", options).unwrap();
        writer
            .write_all(b"MOD\tbadmod\tBad Mod\t1.0.0\tforge\tbadmod.jar\n")
            .unwrap();
        writer.finish().unwrap();

        let context = AnalysisContext {
            source: "manual".to_string(),
            version_id: Some("1.20.1".to_string()),
            loader: Some("forge".to_string()),
            ..AnalysisContext::default()
        };
        let report = analyze_crash_archive(&archive_path, context, &|_, _, _| {}).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.category == "version_mismatch"));
        assert_eq!(report.suspected_mods[0].mod_id, "badmod");
        let _ = std::fs::remove_dir_all(root);
    }
}
