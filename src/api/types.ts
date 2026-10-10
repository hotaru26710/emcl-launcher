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
  defaultJavaPath?: string | null;
  microsoftClientId?: string | null;
  defaultMemoryMb: number;
  gameWidth: number;
  gameHeight: number;
  fullscreen: boolean;
  sharedDirectory: string;
  dataDirectory: string;
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
  microsoftClientId?: string;
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
