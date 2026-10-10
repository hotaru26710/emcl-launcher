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
import type { CommunityInstallResult, Instance, ModrinthSearchResult, ModrinthVersionInfo } from "../api/types";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { formatDate } from "../lib/format";
import { useLauncherStore } from "../stores/launcher";

const PAGE_SIZE = 20;

const tagLabels: Record<string, string> = {
  adventure: "冒险",
  cursed: "搞怪",
  decoration: "装饰",
  economy: "经济",
  equipment: "装备",
  food: "食物",
  game_mechanics: "游戏机制",
  library: "前置库",
  magic: "魔法",
  management: "管理",
  minigame: "小游戏",
  mobs: "生物",
  optimization: "性能优化",
  social: "社交",
  storage: "存储",
  technology: "科技",
  transportation: "交通",
  utility: "实用工具",
  worldgen: "世界生成",
  fabric: "Fabric",
  quilt: "Quilt",
  forge: "Forge",
  neoforge: "NeoForge",
};

function compactNumber(value: number): string {
  return new Intl.NumberFormat("zh-CN", { notation: "compact", maximumFractionDigits: 1 }).format(value);
}

function tagLabel(tag: string): string {
  return tagLabels[tag] ?? tag;
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
  const [loadingMore, setLoadingMore] = useState(false);
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

  useEffect(() => {
    if (!instance) return;
    void search(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [instance?.id]);

  const search = async (reset: boolean) => {
    if (!instance) return;
    const offset = reset ? 0 : results.length;
    if (reset) {
      setLoading(true);
      setSelectedProject(null);
      setVersions([]);
    } else {
      setLoadingMore(true);
    }
    setMessage("");
    try {
      const next = await api.searchModrinthMods({
        query,
        gameVersion: instance.versionId,
        loader: instance.loader,
        offset,
      });
      setResults((current) => (reset ? next : [...current, ...next]));
    } catch (error) {
      setMessage(String(error));
    } finally {
      setLoading(false);
      setLoadingMore(false);
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
    if (
      !window.confirm(
        "安装时会自动检查并下载这个模组的必需依赖；实例里已经存在的依赖文件会跳过，不会重复下载。是否继续？",
      )
    ) {
      return;
    }
    setInstalling(version.id);
    setMessage("");
    try {
      const result: CommunityInstallResult = await api.installModrinthVersion({
        instanceId: instance.id,
        versionId: version.id,
      });
      setMessage(
        `安装完成：新增 ${result.installed} 个，跳过 ${result.skipped} 个已存在文件。` +
          (result.dependencies.length > 1 ? ` 已处理 ${result.dependencies.length} 个文件。` : ""),
      );
    } catch (error) {
      setMessage(String(error));
    } finally {
      setInstalling(null);
    }
  };

  if (!instance) {
    return <div className="text-sm text-slate-500">正在读取实例...</div>;
  }

  const recommended = versions.find((version) => version.recommended) ?? versions[0];
  const canLoadMore = results.length > 0 && results.length % PAGE_SIZE === 0;

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
        title="Modrinth 社区模组"
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
                if (event.key === "Enter") void search(true);
              }}
              placeholder="搜索用于缩小范围，例如 sodium、worldgen、魔法"
              className="w-full rounded-xl border border-white/10 bg-slate-950/55 py-2.5 pl-9 pr-3 text-sm text-white outline-none placeholder:text-slate-700 focus:border-cyan-300/40"
            />
          </div>
          <button
            type="button"
            disabled={loading}
            onClick={() => void search(true)}
            className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:opacity-40"
          >
            <Search size={16} />
            {loading ? "加载中..." : "搜索"}
          </button>
        </div>
        <p className="mt-3 text-xs text-slate-600">
          默认陈列 20 个与当前实例兼容的热门模组；搜索只用于缩小范围，翻页时继续加载下一批。
        </p>
        {message ? <p className="mt-3 text-xs leading-5 text-cyan-200">{message}</p> : null}
      </div>

      <div className="mt-5 grid gap-4 lg:grid-cols-[1fr_1fr]">
        <section className="glass-panel rounded-[28px] p-4">
          <div className="flex items-center gap-2 text-sm font-semibold text-slate-200">
            <Box size={16} />
            热门 / 搜索结果
            <StatusPill tone="slate">{`${results.length} 个`}</StatusPill>
          </div>
          <div className="mt-4 max-h-[56vh] space-y-3 overflow-y-auto pr-1">
            {loading && results.length === 0 ? (
              <div className="rounded-2xl border border-dashed border-white/10 px-4 py-10 text-center text-sm text-slate-600">
                正在加载 Modrinth 热门模组...
              </div>
            ) : results.length === 0 ? (
              <div className="rounded-2xl border border-dashed border-white/10 px-4 py-10 text-center text-sm text-slate-600">
                没有找到兼容模组。
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
                    <div className="mt-2 flex flex-wrap gap-1.5">
                      <span className="rounded-full bg-white/5 px-2 py-1 text-[10px] text-slate-500">
                        {compactNumber(project.downloads)} 下载
                      </span>
                      {project.categories.slice(0, 4).map((tag) => (
                        <span key={tag} className="rounded-full bg-cyan-300/8 px-2 py-1 text-[10px] text-cyan-300/80">
                          {tagLabel(tag)}
                        </span>
                      ))}
                    </div>
                  </div>
                </button>
              ))
            )}
            {canLoadMore ? (
              <button
                type="button"
                disabled={loadingMore}
                onClick={() => void search(false)}
                className="w-full rounded-xl border border-white/10 bg-white/5 px-4 py-2.5 text-xs text-slate-400 transition hover:bg-white/8 hover:text-slate-200 disabled:opacity-40"
              >
                {loadingMore ? "正在加载..." : "加载下 20 个"}
              </button>
            ) : null}
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
                <div
                  key={version.id}
                  className={[
                    "rounded-2xl border p-3",
                    recommended?.id === version.id
                      ? "border-emerald-300/25 bg-emerald-300/[0.05]"
                      : "border-white/8 bg-slate-950/30",
                  ].join(" ")}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="truncate text-sm font-medium text-slate-100">{version.name}</span>
                        {recommended?.id === version.id ? <StatusPill tone="emerald">推荐</StatusPill> : null}
                      </div>
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
                      访问 Modrinth 官方页 <ExternalLink size={12} />
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
