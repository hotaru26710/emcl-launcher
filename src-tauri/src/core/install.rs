use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use reqwest::header::RANGE;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::core::paths::{is_safe_segment, LauncherPaths};
use crate::core::version::{AssetIndex, DownloadArtifact, Library, Rule, VersionDetails};

pub type ProgressSink = Arc<dyn Fn(InstallProgress) + Send + Sync>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub job_id: String,
    pub version_id: String,
    pub status: String,
    pub stage: String,
    pub message: String,
    pub completed_files: u64,
    pub total_files: u64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSummary {
    pub version_id: String,
    pub client_jar: String,
    pub status_path: String,
    pub library_count: usize,
    pub asset_count: usize,
    pub total_bytes: u64,
}

#[derive(Clone)]
pub struct ProgressReporter {
    inner: Arc<ProgressInner>,
}

struct ProgressInner {
    job_id: String,
    version_id: String,
    sink: ProgressSink,
    stage: Mutex<String>,
    status: Mutex<String>,
    message: Mutex<String>,
    completed_files: AtomicU64,
    total_files: AtomicU64,
    downloaded_bytes: AtomicU64,
    total_bytes: AtomicU64,
    last_emit: Mutex<Instant>,
}

impl ProgressReporter {
    pub fn new(job_id: String, version_id: String, sink: ProgressSink) -> Self {
        Self {
            inner: Arc::new(ProgressInner {
                job_id,
                version_id,
                sink,
                stage: Mutex::new("starting".to_string()),
                status: Mutex::new("running".to_string()),
                message: Mutex::new("正在准备安装".to_string()),
                completed_files: AtomicU64::new(0),
                total_files: AtomicU64::new(0),
                downloaded_bytes: AtomicU64::new(0),
                total_bytes: AtomicU64::new(0),
                last_emit: Mutex::new(Instant::now() - Duration::from_secs(1)),
            }),
        }
    }

    pub fn stage(&self, stage: &str, message: &str, total_files: u64, total_bytes: u64) {
        *self.lock_string(&self.inner.stage) = stage.to_string();
        *self.lock_string(&self.inner.status) = "running".to_string();
        *self.lock_string(&self.inner.message) = message.to_string();
        self.inner.completed_files.store(0, Ordering::Relaxed);
        self.inner.total_files.store(total_files, Ordering::Relaxed);
        self.inner.downloaded_bytes.store(0, Ordering::Relaxed);
        self.inner.total_bytes.store(total_bytes, Ordering::Relaxed);
        self.emit(true);
    }

    pub fn add_bytes(&self, bytes: u64) {
        if bytes == 0 {
            return;
        }
        self.inner
            .downloaded_bytes
            .fetch_add(bytes, Ordering::Relaxed);
        self.emit(false);
    }

    pub fn complete_file(&self, label: &str) {
        self.inner.completed_files.fetch_add(1, Ordering::Relaxed);
        *self.lock_string(&self.inner.message) = label.to_string();
        self.emit(false);
    }

    pub fn finish(&self, message: &str) {
        let total = self.inner.total_files.load(Ordering::Relaxed);
        self.inner.completed_files.store(total, Ordering::Relaxed);
        *self.lock_string(&self.inner.status) = "complete".to_string();
        *self.lock_string(&self.inner.stage) = "complete".to_string();
        *self.lock_string(&self.inner.message) = message.to_string();
        self.emit(true);
    }

    pub fn fail(&self, message: &str) {
        *self.lock_string(&self.inner.status) = "error".to_string();
        *self.lock_string(&self.inner.message) = message.to_string();
        self.emit(true);
    }

    fn emit(&self, force: bool) {
        if !force {
            let mut last_emit = self.lock_instant(&self.inner.last_emit);
            if last_emit.elapsed() < Duration::from_millis(100) {
                return;
            }
            *last_emit = Instant::now();
        } else {
            *self.lock_instant(&self.inner.last_emit) = Instant::now();
        }

        let completed_files = self.inner.completed_files.load(Ordering::Relaxed);
        let total_files = self.inner.total_files.load(Ordering::Relaxed);
        let downloaded_bytes = self.inner.downloaded_bytes.load(Ordering::Relaxed);
        let total_bytes = self.inner.total_bytes.load(Ordering::Relaxed);
        let status = self.lock_string(&self.inner.status).clone();
        let file_ratio = if total_files == 0 {
            0.0
        } else {
            completed_files as f64 / total_files as f64
        };
        let byte_ratio = if total_bytes == 0 {
            0.0
        } else {
            downloaded_bytes as f64 / total_bytes as f64
        };
        let percent = if status == "complete" || (total_files > 0 && completed_files >= total_files)
        {
            100.0
        } else {
            (file_ratio * 40.0 + byte_ratio * 60.0).clamp(0.0, 99.9)
        };

        let progress = InstallProgress {
            job_id: self.inner.job_id.clone(),
            version_id: self.inner.version_id.clone(),
            status,
            stage: self.lock_string(&self.inner.stage).clone(),
            message: self.lock_string(&self.inner.message).clone(),
            completed_files,
            total_files,
            downloaded_bytes,
            total_bytes,
            percent,
        };

        (self.inner.sink)(progress);
    }

    fn lock_string<'a>(&self, lock: &'a Mutex<String>) -> std::sync::MutexGuard<'a, String> {
        lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock_instant<'a>(&self, lock: &'a Mutex<Instant>) -> std::sync::MutexGuard<'a, Instant> {
        lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

struct DownloadRuntime<'a> {
    client: &'a Client,
    mirror: &'a str,
    concurrency: u32,
    force_redownload: bool,
    reporter: &'a ProgressReporter,
}

#[derive(Clone)]
struct DownloadItem {
    url: String,
    destination: PathBuf,
    sha1: Option<String>,
    size: Option<u64>,
    label: String,
    version_id: String,
}

#[derive(Debug, Deserialize)]
struct AssetIndexFile {
    objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallationMarker {
    version_id: String,
    installed_at: String,
    client_jar: String,
    library_count: usize,
    asset_count: usize,
}

pub async fn install_version(
    client: &Client,
    paths: &LauncherPaths,
    details: &VersionDetails,
    mirror: &str,
    concurrency: u32,
    force_redownload: bool,
    reporter: &ProgressReporter,
) -> Result<InstallSummary> {
    if !is_safe_segment(&details.id) {
        bail!("invalid version id in metadata");
    }

    let version_dir = paths.version_dir(&details.id);
    fs::create_dir_all(&version_dir).await?;

    let client_artifact = details
        .downloads
        .client
        .as_ref()
        .context("this version does not provide a client download")?;
    let client_jar = version_dir.join("client.jar");
    let client_item = DownloadItem::from_artifact(
        client_artifact,
        client_jar.clone(),
        "client.jar".to_string(),
        details.id.clone(),
    )?;

    let mut library_items = Vec::new();
    let mut native_jars = Vec::new();
    let mut seen_library_paths = HashSet::new();
    for library in &details.libraries {
        if !rules_allow(library.rules.as_deref()) {
            continue;
        }

        for (artifact, is_native) in library_artifacts(library)? {
            let relative = artifact
                .path
                .as_deref()
                .context("library artifact does not have a path")?;
            let destination = safe_join(&paths.libraries_dir(), relative)?;
            if !seen_library_paths.insert(destination.clone()) {
                continue;
            }
            library_items.push(DownloadItem::from_artifact(
                &artifact,
                destination.clone(),
                relative.to_string(),
                details.id.clone(),
            )?);
            if is_native {
                native_jars.push(destination);
            }
        }
    }

    let mut metadata_items = Vec::new();
    let mut asset_index_path = None;
    if let Some(asset_index) = &details.asset_index {
        if !is_safe_segment(&asset_index.id) {
            bail!("invalid asset index id");
        }
        let path = paths
            .assets_dir()
            .join("indexes")
            .join(format!("{}.json", asset_index.id));
        metadata_items.push(DownloadItem::from_asset_index(
            asset_index,
            path.clone(),
            details.id.clone(),
        )?);
        asset_index_path = Some(path);
    }

    if let Some(logging_client) = details
        .logging
        .as_ref()
        .and_then(|logging| logging.client.as_ref())
    {
        let file_name = logging_client
            .file
            .path
            .as_deref()
            .unwrap_or("log_config.xml");
        let path = safe_join(&paths.shared_dir().join("log_configs"), file_name)?;
        metadata_items.push(DownloadItem::from_artifact(
            &logging_client.file,
            path,
            "logging config".to_string(),
            details.id.clone(),
        )?);
    }

    let runtime = DownloadRuntime {
        client,
        mirror,
        concurrency,
        force_redownload,
        reporter,
    };

    download_many(
        &runtime,
        "client",
        "正在下载客户端",
        vec![client_item.clone()],
    )
    .await?;

    let library_count = library_items.len();
    let library_total = total_known_size(&library_items);
    reporter.stage(
        "libraries",
        "正在下载依赖库",
        library_count as u64,
        library_total,
    );
    download_many(&runtime, "libraries", "正在下载依赖库", library_items).await?;

    let metadata_count = metadata_items.len();
    let metadata_total = total_known_size(&metadata_items);
    reporter.stage(
        "metadata",
        "正在准备资源索引",
        metadata_count as u64,
        metadata_total,
    );
    download_many(&runtime, "metadata", "正在准备资源索引", metadata_items).await?;

    let mut asset_count = 0;
    let mut asset_total_bytes = 0;
    if let Some(index_path) = asset_index_path {
        let index_bytes = fs::read(&index_path)
            .await
            .with_context(|| format!("failed to read asset index {}", index_path.display()))?;
        let asset_index: AssetIndexFile = serde_json::from_slice(&index_bytes)
            .with_context(|| format!("failed to parse asset index {}", index_path.display()))?;

        let mut seen_assets = HashSet::new();
        let mut asset_items = Vec::new();
        asset_total_bytes = asset_index
            .objects
            .values()
            .map(|object| object.size)
            .sum::<u64>();

        for object in asset_index.objects.values() {
            validate_hash(&object.hash)?;
            let prefix = &object.hash[..2];
            let destination = paths
                .assets_dir()
                .join("objects")
                .join(prefix)
                .join(&object.hash);
            if !seen_assets.insert(destination.clone()) {
                continue;
            }
            asset_items.push(DownloadItem {
                url: format!(
                    "https://resources.download.minecraft.net/{prefix}/{}",
                    object.hash
                ),
                destination,
                sha1: Some(object.hash.clone()),
                size: Some(object.size),
                label: format!("asset {}", &object.hash[..8.min(object.hash.len())]),
                version_id: details.id.clone(),
            });
        }

        asset_count = asset_items.len();
        download_many(&runtime, "assets", "正在下载游戏资源", asset_items).await?;
    }

    if !native_jars.is_empty() {
        reporter.stage("natives", "正在解压本机运行库", native_jars.len() as u64, 0);
        extract_natives(native_jars, version_dir.join("natives")).await?;
        let completed = reporter.inner.total_files.load(Ordering::Relaxed);
        reporter.complete_file(&format!("已解压 {completed} 个本机运行库"));
    }

    let total_bytes =
        client_item.size.unwrap_or(0) + library_total + metadata_total + asset_total_bytes;
    let marker = InstallationMarker {
        version_id: details.id.clone(),
        installed_at: chrono::Utc::now().to_rfc3339(),
        client_jar: client_jar.to_string_lossy().into_owned(),
        library_count,
        asset_count,
    };
    let status_path = version_dir.join(".mcl-installed.json");
    let marker_bytes = serde_json::to_vec_pretty(&marker)?;
    crate::core::version::write_bytes_atomic(&status_path, &marker_bytes).await?;

    reporter.finish("安装完成");

    Ok(InstallSummary {
        version_id: details.id.clone(),
        client_jar: client_jar.to_string_lossy().into_owned(),
        status_path: status_path.to_string_lossy().into_owned(),
        library_count,
        asset_count,
        total_bytes,
    })
}

async fn download_many(
    runtime: &DownloadRuntime<'_>,
    stage: &str,
    message: &str,
    items: Vec<DownloadItem>,
) -> Result<()> {
    let total_bytes = total_known_size(&items);
    runtime
        .reporter
        .stage(stage, message, items.len() as u64, total_bytes);

    let iterations = items.into_iter().map(|item| {
        let client = runtime.client.clone();
        let mirror = runtime.mirror.to_string();
        let reporter = runtime.reporter.clone();
        let force_redownload = runtime.force_redownload;
        async move { download_file(&client, &item, &mirror, force_redownload, &reporter).await }
    });

    let results = futures_util::stream::iter(iterations)
        .buffer_unordered(runtime.concurrency.max(1) as usize)
        .collect::<Vec<_>>()
        .await;

    let mut first_error = None;
    for result in results {
        if let Err(error) = result {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }
    }

    if let Some(error) = first_error {
        return Err(error);
    }
    Ok(())
}

async fn download_file(
    client: &Client,
    item: &DownloadItem,
    mirror: &str,
    force_redownload: bool,
    reporter: &ProgressReporter,
) -> Result<()> {
    if !force_redownload && existing_file_is_valid(item).await? {
        reporter.complete_file(&format!("已存在 {}", item.label));
        return Ok(());
    }

    if force_redownload {
        let _ = fs::remove_file(&item.destination).await;
    }
    let part = part_path(&item.destination);
    if force_redownload {
        let _ = fs::remove_file(&part).await;
    }
    if let Some(parent) = part.parent() {
        fs::create_dir_all(parent).await?;
    }

    let mut last_error = None;
    for attempt in 1..=3 {
        match download_attempt(client, item, mirror, &part, reporter).await {
            Ok(()) => {
                publish_part(&part, &item.destination, item).await?;
                reporter.complete_file(&item.label);
                return Ok(());
            }
            Err(error) => {
                let message = error.to_string();
                if message.contains("checksum") || message.contains("incomplete download") {
                    let _ = fs::remove_file(&part).await;
                }
                last_error = Some(error);
                if attempt < 3 {
                    tokio::time::sleep(Duration::from_secs(1 << (attempt - 1))).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("download failed for {}", item.url)))
}

async fn download_attempt(
    client: &Client,
    item: &DownloadItem,
    mirror: &str,
    part: &Path,
    reporter: &ProgressReporter,
) -> Result<()> {
    let mut existing = fs::metadata(part)
        .await
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if let Some(expected) = item.size {
        if existing > expected {
            let _ = fs::remove_file(part).await;
            existing = 0;
        }
    }

    let url = mirror_url(&item.url, mirror, &item.version_id);
    let mut request = client.get(url);
    if existing > 0 {
        request = request.header(RANGE, format!("bytes={existing}-"));
    }

    let response = request
        .send()
        .await
        .with_context(|| format!("failed to request {}", item.url))?;

    if response.status() == StatusCode::RANGE_NOT_SATISFIABLE && existing > 0 {
        if let Some(expected) = item.size {
            if existing == expected {
                verify_file(part, item.sha1.as_deref()).await?;
                return Ok(());
            }
        }
    }

    let response = response
        .error_for_status()
        .with_context(|| format!("request failed for {}", item.url))?;
    let append = response.status() == StatusCode::PARTIAL_CONTENT && existing > 0;
    let mut output = if append {
        tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(part)
            .await?
    } else {
        fs::File::create(part).await?
    };

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        output.write_all(&chunk).await?;
        reporter.add_bytes(chunk.len() as u64);
    }
    output.flush().await?;
    drop(output);

    if let Some(expected) = item.size {
        let actual = fs::metadata(part).await?.len();
        if actual != expected {
            bail!(
                "incomplete download for {}: expected {} bytes, got {}",
                item.url,
                expected,
                actual
            );
        }
    }

    verify_file(part, item.sha1.as_deref()).await
}

async fn publish_part(part: &Path, destination: &Path, item: &DownloadItem) -> Result<()> {
    verify_file(part, item.sha1.as_deref()).await?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).await?;
    }
    if fs::try_exists(destination).await.unwrap_or(false) {
        fs::remove_file(destination).await?;
    }
    fs::rename(part, destination)
        .await
        .with_context(|| format!("failed to move download into {}", destination.display()))?;
    Ok(())
}

async fn existing_file_is_valid(item: &DownloadItem) -> Result<bool> {
    if !fs::try_exists(&item.destination).await.unwrap_or(false) {
        return Ok(false);
    }

    let metadata = fs::metadata(&item.destination).await?;
    if metadata.len() == 0 {
        return Ok(false);
    }
    if let Some(expected) = item.size {
        if metadata.len() != expected {
            return Ok(false);
        }
    }

    if let Some(sha1) = &item.sha1 {
        if verify_file(&item.destination, Some(sha1)).await.is_err() {
            return Ok(false);
        }
    }
    Ok(true)
}

async fn verify_file(path: &Path, expected_sha1: Option<&str>) -> Result<()> {
    let Some(expected) = expected_sha1 else {
        return Ok(());
    };
    let actual = sha1_file(path).await?;
    if !actual.eq_ignore_ascii_case(expected) {
        bail!(
            "checksum mismatch for {}: expected {}, got {}",
            path.display(),
            expected,
            actual
        );
    }
    Ok(())
}

async fn sha1_file(path: &Path) -> Result<String> {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || -> Result<String> {
        let mut file = std::fs::File::open(&path)?;
        let mut hasher = Sha1::new();
        std::io::copy(&mut file, &mut hasher)?;
        Ok(hex::encode(hasher.finalize()))
    })
    .await
    .context("checksum worker failed")?
}

fn library_artifacts(library: &Library) -> Result<Vec<(DownloadArtifact, bool)>> {
    let mut artifacts = Vec::new();
    if let Some(artifact) = library.downloads.artifact.as_ref() {
        if !artifact.url.is_empty() {
            artifacts.push((artifact.clone(), false));
        }
    }

    if let (Some(natives), Some(classifiers)) = (
        library.natives.as_ref(),
        library.downloads.classifiers.as_ref(),
    ) {
        if let Some(classifier) = native_classifier(natives) {
            if let Some(artifact) = classifiers.get(&classifier) {
                if !artifact.url.is_empty() {
                    artifacts.push((artifact.clone(), true));
                }
            }
        }
    }

    Ok(artifacts)
}

fn native_classifier(natives: &HashMap<String, String>) -> Option<String> {
    let os = current_os();
    natives
        .get(os)
        .map(|classifier| classifier.replace("${arch}", &current_arch()))
}

pub fn rules_allow(rules: Option<&[Rule]>) -> bool {
    let Some(rules) = rules else {
        return true;
    };
    if rules.is_empty() {
        return true;
    }

    let mut allowed = false;
    for rule in rules {
        if !rule_matches(rule) {
            continue;
        }
        match rule.action.as_str() {
            "allow" => allowed = true,
            "disallow" => allowed = false,
            _ => {}
        }
    }
    allowed
}

fn rule_matches(rule: &Rule) -> bool {
    if let Some(features) = &rule.features {
        if features.values().any(|enabled| *enabled) {
            return false;
        }
    }

    let Some(os) = &rule.os else {
        return true;
    };
    if let Some(name) = &os.name {
        if name != current_os() {
            return false;
        }
    }
    if let Some(arch) = &os.arch {
        if arch != current_arch_name() {
            return false;
        }
    }
    true
}

fn current_os() -> &'static str {
    match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "osx",
        "linux" => "linux",
        other => other,
    }
}

fn current_arch_name() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" | "amd64" => "x86_64",
        "x86" | "i686" => "x86",
        "aarch64" | "arm64" => "arm64",
        other => other,
    }
}

fn current_arch() -> String {
    match std::env::consts::ARCH {
        "x86_64" | "amd64" => "64".to_string(),
        "x86" | "i686" => "32".to_string(),
        "aarch64" | "arm64" => "arm64".to_string(),
        other => other.to_string(),
    }
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("unsafe relative path in version metadata: {relative}");
    }
    Ok(root.join(relative_path))
}

fn validate_hash(hash: &str) -> Result<()> {
    if hash.len() < 2 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("invalid asset hash: {hash}");
    }
    Ok(())
}

fn total_known_size(items: &[DownloadItem]) -> u64 {
    items.iter().filter_map(|item| item.size).sum()
}

fn part_path(destination: &Path) -> PathBuf {
    let name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download");
    destination.with_file_name(format!("{name}.part"))
}

pub fn mirror_url(url: &str, mirror: &str, version_id: &str) -> String {
    if mirror != "bmclapi" {
        return url.to_string();
    }

    if let Some(rest) = url.strip_prefix("https://libraries.minecraft.net/") {
        return format!("https://bmclapi2.bangbang93.com/maven/{rest}");
    }
    if let Some(rest) = url.strip_prefix("https://resources.download.minecraft.net/") {
        return format!("https://bmclapi2.bangbang93.com/assets/{rest}");
    }
    if url.contains("piston-data.mojang.com")
        && url.ends_with("/client.jar")
        && !version_id.is_empty()
    {
        return format!("https://bmclapi2.bangbang93.com/version/{version_id}/client");
    }
    url.to_string()
}

async fn extract_natives(jars: Vec<PathBuf>, destination: PathBuf) -> Result<()> {
    tokio::task::spawn_blocking(move || -> Result<()> {
        std::fs::create_dir_all(&destination)?;
        for jar in jars {
            let file = std::fs::File::open(&jar)
                .with_context(|| format!("failed to open native archive {}", jar.display()))?;
            let mut archive = zip::ZipArchive::new(file)
                .with_context(|| format!("failed to read native archive {}", jar.display()))?;

            for index in 0..archive.len() {
                let mut entry = archive.by_index(index)?;
                let Some(enclosed) = entry.enclosed_name() else {
                    continue;
                };
                let relative = enclosed.to_path_buf();
                let relative_text = relative.to_string_lossy();
                if entry.is_dir() || relative_text.starts_with("META-INF/") {
                    continue;
                }

                let output_path = destination.join(&relative);
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut output = std::fs::File::create(&output_path)
                    .with_context(|| format!("failed to create {}", output_path.display()))?;
                std::io::copy(&mut entry, &mut output)?;
                output.flush()?;
            }
        }
        Ok(())
    })
    .await
    .context("native extraction worker failed")??;
    Ok(())
}

impl DownloadItem {
    fn from_artifact(
        artifact: &DownloadArtifact,
        destination: PathBuf,
        label: String,
        version_id: String,
    ) -> Result<Self> {
        Ok(Self {
            url: artifact.url.clone(),
            destination,
            sha1: artifact.sha1.clone(),
            size: artifact.size,
            label,
            version_id,
        })
    }

    fn from_asset_index(
        asset_index: &AssetIndex,
        destination: PathBuf,
        version_id: String,
    ) -> Result<Self> {
        Ok(Self {
            url: asset_index.url.clone(),
            destination,
            sha1: Some(asset_index.sha1.clone()),
            size: Some(asset_index.size),
            label: format!("asset index {}", asset_index.id),
            version_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirrors_library_and_asset_urls() {
        assert_eq!(
            mirror_url(
                "https://libraries.minecraft.net/org/example/a.jar",
                "bmclapi",
                "1.21.1"
            ),
            "https://bmclapi2.bangbang93.com/maven/org/example/a.jar"
        );
        assert_eq!(
            mirror_url(
                "https://resources.download.minecraft.net/aa/abcdef",
                "bmclapi",
                "1.21.1"
            ),
            "https://bmclapi2.bangbang93.com/assets/aa/abcdef"
        );
    }

    #[test]
    fn rejects_unsafe_metadata_paths() {
        assert!(safe_join(Path::new("root"), "../escape").is_err());
        assert!(safe_join(Path::new("root"), "org/example/a.jar").is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn resumes_partial_download() {
        use std::io::{Read, Write};

        let payload = b"hello world".to_vec();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let expected = payload.clone();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 2048];
            let length = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..length]).to_lowercase();
            assert!(request.contains("range: bytes=6-"));
            let header = format!(
                "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes 6-{}/{}\r\nConnection: close\r\n\r\n",
                expected.len() - 6,
                expected.len() - 1,
                expected.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(&expected[6..]).unwrap();
        });

        let root = std::env::temp_dir().join(format!("mcl-download-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).await.unwrap();
        let destination = root.join("asset.bin");
        let part = part_path(&destination);
        fs::write(&part, &payload[..6]).await.unwrap();

        let item = DownloadItem {
            url: format!("http://{address}/asset.bin"),
            destination: destination.clone(),
            sha1: Some(hex::encode(Sha1::digest(&payload))),
            size: Some(payload.len() as u64),
            label: "test asset".to_string(),
            version_id: String::new(),
        };
        let reporter = ProgressReporter::new(
            "test-job".to_string(),
            "test-version".to_string(),
            Arc::new(|_: InstallProgress| {}),
        );

        download_file(&Client::new(), &item, "official", false, &reporter)
            .await
            .unwrap();
        assert_eq!(fs::read(&destination).await.unwrap(), payload);

        server.join().unwrap();
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "requires network access"]
    async fn downloads_real_asset_with_checksum() {
        let root =
            std::env::temp_dir().join(format!("mcl-real-asset-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).await.unwrap();
        let destination = root.join("asset.bin");
        let item = DownloadItem {
            url: "https://resources.download.minecraft.net/b6/b62ca8ec10d07e6bf5ac8dae0c8c1d2e6a1e3356"
                .to_string(),
            destination: destination.clone(),
            sha1: Some("b62ca8ec10d07e6bf5ac8dae0c8c1d2e6a1e3356".to_string()),
            size: Some(9101),
            label: "real asset".to_string(),
            version_id: String::new(),
        };
        let reporter = ProgressReporter::new(
            "test-job".to_string(),
            "test-version".to_string(),
            Arc::new(|_: InstallProgress| {}),
        );
        let client = Client::builder()
            .user_agent("MCL-Launcher-test/0.1.0")
            .build()
            .unwrap();

        download_file(&client, &item, "official", false, &reporter)
            .await
            .unwrap();
        verify_file(&destination, item.sha1.as_deref())
            .await
            .unwrap();
        assert_eq!(fs::metadata(&destination).await.unwrap().len(), 9101);

        let _ = std::fs::remove_dir_all(root);
    }
}
