import { CheckCircle2, LoaderCircle, TriangleAlert } from "lucide-react";
import type { InstallProgress } from "../api/types";
import { formatBytes } from "../lib/format";

interface ProgressBarProps {
  progress: InstallProgress;
  compact?: boolean;
}

export function InstallProgressCard({ progress, compact = false }: ProgressBarProps) {
  const isComplete = progress.status === "complete";
  const isError = progress.status === "error";
  const icon = isComplete ? (
    <CheckCircle2 className="text-emerald-300" size={19} />
  ) : isError ? (
    <TriangleAlert className="text-rose-300" size={19} />
  ) : (
    <LoaderCircle className="animate-spin text-cyan-300" size={19} />
  );

  return (
    <div
      className={[
        "animate-card-in glass-panel relative overflow-hidden rounded-[26px] p-5",
        isError
          ? "border-rose-400/20 bg-rose-400/[0.045]"
          : isComplete
            ? "border-emerald-400/20 bg-emerald-400/[0.04]"
            : "border-cyan-300/15 bg-cyan-300/[0.04]",
      ].join(" ")}
    >
      <div className="pointer-events-none absolute -right-12 -top-16 h-36 w-36 rounded-full bg-cyan-300/[0.08] blur-3xl" />
      <div className="relative flex items-start gap-3.5">
        <div className="grid h-10 w-10 shrink-0 place-items-center rounded-2xl border border-white/[0.07] bg-slate-950/45">
          {icon}
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between gap-4">
            <div className="truncate text-sm font-semibold text-slate-100">
              {progress.message}
            </div>
            <div className="shrink-0 rounded-full border border-white/[0.06] bg-white/[0.035] px-2.5 py-1 text-[11px] tabular-nums text-slate-400">
              {progress.percent.toFixed(1)}%
            </div>
          </div>
          <div className="mt-4 h-2.5 overflow-hidden rounded-full bg-slate-950/75 shadow-inner">
            <div
              className={[
                "relative h-full rounded-full transition-[width] duration-300 ease-out",
                isError
                  ? "bg-rose-400"
                  : isComplete
                    ? "bg-gradient-to-r from-emerald-400 to-cyan-300"
                    : "progress-sheen bg-gradient-to-r from-cyan-400 via-sky-400 to-blue-400 shadow-[0_0_18px_rgba(34,211,238,0.35)]",
              ].join(" ")}
              style={{ width: `${Math.max(0, Math.min(100, progress.percent))}%` }}
            />
          </div>
          {!compact ? (
            <div className="mt-3 flex flex-wrap items-center justify-between gap-2 text-[11px] text-slate-500">
              <span className="rounded-full bg-white/[0.035] px-2.5 py-1 capitalize">
                {progress.stage}
              </span>
              <span className="tabular-nums">
                {progress.totalFiles > 0
                  ? `${progress.completedFiles} / ${progress.totalFiles} 文件`
                  : "准备中"}
                {progress.totalBytes > 0
                  ? ` · ${formatBytes(progress.downloadedBytes)} / ${formatBytes(progress.totalBytes)}`
                  : ""}
              </span>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
