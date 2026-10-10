import { useEffect, useState } from "react";
import { ArrowLeft, ExternalLink, GitBranch, Globe2, Package } from "lucide-react";
import { Link, useLocation, useParams } from "react-router-dom";
import { api, errorMessage } from "../api/tauri";
import type { GitHubRelease, McimCurseForgeFile, ModrinthVersionInfo } from "../api/types";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import type { UnifiedModEntry } from "./CommunityMods";

export function CommunityModDetail() {
  const { id } = useParams<{ id: string }>();
  const location = useLocation();
  const stateEntry = (location.state as { entry?: UnifiedModEntry } | null)?.entry;
  const storedEntry = (() => {
    try {
      return JSON.parse(sessionStorage.getItem("emcl:community-entry") ?? "null") as UnifiedModEntry | null;
    } catch {
      return null;
    }
  })();
  const entry = stateEntry ?? storedEntry;
  const [gameVersion, setGameVersion] = useState("");
  const [loader, setLoader] = useState<string | null>(null);
  const [modrinthVersions, setModrinthVersions] = useState<ModrinthVersionInfo[]>([]);
  const [mcimFiles, setMcimFiles] = useState<McimCurseForgeFile[]>([]);
  const [githubReleases, setGithubReleases] = useState<GitHubRelease[]>([]);
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);
  const [message, setMessage] = useState("");

  useEffect(() => {
    if (!id) return;
    void api.getInstance(id).then((instance) => {
      if (instance) {
        setGameVersion(instance.versionId);
        setLoader(instance.loader ?? null);
      }
    });
  }, [id]);

  useEffect(() => {
    if (!entry || !id || !gameVersion) return;
    setLoading(true);
    setMessage("");
    const tasks: Promise<unknown>[] = [];
    if (entry.modrinth) {
      tasks.push(api.getModrinthVersions(entry.modrinth.projectId, gameVersion, loader).then(setModrinthVersions).catch(() => undefined));
    }
    if (entry.mcim) {
      tasks.push(api.getMcimCurseForgeFiles(entry.mcim.id, gameVersion, loader).then(setMcimFiles).catch(() => undefined));
    }
    if (entry.github) {
      tasks.push(api.getGitHubReleases(entry.github.owner, entry.github.name).then(setGithubReleases).catch(() => undefined));
    }
    void Promise.all(tasks).finally(() => setLoading(false));
  }, [entry, id, gameVersion, loader]);

  if (!entry || !id) {
    return <div className="text-sm text-slate-500">没有选中的模组，请返回社区模组页重新选择。</div>;
  }

  const installModrinth = async (version: ModrinthVersionInfo) => {
    if (!window.confirm("将安装这个 Modrinth 文件并自动处理必需依赖，是否继续？")) return;
    setInstalling(`mr-${version.id}`);
    try {
      const result = await api.installModrinthVersion({ instanceId: id, versionId: version.id });
      setMessage(`安装完成：新增 ${result.installed}，跳过 ${result.skipped}`);
    } catch (error) {
      setMessage(errorMessage(error));
    } finally {
      setInstalling(null);
    }
  };

  const installMcim = async (file: McimCurseForgeFile) => {
    if (!window.confirm("将安装这个 MCIM CurseForge 文件并自动处理必需依赖，是否继续？")) return;
    setInstalling(`mcim-${file.id}`);
    try {
      const result = await api.installMcimCurseForgeFile({ instanceId: id, modId: file.modId, fileId: file.id });
      setMessage(`安装完成：新增 ${result.installed}，跳过 ${result.skipped}`);
    } catch (error) {
      setMessage(errorMessage(error));
    } finally {
      setInstalling(null);
    }
  };

  const installGithub = async (asset: GitHubRelease["assets"][number]) => {
    if (!window.confirm("将下载 GitHub Release 中的 .jar 到实例，是否继续？")) return;
    setInstalling(`gh-${asset.id}`);
    try {
      const mods = await api.installGitHubReleaseAsset(id, asset);
      setMessage(`安装完成，当前实例模组数：${mods.length}`);
    } catch (error) {
      setMessage(errorMessage(error));
    } finally {
      setInstalling(null);
    }
  };

  return (
    <div className="animate-page mx-auto max-w-5xl">
      <Link to={`/instances/${id}/community`} className="mb-5 inline-flex items-center gap-2 text-sm text-slate-500 hover:text-slate-200">
        <ArrowLeft size={16} />
        返回统一搜索
      </Link>
      <PageHeader title={entry.title} description={entry.description} />
      <div className="mb-4 flex flex-wrap gap-2">
        {entry.sources.map((source) => <StatusPill key={source} tone={source === "modrinth" ? "cyan" : source === "github" ? "slate" : "amber"}>{source}</StatusPill>)}
        {entry.tags.map((tag) => <StatusPill key={tag} tone="slate">{tag}</StatusPill>)}
      </div>
      {message ? <div className="mb-4 rounded-2xl border border-cyan-300/15 bg-cyan-300/[0.05] px-4 py-3 text-sm text-cyan-100">{message}</div> : null}

      {entry.modrinth ? (
        <section className="glass-panel mb-4 rounded-[28px] p-5">
          <div className="flex items-center gap-2 text-sm font-semibold text-slate-200"><Package size={16} /> Modrinth</div>
          <div className="mt-4 space-y-2">
            {loading && modrinthVersions.length === 0 ? <div className="text-xs text-slate-500">读取版本中...</div> : modrinthVersions.map((version) => (
              <div key={version.id} className="flex items-center gap-3 rounded-2xl border border-white/8 bg-slate-950/30 px-3 py-3">
                <div className="min-w-0 flex-1"><div className="truncate text-xs text-slate-200">{version.name}</div><div className="text-[10px] text-slate-600">{version.versionNumber} · {version.loaders.join(", ")}</div></div>
                {version.recommended ? <StatusPill tone="emerald">推荐</StatusPill> : null}
                <button type="button" disabled={installing === `mr-${version.id}`} onClick={() => void installModrinth(version)} className="rounded-lg bg-cyan-400 px-2.5 py-1.5 text-[11px] font-semibold text-slate-950 disabled:opacity-40">安装</button>
              </div>
            ))}
          </div>
        </section>
      ) : null}

      {entry.mcim ? (
        <section className="glass-panel mb-4 rounded-[28px] p-5">
          <div className="flex items-center gap-2 text-sm font-semibold text-slate-200"><Globe2 size={16} /> MCIM CurseForge</div>
          <div className="mt-4 space-y-2">
            {loading && mcimFiles.length === 0 ? <div className="text-xs text-slate-500">读取文件中...</div> : mcimFiles.map((file) => (
              <div key={file.id} className="flex items-center gap-3 rounded-2xl border border-white/8 bg-slate-950/30 px-3 py-3">
                <div className="min-w-0 flex-1"><div className="truncate text-xs text-slate-200">{file.displayName}</div><div className="text-[10px] text-slate-600">{file.fileName}</div></div>
                <button type="button" disabled={installing === `mcim-${file.id}`} onClick={() => void installMcim(file)} className="rounded-lg bg-cyan-400 px-2.5 py-1.5 text-[11px] font-semibold text-slate-950 disabled:opacity-40">安装</button>
              </div>
            ))}
          </div>
        </section>
      ) : null}

      {entry.github ? (
        <section className="glass-panel rounded-[28px] p-5">
          <div className="flex items-center gap-2 text-sm font-semibold text-slate-200"><GitBranch size={16} /> GitHub Releases</div>
          <div className="mt-4 space-y-3">
            {loading && githubReleases.length === 0 ? <div className="text-xs text-slate-500">读取 Release 中...</div> : githubReleases.map((release) => (
              <div key={release.id} className="rounded-2xl border border-white/8 bg-slate-950/30 p-3">
                <div className="flex items-center justify-between gap-3"><div className="text-xs font-medium text-slate-200">{release.name || release.tagName}</div><a href={release.htmlUrl} target="_blank" rel="noreferrer" className="text-[10px] text-cyan-300">官方页 <ExternalLink size={11} className="inline" /></a></div>
                <div className="mt-2 space-y-1">
                  {release.assets.map((asset) => <div key={asset.id} className="flex items-center gap-3"><span className="min-w-0 flex-1 truncate text-[11px] text-slate-500">{asset.name}</span><button type="button" disabled={installing === `gh-${asset.id}`} onClick={() => void installGithub(asset)} className="rounded-lg bg-cyan-400 px-2 py-1 text-[10px] font-semibold text-slate-950">安装</button></div>)}
                </div>
              </div>
            ))}
          </div>
        </section>
      ) : null}
    </div>
  );
}
