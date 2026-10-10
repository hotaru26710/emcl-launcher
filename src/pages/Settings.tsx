import { useEffect, useState } from "react";
import {
  Check,
  CloudDownload,
  Database,
  FolderOpen,
  Cpu,
  Gauge,
  Globe2,
  RefreshCw,
  ShieldCheck,
  Save,
} from "lucide-react";
import { PageHeader } from "../components/PageHeader";
import { useLauncherStore } from "../stores/launcher";

export function SettingsPage() {
  const settings = useLauncherStore((state) => state.settings);
  const saving = useLauncherStore((state) => state.savingSettings);
  const saveSettings = useLauncherStore((state) => state.saveSettings);
  const detectJava = useLauncherStore((state) => state.detectJava);
  const saveDefaultJava = useLauncherStore((state) => state.saveDefaultJava);
  const javaInstallations = useLauncherStore((state) => state.javaInstallations);

  const [mirror, setMirror] = useState<"official" | "bmclapi">("official");
  const [concurrency, setConcurrency] = useState(8);
  const [defaultJavaPath, setDefaultJavaPath] = useState<string | null>(null);
  const [microsoftClientId, setMicrosoftClientId] = useState("");
  const [detectingJava, setDetectingJava] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (!settings) return;
    setMirror(settings.downloadMirror === "bmclapi" ? "bmclapi" : "official");
    setConcurrency(settings.downloadConcurrency);
    setDefaultJavaPath(settings.defaultJavaPath ?? null);
    setMicrosoftClientId(settings.microsoftClientId ?? "");
  }, [settings]);

  const save = async () => {
    setSaved(false);
    try {
      await saveSettings({
        downloadMirror: mirror,
        downloadConcurrency: concurrency,
        microsoftClientId,
      });
      await saveDefaultJava(defaultJavaPath);
      setSaved(true);
      window.setTimeout(() => setSaved(false), 2000);
    } catch {
      // The shared error banner owns the message.
    }
  };

  return (
    <div className="animate-page mx-auto max-w-4xl">
      <PageHeader
        title="全局设置"
        description="下载、缓存和目录策略只影响本机。远程资源不会上传任何账户或实例数据。"
      />

      <div className="space-y-5">
        <section className="glass-panel hover-lift rounded-[28px] p-5">
          <div className="flex items-center gap-2">
            <Globe2 size={17} className="text-cyan-300" />
            <h2 className="text-sm font-semibold text-slate-200">下载镜像</h2>
          </div>
          <p className="mt-2 text-xs leading-5 text-slate-500">
            官方源稳定优先，BMCLAPI 在中国大陆网络环境下通常更快。
          </p>
          <div className="mt-4 grid gap-3 md:grid-cols-2">
            {[
              {
                id: "official" as const,
                title: "Mojang 官方源",
                description: "直连 piston-meta、libraries 和 resources 服务。",
              },
              {
                id: "bmclapi" as const,
                title: "BMCLAPI 镜像",
                description: "自动重定向客户端、依赖库和资源对象请求。",
              },
            ].map((option) => (
              <button
                key={option.id}
                type="button"
                onClick={() => setMirror(option.id)}
                className={[
                  "rounded-2xl border p-4 text-left transition",
                  mirror === option.id
                    ? "border-cyan-300/30 bg-cyan-300/[0.08]"
                    : "border-white/8 bg-slate-950/30 hover:border-white/15",
                ].join(" ")}
              >
                <div className="flex items-center justify-between gap-3">
                  <span className="text-sm font-medium text-slate-100">{option.title}</span>
                  {mirror === option.id ? (
                    <Check size={16} className="text-cyan-300" />
                  ) : null}
                </div>
                <p className="mt-2 text-xs leading-5 text-slate-500">{option.description}</p>
              </button>
            ))}
          </div>
        </section>

        <section className="glass-panel hover-lift rounded-[28px] p-5">
          <div className="flex items-center gap-2">
            <Gauge size={17} className="text-cyan-300" />
            <h2 className="text-sm font-semibold text-slate-200">并发下载数</h2>
          </div>
          <div className="mt-4 flex items-center gap-4">
            <input
              type="range"
              min={1}
              max={16}
              value={concurrency}
              onChange={(event) => setConcurrency(Number(event.currentTarget.value))}
              className="h-1.5 flex-1 accent-cyan-400"
            />
            <span className="w-20 rounded-lg border border-white/8 bg-slate-950/55 px-3 py-1.5 text-center text-xs tabular-nums text-slate-300">
              {concurrency}
            </span>
          </div>
          <p className="mt-3 text-xs text-slate-600">
            资源文件很多，默认 8 个并发请求；网络不稳时可调低。
          </p>
        </section>

        <section className="glass-panel hover-lift rounded-[28px] p-5">
          <div className="flex items-center gap-2">
            <ShieldCheck size={17} className="text-cyan-300" />
            <h2 className="text-sm font-semibold text-slate-200">Microsoft 登录</h2>
          </div>
          <p className="mt-2 text-xs leading-5 text-slate-500">
            微软设备码登录需要你自己在 Azure 注册的公共客户端应用 ID。留空时只会禁用微软登录，不影响离线账户。
          </p>
          <input
            value={microsoftClientId}
            onChange={(event) => setMicrosoftClientId(event.currentTarget.value.trim())}
            placeholder="Azure Application (client) ID"
            className="mt-4 w-full rounded-xl border border-white/10 bg-slate-950/55 px-3.5 py-2.5 text-sm text-white outline-none placeholder:text-slate-700 focus:border-cyan-300/40"
          />
        </section>

        <section className="glass-panel hover-lift rounded-[28px] p-5">
          <div className="flex items-center justify-between gap-4">
            <div className="flex items-center gap-2">
              <Cpu size={17} className="text-cyan-300" />
              <h2 className="text-sm font-semibold text-slate-200">Java 运行时</h2>
            </div>
            <button
              type="button"
              disabled={detectingJava}
              onClick={() => {
                setDetectingJava(true);
                void detectJava()
                  .then((installations) => {
                    if (!defaultJavaPath && installations[0]) {
                      setDefaultJavaPath(installations[0].path);
                    }
                  })
                  .catch(() => undefined)
                  .finally(() => setDetectingJava(false));
              }}
              className="inline-flex items-center gap-2 rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-xs text-slate-300 transition hover:bg-white/8 disabled:opacity-50"
            >
              <RefreshCw size={14} className={detectingJava ? "animate-spin" : ""} />
              扫描 Java
            </button>
          </div>
          <p className="mt-2 text-xs leading-5 text-slate-500">
            启动器会读取 JAVA_HOME、PATH 和常见 JDK 安装目录；实例没有单独指定时使用这里的默认值。
          </p>
          <div className="mt-4">
            {javaInstallations.length > 0 ? (
              <select
                value={defaultJavaPath ?? ""}
                onChange={(event) => setDefaultJavaPath(event.currentTarget.value || null)}
                className="w-full rounded-xl border border-white/10 bg-slate-950/55 px-3.5 py-2.5 text-sm text-white outline-none focus:border-cyan-300/40"
              >
                <option value="">自动选择</option>
                {javaInstallations.map((installation) => (
                  <option key={installation.path} value={installation.path}>
                    Java {installation.majorVersion ?? "?"} · {installation.vendor ?? "Unknown"} · {installation.path}
                  </option>
                ))}
              </select>
            ) : (
              <div className="rounded-xl border border-dashed border-white/10 bg-slate-950/30 px-4 py-5 text-center text-xs text-slate-600">
                点击“扫描 Java”查找本机 JDK。
              </div>
            )}
          </div>
        </section>

        <section className="glass-panel hover-lift rounded-[28px] p-5">
          <div className="flex items-center gap-2">
            <FolderOpen size={17} className="text-cyan-300" />
            <h2 className="text-sm font-semibold text-slate-200">本地目录</h2>
          </div>
          <div className="mt-4 space-y-3">
            <div className="rounded-xl border border-white/6 bg-slate-950/40 p-3">
              <div className="flex items-center gap-2 text-[11px] uppercase tracking-wider text-slate-600">
                <Database size={13} />
                数据根目录
              </div>
              <div className="mt-1 break-all font-mono text-xs text-slate-400">
                {settings?.dataDirectory ?? "读取中..."}
              </div>
            </div>
            <div className="rounded-xl border border-white/6 bg-slate-950/40 p-3">
              <div className="flex items-center gap-2 text-[11px] uppercase tracking-wider text-slate-600">
                <CloudDownload size={13} />
                共享缓存
              </div>
              <div className="mt-1 break-all font-mono text-xs text-slate-400">
                {settings?.sharedDirectory ?? "读取中..."}
              </div>
            </div>
          </div>
        </section>

        <div className="flex items-center justify-end gap-3">
          {saved ? <span className="text-xs text-emerald-300">设置已保存</span> : null}
          <button
            type="button"
            disabled={saving}
            onClick={() => void save()}
            className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:opacity-40"
          >
            <Save size={16} />
            {saving ? "保存中..." : "保存设置"}
          </button>
        </div>
      </div>
    </div>
  );
}
