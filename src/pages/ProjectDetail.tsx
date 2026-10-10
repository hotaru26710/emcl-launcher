import { useEffect, useState } from "react";
import {
  ArrowLeft,
  FolderGit2,
  Link2,
  Play,
  ScanSearch,
  Trash2,
} from "lucide-react";
import { Link, useParams } from "react-router-dom";
import { api, errorMessage } from "../api/tauri";
import type { BuildArtifact, LinkedProjectInstance, Project } from "../api/types";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { formatBytes, formatDate } from "../lib/format";
import { useLauncherStore } from "../stores/launcher";

export function ProjectDetail() {
  const { id } = useParams<{ id: string }>();
  const instances = useLauncherStore((state) => state.instances);
  const [project, setProject] = useState<Project | null>(null);
  const [links, setLinks] = useState<LinkedProjectInstance[]>([]);
  const [artifacts, setArtifacts] = useState<BuildArtifact[]>([]);
  const [selectedInstance, setSelectedInstance] = useState("");
  const [scanning, setScanning] = useState(false);
  const [building, setBuilding] = useState(false);
  const [buildLogs, setBuildLogs] = useState<string[]>([]);
  const [message, setMessage] = useState("");

  const refresh = async () => {
    if (!id) return;
    try {
      const [nextProject, nextLinks, nextArtifacts] = await Promise.all([
        api.getProject(id),
        api.listProjectInstances(id),
        api.listBuildArtifacts(id),
      ]);
      setProject(nextProject);
      setLinks(nextLinks);
      setArtifacts(nextArtifacts);
    } catch (error) {
      setMessage(errorMessage(error));
    }
  };

  useEffect(() => {
    void refresh();
  }, [id]);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    void api.onProjectBuildLog((log) => {
      if (log.projectId === id) {
        setBuildLogs((current) => [...current, `[${log.stream}] ${log.line}`].slice(-400));
      }
    }).then((stop) => {
      if (disposed) stop();
      else unlisteners.push(stop);
    });
    void api.onProjectBuildExit((exit) => {
      if (exit.projectId === id) {
        setBuilding(false);
        if (!exit.success) setMessage(`构建失败，退出码 ${exit.code ?? "未知"}`);
      }
    }).then((stop) => {
      if (disposed) stop();
      else unlisteners.push(stop);
    });
    return () => {
      disposed = true;
      unlisteners.forEach((stop) => stop());
    };
  }, [id]);

  const scan = async () => {
    if (!id) return;
    setScanning(true);
    setMessage("");
    try {
      const next = await api.scanProjectArtifacts(id);
      setArtifacts(next);
      setMessage(next.length > 0 ? `已识别 ${next.length} 条构建历史。` : "没有识别到构建产物。");
    } catch (error) {
      setMessage(errorMessage(error));
    } finally {
      setScanning(false);
    }
  };

  const build = async () => {
    if (!id) return;
    setBuilding(true);
    setBuildLogs([]);
    setMessage("");
    try {
      const next = await api.runProjectBuild(id);
      setArtifacts(next);
      setMessage("构建完成，已刷新构建产物。");
    } catch (error) {
      setMessage(errorMessage(error));
    } finally {
      setBuilding(false);
    }
  };

  const deploy = async (artifactId: string) => {
    const target = links[0];
    if (!target) {
      setMessage("请先绑定一个实例。");
      return;
    }
    try {
      await api.deployBuildArtifact(target.instanceId, artifactId);
      setMessage(`已部署到 ${target.instanceName}。`);
      await refresh();
    } catch (error) {
      setMessage(errorMessage(error));
    }
  };

  const link = async () => {
    if (!id || !selectedInstance) return;
    try {
      await api.linkProjectInstance(id, selectedInstance, true);
      setSelectedInstance("");
      await refresh();
    } catch (error) {
      setMessage(errorMessage(error));
    }
  };

  const unlink = async (instanceId: string) => {
    if (!id) return;
    try {
      await api.unlinkProjectInstance(id, instanceId);
      await refresh();
    } catch (error) {
      setMessage(errorMessage(error));
    }
  };

  if (!project) {
    return <div className="text-sm text-slate-500">正在读取工程...</div>;
  }

  const linkedIds = new Set(links.map((link) => link.instanceId));
  const availableInstances = instances.filter((instance) => !linkedIds.has(instance.id));

  return (
    <div className="animate-page mx-auto max-w-5xl">
      <Link to="/projects" className="mb-5 inline-flex items-center gap-2 text-sm text-slate-500 hover:text-slate-200">
        <ArrowLeft size={16} />
        返回工程列表
      </Link>

      <PageHeader
        title={project.name}
        description="工程绑定与构建产物识别。后续步骤会在这里增加构建、部署和冲突检测。"
        action={
          <div className="flex gap-2">
            <button
              type="button"
              disabled={building}
              onClick={() => void build()}
              className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:opacity-40"
            >
              <Play size={16} />
              {building ? "构建中..." : "执行构建"}
            </button>
            <button
              type="button"
              disabled={scanning}
              onClick={() => void scan()}
              className="inline-flex items-center gap-2 rounded-xl border border-white/10 bg-white/5 px-4 py-2.5 text-sm text-slate-300 transition hover:bg-white/8 disabled:opacity-40"
            >
              <ScanSearch size={16} className={scanning ? "animate-spin" : ""} />
              {scanning ? "扫描中..." : "扫描产物"}
            </button>
          </div>
        }
      />

      {message ? <div className="mb-4 rounded-2xl border border-rose-300/15 bg-rose-300/[0.06] px-4 py-3 text-sm text-rose-200">{message}</div> : null}

      <div className="grid gap-4 lg:grid-cols-[1.15fr_0.85fr]">
        <section className="glass-panel rounded-[28px] p-5">
          <div className="flex items-center gap-3">
            <div className="grid h-12 w-12 place-items-center rounded-2xl border border-cyan-300/15 bg-cyan-300/8 text-cyan-200">
              <FolderGit2 size={20} />
            </div>
            <div>
              <div className="text-sm font-semibold text-slate-100">工程信息</div>
              <div className="mt-1 text-xs text-slate-600">{project.buildTool.toUpperCase()} · {project.gradleTask}</div>
            </div>
          </div>
          <dl className="mt-5 space-y-3 text-sm">
            <div>
              <dt className="text-xs text-slate-500">根目录</dt>
              <dd className="mt-1 break-all font-mono text-xs text-slate-300">{project.rootPath}</dd>
            </div>
            <div>
              <dt className="text-xs text-slate-500">输出目录</dt>
              <dd className="mt-1 break-all font-mono text-xs text-slate-300">{project.outputDir}</dd>
            </div>
            <div className="flex items-center gap-4">
              <StatusPill tone={project.autoBuild ? "emerald" : "slate"}>{project.autoBuild ? "自动构建" : "手动构建"}</StatusPill>
              <StatusPill tone={project.autoDeploy ? "cyan" : "slate"}>{project.autoDeploy ? "自动部署" : "手动部署"}</StatusPill>
            </div>
          </dl>
        </section>

        <section className="glass-panel rounded-[28px] p-5">
          <div className="flex items-center gap-2 text-sm font-semibold text-slate-200">
            <Link2 size={16} />
            绑定实例
          </div>
          <div className="mt-4 flex gap-2">
            <select
              value={selectedInstance}
              onChange={(event) => setSelectedInstance(event.currentTarget.value)}
              className="min-w-0 flex-1 rounded-xl border border-white/10 bg-slate-950/55 px-3 py-2.5 text-sm text-white outline-none focus:border-cyan-300/40"
            >
              <option value="">选择实例...</option>
              {availableInstances.map((instance) => (
                <option key={instance.id} value={instance.id}>
                  {instance.name} · {instance.versionId}
                </option>
              ))}
            </select>
            <button
              type="button"
              disabled={!selectedInstance}
              onClick={() => void link()}
              className="rounded-xl bg-cyan-400 px-3.5 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:opacity-40"
            >
              绑定
            </button>
          </div>
          <div className="mt-4 space-y-2">
            {links.length === 0 ? (
              <div className="rounded-xl border border-dashed border-white/10 px-3 py-6 text-center text-xs text-slate-600">
                尚未绑定实例。
              </div>
            ) : (
              links.map((link) => (
                <div key={link.instanceId} className="flex items-center gap-3 rounded-xl border border-white/8 bg-slate-950/30 px-3 py-2.5">
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-xs font-medium text-slate-200">{link.instanceName}</div>
                    <div className="mt-0.5 text-[10px] text-slate-600">{link.autoDeploy ? "自动部署" : "手动部署"}</div>
                  </div>
                  <button
                    type="button"
                    onClick={() => void unlink(link.instanceId)}
                    className="rounded-lg p-2 text-slate-600 transition hover:bg-rose-400/10 hover:text-rose-300"
                    aria-label="解除绑定"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))
            )}
          </div>
        </section>
      </div>

      <section className="glass-panel mt-4 rounded-[28px] p-5">
        <div className="flex items-center justify-between gap-4">
          <div>
            <h2 className="text-sm font-semibold text-slate-200">构建产物</h2>
            <p className="mt-1 text-xs text-slate-600">已排除 sources、javadoc、dev 等辅助 jar，并读取主 jar 元数据和 SHA-256。</p>
          </div>
          <Link
            to={`/projects/${project.id}/timeline`}
            className="rounded-lg border border-cyan-300/15 bg-cyan-300/[0.06] px-3 py-2 text-xs text-cyan-200 transition hover:bg-cyan-300/[0.12]"
          >
            打开时间线
          </Link>
        </div>
        <div className="mt-4 space-y-2">
          {artifacts.length === 0 ? (
            <div className="rounded-2xl border border-dashed border-white/10 px-4 py-8 text-center text-xs text-slate-600">
              还没有扫描到构建产物。
            </div>
          ) : (
            artifacts.map((artifact) => (
              <div key={artifact.id} className="rounded-2xl border border-white/8 bg-slate-950/30 p-3">
                <div className="flex items-start justify-between gap-4">
                  <div className="min-w-0">
                    <div className="truncate text-xs font-medium text-slate-200">{artifact.filePath.split(/[\\/]/).pop()}</div>
                    <div className="mt-1 font-mono text-[10px] text-slate-600">{artifact.sha256.slice(0, 16)}… · {formatBytes(artifact.fileSize)}</div>
                  </div>
                  <StatusPill tone="emerald">{artifact.status}</StatusPill>
                </div>
                <div className="mt-2 flex items-center justify-between gap-3 text-[10px] text-slate-600">
                  <span>
                  {formatDate(artifact.builtAt)}
                  {artifact.gitBranch ? ` · ${artifact.gitBranch}` : ""}
                  {artifact.gitCommit ? ` · ${artifact.gitCommit.slice(0, 8)}` : ""}
                  </span>
                  <button
                    type="button"
                    onClick={() => void deploy(artifact.id)}
                    className="rounded-lg border border-cyan-300/15 bg-cyan-300/[0.06] px-2.5 py-1.5 text-cyan-200 transition hover:bg-cyan-300/[0.12]"
                  >
                    部署
                  </button>
                </div>
              </div>
            ))
          )}
        </div>
      </section>

      {buildLogs.length > 0 || building ? (
        <section className="glass-panel mt-4 rounded-[28px] p-5">
          <h2 className="text-sm font-semibold text-slate-200">Gradle 输出</h2>
          <div className="mt-3 h-64 overflow-y-auto rounded-2xl border border-slate-800/80 bg-[#040914] p-3 font-mono text-[11px] leading-5 text-slate-400">
            {buildLogs.length === 0 ? (
              <div className="grid h-full place-items-center text-slate-700">等待构建输出...</div>
            ) : (
              buildLogs.map((line, index) => <div key={`${index}-${line}`}>{line}</div>)
            )}
          </div>
        </section>
      ) : null}
    </div>
  );
}
