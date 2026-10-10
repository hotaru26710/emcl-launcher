import { useEffect, useState } from "react";
import { ArrowLeft, GitBranch, History, RefreshCw } from "lucide-react";
import { Link, useParams } from "react-router-dom";
import { api, errorMessage } from "../api/tauri";
import type { BuildArtifact, Project } from "../api/types";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { formatBytes, formatDate } from "../lib/format";

export function BuildTimeline() {
  const { id } = useParams<{ id: string }>();
  const [project, setProject] = useState<Project | null>(null);
  const [artifacts, setArtifacts] = useState<BuildArtifact[]>([]);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState("");

  const refresh = async () => {
    if (!id) return;
    setLoading(true);
    try {
      const [nextProject, nextArtifacts] = await Promise.all([
        api.getProject(id),
        api.listBuildArtifacts(id),
      ]);
      setProject(nextProject);
      setArtifacts(nextArtifacts);
    } catch (error) {
      setMessage(errorMessage(error));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
  }, [id]);

  return (
    <div className="animate-page mx-auto max-w-5xl">
      <Link to={id ? `/projects/${id}` : "/projects"} className="mb-5 inline-flex items-center gap-2 text-sm text-slate-500 hover:text-slate-200">
        <ArrowLeft size={16} />
        返回工程详情
      </Link>
      <PageHeader
        title="构建时间线"
        description={project ? `${project.name} · ${project.outputDir}` : "构建产物历史"}
        action={
          <button
            type="button"
            disabled={loading}
            onClick={() => void refresh()}
            className="inline-flex items-center gap-2 rounded-xl border border-white/10 bg-white/5 px-4 py-2.5 text-sm text-slate-300 transition hover:bg-white/8 disabled:opacity-40"
          >
            <RefreshCw size={16} className={loading ? "animate-spin" : ""} />
            刷新
          </button>
        }
      />

      {message ? <div className="mb-4 rounded-2xl border border-rose-300/15 bg-rose-300/[0.06] px-4 py-3 text-sm text-rose-200">{message}</div> : null}

      <div className="glass-panel rounded-[30px] p-5">
        {artifacts.length === 0 ? (
          <div className="rounded-2xl border border-dashed border-white/10 px-4 py-12 text-center text-sm text-slate-600">
            还没有构建历史，请先到工程详情页扫描产物。
          </div>
        ) : (
          <div className="relative space-y-4 before:absolute before:bottom-3 before:left-[1.15rem] before:top-3 before:w-px before:bg-white/10">
            {artifacts.map((artifact, index) => (
              <div key={artifact.id} className="relative flex gap-4">
                <div className="z-10 grid h-10 w-10 shrink-0 place-items-center rounded-2xl border border-cyan-300/20 bg-cyan-300/10 text-cyan-200">
                  <History size={17} />
                </div>
                <div className="min-w-0 flex-1 rounded-2xl border border-white/8 bg-slate-950/30 p-4">
                  <div className="flex items-start justify-between gap-4">
                    <div className="min-w-0">
                      <div className="truncate text-sm font-semibold text-slate-100">
                        {artifact.filePath.split(/[\\/]/).pop()}
                      </div>
                      <div className="mt-1 text-xs text-slate-600">
                        {formatDate(artifact.builtAt)} · {formatBytes(artifact.fileSize)}
                      </div>
                    </div>
                    {index === 0 ? <StatusPill tone="emerald">最新</StatusPill> : <StatusPill tone="slate">{artifact.status}</StatusPill>}
                  </div>
                  <div className="mt-3 flex flex-wrap items-center gap-3 text-[11px] text-slate-500">
                    <span className="font-mono">{artifact.sha256.slice(0, 16)}…</span>
                    {artifact.gitBranch ? (
                      <span className="inline-flex items-center gap-1">
                        <GitBranch size={12} />
                        {artifact.gitBranch}
                      </span>
                    ) : null}
                    {artifact.gitCommit ? <span className="font-mono">{artifact.gitCommit.slice(0, 8)}</span> : null}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
