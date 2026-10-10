import { useEffect } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { X } from "lucide-react";
import { api } from "./api/tauri";
import { Layout } from "./components/Layout";
import { InstanceDetail } from "./pages/InstanceDetail";
import { Instances } from "./pages/Instances";
import { Login } from "./pages/Login";
import { SettingsPage } from "./pages/Settings";
import { Versions } from "./pages/Versions";
import { useLauncherStore } from "./stores/launcher";

function GlobalError() {
  const error = useLauncherStore((state) => state.lastError);
  const clearError = useLauncherStore((state) => state.clearError);
  if (!error) return null;

  return (
    <div className="animate-card-in fixed bottom-5 right-5 z-50 flex max-w-xl items-start gap-3 rounded-[24px] border border-rose-400/20 bg-[#25131a]/95 px-4 py-3 text-sm text-rose-100 shadow-2xl backdrop-blur">
      <div className="min-w-0 whitespace-pre-wrap">{error}</div>
      <button
        type="button"
        onClick={clearError}
        className="rounded-lg p-1 text-rose-300 transition hover:bg-white/5 hover:text-white"
        aria-label="关闭错误提示"
      >
        <X size={16} />
      </button>
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
      <Routes>
        <Route element={<Layout />}>
          <Route index element={<Navigate to="/instances" replace />} />
          <Route path="/instances" element={<Instances />} />
          <Route path="/instances/:id" element={<InstanceDetail />} />
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
