import { useState } from "react";
import {
  Boxes,
  ChevronRight,
  HardDrive,
  MemoryStick,
  Plus,
  Trash2,
} from "lucide-react";
import { Link, useNavigate } from "react-router-dom";
import { EmptyState } from "../components/EmptyState";
import { LoaderIcon } from "../components/LoaderIcon";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { formatDate } from "../lib/format";
import { useLauncherStore } from "../stores/launcher";

export function Instances() {
  const navigate = useNavigate();
  const instances = useLauncherStore((state) => state.instances);
  const loading = useLauncherStore((state) => state.loadingInstances);
  const deleteInstance = useLauncherStore((state) => state.deleteInstance);
  const [deleting, setDeleting] = useState<string | null>(null);

  const handleDelete = async (id: string, instanceName: string) => {
    if (!window.confirm(`确定删除实例“${instanceName}”和它的本地目录吗？`)) return;
    setDeleting(id);
    try {
      await deleteInstance(id);
    } catch {
      // The shared error banner owns the message.
    } finally {
      setDeleting(null);
    }
  };

  return (
    <div className="animate-page mx-auto max-w-6xl">
      <PageHeader
        title="游戏实例"
        description="先选择游戏版本，再配置加载器和附加组件。每个实例拥有独立的 mods、config、saves 和日志目录。"
        action={
          <button
            type="button"
            onClick={() => navigate("/instances/create")}
            className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300"
          >
            <Plus size={17} />
            新建实例
          </button>
        }
      />

      {instances.length === 0 && !loading ? (
        <EmptyState
          icon={Boxes}
          title="还没有游戏实例"
          description="进入分步创建流程，先选择 Minecraft 版本，再选择加载器和版本。"
          action={
            <button
              type="button"
              onClick={() => navigate("/instances/create")}
              className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300"
            >
              <Plus size={17} />
              创建第一个实例
            </button>
          }
        />
      ) : (
        <div className="grid gap-4 xl:grid-cols-2">
          {instances.map((instance) => {
            const loader = (instance.loader ?? "vanilla") as "vanilla" | "fabric" | "quilt" | "neoforge" | "forge";
            return (
              <div key={instance.id} className="glass-panel hover-lift group rounded-[28px] p-5">
                <div className="flex items-start gap-4">
                  <LoaderIcon loader={loader} />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <h2 className="truncate text-base font-semibold text-white">{instance.name}</h2>
                      <StatusPill tone={instance.loader ? "cyan" : "slate"}>
                        {instance.loader ? instance.loader.toUpperCase() : "VANILLA"}
                      </StatusPill>
                    </div>
                    <div className="mt-2 flex flex-wrap items-center gap-3 text-xs text-slate-500">
                      <span className="inline-flex items-center gap-1.5">
                        <Boxes size={14} />
                        {instance.versionId}
                      </span>
                      <span className="inline-flex items-center gap-1.5">
                        <MemoryStick size={14} />
                        {instance.memoryMb} MB
                      </span>
                      <span>更新于 {formatDate(instance.updatedAt)}</span>
                    </div>
                  </div>
                  <button
                    type="button"
                    disabled={deleting === instance.id}
                    onClick={() => void handleDelete(instance.id, instance.name)}
                    className="rounded-lg p-2 text-slate-600 opacity-0 transition hover:bg-rose-400/10 hover:text-rose-300 group-hover:opacity-100 disabled:opacity-30"
                    aria-label={`删除 ${instance.name}`}
                  >
                    <Trash2 size={16} />
                  </button>
                </div>

                <div className="mt-4 flex items-center gap-2 rounded-xl border border-white/6 bg-slate-950/35 px-3 py-2 text-xs text-slate-500">
                  <HardDrive size={14} className="shrink-0" />
                  <span className="truncate">{instance.directory}</span>
                </div>

                <Link
                  to={`/instances/${instance.id}`}
                  className="mt-4 inline-flex items-center gap-1 text-sm font-medium text-cyan-300 transition hover:text-cyan-200"
                >
                  打开实例
                  <ChevronRight size={16} />
                </Link>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
