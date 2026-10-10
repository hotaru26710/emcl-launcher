import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AccountSummary,
  CreateInstanceInput,
  DeviceCodeResponse,
  InstallProgress,
  InstallSummary,
  InstallVersionInput,
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
  setActiveAccount: (id: string) => invoke<void>("set_active_account", { id }),
  getActiveAccount: () => invoke<AccountSummary | null>("get_active_account"),
  startMicrosoftLogin: () => invoke<DeviceCodeResponse>("start_microsoft_login"),
  pollMicrosoftLogin: (deviceCode: string) =>
    invoke<MicrosoftPollResult>("poll_microsoft_login", { deviceCode }),
  refreshMicrosoftAccount: (id: string) =>
    invoke<AccountSummary>("refresh_microsoft_account", { id }),
  installVersion: (input: InstallVersionInput) =>
    invoke<InstallSummary>("install_version", { input }),
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
};

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}
