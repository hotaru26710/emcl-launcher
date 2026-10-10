import { create } from "zustand";
import { api, errorMessage } from "../api/tauri";
import type {
  CreateInstanceInput,
  InstallProgress,
  Instance,
  Settings,
  UpdateInstanceInput,
  UpdateSettingsInput,
  VersionManifest,
} from "../api/types";

interface ActiveInstall {
  jobId: string;
  versionId: string;
  instanceId?: string | null;
}

interface LauncherStore {
  initialized: boolean;
  initializing: boolean;
  instances: Instance[];
  manifest: VersionManifest | null;
  settings: Settings | null;
  loadingInstances: boolean;
  loadingManifest: boolean;
  savingSettings: boolean;
  activeInstall: ActiveInstall | null;
  installProgress: Record<string, InstallProgress>;
  lastError: string | null;
  initialize: () => Promise<void>;
  refreshInstances: () => Promise<void>;
  refreshManifest: (forceRefresh?: boolean) => Promise<void>;
  createInstance: (input: CreateInstanceInput) => Promise<Instance>;
  updateInstance: (id: string, input: UpdateInstanceInput) => Promise<Instance>;
  deleteInstance: (id: string) => Promise<void>;
  saveSettings: (input: UpdateSettingsInput) => Promise<Settings>;
  startInstall: (versionId: string, instanceId?: string | null) => Promise<void>;
  setInstallProgress: (progress: InstallProgress) => void;
  clearActiveInstall: () => void;
  clearError: () => void;
}

function upsertInstance(instances: Instance[], instance: Instance): Instance[] {
  const exists = instances.some((item) => item.id === instance.id);
  const next = exists
    ? instances.map((item) => (item.id === instance.id ? instance : item))
    : [instance, ...instances];
  return next.sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
}

export const useLauncherStore = create<LauncherStore>((set, get) => ({
  initialized: false,
  initializing: false,
  instances: [],
  manifest: null,
  settings: null,
  loadingInstances: false,
  loadingManifest: false,
  savingSettings: false,
  activeInstall: null,
  installProgress: {},
  lastError: null,

  initialize: async () => {
    if (get().initialized || get().initializing) return;
    set({ initializing: true, lastError: null });
    try {
      const [settings, instances, manifest] = await Promise.all([
        api.getSettings(),
        api.listInstances(),
        api.getVersionManifest(false),
      ]);
      set({
        settings,
        instances,
        manifest,
        initialized: true,
        initializing: false,
      });
    } catch (error) {
      set({
        initializing: false,
        lastError: errorMessage(error),
      });
    }
  },

  refreshInstances: async () => {
    set({ loadingInstances: true, lastError: null });
    try {
      const instances = await api.listInstances();
      set({ instances, loadingInstances: false });
    } catch (error) {
      set({ loadingInstances: false, lastError: errorMessage(error) });
      throw error;
    }
  },

  refreshManifest: async (forceRefresh = false) => {
    set({ loadingManifest: true, lastError: null });
    try {
      const manifest = await api.getVersionManifest(forceRefresh);
      set({ manifest, loadingManifest: false });
    } catch (error) {
      set({ loadingManifest: false, lastError: errorMessage(error) });
      throw error;
    }
  },

  createInstance: async (input) => {
    try {
      const instance = await api.createInstance(input);
      set((state) => ({
        instances: upsertInstance(state.instances, instance),
        lastError: null,
      }));
      return instance;
    } catch (error) {
      set({ lastError: errorMessage(error) });
      throw error;
    }
  },

  updateInstance: async (id, input) => {
    try {
      const instance = await api.updateInstance(id, input);
      set((state) => ({
        instances: upsertInstance(state.instances, instance),
        lastError: null,
      }));
      return instance;
    } catch (error) {
      set({ lastError: errorMessage(error) });
      throw error;
    }
  },

  deleteInstance: async (id) => {
    try {
      await api.deleteInstance(id);
      set((state) => ({
        instances: state.instances.filter((instance) => instance.id !== id),
        lastError: null,
      }));
    } catch (error) {
      set({ lastError: errorMessage(error) });
      throw error;
    }
  },

  saveSettings: async (input) => {
    set({ savingSettings: true, lastError: null });
    try {
      const settings = await api.updateSettings(input);
      set({ settings, savingSettings: false });
      return settings;
    } catch (error) {
      set({ savingSettings: false, lastError: errorMessage(error) });
      throw error;
    }
  },

  startInstall: async (versionId, instanceId = null) => {
    const jobId = crypto.randomUUID();
    set((state) => ({
      activeInstall: { jobId, versionId, instanceId },
      installProgress: {
        ...state.installProgress,
        [jobId]: {
          jobId,
          versionId,
          status: "running",
          stage: "starting",
          message: "正在准备安装",
          completedFiles: 0,
          totalFiles: 0,
          downloadedBytes: 0,
          totalBytes: 0,
          percent: 0,
        },
      },
      lastError: null,
    }));

    try {
      await api.installVersion({ versionId, instanceId, jobId });
      await get().refreshInstances().catch(() => undefined);
    } catch (error) {
      const message = errorMessage(error);
      set((state) => ({
        lastError: message,
        installProgress: {
          ...state.installProgress,
          [jobId]: {
            ...(state.installProgress[jobId] ?? {
              jobId,
              versionId,
              completedFiles: 0,
              totalFiles: 0,
              downloadedBytes: 0,
              totalBytes: 0,
            }),
            status: "error",
            stage: "error",
            message,
            percent: state.installProgress[jobId]?.percent ?? 0,
          } as InstallProgress,
        },
      }));
      throw error;
    } finally {
      await get().refreshInstances().catch(() => undefined);
    }
  },

  setInstallProgress: (progress) => {
    set((state) => {
      return {
        activeInstall: state.activeInstall,
        installProgress: {
          ...state.installProgress,
          [progress.jobId]: progress,
        },
        lastError: progress.status === "error" ? progress.message : state.lastError,
      };
    });
  },

  clearActiveInstall: () => set({ activeInstall: null }),
  clearError: () => set({ lastError: null }),
}));
