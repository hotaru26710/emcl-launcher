import { useEffect, useMemo, useState } from "react";
import {
  ArrowLeft,
  Check,
  ChevronRight,
  MemoryStick,
  Search,
  Sparkles,
} from "lucide-react";
import { useNavigate } from "react-router-dom";
import { api } from "../api/tauri";
import type { LoaderType, LoaderVersionInfo } from "../api/types";
import { LoaderIcon, loaderLabel } from "../components/LoaderIcon";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { formatDate, versionTypeLabel } from "../lib/format";
import { useLauncherStore } from "../stores/launcher";

type VersionFilter = "release" | "snapshot" | "all";
type PrimaryLoader = "vanilla" | LoaderType;

export function CreateInstance() {
  const navigate = useNavigate();
  const manifest = useLauncherStore((state) => state.manifest);
  const settings = useLauncherStore((state) => state.settings);
  const createInstance = useLauncherStore((state) => state.createInstance);
  const startInstall = useLauncherStore((state) => state.startInstall);
  const installLoader = useLauncherStore((state) => state.installLoader);

  const [step, setStep] = useState(1);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<VersionFilter>("release");
  const [versionId, setVersionId] = useState("1.21.1");
  const [loader, setLoader] = useState<PrimaryLoader>("vanilla");
  const [loaderVersion, setLoaderVersion] = useState("");
  const [loaderVersions, setLoaderVersions] = useState<LoaderVersionInfo[]>([]);
  const [loaderVersionsLoading, setLoaderVersionsLoading] = useState(false);
  const [optifine, setOptifine] = useState(false);
  const [name, setName] = useState("");
  const [memoryMb, setMemoryMb] = useState(settings?.defaultMemoryMb ?? 4096);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (settings?.defaultMemoryMb) setMemoryMb(settings.defaultMemoryMb);
  }, [settings?.defaultMemoryMb]);

  const versions = useMemo(() => {
    const search = query.trim().toLowerCase();
    return (manifest?.versions ?? [])
      .filter((version) => {
        if (filter === "all") return true;
        if (filter === "release") return version.versionType === "release";
        return version.versionType === "snapshot";
      })
      .filter((version) => !search || version.id.toLowerCase().includes(search))
      .slice(0, 160);
  }, [filter, manifest, query]);

  const optifineCompatible = loader === "forge";
  const canChooseOptifine = optifineCompatible;

  useEffect(() => {
    if (!optifineCompatible) setOptifine(false);
  }, [optifineCompatible]);

  useEffect(() => {
    if (step !== 2 || loader === "vanilla") {
      setLoaderVersions([]);
      return;
    }
    let disposed = false;
    setLoaderVersionsLoading(true);
    void api
      .getLoaderVersions(loader, versionId)
      .then((versions) => {
        if (!disposed) setLoaderVersions(versions);
      })
      .catch(() => {
        if (!disposed) setLoaderVersions([]);
      })
      .finally(() => {
        if (!disposed) setLoaderVersionsLoading(false);
      });
    return () => {
      disposed = true;
    };
  }, [step, loader, versionId]);

  const chooseLoader = (choice: PrimaryLoader) => {
    setLoader(choice);
    setLoaderVersion("");
  };

  const handleCreate = async () => {
    setSubmitting(true);
    try {
      const created = await createInstance({
        name: name.trim() || `${versionId} ${loaderLabel(loader)}`,
        versionId,
        memoryMb,
        loader: loader === "vanilla" ? null : loader,
        loaderVersion: loader === "vanilla" || !loaderVersion.trim() ? null : loaderVersion.trim(),
      });
      if (loader === "vanilla") {
        void startInstall(created.versionId, created.id).catch(() => undefined);
      } else {
        void installLoader(created.id, loader, loaderVersion.trim() || null).catch(() => undefined);
      }
      navigate("/instances");
    } catch {
      // The shared error banner owns the message.
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="animate-page mx-auto max-w-6xl">
      <button
        type="button"
        onClick={() => navigate("/instances")}
        className="mb-5 inline-flex items-center gap-2 text-sm text-slate-500 transition hover:text-slate-200"
      >
        <ArrowLeft size={16} />
        返回实例列表
      </button>

      <PageHeader
        title="创建游戏实例"
        description="分三步完成：选择 Minecraft 版本、选择加载器和附加组件、填写实例名称与资源。"
      />

      <div className="mb-6 flex items-center gap-3">
        {[1, 2, 3].map((item) => (
          <div key={item} className="flex items-center gap-3">
            <div
              className={[
                "grid h-8 w-8 place-items-center rounded-full text-xs font-semibold transition",
                step >= item ? "bg-cyan-400 text-slate-950" : "border border-white/10 bg-white/5 text-slate-500",
              ].join(" ")}
            >
              {step > item ? <Check size={15} /> : item}
            </div>
            <span className={step >= item ? "text-sm text-slate-200" : "text-sm text-slate-600"}>
              {item === 1 ? "游戏版本" : item === 2 ? "加载器" : "创建确认"}
            </span>
            {item < 3 ? <div className="h-px w-10 bg-white/10" /> : null}
          </div>
        ))}
      </div>

      {step === 1 ? (
        <section className="glass-panel animate-card-in rounded-[30px] p-5">
          <div className="flex flex-wrap items-center gap-3">
            <div className="relative min-w-64 flex-1">
              <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-slate-600" size={16} />
              <input
                value={query}
                onChange={(event) => setQuery(event.currentTarget.value)}
                placeholder="搜索 Minecraft 版本"
                className="w-full rounded-xl border border-white/10 bg-slate-950/55 py-2.5 pl-9 pr-3 text-sm text-white outline-none placeholder:text-slate-700 focus:border-cyan-300/40"
              />
            </div>
            {(["release", "snapshot", "all"] as const).map((value) => (
              <button
                key={value}
                type="button"
                onClick={() => setFilter(value)}
                className={[
                  "rounded-xl px-3 py-2 text-xs transition",
                  filter === value ? "bg-cyan-300/15 text-cyan-100" : "text-slate-500 hover:bg-white/5 hover:text-slate-200",
                ].join(" ")}
              >
                {value === "release" ? "正式版" : value === "snapshot" ? "快照" : "全部"}
              </button>
            ))}
          </div>

          <div className="mt-5 grid max-h-[52vh] gap-3 overflow-y-auto pr-1 md:grid-cols-2 xl:grid-cols-3">
            {versions.map((version) => (
              <button
                key={version.id}
                type="button"
                onClick={() => setVersionId(version.id)}
                className={[
                  "rounded-2xl border p-4 text-left transition hover:-translate-y-0.5",
                  versionId === version.id
                    ? "border-cyan-300/30 bg-cyan-300/[0.08]"
                    : "border-white/8 bg-slate-950/30 hover:border-white/15",
                ].join(" ")}
              >
                <div className="flex items-center justify-between gap-3">
                  <span className="font-semibold text-slate-100">{version.id}</span>
                  <StatusPill tone={version.versionType === "release" ? "emerald" : "amber"}>
                    {versionTypeLabel(version.versionType)}
                  </StatusPill>
                </div>
                <div className="mt-2 text-xs text-slate-600">{formatDate(version.releaseTime)}</div>
              </button>
            ))}
          </div>

          <div className="mt-5 flex justify-end">
            <button
              type="button"
              onClick={() => setStep(2)}
              className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300"
            >
              下一步
              <ChevronRight size={16} />
            </button>
          </div>
        </section>
      ) : null}

      {step === 2 ? (
        <section className="glass-panel animate-card-in rounded-[30px] p-5">
          <div className="flex items-start justify-between gap-4">
            <div>
              <h2 className="text-base font-semibold text-white">选择加载器</h2>
              <p className="mt-1 text-xs leading-5 text-slate-500">
                主加载器只能选择一个；OptiFine 作为附加组件，仅在兼容的 Forge 组合下可选择。
              </p>
            </div>
            <StatusPill tone="cyan">{versionId}</StatusPill>
          </div>

          <div className="mt-5 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {(["vanilla", "fabric", "quilt", "neoforge", "forge"] as const).map((choice) => (
              <button
                key={choice}
                type="button"
                onClick={() => chooseLoader(choice)}
                className={[
                  "flex items-center gap-4 rounded-2xl border p-4 text-left transition hover:-translate-y-0.5",
                  loader === choice
                    ? "border-cyan-300/30 bg-cyan-300/[0.08]"
                    : "border-white/8 bg-slate-950/30 hover:border-white/15",
                ].join(" ")}
              >
                <LoaderIcon loader={choice} />
                <div className="min-w-0">
                  <div className="text-sm font-semibold text-slate-100">{loaderLabel(choice)}</div>
                  <div className="mt-1 text-xs text-slate-600">
                    {choice === "vanilla" ? "原版客户端" : "自动安装加载器依赖"}
                  </div>
                </div>
              </button>
            ))}

            <button
              type="button"
              disabled={!canChooseOptifine}
              onClick={() => setOptifine((value) => !value)}
              className={[
                "flex items-center gap-4 rounded-2xl border p-4 text-left transition hover:-translate-y-0.5 disabled:cursor-not-allowed disabled:opacity-35",
                optifine
                  ? "border-emerald-300/30 bg-emerald-300/[0.08]"
                  : "border-white/8 bg-slate-950/30 hover:border-white/15",
              ].join(" ")}
            >
              <LoaderIcon loader="optifine" />
              <div className="min-w-0">
                <div className="text-sm font-semibold text-slate-100">OptiFine</div>
                <div className="mt-1 text-xs text-slate-600">
                  {canChooseOptifine ? "附加组件，安装后手动放入 mods" : "仅 Forge 组合可用"}
                </div>
              </div>
            </button>
          </div>

          {loader !== "vanilla" ? (
            <label className="mt-5 block max-w-md">
              <span className="text-xs font-medium text-slate-400">加载器版本</span>
              <select
                value={loaderVersion}
                disabled={loaderVersionsLoading}
                onChange={(event) => setLoaderVersion(event.currentTarget.value)}
                className="mt-2 w-full rounded-xl border border-white/10 bg-slate-950/55 px-3.5 py-2.5 text-sm text-white outline-none focus:border-cyan-300/40 disabled:opacity-50"
              >
                <option value="">自动选择最新稳定版</option>
                {loaderVersions.map((version) => (
                  <option key={version.version} value={version.version}>
                    {version.version}{version.stable ? " · 稳定" : ""}
                  </option>
                ))}
              </select>
              {loaderVersionsLoading ? (
                <span className="mt-2 block text-[11px] text-slate-600">正在读取加载器版本...</span>
              ) : null}
            </label>
          ) : null}

          {optifine && !optifineCompatible ? (
            <p className="mt-4 rounded-xl border border-amber-300/15 bg-amber-300/[0.06] px-3 py-2 text-xs text-amber-100">
              当前主加载器与 OptiFine 不兼容，已取消 OptiFine。
            </p>
          ) : null}

          <div className="mt-5 flex justify-between">
            <button
              type="button"
              onClick={() => setStep(1)}
              className="rounded-xl px-4 py-2.5 text-sm text-slate-400 transition hover:bg-white/5 hover:text-white"
            >
              上一步
            </button>
            <button
              type="button"
              onClick={() => setStep(3)}
              className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300"
            >
              下一步
              <ChevronRight size={16} />
            </button>
          </div>
        </section>
      ) : null}

      {step === 3 ? (
        <section className="glass-panel animate-card-in rounded-[30px] p-5">
          <div className="grid gap-5 lg:grid-cols-[1fr_0.8fr]">
            <div className="space-y-5">
              <label className="block">
                <span className="text-xs font-medium text-slate-400">实例名称</span>
                <input
                  value={name}
                  onChange={(event) => setName(event.currentTarget.value)}
                  placeholder={`${versionId} ${loaderLabel(loader)}`}
                  className="mt-2 w-full rounded-xl border border-white/10 bg-slate-950/55 px-3.5 py-2.5 text-sm text-white outline-none placeholder:text-slate-700 focus:border-cyan-300/40"
                />
              </label>
              <label className="block">
                <span className="flex items-center gap-2 text-xs font-medium text-slate-400">
                  <MemoryStick size={14} />
                  内存上限
                </span>
                <div className="mt-2 flex items-center gap-3">
                  <input
                    type="range"
                    min={512}
                    max={32768}
                    step={512}
                    value={memoryMb}
                    onChange={(event) => setMemoryMb(Number(event.currentTarget.value))}
                    className="h-1.5 flex-1 accent-cyan-400"
                  />
                  <span className="w-24 rounded-lg border border-white/8 bg-slate-950/55 px-2 py-1 text-center text-xs tabular-nums text-slate-300">
                    {memoryMb} MB
                  </span>
                </div>
              </label>
            </div>

            <div className="rounded-3xl border border-white/8 bg-slate-950/30 p-5">
              <div className="flex items-center gap-3">
                <LoaderIcon loader={loader} />
                <div>
                  <div className="text-lg font-semibold text-white">{versionId}</div>
                  <div className="text-sm text-slate-500">{loaderLabel(loader)}</div>
                </div>
              </div>
              <div className="mt-5 space-y-3 text-sm">
                <div className="flex items-center justify-between gap-3">
                  <span className="text-slate-500">加载器</span>
                  <span className="text-slate-200">{loaderLabel(loader)}</span>
                </div>
                {loader !== "vanilla" ? (
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-slate-500">加载器版本</span>
                    <span className="truncate text-slate-200">{loaderVersion || "自动选择"}</span>
                  </div>
                ) : null}
                {optifine ? (
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-slate-500">OptiFine</span>
                    <StatusPill tone="emerald">手动安装</StatusPill>
                  </div>
                ) : null}
              </div>
              <p className="mt-4 text-xs leading-5 text-slate-600">
                创建后先在左下角显示安装进度，完成后实例会出现在列表里。
              </p>
            </div>
          </div>

          <div className="mt-5 flex justify-between">
            <button
              type="button"
              onClick={() => setStep(2)}
              className="rounded-xl px-4 py-2.5 text-sm text-slate-400 transition hover:bg-white/5 hover:text-white"
            >
              上一步
            </button>
            <button
              type="button"
              disabled={submitting}
              onClick={() => void handleCreate()}
              className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-5 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:opacity-40"
            >
              <Sparkles size={16} />
              {submitting ? "正在创建..." : "创建并安装"}
            </button>
          </div>
        </section>
      ) : null}
    </div>
  );
}
