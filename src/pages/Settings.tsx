import { useEffect, useState } from "react";
import {
  Check,
  CloudDownload,
  Database,
  FolderOpen,
  Cpu,
  Gamepad2,
  Gauge,
  Globe2,
  Palette,
  RefreshCw,
  ShieldCheck,
  Save,
} from "lucide-react";
import type { CustomThemeColors, ThemeMode } from "../api/types";
import { PageHeader } from "../components/PageHeader";
import { applyTheme } from "../components/ThemeManager";
import { useLauncherStore } from "../stores/launcher";

const themePresets: Array<{ id: ThemeMode; label: string; colors: string[] }> = [
  { id: "system", label: "跟随系统", colors: ["#07111f", "#ffffff", "#22d3ee"] },
  { id: "dark", label: "深色", colors: ["#07111f", "#0b1929", "#22d3ee"] },
  { id: "light", label: "亮色", colors: ["#eef4f8", "#ffffff", "#0891b2"] },
  { id: "pink", label: "粉白", colors: ["#fff1f6", "#ffffff", "#ec4899"] },
  { id: "ocean", label: "海洋", colors: ["#061827", "#0a263a", "#38bdf8"] },
  { id: "forest", label: "森林", colors: ["#071710", "#0d2419", "#34d399"] },
  { id: "amber", label: "琥珀", colors: ["#1b1206", "#2a1b09", "#f59e0b"] },
  { id: "custom", label: "自定义", colors: ["#101827", "#1f2937", "#f472b6"] },
];

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
  const [theme, setTheme] = useState<ThemeMode>("system");
  const [customTheme, setCustomTheme] = useState<CustomThemeColors>({
    accent: "#22d3ee",
    background: "#07111f",
    panel: "#0b1929",
    text: "#e2e8f0",
    muted: "#94a3b8",
  });
  const [defaultMemoryMb, setDefaultMemoryMb] = useState(4096);
  const [gameWidth, setGameWidth] = useState(854);
  const [gameHeight, setGameHeight] = useState(480);
  const [fullscreen, setFullscreen] = useState(false);
  const [detectingJava, setDetectingJava] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (!settings) return;
    setMirror(settings.downloadMirror === "bmclapi" ? "bmclapi" : "official");
    setConcurrency(settings.downloadConcurrency);
    setDefaultJavaPath(settings.defaultJavaPath ?? null);
    setMicrosoftClientId(settings.microsoftClientId ?? "");
    setTheme(settings.theme ?? "system");
    if (settings.customTheme) {
      try {
        setCustomTheme((current) => ({ ...current, ...JSON.parse(settings.customTheme!) }));
      } catch {
        // Keep defaults for invalid custom theme JSON.
      }
    }
    setDefaultMemoryMb(settings.defaultMemoryMb);
    setGameWidth(settings.gameWidth);
    setGameHeight(settings.gameHeight);
    setFullscreen(settings.fullscreen);
  }, [settings]);

  const previewTheme = (nextTheme: ThemeMode, nextCustom = customTheme) => {
    setTheme(nextTheme);
    setCustomTheme(nextCustom);
    const json = JSON.stringify(nextCustom);
    applyTheme(nextTheme, json);
    void saveSettings({
      theme: nextTheme,
      customTheme: json,
    });
  };

  const save = async () => {
    setSaved(false);
    try {
      await saveSettings({
        downloadMirror: mirror,
        downloadConcurrency: concurrency,
        microsoftClientId,
        theme,
        customTheme: JSON.stringify(customTheme),
        defaultMemoryMb,
        gameWidth,
        gameHeight,
        fullscreen,
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
            <Palette size={17} className="text-cyan-300" />
            <h2 className="text-sm font-semibold text-slate-200">主题</h2>
          </div>
          <p className="mt-2 text-xs leading-5 text-slate-500">选择预设主题，或启用自定义颜色。跟随系统会根据 Windows 的亮色/深色模式自动切换。</p>
          <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {themePresets.map((preset) => (
              <button
                key={preset.id}
                type="button"
                onClick={() => previewTheme(preset.id)}
                className={[
                  "rounded-2xl border p-3 text-left transition",
                  theme === preset.id ? "border-cyan-300/30 bg-cyan-300/[0.08]" : "border-white/8 bg-slate-950/30 hover:border-white/15",
                ].join(" ")}
              >
                <div className="flex gap-1.5">
                  {preset.colors.map((color) => (
                    <span key={color} className="h-4 w-4 rounded-full border border-white/20" style={{ background: color }} />
                  ))}
                </div>
                <div className="mt-3 text-xs font-medium text-slate-200">{preset.label}</div>
              </button>
            ))}
          </div>

          {theme === "custom" ? (
            <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
              {([
                ["accent", "强调色"],
                ["background", "背景色"],
                ["panel", "面板色"],
                ["text", "文字色"],
                ["muted", "次要文字"],
              ] as const).map(([key, label]) => (
                <label key={key} className="block">
                  <span className="text-xs text-slate-500">{label}</span>
                  <div className="mt-2 flex items-center gap-2 rounded-xl border border-white/10 bg-slate-950/50 p-2">
                    <input
                      type="color"
                      value={customTheme[key]}
                      onChange={(event) => {
                        const next = { ...customTheme, [key]: event.currentTarget.value };
                        previewTheme("custom", next);
                      }}
                      className="h-8 w-10 rounded border-0 bg-transparent p-0"
                    />
                    <span className="font-mono text-[10px] text-slate-500">{customTheme[key]}</span>
                  </div>
                </label>
              ))}
            </div>
          ) : null}
        </section>

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
            <Gamepad2 size={17} className="text-cyan-300" />
            <h2 className="text-sm font-semibold text-slate-200">游戏默认值</h2>
          </div>
          <p className="mt-2 text-xs leading-5 text-slate-500">
            新建实例时使用这些默认值；实例详情页仍可单独覆盖内存和 JVM 参数。
          </p>
          <div className="mt-4 space-y-4">
            <label className="block">
              <span className="text-xs text-slate-500">默认内存上限</span>
              <div className="mt-2 flex items-center gap-3">
                <input
                  type="range"
                  min={512}
                  max={32768}
                  step={512}
                  value={defaultMemoryMb}
                  onChange={(event) => setDefaultMemoryMb(Number(event.currentTarget.value))}
                  className="h-1.5 flex-1 accent-cyan-400"
                />
                <span className="w-24 rounded-lg border border-white/8 bg-slate-950/55 px-2 py-1 text-center text-xs tabular-nums text-slate-300">
                  {defaultMemoryMb} MB
                </span>
              </div>
            </label>
            <div className="grid grid-cols-[1fr_1fr_auto] items-end gap-3">
              <label>
                <span className="text-xs text-slate-500">窗口宽度</span>
                <input
                  type="number"
                  min={320}
                  max={7680}
                  value={gameWidth}
                  onChange={(event) => setGameWidth(Number(event.currentTarget.value))}
                  className="mt-2 w-full rounded-xl border border-white/10 bg-slate-950/55 px-3 py-2.5 text-sm text-white outline-none focus:border-cyan-300/40"
                />
              </label>
              <label>
                <span className="text-xs text-slate-500">窗口高度</span>
                <input
                  type="number"
                  min={240}
                  max={4320}
                  value={gameHeight}
                  onChange={(event) => setGameHeight(Number(event.currentTarget.value))}
                  className="mt-2 w-full rounded-xl border border-white/10 bg-slate-950/55 px-3 py-2.5 text-sm text-white outline-none focus:border-cyan-300/40"
                />
              </label>
              <button
                type="button"
                onClick={() => setFullscreen((value) => !value)}
                className={[
                  "mb-0.5 rounded-xl border px-3.5 py-2.5 text-xs font-medium transition",
                  fullscreen
                    ? "border-cyan-300/25 bg-cyan-300/10 text-cyan-200"
                    : "border-white/8 bg-white/5 text-slate-400",
                ].join(" ")}
              >
                {fullscreen ? "全屏" : "窗口"}
              </button>
            </div>
          </div>
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
