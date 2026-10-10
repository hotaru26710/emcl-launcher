import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  CreateInstanceInput,
  InstallProgress,
  InstallSummary,
  InstallVersionInput,
  Instance,
  LauncherInfo,
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
  installVersion: (input: InstallVersionInput) =>
    invoke<InstallSummary>("install_version", { input }),
  onInstallProgress: (callback: (progress: InstallProgress) => void): Promise<UnlistenFn> =>
    listen<InstallProgress>("install-progress", (event) => callback(event.payload)),
};

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}
