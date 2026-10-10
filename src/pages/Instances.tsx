import { useEffect, useState } from "react";
import {
  Boxes,
  ChevronRight,
  Cpu,
  HardDrive,
  MemoryStick,
  Plus,
  Trash2,
  X,
} from "lucide-react";
import { Link } from "react-router-dom";
import { EmptyState } from "../components/EmptyState";
import { InstallProgressCard } from "../components/ProgressBar";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { formatDate, versionTypeLabel } from "../lib/format";
import type { LoaderType } from "../api/types";
import { useLauncherStore } from "../stores/launcher";

export function Instances() {
  const instances = useLauncherStore((state) => state.instances);
  const manifest = useLauncherStore((state) => state.manifest);
  const settings = useLauncherStore((state) => state.settings);
  const loading = useLauncherStore((state) => state.loadingInstances);
  const createInstance = useLauncherStore((state) => state.createInstance);
  const deleteInstance = useLauncherStore((state) => state.deleteInstance);
  const startInstall = useLauncherStore((state) => state.startInstall);
  const installLoader = useLauncherStore((state) => state.installLoader);
  const activeInstall = useLauncherStore((state) => state.activeInstall);
  const installProgress = useLauncherStore((state) => state.installProgress);

  const [dialogOpen, setDialogOpen] = useState(false);
  const [name, setName] = useState("");
  const [versionId, setVersionId] = useState("1.21.1");
  const [memoryMb, setMemoryMb] = useState(4096);
  const [loader, setLoader] = useState<"vanilla" | LoaderType>("vanilla");
  const [loaderVersion, setLoaderVersion] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [creatingInstanceId, setCreatingInstanceId] = useState<string | null>(null);

  useEffect(() => {
    if (!dialogOpen) return;
    const fallback = manifest?.latest.release ?? "1.21.1";
    setVersionId((current) => current || fallback);
    if (settings?.defaultMemoryMb) setMemoryMb(settings.defaultMemoryMb);
  }, [dialogOpen, manifest, settings?.defaultMemoryMb]);

  const creationProgress =
    activeInstall?.instanceId === creatingInstanceId
      ? installProgress[activeInstall.jobId]
      : undefined;

  const handleCreate = async () => {
    if (!name.trim()) return;
    setSubmitting(true);
    setCreatingInstanceId(null);
    try {
      const created = await createInstance({
        name: name.trim(),
        versionId,
        memoryMb,
        loader: loader === "vanilla" ? null : loader,
        loaderVersion: loader === "vanilla" || !loaderVersion.trim() ? null : loaderVersion.trim(),
      });
      setCreatingInstanceId(created.id);
      if (loader === "vanilla") {
        await startInstall(created.versionId, created.id);
      } else {
        await installLoader(created.id, loader, loaderVersion.trim() || null);
      }
      setName("");
      setMemoryMb(settings?.defaultMemoryMb ?? 4096);
      setLoader("vanilla");
      setLoaderVersion("");
      setCreatingInstanceId(null);
      setDialogOpen(false);
    } catch {
      // Keep the dialog open so the failed progress/error remains visible.
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async (id: string, instanceName: string) => {
    if (!window.confirm(`确定删除实例“${instanceName}”和它的本地目录吗？`)) return;
    try {
      await deleteInstance(id);
    } catch {
      // The shared error banner owns the message.
    }
  };

  return (
    <div className="animate-page mx-auto max-w-6xl">
      <PageHeader
        title="游戏实例"
        description="每个实例拥有独立的 mods、config、saves 和日志目录，游戏文件与资源库则按全局共享缓存保存。"
        action={
          <button
            type="button"
            onClick={() => setDialogOpen(true)}
            className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300"
          >
            <Plus size={17} />
            新建实例
          </button>
        }
      />

      {instances.length === 0 && !loading ? (
        <EmptyState
          icon={Boxes}
          title="还没有游戏实例"
          description="先创建一个实例。现在可以直接选择任意 Mojang 正式版；加载器会在后续步骤中接入。"
          action={
            <button
              type="button"
              onClick={() => setDialogOpen(true)}
              className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300"
            >
              <Plus size={17} />
              新建第一个实例
            </button>
          }
        />
      ) : (
        <div className="grid gap-4 xl:grid-cols-2">
          {instances.map((instance) => (
            <div
              key={instance.id}
              className="glass-panel hover-lift group rounded-[28px] p-5"
            >
              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <h2 className="truncate text-base font-semibold text-white">
                      {instance.name}
                    </h2>
                    <StatusPill tone={instance.loader ? "cyan" : "slate"}>
                      {instance.loader ? instance.loader.toUpperCase() : "VANILLA"}
                    </StatusPill>
                  </div>
                  <div className="mt-2 flex flex-wrap items-center gap-3 text-xs text-slate-500">
                    <span className="inline-flex items-center gap-1.5">
                      <Boxes size={14} />
                      {instance.versionId}
                    </span>
                    <span className="inline-flex items-center gap-1.5">
                      <MemoryStick size={14} />
                      {instance.memoryMb} MB
                    </span>
                    <span>更新于 {formatDate(instance.updatedAt)}</span>
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => void handleDelete(instance.id, instance.name)}
                  className="rounded-lg p-2 text-slate-600 opacity-0 transition hover:bg-rose-400/10 hover:text-rose-300 group-hover:opacity-100"
                  aria-label={`删除 ${instance.name}`}
                >
                  <Trash2 size={16} />
                </button>
              </div>

              <div className="mt-4 flex items-center gap-2 rounded-xl border border-white/6 bg-slate-950/35 px-3 py-2 text-xs text-slate-500">
                <HardDrive size={14} className="shrink-0" />
                <span className="truncate">{instance.directory}</span>
              </div>

              <Link
                to={`/instances/${instance.id}`}
                className="mt-4 inline-flex items-center gap-1 text-sm font-medium text-cyan-300 transition hover:text-cyan-200"
              >
                打开实例
                <ChevronRight size={16} />
              </Link>
            </div>
          ))}
        </div>
      )}

      {dialogOpen ? (
        <div className="fixed inset-0 z-40 grid place-items-center bg-slate-950/75 p-6 backdrop-blur-sm">
          <div className="glass-panel animate-card-in w-full max-w-lg rounded-[34px] p-6 shadow-2xl">
            <div className="flex items-start justify-between">
              <div>
                <h2 className="text-lg font-semibold text-white">
                  {submitting ? "正在创建实例" : "新建游戏实例"}
                </h2>
                <p className="mt-1 text-sm text-slate-500">
                  {submitting
                    ? "实例目录已就绪，正在安装游戏和加载器；完成后会自动关闭。"
                    : "创建独立存档与模组目录。"}
                </p>
              </div>
              <button
                type="button"
                disabled={submitting}
                onClick={() => setDialogOpen(false)}
                className="rounded-lg p-2 text-slate-500 transition hover:bg-white/5 hover:text-white disabled:opacity-30"
                aria-label="关闭"
              >
                <X size={18} />
              </button>
            </div>

            {submitting ? (
              <div className="mt-5">
                {creationProgress ? (
                  <InstallProgressCard progress={creationProgress} compact />
                ) : (
                  <div className="rounded-2xl border border-cyan-300/15 bg-cyan-300/[0.04] p-4 text-sm text-cyan-100">
                    正在创建实例目录...
                  </div>
                )}
              </div>
            ) : null}

            <fieldset disabled={submitting} className="mt-6 space-y-4 disabled:opacity-45">
              <label className="block">
                <span className="text-xs font-medium text-slate-400">实例名称</span>
                <input
                  value={name}
                  onChange={(event) => setName(event.currentTarget.value)}
                  autoFocus
                  placeholder="例如：生存服 1.21.1"
                  className="mt-2 w-full rounded-xl border border-white/10 bg-slate-950/55 px-3.5 py-2.5 text-sm text-white outline-none transition placeholder:text-slate-700 focus:border-cyan-300/40"
                />
              </label>

              <label className="block">
                <span className="text-xs font-medium text-slate-400">游戏版本</span>
                <select
                  value={versionId}
                  onChange={(event) => setVersionId(event.currentTarget.value)}
                  className="mt-2 w-full rounded-xl border border-white/10 bg-slate-950/55 px-3.5 py-2.5 text-sm text-white outline-none focus:border-cyan-300/40"
                >
                  {(manifest?.versions ?? []).map((version) => (
                    <option key={version.id} value={version.id}>
                      {version.id} · {versionTypeLabel(version.versionType)}
                    </option>
                  ))}
                </select>
              </label>

              <div className="grid grid-cols-2 gap-3">
                <label className="block">
                  <span className="text-xs font-medium text-slate-400">加载器</span>
                  <select
                    value={loader}
                    onChange={(event) => setLoader(event.currentTarget.value as "vanilla" | LoaderType)}
                    className="mt-2 w-full rounded-xl border border-white/10 bg-slate-950/55 px-3.5 py-2.5 text-sm text-white outline-none focus:border-cyan-300/40"
                  >
                    <option value="vanilla">Vanilla</option>
                    <option value="fabric">Fabric</option>
                    <option value="quilt">Quilt</option>
                    <option value="neoforge">NeoForge</option>
                    <option value="forge">Forge</option>
                  </select>
                </label>
                <label className="block">
                  <span className="text-xs font-medium text-slate-400">加载器版本</span>
                  <input
                    value={loaderVersion}
                    disabled={loader === "vanilla"}
                    onChange={(event) => setLoaderVersion(event.currentTarget.value)}
                    placeholder={loader === "vanilla" ? "不需要" : "留空自动选择"}
                    className="mt-2 w-full rounded-xl border border-white/10 bg-slate-950/55 px-3.5 py-2.5 text-sm text-white outline-none placeholder:text-slate-700 focus:border-cyan-300/40 disabled:opacity-35"
                  />
                </label>
              </div>

              <label className="block">
                <span className="flex items-center gap-2 text-xs font-medium text-slate-400">
                  <Cpu size={14} />
                  内存上限
                </span>
                <div className="mt-2 flex items-center gap-3">
                  <input
                    type="range"
                    min={512}
                    max={16384}
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
            </fieldset>

            <div className="mt-7 flex justify-end gap-3">
              <button
                type="button"
                disabled={submitting}
                onClick={() => setDialogOpen(false)}
                className="rounded-xl px-4 py-2.5 text-sm text-slate-400 transition hover:bg-white/5 hover:text-white disabled:opacity-30"
              >
                取消
              </button>
              <button
                type="button"
                disabled={submitting || !name.trim()}
                onClick={() => void handleCreate()}
                className="rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:opacity-40"
              >
                {submitting ? "安装中，完成自动关闭" : "创建实例"}
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
