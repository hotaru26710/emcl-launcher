use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context, Result};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;

use crate::db::models::Project;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBuildLog {
    pub project_id: String,
    pub stream: String,
    pub line: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBuildExit {
    pub project_id: String,
    pub success: bool,
    pub code: Option<i32>,
}

pub async fn build_project(app: AppHandle, project: &Project) -> Result<()> {
    let root = PathBuf::from(&project.root_path);
    let gradlew = if cfg!(windows) {
        root.join("gradlew.bat")
    } else {
        root.join("gradlew")
    };
    if !gradlew.is_file() {
        bail!("工程缺少 gradlew / gradlew.bat");
    }

    let task = project.gradle_task.trim();
    let task = if task.is_empty() { "build" } else { task };
    let mut command = Command::new(&gradlew);
    command
        .arg(task)
        .current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        command.creation_flags(0x0800_0000);
    }

    let mut child = command
        .spawn()
        .with_context(|| format!("无法执行 {}", gradlew.display()))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let tail = Arc::new(Mutex::new(Vec::<String>::new()));
    if let Some(stdout) = stdout {
        tokio::spawn(pump_build_output(
            app.clone(),
            project.id.clone(),
            "stdout",
            stdout,
            tail.clone(),
        ));
    }
    if let Some(stderr) = stderr {
        tokio::spawn(pump_build_output(
            app.clone(),
            project.id.clone(),
            "stderr",
            stderr,
            tail.clone(),
        ));
    }
    let status = child.wait().await?;
    let _ = app.emit(
        "project-build-exit",
        ProjectBuildExit {
            project_id: project.id.clone(),
            success: status.success(),
            code: status.code(),
        },
    );
    if !status.success() {
        let tail = tail
            .lock()
            .map(|lines| lines.join("\n"))
            .unwrap_or_default();
        bail!("Gradle 构建失败，退出码 {:?}\n{}", status.code(), tail);
    }

    Ok(())
}

async fn pump_build_output<R: AsyncRead + Unpin>(
    app: AppHandle,
    project_id: String,
    stream: &'static str,
    reader: R,
    tail: Arc<Mutex<Vec<String>>>,
) {
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if let Ok(mut tail) = tail.lock() {
            tail.push(line.clone());
            if tail.len() > 80 {
                tail.remove(0);
            }
        }
        let _ = app.emit(
            "project-build-log",
            ProjectBuildLog {
                project_id: project_id.clone(),
                stream: stream.to_string(),
                line,
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
        );
    }
}
