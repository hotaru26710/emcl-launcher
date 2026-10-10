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
  sharedDirectory: string;
  dataDirectory: string;
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
