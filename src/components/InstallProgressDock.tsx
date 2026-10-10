import { AlertCircle, CheckCircle2, Download, X } from "lucide-react";
import type { InstallProgress } from "../api/types";
import { useLauncherStore } from "../stores/launcher";

function ProgressItem({ progress }: { progress: InstallProgress }) {
  const removeInstallProgress = useLauncherStore((state) => state.removeInstallProgress);
  const running = progress.status === "running";
  const failed = progress.status === "error";

  return (
    <div className="rounded-xl bg-black/[0.12] px-2.5 py-2">
      <div className="flex min-w-0 items-center gap-2">
        {failed ? (
          <AlertCircle className="shrink-0 text-rose-300" size={14} />
        ) : progress.status === "complete" ? (
          <CheckCircle2 className="shrink-0 text-emerald-300" size={14} />
        ) : (
          <Download className="shrink-0 animate-pulse text-cyan-200" size={14} />
        )}
        <span className="min-w-0 flex-1 truncate text-[11px] font-medium text-slate-200">
          {progress.message}
        </span>
        <span className="shrink-0 text-[10px] tabular-nums text-slate-400">
          {progress.percent.toFixed(0)}%
        </span>
        {!running ? (
          <button
            type="button"
            onClick={() => removeInstallProgress(progress.jobId)}
            className="grid h-5 w-5 shrink-0 place-items-center rounded-full text-slate-500 transition hover:bg-white/10 hover:text-white"
            aria-label="关闭下载进度"
          >
            <X size={12} />
          </button>
        ) : null}
      </div>
      <div className="mt-2 h-1 overflow-hidden rounded-full bg-slate-950/60">
        <div
          className={[
            "h-full rounded-full transition-[width] duration-300",
            failed
              ? "bg-rose-400"
              : progress.status === "complete"
                ? "bg-emerald-400"
                : "progress-sheen bg-gradient-to-r from-cyan-400 via-sky-400 to-blue-400",
          ].join(" ")}
          style={{ width: `${Math.max(0, Math.min(100, progress.percent))}%` }}
        />
      </div>
      <div className="mt-1.5 truncate text-[9px] text-slate-500">
        {progress.stage}
        {progress.totalFiles > 0 ? ` · ${progress.completedFiles}/${progress.totalFiles} 文件` : ""}
      </div>
    </div>
  );
}

export function InstallProgressDock() {
  const installProgress = useLauncherStore((state) => state.installProgress);
  const jobs = Object.values(installProgress).sort((left, right) => {
    if (left.status === "running" && right.status !== "running") return -1;
    if (left.status !== "running" && right.status === "running") return 1;
    return 0;
  });
  const activeCount = jobs.filter((job) => job.status === "running").length;
  const latest = jobs[0];

  if (jobs.length === 0) return null;

  return (
    <div className="group/progress relative mt-auto mb-2 rounded-2xl border border-cyan-300/[0.09] bg-cyan-300/[0.035] p-1.5 transition-all duration-300 group-hover/sidebar:p-2.5">
      <div className="flex h-9 items-center justify-center transition-all duration-300 group-hover/sidebar:h-0 group-hover/sidebar:overflow-hidden group-hover/sidebar:opacity-0">
        <div className="relative grid h-8 w-8 place-items-center rounded-xl bg-cyan-300/[0.08] text-cyan-200">
          <Download size={16} />
          {activeCount > 0 ? (
            <span className="absolute -right-1 -top-1 grid h-4 min-w-4 place-items-center rounded-full bg-cyan-300 px-1 text-[9px] font-bold text-slate-950">
              {activeCount}
            </span>
          ) : null}
        </div>
        <div className="absolute bottom-1 left-2 right-2 h-0.5 overflow-hidden rounded-full bg-slate-950/70">
          <div
            className="progress-sheen h-full rounded-full bg-gradient-to-r from-cyan-400 to-blue-400 transition-[width] duration-300"
            style={{ width: `${latest?.percent ?? 0}%` }}
          />
        </div>
      </div>

      <div className="max-h-0 overflow-hidden opacity-0 transition-all duration-300 group-hover/sidebar:max-h-72 group-hover/sidebar:opacity-100">
        <div className="mb-2 flex items-center justify-between gap-2 px-1">
          <span className="text-[10px] font-semibold tracking-[0.12em] text-slate-300">下载与安装</span>
          <span className="rounded-full bg-cyan-300/10 px-1.5 py-0.5 text-[9px] text-cyan-200">
            {activeCount > 0 ? `${activeCount} 项进行中` : "最近任务"}
          </span>
        </div>
        <div className="max-h-60 space-y-1.5 overflow-y-auto pr-0.5">
          {jobs.map((progress) => (
            <ProgressItem key={progress.jobId} progress={progress} />
          ))}
        </div>
      </div>
    </div>
  );
}
