import { useEffect, useState } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { ArrowLeftRight, X } from "lucide-react";
import { api } from "./api/tauri";
import { Layout } from "./components/Layout";
import { ThemeManager } from "./components/ThemeManager";
import { InstanceDetail } from "./pages/InstanceDetail";
import { Instances } from "./pages/Instances";
import { CreateInstance } from "./pages/CreateInstance";
import { CommunityMods } from "./pages/CommunityMods";
import { CommunityModDetail } from "./pages/CommunityModDetail";
import { ConflictReport } from "./pages/ConflictReport";
import { BuildTimeline } from "./pages/BuildTimeline";
import { ProjectDetail } from "./pages/ProjectDetail";
import { Projects } from "./pages/Projects";
import { Login } from "./pages/Login";
import { SettingsPage } from "./pages/Settings";
import { Versions } from "./pages/Versions";
import { useLauncherStore } from "./stores/launcher";

function GlobalError() {
  const error = useLauncherStore((state) => state.lastError);
  const clearError = useLauncherStore((state) => state.clearError);
  const settings = useLauncherStore((state) => state.settings);
  const saveSettings = useLauncherStore((state) => state.saveSettings);
  const [switching, setSwitching] = useState(false);

  if (!error) return null;

  const networkLike = /请求失败|请求被拒绝|无法连接|连接失败|timeout|timed out|network|dns|tls|ssl|connect|maven/i.test(error);
  const currentMirror = settings?.downloadMirror === "bmclapi" ? "bmclapi" : "official";
  const targetMirror = currentMirror === "bmclapi" ? "official" : "bmclapi";

  const switchMirror = async () => {
    setSwitching(true);
    try {
      await saveSettings({ downloadMirror: targetMirror });
      clearError();
    } catch {
      // The store will surface the switch failure instead.
    } finally {
      setSwitching(false);
    }
  };

  return (
    <div className="animate-card-in fixed bottom-5 right-5 z-50 w-[min(34rem,calc(100vw-2.5rem))] rounded-[26px] border border-rose-400/20 bg-[#25131a]/95 p-4 text-sm text-rose-100 shadow-2xl backdrop-blur-xl">
      <div className="flex items-start gap-3">
        <div className="min-w-0 flex-1 whitespace-pre-wrap">{error}</div>
        <button
          type="button"
          onClick={clearError}
          className="rounded-lg p-1 text-rose-300 transition hover:bg-white/5 hover:text-white"
          aria-label="关闭错误提示"
        >
          <X size={16} />
        </button>
      </div>
      {networkLike ? (
        <div className="mt-3 flex flex-wrap items-center gap-2 border-t border-rose-300/10 pt-3">
          <span className="text-xs text-rose-200/70">网络请求失败，可以切换下载源：</span>
          <button
            type="button"
            disabled={switching}
            onClick={() => void switchMirror()}
            className="inline-flex items-center gap-1.5 rounded-xl border border-cyan-300/20 bg-cyan-300/10 px-3 py-1.5 text-xs font-medium text-cyan-100 transition hover:bg-cyan-300/15 disabled:opacity-40"
          >
            <ArrowLeftRight size={13} />
            {switching ? "切换中..." : `切换到 ${targetMirror === "bmclapi" ? "BMCLAPI 镜像" : "官方源"}`}
          </button>
        </div>
      ) : null}
    </div>
  );
}

function App() {
  const initialize = useLauncherStore((state) => state.initialize);
  const setInstallProgress = useLauncherStore((state) => state.setInstallProgress);
  const appendLaunchLog = useLauncherStore((state) => state.appendLaunchLog);
  const setInstanceRunning = useLauncherStore((state) => state.setInstanceRunning);

  useEffect(() => {
    void initialize();
  }, [initialize]);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    const register = (listener: Promise<() => void>) => {
      void listener.then((stopListening) => {
        if (disposed) {
          stopListening();
        } else {
          unlisteners.push(stopListening);
        }
      });
    };

    register(api.onInstallProgress(setInstallProgress));
    register(api.onLaunchLog(appendLaunchLog));
    register(
      api.onLaunchExit((exit) => {
        setInstanceRunning(exit.instanceId, false);
      }),
    );

    return () => {
      disposed = true;
      unlisteners.forEach((stopListening) => stopListening());
    };
  }, [setInstallProgress, appendLaunchLog, setInstanceRunning]);

  return (
    <HashRouter>
      <ThemeManager />
      <Routes>
        <Route element={<Layout />}>
          <Route index element={<Navigate to="/instances" replace />} />
          <Route path="/instances" element={<Instances />} />
          <Route path="/instances/create" element={<CreateInstance />} />
          <Route path="/instances/:id/community" element={<CommunityMods />} />
          <Route path="/instances/:id/community-detail" element={<CommunityModDetail />} />
          <Route path="/instances/:id/conflicts" element={<ConflictReport />} />
          <Route path="/instances/:id" element={<InstanceDetail />} />
          <Route path="/projects" element={<Projects />} />
          <Route path="/projects/:id" element={<ProjectDetail />} />
          <Route path="/projects/:id/timeline" element={<BuildTimeline />} />
          <Route path="/versions" element={<Versions />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="/login" element={<Login />} />
          <Route path="*" element={<Navigate to="/instances" replace />} />
        </Route>
      </Routes>
      <GlobalError />
    </HashRouter>
  );
}

export default App;
