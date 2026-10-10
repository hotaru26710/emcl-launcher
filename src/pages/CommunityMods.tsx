import { useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowLeft,
  Box,
  ChevronRight,
  Search,
  Sparkles,
} from "lucide-react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { api } from "../api/tauri";
import type { GitHubRepository, Instance, McimCurseForgeSearchResult, ModrinthSearchResult } from "../api/types";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { useLauncherStore } from "../stores/launcher";

type SourceId = "modrinth" | "mcim" | "github";

export interface UnifiedModEntry {
  key: string;
  title: string;
  description: string;
  iconUrl?: string | null;
  tags: string[];
  sources: SourceId[];
  score: number;
  modrinth?: ModrinthSearchResult;
  mcim?: McimCurseForgeSearchResult;
  github?: GitHubRepository;
  recommended?: boolean;
}

const sourceLabels: Record<SourceId, string> = {
  modrinth: "Modrinth",
  mcim: "MCIM CurseForge",
  github: "GitHub",
};

function normalizeKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]/g, "");
}

function mergeEntries(current: UnifiedModEntry[], incoming: UnifiedModEntry[]): UnifiedModEntry[] {
  const map = new Map(current.map((entry) => [entry.key, entry]));
  for (const entry of incoming) {
    const existing = map.get(entry.key);
    if (!existing) {
      map.set(entry.key, entry);
      continue;
    }
    existing.sources = Array.from(new Set([...existing.sources, ...entry.sources]));
    existing.tags = Array.from(new Set([...existing.tags, ...entry.tags])).slice(0, 8);
    existing.score += entry.score;
    existing.modrinth ??= entry.modrinth;
    existing.mcim ??= entry.mcim;
    existing.github ??= entry.github;
  }
  return Array.from(map.values()).sort((left, right) => right.score - left.score);
}

function fromModrinth(item: ModrinthSearchResult): UnifiedModEntry {
  return {
    key: normalizeKey(item.slug || item.title),
    title: item.title,
    description: item.description,
    iconUrl: item.iconUrl,
    tags: item.categories,
    sources: ["modrinth"],
    score: item.downloads,
    modrinth: item,
  };
}

function fromMcim(item: McimCurseForgeSearchResult): UnifiedModEntry {
  return {
    key: normalizeKey(item.slug || item.title),
    title: item.title,
    description: item.description,
    iconUrl: item.iconUrl,
    tags: item.categories,
    sources: ["mcim"],
    score: item.downloads,
    mcim: item,
  };
}

function fromGithub(item: GitHubRepository): UnifiedModEntry {
  return {
    key: normalizeKey(item.name || item.fullName),
    title: item.name,
    description: item.description ?? item.fullName,
    tags: item.topics,
    sources: ["github"],
    score: item.stars * 1000,
    github: item,
  };
}

export function CommunityMods() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const instances = useLauncherStore((state) => state.instances);
  const [instance, setInstance] = useState<Instance | null>(
    instances.find((item) => item.id === id) ?? null,
  );
  const [query, setQuery] = useState("");
  const [entries, setEntries] = useState<UnifiedModEntry[]>([]);
  const [offsets, setOffsets] = useState<Record<SourceId, number>>({ modrinth: 0, mcim: 0, github: 0 });
  const [hasMore, setHasMore] = useState<Record<SourceId, boolean>>({ modrinth: true, mcim: true, github: true });
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState("");
  const sentinelRef = useRef<HTMLDivElement>(null);

  const recommended = useMemo(
    () => entries.filter((entry) => entry.recommended).slice(0, 3),
    [entries],
  );

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
    void load(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [instance?.id]);

  useEffect(() => {
    const element = sentinelRef.current;
    if (!element) return;
    const observer = new IntersectionObserver((items) => {
      if (items[0]?.isIntersecting && !loading) {
        void load(false);
      }
    }, { rootMargin: "300px" });
    observer.observe(element);
    return () => observer.disconnect();
  }, [loading]);

  const load = async (reset: boolean) => {
    if (!instance || loading) return;
    setLoading(true);
    setMessage("");
    const nextOffsets = reset ? { modrinth: 0, mcim: 0, github: 0 } : offsets;
    try {
      const tasks: Promise<UnifiedModEntry[]>[] = [];
      if (hasMore.modrinth || reset) {
        tasks.push(api.searchModrinthMods({ query, gameVersion: instance.versionId, loader: instance.loader, offset: nextOffsets.modrinth }).then((items) => {
          setHasMore((state) => ({ ...state, modrinth: items.length >= 20 }));
          setOffsets((state) => ({ ...state, modrinth: nextOffsets.modrinth + items.length }));
          return items.map(fromModrinth);
        }));
      }
      if (hasMore.mcim || reset) {
        tasks.push(api.searchMcimCurseForgeMods({ query, gameVersion: instance.versionId, loader: instance.loader, offset: nextOffsets.mcim }).then((items) => {
          setHasMore((state) => ({ ...state, mcim: items.length >= 20 }));
          setOffsets((state) => ({ ...state, mcim: nextOffsets.mcim + items.length }));
          return items.map(fromMcim);
        }));
      }
      if (hasMore.github || reset) {
        tasks.push(api.searchGitHubMods(query, nextOffsets.github).then((items) => {
          setHasMore((state) => ({ ...state, github: items.length >= 30 }));
          setOffsets((state) => ({ ...state, github: nextOffsets.github + items.length }));
          return items.map(fromGithub);
        }));
      }

      const results = await Promise.allSettled(tasks);
      const additions = results.flatMap((result) => result.status === "fulfilled" ? result.value : []);
      if (results.some((result) => result.status === "rejected")) {
        setMessage("部分来源加载失败，已显示可用结果。");
      }
      setEntries((current) => mergeEntries(reset ? [] : current, additions).map((entry, index) => ({
        ...entry,
        recommended: reset && index < 3,
      })));
    } finally {
      setLoading(false);
    }
  };

  const openDetail = (entry: UnifiedModEntry) => {
    sessionStorage.setItem("emcl:community-entry", JSON.stringify(entry));
    navigate(`/instances/${id}/community-detail`, { state: { entry } });
  };

  if (!instance) {
    return <div className="text-sm text-slate-500">正在读取实例...</div>;
  }

  return (
    <div className="animate-page mx-auto max-w-5xl">
      <Link to={`/instances/${instance.id}`} className="mb-5 inline-flex items-center gap-2 text-sm text-slate-500 hover:text-slate-200">
        <ArrowLeft size={16} />
        返回实例详情
      </Link>
      <PageHeader
        title="社区模组"
        description={`统一搜索 Modrinth、MCIM CurseForge 和 GitHub Releases · ${instance.versionId}${instance.loader ? ` · ${instance.loader}` : ""}`}
      />

      <div className="glass-panel rounded-[30px] p-5">
        <div className="flex gap-3">
          <div className="relative min-w-0 flex-1">
            <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-slate-600" size={16} />
            <input
              value={query}
              onChange={(event) => setQuery(event.currentTarget.value)}
              onKeyDown={(event) => { if (event.key === "Enter") void load(true); }}
              placeholder="搜索模组；留空显示多平台推荐"
              className="w-full rounded-xl border border-white/10 bg-slate-950/55 py-2.5 pl-9 pr-3 text-sm text-white outline-none placeholder:text-slate-700 focus:border-cyan-300/40"
            />
          </div>
          <button type="button" disabled={loading} onClick={() => void load(true)} className="rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 disabled:opacity-40">
            {loading ? "加载中..." : "搜索"}
          </button>
        </div>
        {message ? <p className="mt-3 text-xs text-amber-200/80">{message}</p> : null}
      </div>

      {recommended.length > 0 ? (
        <section className="mt-5">
          <div className="mb-3 flex items-center gap-2 text-sm font-semibold text-slate-200">
            <Sparkles size={16} className="text-cyan-300" />
            推荐模组
          </div>
          <div className="grid gap-3">
            {recommended.map((entry) => (
              <ModCard key={`rec-${entry.key}`} entry={entry} onOpen={openDetail} featured />
            ))}
          </div>
        </section>
      ) : null}

      <section className="mt-5 grid gap-3">
        {entries.map((entry) => (
          <ModCard key={entry.key} entry={entry} onOpen={openDetail} />
        ))}
        <div ref={sentinelRef} className="h-8" />
        {loading ? <div className="py-6 text-center text-sm text-slate-500">正在加载更多来源...</div> : null}
      </section>
    </div>
  );
}

function ModCard({ entry, onOpen, featured = false }: { entry: UnifiedModEntry; onOpen: (entry: UnifiedModEntry) => void; featured?: boolean }) {
  return (
    <button
      type="button"
      onClick={() => onOpen(entry)}
      className={[
        "flex w-full items-center gap-4 rounded-[26px] border p-4 text-left transition hover:-translate-y-0.5 hover:border-cyan-300/25",
        featured ? "border-cyan-300/20 bg-cyan-300/[0.055]" : "border-white/8 bg-white/[0.025]",
      ].join(" ")}
    >
      {entry.iconUrl ? <img src={entry.iconUrl} alt="" className="h-14 w-14 rounded-2xl object-cover" /> : <div className="grid h-14 w-14 place-items-center rounded-2xl bg-slate-800 text-slate-500"><Box size={22} /></div>}
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="truncate text-base font-semibold text-slate-100">{entry.title}</span>
          {featured ? <StatusPill tone="emerald">推荐</StatusPill> : null}
          {entry.sources.map((source) => <StatusPill key={source} tone={source === "modrinth" ? "cyan" : source === "github" ? "slate" : "amber"}>{sourceLabels[source]}</StatusPill>)}
        </div>
        <p className="mt-1 line-clamp-2 text-xs leading-5 text-slate-500">{entry.description}</p>
        <div className="mt-2 flex flex-wrap gap-1.5">
          {entry.tags.slice(0, 8).map((tag) => <span key={tag} className="rounded-full bg-white/5 px-2 py-1 text-[10px] text-slate-500">{tag}</span>)}
        </div>
      </div>
      <ChevronRight className="shrink-0 text-slate-600" size={20} />
    </button>
  );
}
