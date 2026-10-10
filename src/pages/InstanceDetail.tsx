import { useEffect, useRef, useState } from "react";
import {
  AlertTriangle,
  ArrowLeft,
  CloudDownload,
  Cpu,
  FolderOpen,
  HardDrive,
  MemoryStick,
  Package,
  Play,
  Power,
  PowerOff,
  Save,
  ShieldAlert,
  Square,
  Terminal,
  Trash2,
} from "lucide-react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api } from "../api/tauri";
import type { CrashAnalysis, Instance, JavaRecommendation, ModInfo } from "../api/types";
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
  const startInstance = useLauncherStore((state) => state.startInstance);
  const stopInstance = useLauncherStore((state) => state.stopInstance);
  const activeInstall = useLauncherStore((state) => state.activeInstall);
  const installProgress = useLauncherStore((state) => state.installProgress);
  const runningInstances = useLauncherStore((state) => state.runningInstances);
  const launchLogs = useLauncherStore((state) => state.launchLogs);
  const settings = useLauncherStore((state) => state.settings);
  const detectJava = useLauncherStore((state) => state.detectJava);
  const javaInstallations = useLauncherStore((state) => state.javaInstallations);

  const [instance, setInstance] = useState<Instance | null>(
    instances.find((item) => item.id === id) ?? null,
  );
  const [loading, setLoading] = useState(!instance);
  const [memoryMb, setMemoryMb] = useState(instance?.memoryMb ?? 4096);
  const [jvmArgs, setJvmArgs] = useState(instance?.jvmArgs ?? "");
  const [javaPath, setJavaPath] = useState(instance?.javaPath ?? "");
  const [recommendation, setRecommendation] = useState<JavaRecommendation | null>(null);
  const [detectingJava, setDetectingJava] = useState(false);
  const [mods, setMods] = useState<ModInfo[]>([]);
  const [loadingMods, setLoadingMods] = useState(false);
  const [crashAnalysis, setCrashAnalysis] = useState<CrashAnalysis | null>(null);
  const [analyzingCrash, setAnalyzingCrash] = useState(false);
  const [saving, setSaving] = useState(false);
  const logRef = useRef<HTMLDivElement>(null);

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
    setJavaPath(instance.javaPath ?? settings?.defaultJavaPath ?? "");

    let disposed = false;
    void api
      .recommendJava(instance.versionId)
      .then((result) => {
        if (!disposed) setRecommendation(result);
      })
      .catch(() => undefined);
    if (javaInstallations.length === 0) {
      void detectJava().catch(() => undefined);
    }

    return () => {
      disposed = true;
    };
  }, [instance, settings?.defaultJavaPath]);

  const running = Boolean(instance && runningInstances[instance.id]);
  const logs = instance ? launchLogs[instance.id] ?? [] : [];

  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [logs.length]);

  useEffect(() => {
    if (!instance?.id) return;
    let disposed = false;
    setLoadingMods(true);
    void api
      .listMods(instance.id)
      .then((next) => {
        if (!disposed) setMods(next);
      })
      .catch(() => undefined)
      .finally(() => {
        if (!disposed) setLoadingMods(false);
      });
    return () => {
      disposed = true;
    };
  }, [instance?.id]);

  useEffect(() => {
    if (!instance?.id) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type !== "drop") return;
        void api
          .installMods(instance.id, event.payload.paths)
          .then(setMods)
          .catch(() => undefined);
      })
      .then((stopListening) => {
        if (disposed) stopListening();
        else unlisten = stopListening;
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [instance?.id]);

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
      const next = await updateInstance(instance.id, {
        memoryMb,
        jvmArgs,
        javaPath: javaPath || null,
      });
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

  const analyzeCrash = async () => {
    setAnalyzingCrash(true);
    try {
      setCrashAnalysis(await api.analyzeCrashLog(instance.id));
    } catch {
      // The shared error banner owns the message.
    } finally {
      setAnalyzingCrash(false);
    }
  };

  const install = async () => {
    try {
      await startInstall(instance.versionId, instance.id);
    } catch {
      // The shared error banner owns the message.
    }
  };

  const launch = async () => {
    try {
      await startInstance(instance.id);
    } catch {
      // The shared error banner owns the message.
    }
  };

  const stop = async () => {
    try {
      await stopInstance(instance.id);
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
            {running ? (
              <button
                type="button"
                onClick={() => void stop()}
                className="inline-flex items-center gap-2 rounded-xl border border-rose-300/20 bg-rose-400/10 px-4 py-2.5 text-sm font-semibold text-rose-200 transition hover:bg-rose-400/15"
              >
                <Square size={16} />
                停止游戏
              </button>
            ) : (
              <button
                type="button"
                onClick={() => void launch()}
                className="inline-flex items-center gap-2 rounded-xl bg-emerald-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-emerald-300"
              >
                <Play size={17} />
                启动游戏
              </button>
            )}
            <Link
              to={`/instances/${instance.id}/conflicts`}
              className="inline-flex items-center gap-2 rounded-xl border border-amber-300/15 bg-amber-300/[0.06] px-4 py-2.5 text-sm font-semibold text-amber-200 transition hover:bg-amber-300/[0.12]"
            >
              <ShieldAlert size={16} />
              冲突报告
            </Link>
            <button
              type="button"
              disabled={progress?.status === "running" || running}
              onClick={() => void install()}
              className="inline-flex items-center gap-2 rounded-xl border border-cyan-300/15 bg-cyan-300/[0.08] px-4 py-2.5 text-sm font-semibold text-cyan-200 transition hover:bg-cyan-300/[0.13] disabled:opacity-40"
            >
              <CloudDownload size={17} />
              安装文件
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
              <span className="flex items-center justify-between gap-3 text-xs font-medium text-slate-400">
                <span className="flex items-center gap-2">
                  <Cpu size={14} />
                  Java 运行时
                </span>
                <button
                  type="button"
                  disabled={detectingJava}
                  onClick={() => {
                    setDetectingJava(true);
                    void detectJava()
                      .then((installations) => {
                        if (!javaPath && installations[0]) {
                          setJavaPath(installations[0].path);
                        }
                      })
                      .catch(() => undefined)
                      .finally(() => setDetectingJava(false));
                  }}
                  className="text-[11px] font-normal text-cyan-300 transition hover:text-cyan-200 disabled:opacity-40"
                >
                  {detectingJava ? "扫描中..." : "重新扫描"}
                </button>
              </span>
              <select
                value={javaPath}
                onChange={(event) => setJavaPath(event.currentTarget.value)}
                className="mt-2 w-full rounded-xl border border-white/10 bg-slate-950/55 px-3.5 py-2.5 text-sm text-white outline-none focus:border-cyan-300/40"
              >
                <option value="">自动选择推荐 Java</option>
                {javaInstallations.map((installation) => (
                  <option key={installation.path} value={installation.path}>
                    Java {installation.majorVersion ?? "?"} · {installation.vendor ?? "Unknown"} · {installation.path}
                  </option>
                ))}
              </select>
              {recommendation ? (
                <div className="mt-2 flex items-center justify-between gap-3 text-[11px] leading-5 text-slate-600">
                  <span>{recommendation.reason}</span>
                  {recommendation.installation ? (
                    <button
                      type="button"
                      onClick={() => setJavaPath(recommendation.installation!.path)}
                      className="shrink-0 text-cyan-400 hover:text-cyan-200"
                    >
                      使用推荐
                    </button>
                  ) : null}
                </div>
              ) : null}
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
                <dt className="text-slate-500">运行状态</dt>
                <dd>
                  <StatusPill tone={running ? "emerald" : "slate"}>
                    {running ? "运行中" : "未运行"}
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

          <div className="glass-panel rounded-[28px] p-5">
            <div className="flex items-center justify-between gap-4">
              <div className="flex items-center gap-2 text-sm font-semibold text-slate-200">
                <Package size={16} />
                模组
              </div>
              <div className="flex items-center gap-2">
                <Link
                  to={`/instances/${instance.id}/community`}
                  className="rounded-lg border border-cyan-300/15 bg-cyan-300/[0.06] px-2.5 py-1.5 text-[11px] text-cyan-200 transition hover:bg-cyan-300/[0.12]"
                >
                  浏览社区模组
                </Link>
                <StatusPill tone={mods.length > 0 ? "cyan" : "slate"}>{`${mods.length} 个`}</StatusPill>
              </div>
            </div>
            <div className="mt-3 rounded-2xl border border-dashed border-cyan-300/20 bg-cyan-300/[0.035] px-4 py-4 text-center text-xs leading-5 text-slate-500">
              将 `.jar` 模组拖到窗口里即可安装；安装后启停只需重命名文件。
            </div>
            {loadingMods ? (
              <div className="mt-4 text-center text-xs text-slate-600">正在读取模组...</div>
            ) : mods.length === 0 ? (
              <div className="mt-4 text-center text-xs text-slate-700">当前实例还没有模组。</div>
            ) : (
              <div className="mt-4 space-y-2">
                {mods.map((mod) => (
                  <div
                    key={mod.fileName}
                    className="flex items-center gap-3 rounded-2xl border border-white/[0.06] bg-slate-950/30 px-3 py-3"
                  >
                    <div className={[
                      "grid h-9 w-9 shrink-0 place-items-center rounded-xl",
                      mod.enabled ? "bg-cyan-300/10 text-cyan-200" : "bg-slate-800 text-slate-600",
                    ].join(" ")}>
                      <Package size={16} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-xs font-medium text-slate-200">{mod.name}</div>
                      <div className="mt-0.5 truncate font-mono text-[10px] text-slate-600">
                        {[mod.version, mod.loader, mod.fileName].filter(Boolean).join(" · ")}
                      </div>
                    </div>
                    <button
                      type="button"
                      onClick={() =>
                        void api
                          .toggleMod(instance.id, mod.fileName, !mod.enabled)
                          .then(setMods)
                          .catch(() => undefined)
                      }
                      className="rounded-lg p-2 text-slate-500 transition hover:bg-white/5 hover:text-cyan-200"
                      aria-label={mod.enabled ? "禁用模组" : "启用模组"}
                    >
                      {mod.enabled ? <Power size={15} /> : <PowerOff size={15} />}
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        if (!window.confirm(`确定删除模组“${mod.name}”吗？`)) return;
                        void api.deleteMod(instance.id, mod.fileName).then(setMods).catch(() => undefined);
                      }}
                      className="rounded-lg p-2 text-slate-600 transition hover:bg-rose-400/10 hover:text-rose-300"
                      aria-label="删除模组"
                    >
                      <Trash2 size={15} />
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>

          <div className="glass-panel hover-lift rounded-[28px] p-5">
            <div className="flex items-center justify-between gap-3">
              <div className="flex items-center gap-2 text-sm font-semibold text-slate-200">
                <Cpu size={16} />
                日志
              </div>
              <button
                type="button"
                disabled={analyzingCrash}
                onClick={() => void analyzeCrash()}
                className="rounded-lg border border-cyan-300/15 bg-cyan-300/[0.06] px-2.5 py-1.5 text-[11px] text-cyan-200 transition hover:bg-cyan-300/[0.12] disabled:opacity-40"
              >
                {analyzingCrash ? "分析中..." : "分析最近日志"}
              </button>
            </div>
            <div
              ref={logRef}
              className="mt-3 h-64 overflow-y-auto rounded-2xl border border-slate-800/80 bg-[#040914] p-3 font-mono text-[11px] leading-5 text-slate-400 shadow-inner"
            >
              {logs.length === 0 ? (
                <div className="grid h-full place-items-center text-center text-slate-700">
                  启动游戏后，stdout / stderr 会实时显示在这里。
                </div>
              ) : (
                logs.map((log, index) => (
                  <div
                    key={`${log.timestamp}-${index}`}
                    className={log.stream === "stderr" ? "text-amber-300/80" : "text-slate-400"}
                  >
                    <span className="mr-2 select-none text-slate-700">
                      {log.stream === "stderr" ? "!" : ">"}
                    </span>
                    {log.line}
                  </div>
                ))
              )}
            </div>
            {crashAnalysis ? (
              <div className={[
                "mt-3 rounded-2xl border p-4",
                crashAnalysis.severity === "error"
                  ? "border-rose-300/15 bg-rose-300/[0.06]"
                  : "border-amber-300/15 bg-amber-300/[0.06]",
              ].join(" ")}>
                <div className="flex items-center gap-2 text-sm font-semibold text-slate-100">
                  <AlertTriangle size={15} />
                  {crashAnalysis.summary}
                </div>
                <p className="mt-2 text-xs leading-5 text-slate-400">{crashAnalysis.suggestion}</p>
                {crashAnalysis.matchedLines.length > 0 ? (
                  <div className="mt-3 max-h-32 overflow-y-auto rounded-xl bg-slate-950/50 p-3 font-mono text-[10px] leading-5 text-slate-500">
                    {crashAnalysis.matchedLines.map((line, index) => (
                      <div key={`${index}-${line}`}>{line}</div>
                    ))}
                  </div>
                ) : null}
              </div>
            ) : null}
          </div>
        </section>
      </div>
    </div>
  );
}
