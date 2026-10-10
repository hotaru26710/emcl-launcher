import { useEffect, useState } from "react";
import { X } from "lucide-react";
import { InstallProgressCard } from "./ProgressBar";
import { useLauncherStore } from "../stores/launcher";

export function InstallProgressDock() {
  const activeInstall = useLauncherStore((state) => state.activeInstall);
  const installProgress = useLauncherStore((state) => state.installProgress);
  const clearActiveInstall = useLauncherStore((state) => state.clearActiveInstall);
  const [visible, setVisible] = useState(true);
  const progress = activeInstall ? installProgress[activeInstall.jobId] : undefined;

  useEffect(() => {
    setVisible(true);
    if (!progress || progress.status !== "complete") return;
    const timer = window.setTimeout(() => {
      setVisible(false);
      clearActiveInstall();
    }, 3500);
    return () => window.clearTimeout(timer);
  }, [progress?.status, clearActiveInstall]);

  if (!activeInstall || !progress || !visible) return null;

  return (
    <div className="animate-card-in fixed bottom-5 left-5 z-50 w-[min(30rem,calc(100vw-2.5rem))]">
      <div className="relative">
        <InstallProgressCard progress={progress} />
        {progress.status !== "running" ? (
          <button
            type="button"
            onClick={() => {
              setVisible(false);
              clearActiveInstall();
            }}
            className="absolute -right-2 -top-2 grid h-7 w-7 place-items-center rounded-full border border-white/10 bg-slate-950 text-slate-400 shadow-lg transition hover:text-white"
            aria-label="关闭进度"
          >
            <X size={13} />
          </button>
        ) : null}
      </div>
    </div>
  );
}
