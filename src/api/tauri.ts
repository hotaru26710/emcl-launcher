import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AccountSummary,
  BuildArtifact,
  CommunityInstallResult,
  ConflictFinding,
  CrashAnalysis,
  ConflictReport,
  CreateInstanceInput,
  CreateProjectInput,
  DeviceCodeResponse,
  InstallLoaderInput,
  InstallProgress,
  InstallSummary,
  InstallVersionInput,
  LoaderInstallResult,
  LoaderVersionInfo,
  ModInfo,
  ModMetadata,
  InstallModrinthInput,
  ModrinthSearchResult,
  ModrinthVersionInfo,
  Project,
  ProjectBuildExit,
  ProjectBuildLog,
  ProjectInspection,
  LinkedProjectInstance,
  SearchModrinthInput,
  UpdateProjectInput,
  Instance,
  JavaInstallation,
  JavaRecommendation,
  LauncherInfo,
  LaunchExit,
  MicrosoftPollResult,
  LaunchLog,
  LaunchResult,
  Settings,
  UpdateInstanceInput,
  UpdateSettingsInput,
  VersionDetails,
  VersionManifest,
} from "./types";

export const api = {
  getLauncherInfo: () => invoke<LauncherInfo>("get_launcher_info"),
  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (input: UpdateSettingsInput) =>
    invoke<Settings>("update_settings", { input }),
  listInstances: () => invoke<Instance[]>("list_instances"),
  getInstance: (id: string) => invoke<Instance | null>("get_instance", { id }),
  createInstance: (input: CreateInstanceInput) =>
    invoke<Instance>("create_instance", { input }),
  updateInstance: (id: string, input: UpdateInstanceInput) =>
    invoke<Instance>("update_instance", { id, input }),
  deleteInstance: (id: string) => invoke<void>("delete_instance", { id }),
  getVersionManifest: (forceRefresh = false) =>
    invoke<VersionManifest>("get_version_manifest", { forceRefresh }),
  getVersionDetails: (versionId: string) =>
    invoke<VersionDetails>("get_version_details", { versionId }),
  detectJava: () => invoke<JavaInstallation[]>("detect_java"),
  recommendJava: (versionId: string) =>
    invoke<JavaRecommendation>("recommend_java", { versionId }),
  getDefaultJava: () => invoke<string | null>("get_default_java"),
  setDefaultJava: (javaPath: string | null) =>
    invoke<void>("set_default_java", { javaPath }),
  listAccounts: () => invoke<AccountSummary[]>("list_accounts"),
  createOfflineAccount: (username: string) =>
    invoke<AccountSummary>("create_offline_account", { username }),
  deleteAccount: (id: string) => invoke<void>("delete_account", { id }),
  inspectProjectDirectory: (path: string) =>
    invoke<ProjectInspection>("inspect_project_directory", { path }),
  listProjects: () => invoke<Project[]>("list_projects"),
  getProject: (id: string) => invoke<Project | null>("get_project", { id }),
  createProject: (input: CreateProjectInput) =>
    invoke<Project>("create_project", { input }),
  updateProject: (id: string, input: UpdateProjectInput) =>
    invoke<Project>("update_project", { id, input }),
  deleteProject: (id: string) => invoke<void>("delete_project", { id }),
  linkProjectInstance: (projectId: string, instanceId: string, autoDeploy = true) =>
    invoke<void>("link_project_instance", { projectId, instanceId, autoDeploy }),
  unlinkProjectInstance: (projectId: string, instanceId: string) =>
    invoke<void>("unlink_project_instance", { projectId, instanceId }),
  listProjectInstances: (projectId: string) =>
    invoke<LinkedProjectInstance[]>("list_project_instances", { projectId }),
  scanProjectArtifacts: (projectId: string) =>
    invoke<BuildArtifact[]>("scan_project_artifacts", { projectId }),
  listBuildArtifacts: (projectId: string) =>
    invoke<BuildArtifact[]>("list_build_artifacts", { projectId }),
  getBuildArtifact: (id: string) =>
    invoke<BuildArtifact | null>("get_build_artifact", { id }),
  runProjectBuild: (projectId: string) =>
    invoke<BuildArtifact[]>("run_project_build", { projectId }),
  deployBuildArtifact: (instanceId: string, artifactId: string) =>
    invoke<ModInfo[]>("deploy_build_artifact", { instanceId, artifactId }),
  checkInstanceConflicts: (instanceId: string, artifactId?: string | null) =>
    invoke<ConflictFinding[]>("check_instance_conflicts", { instanceId, artifactId }),
  listConflictReports: (instanceId: string) =>
    invoke<ConflictReport[]>("list_conflict_reports", { instanceId }),
  refreshInstanceModMetadata: (instanceId: string) =>
    invoke<ModMetadata[]>("refresh_instance_mod_metadata", { instanceId }),
  startProjectWatcher: (projectId: string) =>
    invoke<void>("start_project_watcher", { projectId }),
  stopProjectWatcher: (projectId: string) =>
    invoke<void>("stop_project_watcher", { projectId }),
  analyzeCrashLog: (instanceId: string) =>
    invoke<CrashAnalysis>("analyze_crash_log", { instanceId }),
  setActiveAccount: (id: string) => invoke<void>("set_active_account", { id }),
  getActiveAccount: () => invoke<AccountSummary | null>("get_active_account"),
  startMicrosoftLogin: () => invoke<DeviceCodeResponse>("start_microsoft_login"),
  pollMicrosoftLogin: (deviceCode: string) =>
    invoke<MicrosoftPollResult>("poll_microsoft_login", { deviceCode }),
  refreshMicrosoftAccount: (id: string) =>
    invoke<AccountSummary>("refresh_microsoft_account", { id }),
  installVersion: (input: InstallVersionInput) =>
    invoke<InstallSummary>("install_version", { input }),
  installLoader: (input: InstallLoaderInput) =>
    invoke<LoaderInstallResult>("install_loader", { input }),
  getLoaderVersions: (loader: string, gameVersion: string) =>
    invoke<LoaderVersionInfo[]>("get_loader_versions", { loader, gameVersion }),
  listMods: (instanceId: string) => invoke<ModInfo[]>("list_mods", { instanceId }),
  installMods: (instanceId: string, paths: string[]) =>
    invoke<ModInfo[]>("install_mods", { instanceId, paths }),
  toggleMod: (instanceId: string, fileName: string, enabled: boolean) =>
    invoke<ModInfo[]>("toggle_mod", { instanceId, fileName, enabled }),
  deleteMod: (instanceId: string, fileName: string) =>
    invoke<ModInfo[]>("delete_mod", { instanceId, fileName }),
  searchModrinthMods: (input: SearchModrinthInput) =>
    invoke<ModrinthSearchResult[]>("search_modrinth_mods", { input }),
  getModrinthVersions: (projectId: string, gameVersion: string, loader?: string | null) =>
    invoke<ModrinthVersionInfo[]>("get_modrinth_versions", { projectId, gameVersion, loader }),
  installModrinthVersion: (input: InstallModrinthInput) =>
    invoke<CommunityInstallResult>("install_modrinth_version", { input }),
  launchInstance: (instanceId: string) =>
    invoke<LaunchResult>("launch_instance", { instanceId }),
  stopInstance: (instanceId: string) => invoke<void>("stop_instance", { instanceId }),
  isInstanceRunning: (instanceId: string) =>
    invoke<boolean>("is_instance_running", { instanceId }),
  onInstallProgress: (callback: (progress: InstallProgress) => void): Promise<UnlistenFn> =>
    listen<InstallProgress>("install-progress", (event) => callback(event.payload)),
  onLaunchLog: (callback: (log: LaunchLog) => void): Promise<UnlistenFn> =>
    listen<LaunchLog>("launch-log", (event) => callback(event.payload)),
  onLaunchExit: (callback: (exit: LaunchExit) => void): Promise<UnlistenFn> =>
    listen<LaunchExit>("launch-exit", (event) => callback(event.payload)),
  onProjectBuildLog: (callback: (log: ProjectBuildLog) => void): Promise<UnlistenFn> =>
    listen<ProjectBuildLog>("project-build-log", (event) => callback(event.payload)),
  onProjectBuildExit: (callback: (exit: ProjectBuildExit) => void): Promise<UnlistenFn> =>
    listen<ProjectBuildExit>("project-build-exit", (event) => callback(event.payload)),
};

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}
