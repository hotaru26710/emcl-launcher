import { useMemo, useState } from "react";
import {
  Boxes,
  CloudDownload,
  RefreshCw,
  Search,
  Sparkles,
  TriangleAlert,
} from "lucide-react";
import type { VersionSummary } from "../api/types";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { formatDate, versionTypeLabel } from "../lib/format";
import { useLauncherStore } from "../stores/launcher";

type VersionFilter = "release" | "snapshot" | "old" | "all";

function filterVersion(version: VersionSummary, filter: VersionFilter): boolean {
  if (filter === "all") return true;
  if (filter === "old") {
    return version.versionType === "old_beta" || version.versionType === "old_alpha";
  }
  return version.versionType === filter;
}

export function Versions() {
  const manifest = useLauncherStore((state) => state.manifest);
  const loading = useLauncherStore((state) => state.loadingManifest);
  const refreshManifest = useLauncherStore((state) => state.refreshManifest);
  const startInstall = useLauncherStore((state) => state.startInstall);
  const activeInstall = useLauncherStore((state) => state.activeInstall);
  const installProgress = useLauncherStore((state) => state.installProgress);

  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<VersionFilter>("release");

  const versions = useMemo(() => {
    const search = query.trim().toLowerCase();
    return (manifest?.versions ?? [])
      .filter((version) => filterVersion(version, filter))
      .filter((version) => !search || version.id.toLowerCase().includes(search))
      .slice(0, 240);
  }, [filter, manifest, query]);

  const activeProgress = activeInstall ? installProgress[activeInstall.jobId] : undefined;
  const installing = activeProgress?.status === "running";

  const install = async (versionId: string) => {
    try {
      await startInstall(versionId);
    } catch {
      // The shared error banner owns the message.
    }
  };

  return (
    <div className="animate-page mx-auto max-w-6xl">
      <PageHeader
        title="版本清单"
        description="从 Mojang 官方清单读取版本信息，并使用全局共享缓存安装 client.jar、libraries 和 assets。支持官方源与 BMCLAPI 镜像。"
        action={
          <button
            type="button"
            onClick={() => void refreshManifest(true)}
            disabled={loading}
            className="inline-flex items-center gap-2 rounded-xl border border-white/10 bg-white/5 px-4 py-2.5 text-sm text-slate-200 transition hover:bg-white/8 disabled:opacity-50"
          >
            <RefreshCw size={16} className={loading ? "animate-spin" : ""} />
            刷新清单
          </button>
        }
      />

      <div className="mb-6 grid gap-4 md:grid-cols-2">
        {[
          { label: "最新正式版", value: manifest?.latest.release, tone: "cyan" as const },
          { label: "最新快照", value: manifest?.latest.snapshot, tone: "amber" as const },
        ].map((item) => (
          <div
            key={item.label}
            className="glass-panel hover-lift rounded-[28px] p-5"
          >
            <div className="flex items-start justify-between gap-4">
              <div>
                <div className="flex items-center gap-2 text-xs text-slate-500">
                  <Sparkles size={14} />
                  {item.label}
                </div>
                <div className="mt-3 text-2xl font-semibold tracking-tight text-white">
                  {item.value ?? "读取中..."}
                </div>
              </div>
              <StatusPill tone={item.tone}>
                {item.tone === "cyan" ? "RELEASE" : "SNAPSHOT"}
              </StatusPill>
            </div>
            {item.value ? (
              <button
                type="button"
                disabled={Boolean(installing)}
                onClick={() => void install(item.value!)}
                className="mt-4 inline-flex items-center gap-2 text-sm font-medium text-cyan-300 transition hover:text-cyan-200 disabled:opacity-40"
              >
                <CloudDownload size={16} />
                安装到共享目录
              </button>
            ) : null}
          </div>
        ))}
      </div>

      <div className="glass-panel animate-card-in overflow-hidden rounded-[28px]">
        <div className="flex flex-wrap items-center gap-3 border-b border-white/7 p-4">
          <div className="relative min-w-64 flex-1">
            <Search
              size={16}
              className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-slate-600"
            />
            <input
              value={query}
              onChange={(event) => setQuery(event.currentTarget.value)}
              placeholder="搜索版本号，例如 1.21.1"
              className="w-full rounded-xl border border-white/8 bg-slate-950/45 py-2.5 pl-9 pr-3 text-sm text-white outline-none placeholder:text-slate-700 focus:border-cyan-300/35"
            />
          </div>
          <div className="flex rounded-xl border border-white/8 bg-slate-950/40 p-1">
            {(
              [
                ["release", "正式版"],
                ["snapshot", "快照"],
                ["old", "远古版"],
                ["all", "全部"],
              ] as const
            ).map(([value, label]) => (
              <button
                key={value}
                type="button"
                onClick={() => setFilter(value)}
                className={[
                  "rounded-lg px-3 py-1.5 text-xs transition",
                  filter === value
                    ? "bg-cyan-400/15 text-cyan-100"
                    : "text-slate-500 hover:text-slate-200",
                ].join(" ")}
              >
                {label}
              </button>
            ))}
          </div>
        </div>

        {loading && !manifest ? (
          <div className="grid min-h-60 place-items-center text-sm text-slate-500">
            正在读取版本清单...
          </div>
        ) : versions.length === 0 ? (
          <div className="grid min-h-60 place-items-center p-8 text-center">
            <div>
              <TriangleAlert className="mx-auto text-amber-300" size={24} />
              <p className="mt-3 text-sm text-slate-400">没有找到符合条件的版本。</p>
            </div>
          </div>
        ) : (
          <div className="divide-y divide-white/5">
            {versions.map((version) => (
              <div
                key={version.id}
                className="group flex items-center gap-4 px-5 py-3.5 transition duration-300 hover:bg-cyan-300/[0.025]"
              >
                <div className="grid h-10 w-10 shrink-0 place-items-center rounded-2xl bg-slate-800/75 text-slate-400 transition duration-300 group-hover:scale-105 group-hover:bg-cyan-300/10 group-hover:text-cyan-200">
                  <Boxes size={17} />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-slate-100">{version.id}</span>
                    <StatusPill
                      tone={
                        version.versionType === "release"
                          ? "emerald"
                          : version.versionType === "snapshot"
                            ? "amber"
                            : "slate"
                      }
                    >
                      {versionTypeLabel(version.versionType)}
                    </StatusPill>
                  </div>
                  <div className="mt-1 text-xs text-slate-600">
                    发布于 {formatDate(version.releaseTime)}
                  </div>
                </div>
                <button
                  type="button"
                  disabled={Boolean(installing)}
                  onClick={() => void install(version.id)}
                  className="inline-flex shrink-0 items-center gap-2 rounded-xl border border-cyan-300/15 bg-cyan-300/[0.07] px-3.5 py-2 text-xs font-medium text-cyan-200 transition hover:bg-cyan-300/[0.12] disabled:opacity-35"
                >
                  <CloudDownload size={15} />
                  {activeProgress?.versionId === version.id && installing ? "安装中" : "安装"}
                </button>
              </div>
            ))}
          </div>
        )}

        {(manifest?.versions.length ?? 0) > versions.length ? (
          <div className="border-t border-white/6 px-5 py-3 text-center text-[11px] text-slate-600">
            当前显示 {versions.length} 个版本，输入版本号可继续筛选。
          </div>
        ) : null}
      </div>
    </div>
  );
}
