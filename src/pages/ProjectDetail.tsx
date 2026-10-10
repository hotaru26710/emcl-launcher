import { useEffect, useState } from "react";
import {
  ArrowLeft,
  FolderGit2,
  Link2,
  ScanSearch,
  Trash2,
} from "lucide-react";
import { Link, useParams } from "react-router-dom";
import { api, errorMessage } from "../api/tauri";
import type { LinkedProjectInstance, Project } from "../api/types";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { useLauncherStore } from "../stores/launcher";

export function ProjectDetail() {
  const { id } = useParams<{ id: string }>();
  const instances = useLauncherStore((state) => state.instances);
  const [project, setProject] = useState<Project | null>(null);
  const [links, setLinks] = useState<LinkedProjectInstance[]>([]);
  const [selectedInstance, setSelectedInstance] = useState("");
  const [message, setMessage] = useState("");

  const refresh = async () => {
    if (!id) return;
    try {
      const [nextProject, nextLinks] = await Promise.all([
        api.getProject(id),
        api.listProjectInstances(id),
      ]);
      setProject(nextProject);
      setLinks(nextLinks);
    } catch (error) {
      setMessage(errorMessage(error));
    }
  };

  useEffect(() => {
    void refresh();
  }, [id]);

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
          <button
            type="button"
            className="inline-flex items-center gap-2 rounded-xl border border-white/10 bg-white/5 px-4 py-2.5 text-sm text-slate-300 transition hover:bg-white/8"
          >
            <ScanSearch size={16} />
            扫描产物
          </button>
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
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-sm font-semibold text-slate-200">构建产物</h2>
            <p className="mt-1 text-xs text-slate-600">下一步会扫描输出目录、排除非主 jar 并读取 jar 内元数据。</p>
          </div>
          <StatusPill tone="slate">待接入</StatusPill>
        </div>
      </section>
    </div>
  );
}
