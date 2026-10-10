import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Check,
  FolderGit2,
  Plus,
  RefreshCw,
  Trash2,
  X,
} from "lucide-react";
import { Link, useNavigate } from "react-router-dom";
import { api, errorMessage } from "../api/tauri";
import type { Project, ProjectInspection } from "../api/types";
import { EmptyState } from "../components/EmptyState";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { formatDate } from "../lib/format";
import { useLauncherStore } from "../stores/launcher";

export function Projects() {
  const navigate = useNavigate();
  const instances = useLauncherStore((state) => state.instances);
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [inspection, setInspection] = useState<ProjectInspection | null>(null);
  const [name, setName] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const [gradleTask, setGradleTask] = useState("build");
  const [autoBuild, setAutoBuild] = useState(false);
  const [autoDeploy, setAutoDeploy] = useState(true);
  const [selectedInstances, setSelectedInstances] = useState<string[]>([]);
  const [message, setMessage] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const refresh = async () => {
    setLoading(true);
    try {
      setProjects(await api.listProjects());
    } catch (error) {
      setMessage(errorMessage(error));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
  }, []);

  const chooseDirectory = async () => {
    setMessage("");
    const selected = await open({ directory: true, multiple: false, title: "选择 Gradle 模组工程" });
    if (!selected || Array.isArray(selected)) return;
    try {
      const detected = await api.inspectProjectDirectory(selected);
      setInspection(detected);
      setName(detected.name);
      setOutputDir(detected.outputDir);
      setGradleTask(detected.gradleTask);
      setDialogOpen(true);
    } catch (error) {
      setMessage(errorMessage(error));
    }
  };

  const create = async () => {
    if (!inspection) return;
    setSubmitting(true);
    setMessage("");
    try {
      const project = await api.createProject({
        name,
        rootPath: inspection.rootPath,
        outputDir,
        gradleTask,
        autoBuild,
        autoDeploy,
        instanceIds: selectedInstances,
      });
      setDialogOpen(false);
      setInspection(null);
      setSelectedInstances([]);
      await refresh();
      navigate(`/projects/${project.id}`);
    } catch (error) {
      setMessage(errorMessage(error));
    } finally {
      setSubmitting(false);
    }
  };

  const remove = async (project: Project) => {
    if (!window.confirm(`确定移除工程“${project.name}”吗？这不会删除源文件。`)) return;
    try {
      await api.deleteProject(project.id);
      await refresh();
    } catch (error) {
      setMessage(errorMessage(error));
    }
  };

  return (
    <div className="animate-page mx-auto max-w-6xl">
      <PageHeader
        title="开发工程"
        description="绑定 Gradle 模组工程，识别构建产物，并在后续步骤中自动部署到实例。"
        action={
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => void refresh()}
              className="rounded-xl border border-white/10 bg-white/5 p-2.5 text-slate-300 transition hover:bg-white/8"
              aria-label="刷新"
            >
              <RefreshCw size={17} className={loading ? "animate-spin" : ""} />
            </button>
            <button
              type="button"
              onClick={() => void chooseDirectory()}
              className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300"
            >
              <Plus size={17} />
              绑定工程
            </button>
          </div>
        }
      />

      {message ? (
        <div className="mb-4 rounded-2xl border border-cyan-300/15 bg-cyan-300/[0.05] px-4 py-3 text-sm text-cyan-100">
          {message}
        </div>
      ) : null}

      {projects.length === 0 && !loading ? (
        <EmptyState
          icon={FolderGit2}
          title="还没有开发工程"
          description="选择 Gradle 模组工程目录，启动器会解析工程名、输出目录和默认构建任务。"
          action={
            <button
              type="button"
              onClick={() => void chooseDirectory()}
              className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300"
            >
              <Plus size={17} />
              绑定第一个工程
            </button>
          }
        />
      ) : (
        <div className="grid gap-4 xl:grid-cols-2">
          {projects.map((project) => (
            <div key={project.id} className="glass-panel hover-lift rounded-[28px] p-5">
              <div className="flex items-start gap-4">
                <div className="grid h-12 w-12 shrink-0 place-items-center rounded-2xl border border-cyan-300/15 bg-cyan-300/8 text-cyan-200">
                  <FolderGit2 size={20} />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <h2 className="truncate text-base font-semibold text-white">{project.name}</h2>
                    <StatusPill tone="cyan">{project.buildTool.toUpperCase()}</StatusPill>
                  </div>
                  <div className="mt-2 break-all font-mono text-xs text-slate-500">{project.rootPath}</div>
                  <div className="mt-2 text-xs text-slate-600">输出：{project.outputDir}</div>
                </div>
                <button
                  type="button"
                  onClick={() => void remove(project)}
                  className="rounded-lg p-2 text-slate-600 transition hover:bg-rose-400/10 hover:text-rose-300"
                  aria-label="移除工程"
                >
                  <Trash2 size={16} />
                </button>
              </div>
              <div className="mt-4 flex items-center justify-between">
                <span className="text-xs text-slate-600">更新于 {formatDate(project.updatedAt)}</span>
                <Link to={`/projects/${project.id}`} className="text-sm text-cyan-300 hover:text-cyan-200">
                  打开工程
                </Link>
              </div>
            </div>
          ))}
        </div>
      )}

      {dialogOpen && inspection ? (
        <div className="fixed inset-0 z-40 grid place-items-center bg-slate-950/75 p-6 backdrop-blur-sm">
          <div className="glass-panel animate-card-in w-full max-w-2xl rounded-[34px] p-6">
            <div className="flex items-start justify-between">
              <div>
                <h2 className="text-lg font-semibold text-white">绑定开发工程</h2>
                <p className="mt-1 break-all text-xs text-slate-500">{inspection.rootPath}</p>
              </div>
              <button
                type="button"
                onClick={() => setDialogOpen(false)}
                className="rounded-lg p-2 text-slate-500 hover:bg-white/5 hover:text-white"
                aria-label="关闭"
              >
                <X size={18} />
              </button>
            </div>

            <div className="mt-6 grid gap-4 md:grid-cols-2">
              <label>
                <span className="text-xs text-slate-500">工程名称</span>
                <input
                  value={name}
                  onChange={(event) => setName(event.currentTarget.value)}
                  className="mt-2 w-full rounded-xl border border-white/10 bg-slate-950/55 px-3.5 py-2.5 text-sm text-white outline-none focus:border-cyan-300/40"
                />
              </label>
              <label>
                <span className="text-xs text-slate-500">Gradle 任务</span>
                <input
                  value={gradleTask}
                  onChange={(event) => setGradleTask(event.currentTarget.value)}
                  className="mt-2 w-full rounded-xl border border-white/10 bg-slate-950/55 px-3.5 py-2.5 text-sm text-white outline-none focus:border-cyan-300/40"
                />
              </label>
              <label className="md:col-span-2">
                <span className="text-xs text-slate-500">输出目录</span>
                <input
                  value={outputDir}
                  onChange={(event) => setOutputDir(event.currentTarget.value)}
                  className="mt-2 w-full rounded-xl border border-white/10 bg-slate-950/55 px-3.5 py-2.5 text-sm text-white outline-none focus:border-cyan-300/40"
                />
              </label>
            </div>

            <div className="mt-5 flex gap-4">
              {[
                ["自动构建", autoBuild, setAutoBuild] as const,
                ["自动部署", autoDeploy, setAutoDeploy] as const,
              ].map(([label, value, setter]) => (
                <button
                  key={label}
                  type="button"
                  onClick={() => setter((current) => !current)}
                  className={[
                    "inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-xs transition",
                    value
                      ? "border-cyan-300/20 bg-cyan-300/10 text-cyan-200"
                      : "border-white/8 bg-white/5 text-slate-500",
                  ].join(" ")}
                >
                  <span className={["grid h-4 w-4 place-items-center rounded", value ? "bg-cyan-300 text-slate-950" : "bg-slate-800"].join(" ")}>
                    {value ? <Check size={11} /> : null}
                  </span>
                  {label}
                </button>
              ))}
            </div>

            <div className="mt-5">
              <div className="text-xs text-slate-500">绑定实例</div>
              <div className="mt-2 grid max-h-40 gap-2 overflow-y-auto md:grid-cols-2">
                {instances.map((instance) => {
                  const selected = selectedInstances.includes(instance.id);
                  return (
                    <button
                      key={instance.id}
                      type="button"
                      onClick={() =>
                        setSelectedInstances((current) =>
                          selected ? current.filter((id) => id !== instance.id) : [...current, instance.id],
                        )
                      }
                      className={[
                        "rounded-xl border px-3 py-2 text-left text-xs transition",
                        selected ? "border-cyan-300/20 bg-cyan-300/8 text-cyan-100" : "border-white/8 bg-slate-950/30 text-slate-500",
                      ].join(" ")}
                    >
                      {instance.name} · {instance.versionId}
                    </button>
                  );
                })}
              </div>
            </div>

            {message ? <p className="mt-4 text-xs text-rose-300">{message}</p> : null}

            <div className="mt-6 flex justify-end gap-3">
              <button
                type="button"
                onClick={() => setDialogOpen(false)}
                className="rounded-xl px-4 py-2.5 text-sm text-slate-400 hover:bg-white/5 hover:text-white"
              >
                取消
              </button>
              <button
                type="button"
                disabled={submitting || !name.trim()}
                onClick={() => void create()}
                className="rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:opacity-40"
              >
                {submitting ? "绑定中..." : "绑定工程"}
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
