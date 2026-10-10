use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

use crate::core::install::rules_allow;
use crate::core::paths::LauncherPaths;
use crate::core::version::{Argument, VersionDetails};
use crate::db::models::{Instance, Settings, StoredAccount};

pub type ProcessRegistry = Arc<Mutex<HashMap<String, Arc<Mutex<Child>>>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchLog {
    pub instance_id: String,
    pub stream: String,
    pub line: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchExit {
    pub instance_id: String,
    pub code: Option<i32>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub instance_id: String,
    pub pid: u32,
}

pub fn build_command(
    instance: &Instance,
    details: &VersionDetails,
    paths: &LauncherPaths,
    settings: &Settings,
    account: &StoredAccount,
    java_path: &Path,
) -> Result<Command> {
    let version_dir = paths.version_dir(&details.id);
    let client_jar = version_dir.join("client.jar");
    if !client_jar.is_file() {
        bail!("客户端文件尚未安装，请先执行安装");
    }

    let mut classpath_entries = Vec::new();
    for library in &details.libraries {
        if !rules_allow(library.rules.as_deref()) {
            continue;
        }
        let Some(artifact) = library.downloads.artifact.as_ref() else {
            continue;
        };
        let Some(relative) = artifact.path.as_deref() else {
            continue;
        };
        let path = resolve_relative(&paths.libraries_dir(), relative)?;
        if path.is_file() {
            classpath_entries.push(path.to_string_lossy().into_owned());
        }
    }
    classpath_entries.push(client_jar.to_string_lossy().into_owned());
    let classpath_separator = if cfg!(windows) { ";" } else { ":" };
    let classpath = classpath_entries.join(classpath_separator);

    let natives_directory = version_dir.join("natives");
    std::fs::create_dir_all(&natives_directory)?;
    let game_directory = PathBuf::from(&instance.directory);
    let assets_root = paths.assets_dir();
    let asset_index_name = details
        .assets
        .clone()
        .or_else(|| details.asset_index.as_ref().map(|index| index.id.clone()))
        .unwrap_or_else(|| "legacy".to_string());
    let access_token = account
        .access_token
        .clone()
        .unwrap_or_else(|| "0".to_string());

    let mut substitutions = HashMap::new();
    substitutions.insert("auth_player_name", account.username.clone());
    substitutions.insert("version_name", details.id.clone());
    substitutions.insert(
        "game_directory",
        game_directory.to_string_lossy().into_owned(),
    );
    substitutions.insert("assets_root", assets_root.to_string_lossy().into_owned());
    substitutions.insert("assets_index_name", asset_index_name);
    substitutions.insert("auth_uuid", account.uuid.clone());
    substitutions.insert("auth_access_token", access_token.clone());
    substitutions.insert("auth_session", format!("token:{access_token}"));
    substitutions.insert(
        "user_type",
        if account.account_type == "microsoft" {
            "msa"
        } else {
            "legacy"
        }
        .to_string(),
    );
    substitutions.insert("version_type", details.version_type.clone());
    substitutions.insert(
        "natives_directory",
        natives_directory.to_string_lossy().into_owned(),
    );
    substitutions.insert("launcher_name", "EMCL".to_string());
    substitutions.insert("launcher_version", env!("CARGO_PKG_VERSION").to_string());
    substitutions.insert("classpath", classpath.clone());
    substitutions.insert("classpath_separator", classpath_separator.to_string());
    substitutions.insert(
        "library_directory",
        paths.libraries_dir().to_string_lossy().into_owned(),
    );
    substitutions.insert("user_properties", "{}".to_string());
    substitutions.insert("clientid", String::new());
    substitutions.insert("auth_xuid", String::new());
    substitutions.insert("resolution_width", settings.game_width.to_string());
    substitutions.insert("resolution_height", settings.game_height.to_string());
    substitutions.insert("quickPlayPath", String::new());

    let mut jvm_args = vec![
        format!("-Xmx{}M", instance.memory_mb),
        format!("-Xms{}M", (instance.memory_mb / 4).clamp(256, 2048)),
        format!("-Djava.library.path={}", natives_directory.display()),
        format!("-Djna.tmpdir={}", natives_directory.display()),
        format!(
            "-Dorg.lwjgl.system.SharedLibraryExtractPath={}",
            natives_directory.display()
        ),
        format!("-Dio.netty.native.workdir={}", natives_directory.display()),
    ];

    let mut has_classpath = false;
    if let Some(arguments) = &details.arguments {
        let expanded = expand_arguments(&arguments.jvm, &substitutions);
        has_classpath = expanded
            .iter()
            .any(|argument| argument == "-cp" || argument == "-classpath");
        jvm_args.extend(expanded);
    }

    if !has_classpath {
        jvm_args.push("-cp".to_string());
        jvm_args.push(classpath.clone());
    }

    if !instance.jvm_args.trim().is_empty() {
        jvm_args.extend(
            shell_words::split(&instance.jvm_args)
                .context("无法解析实例 JVM 参数")?
                .into_iter()
                .map(|argument| substitute(&argument, &substitutions)),
        );
    }

    let mut game_args = if let Some(arguments) = &details.arguments {
        expand_arguments(&arguments.game, &substitutions)
    } else if let Some(legacy) = &details.minecraft_arguments {
        shell_words::split(legacy)
            .context("无法解析旧版游戏参数")?
            .into_iter()
            .map(|argument| substitute(&argument, &substitutions))
            .collect()
    } else {
        Vec::new()
    };
    if settings.fullscreen && !game_args.iter().any(|argument| argument == "--fullscreen") {
        game_args.push("--fullscreen".to_string());
    }

    let main_class = details.main_class.clone().context("版本清单没有提供主类")?;

    let mut command = Command::new(java_path);
    command
        .current_dir(&game_directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .args(jvm_args)
        .arg(main_class)
        .args(game_args);
    Ok(command)
}

pub struct LaunchRequest<'a> {
    pub app: AppHandle,
    pub registry: ProcessRegistry,
    pub instance: Instance,
    pub details: VersionDetails,
    pub paths: &'a LauncherPaths,
    pub settings: &'a Settings,
    pub account: StoredAccount,
    pub java_path: PathBuf,
}

pub async fn launch_instance(request: LaunchRequest<'_>) -> Result<LaunchResult> {
    let LaunchRequest {
        app,
        registry,
        instance,
        details,
        paths,
        settings,
        account,
        java_path,
    } = request;

    let mut command = build_command(&instance, &details, paths, settings, &account, &java_path)?;
    let mut child = command
        .spawn()
        .with_context(|| format!("无法使用 {} 启动游戏", java_path.display()))?;
    let pid = child.id().context("游戏进程没有 PID")?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let process = Arc::new(Mutex::new(child));
    registry
        .lock()
        .map_err(|error| anyhow::anyhow!("进程注册表不可用: {error}"))?
        .insert(instance.id.clone(), process.clone());

    let log_path = PathBuf::from(&instance.directory)
        .join("logs")
        .join("latest.log");
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let log_file = Arc::new(tokio::sync::Mutex::new(fs::File::create(&log_path).await?));

    if let Some(stdout) = stdout {
        tokio::spawn(pump_output(
            app.clone(),
            instance.id.clone(),
            "stdout".to_string(),
            stdout,
            log_file.clone(),
        ));
    }
    if let Some(stderr) = stderr {
        tokio::spawn(pump_output(
            app.clone(),
            instance.id.clone(),
            "stderr".to_string(),
            stderr,
            log_file.clone(),
        ));
    }

    let waiter_registry = registry.clone();
    let waiter_instance_id = instance.id.clone();
    tokio::spawn(async move {
        let status = loop {
            let result = {
                process
                    .lock()
                    .ok()
                    .and_then(|mut child| child.try_wait().ok().flatten())
            };
            if let Some(status) = result {
                break status;
            }
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        };

        if let Ok(mut registry) = waiter_registry.lock() {
            registry.remove(&waiter_instance_id);
        }
        let code = status.code();
        let _ = app.emit(
            "launch-exit",
            LaunchExit {
                instance_id: waiter_instance_id,
                code,
                success: status.success(),
            },
        );
    });

    Ok(LaunchResult {
        instance_id: instance.id,
        pid,
    })
}

pub fn stop_instance(registry: &ProcessRegistry, instance_id: &str) -> Result<()> {
    let process = registry
        .lock()
        .map_err(|error| anyhow::anyhow!("进程注册表不可用: {error}"))?
        .get(instance_id)
        .cloned()
        .context("这个实例没有正在运行的游戏进程")?;

    let pid = {
        let child = process
            .lock()
            .map_err(|error| anyhow::anyhow!("游戏进程不可用: {error}"))?;
        child.id()
    };

    #[cfg(windows)]
    if let Some(pid) = pid {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status();
    }

    let mut child = process
        .lock()
        .map_err(|error| anyhow::anyhow!("游戏进程不可用: {error}"))?;
    let _ = child.start_kill();
    Ok(())
}

pub fn is_running(registry: &ProcessRegistry, instance_id: &str) -> bool {
    registry
        .lock()
        .map(|registry| registry.contains_key(instance_id))
        .unwrap_or(false)
}

async fn pump_output<R: AsyncRead + Unpin>(
    app: AppHandle,
    instance_id: String,
    stream: String,
    reader: R,
    log_file: Arc<tokio::sync::Mutex<fs::File>>,
) {
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        {
            let mut file = log_file.lock().await;
            let _ = file
                .write_all(format!("[{stream}] {line}\n").as_bytes())
                .await;
        }
        let _ = app.emit(
            "launch-log",
            LaunchLog {
                instance_id: instance_id.clone(),
                stream: stream.clone(),
                line,
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
        );
    }
}

fn expand_arguments(arguments: &[Argument], substitutions: &HashMap<&str, String>) -> Vec<String> {
    let mut expanded = Vec::new();
    for argument in arguments {
        match argument {
            Argument::Plain(value) => expanded.push(substitute(value, substitutions)),
            Argument::Conditional { rules, value } => {
                if rules_allow(Some(rules)) {
                    match value {
                        serde_json::Value::String(value) => {
                            expanded.push(substitute(value, substitutions))
                        }
                        serde_json::Value::Array(values) => {
                            for value in values.iter().filter_map(serde_json::Value::as_str) {
                                expanded.push(substitute(value, substitutions));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    expanded
}

fn substitute(value: &str, substitutions: &HashMap<&str, String>) -> String {
    substitutions
        .iter()
        .fold(value.to_string(), |result, (key, replacement)| {
            result.replace(&format!("${{{key}}}"), replacement)
        })
}

fn resolve_relative(root: &Path, relative: &str) -> Result<PathBuf> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("版本库路径不安全: {relative}");
    }
    Ok(root.join(relative_path))
}

#[allow(dead_code)]
fn timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_secs().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_launch_placeholders() {
        let mut values = HashMap::new();
        values.insert("auth_player_name", "Steve".to_string());
        values.insert("assets_index_name", "17".to_string());
        assert_eq!(
            substitute(
                "--username ${auth_player_name} --assetIndex ${assets_index_name}",
                &values
            ),
            "--username Steve --assetIndex 17"
        );
    }

    #[test]
    fn rejects_unsafe_library_paths() {
        assert!(resolve_relative(Path::new("root"), "../escape").is_err());
    }
}
