import { useEffect, useState } from "react";
import {
  ArrowLeft,
  CloudDownload,
  Cpu,
  FolderOpen,
  HardDrive,
  MemoryStick,
  Save,
  Terminal,
  Trash2,
} from "lucide-react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { api } from "../api/tauri";
import type { Instance } from "../api/types";
import { InstallProgressCard } from "../components/ProgressBar";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { formatDate } from "../lib/format";
import { useLauncherStore } from "../stores/launcher";

export function InstanceDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const instances = useLauncherStore((state) => state.instances);
  const updateInstance = useLauncherStore((state) => state.updateInstance);
  const deleteInstance = useLauncherStore((state) => state.deleteInstance);
  const startInstall = useLauncherStore((state) => state.startInstall);
  const activeInstall = useLauncherStore((state) => state.activeInstall);
  const installProgress = useLauncherStore((state) => state.installProgress);

  const [instance, setInstance] = useState<Instance | null>(
    instances.find((item) => item.id === id) ?? null,
  );
  const [loading, setLoading] = useState(!instance);
  const [memoryMb, setMemoryMb] = useState(instance?.memoryMb ?? 4096);
  const [jvmArgs, setJvmArgs] = useState(instance?.jvmArgs ?? "");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!id) return;
    const cached = instances.find((item) => item.id === id);
    if (cached) {
      setInstance(cached);
      setLoading(false);
      return;
    }

    let disposed = false;
    setLoading(true);
    void api
      .getInstance(id)
      .then((result) => {
        if (!disposed) setInstance(result);
      })
      .catch(() => undefined)
      .finally(() => {
        if (!disposed) setLoading(false);
      });

    return () => {
      disposed = true;
    };
  }, [id, instances]);

  useEffect(() => {
    if (!instance) return;
    setMemoryMb(instance.memoryMb);
    setJvmArgs(instance.jvmArgs);
  }, [instance]);

  if (!id || (!loading && !instance)) {
    return (
      <div className="mx-auto max-w-3xl rounded-2xl border border-white/8 bg-white/[0.02] p-8 text-center text-slate-400">
        找不到这个实例。
      </div>
    );
  }

  if (!instance) {
    return <div className="text-sm text-slate-500">正在读取实例...</div>;
  }

  const progress =
    activeInstall?.instanceId === instance.id
      ? installProgress[activeInstall.jobId]
      : undefined;

  const save = async () => {
    if (!instance) return;
    setSaving(true);
    try {
      const next = await updateInstance(instance.id, { memoryMb, jvmArgs });
      setInstance(next);
    } catch {
      // The shared error banner owns the message.
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    if (!window.confirm(`确定删除实例“${instance.name}”吗？`)) return;
    try {
      await deleteInstance(instance.id);
      navigate("/instances");
    } catch {
      // The shared error banner owns the message.
    }
  };

  const install = async () => {
    try {
      await startInstall(instance.versionId, instance.id);
    } catch {
      // The shared error banner owns the message.
    }
  };

  return (
    <div className="animate-page mx-auto max-w-5xl">
      <Link
        to="/instances"
        className="mb-5 inline-flex items-center gap-2 text-sm text-slate-500 transition hover:text-slate-200"
      >
        <ArrowLeft size={16} />
        返回实例列表
      </Link>

      <PageHeader
        title={instance.name}
        description={`基于 Minecraft ${instance.versionId}${instance.loader ? ` · ${instance.loader}` : " · Vanilla"}`}
        action={
          <div className="flex items-center gap-2">
            <button
              type="button"
              disabled={progress?.status === "running"}
              onClick={() => void install()}
              className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:opacity-40"
            >
              <CloudDownload size={17} />
              安装游戏文件
            </button>
            <button
              type="button"
              onClick={() => void remove()}
              className="rounded-xl border border-rose-400/15 bg-rose-400/[0.06] p-2.5 text-rose-300 transition hover:bg-rose-400/12"
              aria-label="删除实例"
            >
              <Trash2 size={17} />
            </button>
          </div>
        }
      />

      {progress ? (
        <div className="mb-6">
          <InstallProgressCard progress={progress} />
        </div>
      ) : null}

      <div className="grid gap-4 lg:grid-cols-[1.2fr_0.8fr]">
        <section className="glass-panel hover-lift rounded-[28px] p-5">
          <h2 className="text-sm font-semibold text-slate-200">运行配置</h2>
          <div className="mt-5 space-y-5">
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

            <label className="block">
              <span className="flex items-center gap-2 text-xs font-medium text-slate-400">
                <Terminal size={14} />
                额外 JVM 参数
              </span>
              <textarea
                value={jvmArgs}
                onChange={(event) => setJvmArgs(event.currentTarget.value)}
                rows={4}
                placeholder="-XX:+UseG1GC 等参数，留空使用默认值"
                className="mt-2 w-full resize-none rounded-xl border border-white/10 bg-slate-950/55 px-3.5 py-3 text-sm text-white outline-none placeholder:text-slate-700 focus:border-cyan-300/40"
              />
            </label>

            <button
              type="button"
              disabled={saving}
              onClick={() => void save()}
              className="inline-flex items-center gap-2 rounded-xl border border-cyan-300/15 bg-cyan-300/[0.07] px-4 py-2.5 text-sm font-medium text-cyan-200 transition hover:bg-cyan-300/[0.12] disabled:opacity-40"
            >
              <Save size={16} />
              {saving ? "保存中..." : "保存运行配置"}
            </button>
          </div>
        </section>

        <section className="space-y-4">
          <div className="glass-panel hover-lift rounded-[28px] p-5">
            <h2 className="text-sm font-semibold text-slate-200">实例信息</h2>
            <dl className="mt-4 space-y-3 text-sm">
              <div className="flex items-center justify-between gap-4">
                <dt className="text-slate-500">游戏版本</dt>
                <dd className="text-slate-200">{instance.versionId}</dd>
              </div>
              <div className="flex items-center justify-between gap-4">
                <dt className="text-slate-500">加载器</dt>
                <dd>
                  <StatusPill tone={instance.loader ? "cyan" : "slate"}>
                    {instance.loader ? instance.loader.toUpperCase() : "VANILLA"}
                  </StatusPill>
                </dd>
              </div>
              <div className="flex items-center justify-between gap-4">
                <dt className="text-slate-500">Java</dt>
                <dd className="truncate text-slate-300">{instance.javaPath ?? "自动检测"}</dd>
              </div>
              <div className="flex items-center justify-between gap-4">
                <dt className="text-slate-500">最后启动</dt>
                <dd className="text-slate-300">{formatDate(instance.lastPlayedAt)}</dd>
              </div>
            </dl>
          </div>

          <div className="glass-panel hover-lift rounded-[28px] p-5">
            <div className="flex items-center gap-2 text-sm font-semibold text-slate-200">
              <FolderOpen size={16} />
              实例目录
            </div>
            <div className="mt-3 flex items-start gap-2 rounded-xl border border-white/6 bg-slate-950/45 p-3 text-xs leading-5 text-slate-500">
              <HardDrive size={14} className="mt-0.5 shrink-0" />
              <span className="break-all">{instance.directory}</span>
            </div>
            <p className="mt-3 text-xs leading-5 text-slate-600">
              mods、config、saves 和日志都隔离在这个目录内。
            </p>
          </div>

          <div className="glass-panel hover-lift rounded-[28px] p-5">
            <div className="flex items-center gap-2 text-sm font-semibold text-slate-200">
              <Cpu size={16} />
              日志
            </div>
            <div className="mt-3 grid min-h-24 place-items-center rounded-xl border border-dashed border-white/8 bg-slate-950/35 p-4 text-center text-xs leading-5 text-slate-600">
              启动游戏后，实时日志会显示在这里。
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}
