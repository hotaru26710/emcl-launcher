export interface ProjectBuildLog {
  projectId: string;
  stream: "stdout" | "stderr" | string;
  line: string;
  timestamp: string;
}

export interface ProjectBuildExit {
  projectId: string;
  success: boolean;
  code?: number | null;
}

export interface ConflictFinding {
  severity: string;
  conflictType: string;
  sourceModId?: string | null;
  targetModId?: string | null;
  detail: string;
  suggestion: string;
}

export interface CrashAnalysis {
  category: string;
  summary: string;
  severity: string;
  matchedLines: string[];
  suggestion: string;
}

export interface CrashAnalysisProgress {
  jobId: string;
  status: "running" | "complete" | "error" | string;
  stage: string;
  message: string;
  percent: number;
  instanceId?: string | null;
}

export interface CrashGameContext {
  minecraftVersion?: string | null;
  loader?: string | null;
  loaderVersion?: string | null;
  javaVersion?: string | null;
  operatingSystem?: string | null;
}

export interface CrashFinding {
  category: string;
  title: string;
  severity: string;
  confidence: number;
  evidence: string[];
  explanation: string;
  suggestion: string;
}

export interface SuspectMod {
  modId: string;
  name: string;
  version?: string | null;
  score: number;
  confidence: string;
  reasons: string[];
}

export interface CrashAnalysisReport {
  id: string;
  source: string;
  archivePath: string;
  instanceId?: string | null;
  instanceName?: string | null;
  createdAt: string;
  severity: string;
  summary: string;
  confidence: number;
  game: CrashGameContext;
  findings: CrashFinding[];
  suspectedMods: SuspectMod[];
  recommendations: string[];
  evidence: string[];
  includedFiles: string[];
  analyzedLines: number;
}

export interface CrashReportReady {
  instanceId: string;
  archivePath: string;
  report: CrashAnalysisReport;
}

export interface DependencyRepairResult {
  installed: string[];
  failed: string[];
  remaining: ConflictFinding[];
}

export interface ConflictReport {
  id: number;
  instanceId: string;
  severity: string;
  conflictType: string;
  sourceModId?: string | null;
  targetModId?: string | null;
  detail: string;
  suggestion: string;
  createdAt: string;
}

export interface ModMetadata {
  instanceId: string;
  modId?: string | null;
  modName?: string | null;
  version?: string | null;
  loader?: string | null;
  fileName: string;
  sha256: string;
  sourceType: string;
  dependsJson: string;
  breaksJson: string;
  conflictsJson: string;
}

export interface VersionManifest {
  latest: {
    release: string;
    snapshot: string;
  };
  versions: VersionSummary[];
}

export interface VersionSummary {
  id: string;
  versionType: "release" | "snapshot" | "old_beta" | "old_alpha" | string;
  url: string;
  time: string;
  releaseTime: string;
  sha1?: string | null;
  complianceLevel?: number | null;
}

export interface DownloadArtifact {
  path?: string | null;
  sha1?: string | null;
  size?: number | null;
  url: string;
}

export interface VersionDetails {
  id: string;
  versionType: string;
  time: string;
  releaseTime: string;
  mainClass?: string | null;
  assets?: string | null;
  assetIndex?: {
    id: string;
    sha1: string;
    size: number;
    totalSize?: number | null;
    url: string;
  } | null;
  downloads: {
    client?: DownloadArtifact | null;
    server?: DownloadArtifact | null;
  };
  libraries: Array<{
    name: string;
    downloads: {
      artifact?: DownloadArtifact | null;
      classifiers?: Record<string, DownloadArtifact> | null;
    };
    natives?: Record<string, string> | null;
  }>;
  javaVersion?: {
    component: string;
    majorVersion: number;
  } | null;
  logging?: unknown;
}

export interface ProjectInspection {
  name: string;
  rootPath: string;
  buildTool: string;
  outputDir: string;
  gradleTask: string;
}

export interface Project {
  id: string;
  name: string;
  rootPath: string;
  buildTool: string;
  outputDir: string;
  gradleTask: string;
  autoBuild: boolean;
  autoDeploy: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface LinkedProjectInstance {
  projectId: string;
  instanceId: string;
  instanceName: string;
  autoDeploy: boolean;
}

export interface CreateProjectInput {
  name: string;
  rootPath: string;
  outputDir?: string | null;
  gradleTask?: string | null;
  autoBuild?: boolean | null;
  autoDeploy?: boolean | null;
  instanceIds?: string[] | null;
}

export interface UpdateProjectInput {
  name?: string | null;
  outputDir?: string | null;
  gradleTask?: string | null;
  autoBuild?: boolean | null;
  autoDeploy?: boolean | null;
}

export interface BuildArtifact {
  id: string;
  projectId: string;
  filePath: string;
  sha256: string;
  fileSize: number;
  builtAt: string;
  gitCommit?: string | null;
  gitBranch?: string | null;
  status: string;
  deployedInstanceIds: string[];
}

export interface Instance {
  id: string;
  name: string;
  versionId: string;
  loader?: string | null;
  loaderVersion?: string | null;
  javaPath?: string | null;
  jvmArgs: string;
  memoryMb: number;
  directory: string;
  createdAt: string;
  updatedAt: string;
  lastPlayedAt?: string | null;
}

export interface CreateInstanceInput {
  name: string;
  versionId: string;
  loader?: string | null;
  loaderVersion?: string | null;
  memoryMb?: number | null;
  jvmArgs?: string | null;
}

export interface UpdateInstanceInput {
  name?: string | null;
  versionId?: string | null;
  loader?: string | null;
  loaderVersion?: string | null;
  javaPath?: string | null;
  memoryMb?: number | null;
  jvmArgs?: string | null;
}

export interface Settings {
  downloadMirror: "official" | "bmclapi" | string;
  downloadConcurrency: number;
  modApiMode: "auto" | "official" | string;
  curseforgeApiKey?: string | null;
  defaultJavaPath?: string | null;
  microsoftClientId?: string | null;
  theme: ThemeMode;
  customTheme?: string | null;
  defaultMemoryMb: number;
  gameWidth: number;
  gameHeight: number;
  fullscreen: boolean;
  sharedDirectory: string;
  dataDirectory: string;
}

export type ThemeMode = "system" | "dark" | "light" | "pink" | "ocean" | "forest" | "amber" | "custom";

export interface CustomThemeColors {
  accent: string;
  background: string;
  panel: string;
  text: string;
  muted: string;
}

export type LoaderType = "fabric" | "quilt" | "neoforge" | "forge";

export interface LoaderVersionInfo {
  version: string;
  stable: boolean;
}

export interface LoaderInstallResult {
  versionId: string;
  loader: LoaderType;
  loaderVersion: string;
  profilePath: string;
}

export interface InstallLoaderInput {
  instanceId: string;
  loader: LoaderType;
  loaderVersion?: string | null;
  jobId?: string | null;
}

export interface CommunityInstallResult {
  installed: number;
  skipped: number;
  dependencies: string[];
  mods: ModInfo[];
}

export interface McimCurseForgeSearchResult {
  id: number;
  projectId: string;
  slug: string;
  title: string;
  description: string;
  iconUrl?: string | null;
  downloads: number;
  author: string;
  categories: string[];
}

export interface McimCurseForgeFile {
  id: number;
  modId: number;
  displayName: string;
  fileName: string;
  fileDate: string;
  fileLength: number;
  downloadUrl?: string | null;
  gameVersions: string[];
  releaseType: number;
  sha1?: string | null;
}

export interface SearchMcimInput {
  query: string;
  gameVersion: string;
  loader?: string | null;
  offset?: number | null;
}

export interface InstallMcimInput {
  instanceId: string;
  modId: number;
  fileId: number;
}

export interface GitHubRepository {
  id: number;
  fullName: string;
  name: string;
  owner: string;
  description?: string | null;
  htmlUrl: string;
  stars: number;
  forks: number;
  updatedAt: string;
  topics: string[];
}

export interface GitHubAsset {
  id: number;
  name: string;
  size: number;
  downloadUrl: string;
  contentType?: string | null;
}

export interface GitHubRelease {
  id: number;
  tagName: string;
  name?: string | null;
  htmlUrl: string;
  publishedAt?: string | null;
  prerelease: boolean;
  assets: GitHubAsset[];
}

export interface ModProjectDetails {
  platform: "modrinth" | "curseforge" | string;
  projectId: string;
  slug: string;
  title: string;
  description: string;
  body: string;
  iconUrl?: string | null;
  author: string;
  downloads: number;
  follows: number;
  categories: string[];
  gameVersions: string[];
  loaders: string[];
  updatedAt: string;
}

export interface ModSourceVersionInfo {
  platform: "modrinth" | "curseforge" | string;
  projectId: string;
  versionId: string;
  name: string;
  versionNumber: string;
  gameVersions: string[];
  loaders: string[];
  releaseType: string;
  publishedAt: string;
  downloads: number;
  files: Array<{
    url: string;
    fileName: string;
    primary: boolean;
    size: number;
    sha1?: string | null;
    sha512?: string | null;
  }>;
  dependencies: Array<{
    projectId?: string | null;
    versionId?: string | null;
    dependencyType: string;
  }>;
}

export interface ModrinthSearchResult {
  projectId: string;
  slug: string;
  title: string;
  description: string;
  iconUrl?: string | null;
  downloads: number;
  follows: number;
  author: string;
  categories: string[];
  versions: string[];
}

export interface ModrinthHashes {
  sha1?: string | null;
  sha512?: string | null;
}

export interface ModrinthFile {
  url: string;
  filename: string;
  primary: boolean;
  size: number;
  hashes: ModrinthHashes;
}

export interface ModrinthVersionInfo {
  id: string;
  projectId: string;
  name: string;
  versionNumber: string;
  versionType: string;
  gameVersions: string[];
  loaders: string[];
  datePublished: string;
  downloads: number;
  recommended: boolean;
  files: ModrinthFile[];
}

export interface SearchModrinthInput {
  query: string;
  gameVersion: string;
  loader?: string | null;
  offset?: number | null;
}

export interface InstallModrinthInput {
  instanceId: string;
  versionId: string;
  projectId?: string | null;
}

export interface InstalledMod {
  instanceId: string;
  modId: string;
  modName?: string | null;
  version?: string | null;
  platform: string;
  projectId: string;
  versionId?: string | null;
  fileName: string;
  sha256: string;
  sourceType: string;
  updateAvailable: boolean;
  latestVersionId?: string | null;
  dependencyOf?: string | null;
}

export interface ModUpdate {
  modId: string;
  modName?: string | null;
  fileName: string;
  platform: string;
  projectId: string;
  currentVersion?: string | null;
  latestVersion?: string | null;
  latestVersionId: string;
  updateAvailable: boolean;
}

export interface ModInfo {
  fileName: string;
  name: string;
  modId?: string | null;
  version?: string | null;
  loader?: string | null;
  description?: string | null;
  enabled: boolean;
}

export interface AccountSummary {
  id: string;
  username: string;
  uuid: string;
  accountType: "offline" | "microsoft" | string;
  expiresAt?: string | null;
  createdAt: string;
  active: boolean;
}

export interface DeviceCodeResponse {
  deviceCode: string;
  userCode: string;
  verificationUri: string;
  expiresIn: number;
  interval: number;
  message: string;
}

export interface MicrosoftPollResult {
  status: string;
  message: string;
  account?: AccountSummary | null;
}

export interface JavaInstallation {
  path: string;
  javaHome?: string | null;
  version?: string | null;
  majorVersion?: number | null;
  vendor?: string | null;
  architecture?: string | null;
  is64Bit: boolean;
}

export interface JavaRecommendation {
  requiredMajor?: number | null;
  installation?: JavaInstallation | null;
  reason: string;
}

export interface UpdateSettingsInput {
  downloadMirror?: string;
  downloadConcurrency?: number;
  modApiMode?: "auto" | "official" | string;
  curseforgeApiKey?: string;
  microsoftClientId?: string;
  theme?: ThemeMode;
  customTheme?: string;
  defaultMemoryMb?: number;
  gameWidth?: number;
  gameHeight?: number;
  fullscreen?: boolean;
}

export interface InstallVersionInput {
  versionId: string;
  instanceId?: string | null;
  jobId?: string | null;
  forceRedownload?: boolean;
}

export interface InstallProgress {
  jobId: string;
  versionId: string;
  status: "running" | "complete" | "error" | string;
  stage: string;
  message: string;
  completedFiles: number;
  totalFiles: number;
  downloadedBytes: number;
  totalBytes: number;
  percent: number;
}

export interface InstallSummary {
  versionId: string;
  clientJar: string;
  statusPath: string;
  libraryCount: number;
  assetCount: number;
  totalBytes: number;
}

export interface LaunchResult {
  instanceId: string;
  pid: number;
}

export interface LaunchLog {
  instanceId: string;
  stream: "stdout" | "stderr" | string;
  line: string;
  timestamp: string;
}

export interface LaunchExit {
  instanceId: string;
  code?: number | null;
  success: boolean;
}

export interface LauncherInfo {
  dataDirectory: string;
  sharedDirectory: string;
  instancesDirectory: string;
  databasePath: string;
}
