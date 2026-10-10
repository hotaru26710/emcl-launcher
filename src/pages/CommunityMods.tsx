import { useEffect, useState } from "react";
import {
  ArrowLeft,
  Box,
  Download,
  ExternalLink,
  Search,
  Sparkles,
} from "lucide-react";
import { Link, useParams } from "react-router-dom";
import { api } from "../api/tauri";
import type { Instance, ModrinthSearchResult, ModrinthVersionInfo } from "../api/types";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { formatDate } from "../lib/format";
import { useLauncherStore } from "../stores/launcher";

function compactNumber(value: number): string {
  return new Intl.NumberFormat("zh-CN", { notation: "compact", maximumFractionDigits: 1 }).format(value);
}

export function CommunityMods() {
  const { id } = useParams<{ id: string }>();
  const instances = useLauncherStore((state) => state.instances);
  const [instance, setInstance] = useState<Instance | null>(
    instances.find((item) => item.id === id) ?? null,
  );
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ModrinthSearchResult[]>([]);
  const [selectedProject, setSelectedProject] = useState<ModrinthSearchResult | null>(null);
  const [versions, setVersions] = useState<ModrinthVersionInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [versionsLoading, setVersionsLoading] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);
  const [message, setMessage] = useState("");

  useEffect(() => {
    if (!id) return;
    const cached = instances.find((item) => item.id === id);
    if (cached) {
      setInstance(cached);
      return;
    }
    void api.getInstance(id).then(setInstance).catch(() => undefined);
  }, [id, instances]);

  const search = async () => {
    if (!instance) return;
    setLoading(true);
    setSelectedProject(null);
    setVersions([]);
    setMessage("");
    try {
      const next = await api.searchModrinthMods({
        query,
        gameVersion: instance.versionId,
        loader: instance.loader,
      });
      setResults(next);
    } catch (error) {
      setMessage(String(error));
    } finally {
      setLoading(false);
    }
  };

  const selectProject = async (project: ModrinthSearchResult) => {
    if (!instance) return;
    setSelectedProject(project);
    setVersionsLoading(true);
    setMessage("");
    try {
      const next = await api.getModrinthVersions(
        project.projectId,
        instance.versionId,
        instance.loader,
      );
      setVersions(next);
    } catch (error) {
      setMessage(String(error));
    } finally {
      setVersionsLoading(false);
    }
  };

  const install = async (version: ModrinthVersionInfo) => {
    if (!instance) return;
    setInstalling(version.id);
    setMessage("");
    try {
      const mods = await api.installModrinthVersion({
        instanceId: instance.id,
        versionId: version.id,
      });
      setMessage(`已安装 ${version.name}，当前实例模组数：${mods.length}`);
    } catch (error) {
      setMessage(String(error));
    } finally {
      setInstalling(null);
    }
  };

  if (!instance) {
    return <div className="text-sm text-slate-500">正在读取实例...</div>;
  }

  return (
    <div className="animate-page mx-auto max-w-6xl">
      <Link
        to={`/instances/${instance.id}`}
        className="mb-5 inline-flex items-center gap-2 text-sm text-slate-500 transition hover:text-slate-200"
      >
        <ArrowLeft size={16} />
        返回实例详情
      </Link>

      <PageHeader
        title="社区模组"
        description={`当前实例：${instance.name} · Minecraft ${instance.versionId}${instance.loader ? ` · ${instance.loader}` : ""}`}
      />

      <div className="glass-panel rounded-[30px] p-5">
        <div className="flex flex-wrap gap-3">
          <div className="relative min-w-64 flex-1">
            <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-slate-600" size={16} />
            <input
              value={query}
              onChange={(event) => setQuery(event.currentTarget.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") void search();
              }}
              placeholder="搜索 Modrinth 模组，例如 sodium、iris"
              className="w-full rounded-xl border border-white/10 bg-slate-950/55 py-2.5 pl-9 pr-3 text-sm text-white outline-none placeholder:text-slate-700 focus:border-cyan-300/40"
            />
          </div>
          <button
            type="button"
            disabled={loading}
            onClick={() => void search()}
            className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:opacity-40"
          >
            <Search size={16} />
            {loading ? "搜索中..." : "搜索"}
          </button>
        </div>
        <p className="mt-3 text-xs text-slate-600">
          仅展示与 Minecraft {instance.versionId}{instance.loader ? ` 和 ${instance.loader}` : ""} 兼容的模组；安装会自动处理必需依赖。
        </p>
        {message ? <p className="mt-3 text-xs leading-5 text-cyan-200">{message}</p> : null}
      </div>

      <div className="mt-5 grid gap-4 lg:grid-cols-[1fr_1fr]">
        <section className="glass-panel rounded-[28px] p-4">
          <div className="flex items-center gap-2 text-sm font-semibold text-slate-200">
            <Box size={16} />
            搜索结果
            <StatusPill tone="slate">{`${results.length} 个`}</StatusPill>
          </div>
          <div className="mt-4 max-h-[56vh] space-y-3 overflow-y-auto pr-1">
            {results.length === 0 ? (
              <div className="rounded-2xl border border-dashed border-white/10 px-4 py-10 text-center text-sm text-slate-600">
                输入关键词后搜索社区模组。
              </div>
            ) : (
              results.map((project) => (
                <button
                  key={project.projectId}
                  type="button"
                  onClick={() => void selectProject(project)}
                  className={[
                    "flex w-full items-start gap-3 rounded-2xl border p-3 text-left transition hover:-translate-y-0.5",
                    selectedProject?.projectId === project.projectId
                      ? "border-cyan-300/30 bg-cyan-300/[0.08]"
                      : "border-white/8 bg-slate-950/30 hover:border-white/15",
                  ].join(" ")}
                >
                  {project.iconUrl ? (
                    <img src={project.iconUrl} alt="" className="h-12 w-12 rounded-2xl object-cover" />
                  ) : (
                    <div className="grid h-12 w-12 place-items-center rounded-2xl bg-slate-800 text-slate-500">
                      <Box size={20} />
                    </div>
                  )}
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate text-sm font-semibold text-slate-100">{project.title}</span>
                      <span className="shrink-0 text-[10px] text-slate-600">by {project.author}</span>
                    </div>
                    <p className="mt-1 line-clamp-2 text-xs leading-5 text-slate-500">{project.description}</p>
                    <div className="mt-2 flex items-center gap-3 text-[10px] text-slate-600">
                      <span><Download size={11} className="mr-1 inline" />{compactNumber(project.downloads)}</span>
                      <span>{project.categories.slice(0, 3).join(" · ")}</span>
                    </div>
                  </div>
                </button>
              ))
            )}
          </div>
        </section>

        <section className="glass-panel rounded-[28px] p-4">
          <div className="flex items-center gap-2 text-sm font-semibold text-slate-200">
            <Sparkles size={16} />
            兼容版本
          </div>
          {!selectedProject ? (
            <div className="mt-4 rounded-2xl border border-dashed border-white/10 px-4 py-10 text-center text-sm text-slate-600">
              从左侧选择一个模组。
            </div>
          ) : versionsLoading ? (
            <div className="mt-4 py-10 text-center text-sm text-slate-500">正在读取版本...</div>
          ) : versions.length === 0 ? (
            <div className="mt-4 rounded-2xl border border-dashed border-white/10 px-4 py-10 text-center text-sm text-slate-600">
              没有找到与当前实例兼容的版本。
            </div>
          ) : (
            <div className="mt-4 max-h-[56vh] space-y-3 overflow-y-auto pr-1">
              {versions.map((version) => (
                <div key={version.id} className="rounded-2xl border border-white/8 bg-slate-950/30 p-3">
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="truncate text-sm font-medium text-slate-100">{version.name}</div>
                      <div className="mt-1 text-xs text-slate-600">
                        {version.versionNumber} · {formatDate(version.datePublished)} · {compactNumber(version.downloads)} 下载
                      </div>
                    </div>
                    <StatusPill tone={version.versionType === "release" ? "emerald" : "amber"}>
                      {version.versionType}
                    </StatusPill>
                  </div>
                  <div className="mt-2 flex flex-wrap gap-1.5">
                    {version.loaders.slice(0, 4).map((loader) => (
                      <span key={loader} className="rounded-full bg-white/5 px-2 py-1 text-[10px] text-slate-500">
                        {loader}
                      </span>
                    ))}
                    {version.gameVersions.slice(0, 3).map((game) => (
                      <span key={game} className="rounded-full bg-cyan-300/8 px-2 py-1 text-[10px] text-cyan-300/80">
                        {game}
                      </span>
                    ))}
                  </div>
                  <div className="mt-3 flex items-center justify-between gap-3">
                    <a
                      href={`https://modrinth.com/mod/${selectedProject.slug}`}
                      target="_blank"
                      rel="noreferrer"
                      className="inline-flex items-center gap-1 text-xs text-slate-500 hover:text-slate-300"
                    >
                      查看项目 <ExternalLink size={12} />
                    </a>
                    <button
                      type="button"
                      disabled={installing === version.id}
                      onClick={() => void install(version)}
                      className="inline-flex items-center gap-1.5 rounded-xl bg-cyan-400 px-3 py-2 text-xs font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:opacity-40"
                    >
                      <Download size={13} />
                      {installing === version.id ? "安装中..." : "安装到实例"}
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
