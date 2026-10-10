import { useEffect, useState } from "react";
import { AlertTriangle, ArrowLeft, Download, RefreshCw, ShieldAlert } from "lucide-react";
import { Link, useParams } from "react-router-dom";
import { api, errorMessage } from "../api/tauri";
import type { ConflictReport as ConflictReportType } from "../api/types";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { formatDate } from "../lib/format";
import { useLauncherStore } from "../stores/launcher";

export function ConflictReport() {
  const { id } = useParams<{ id: string }>();
  const [reports, setReports] = useState<ConflictReportType[]>([]);
  const [loading, setLoading] = useState(false);
  const [repairing, setRepairing] = useState(false);
  const [message, setMessage] = useState("");
  const repairDependencies = useLauncherStore((state) => state.repairInstanceDependencies);

  const refresh = async () => {
    if (!id) return;
    setLoading(true);
    try {
      setReports(await api.listConflictReports(id));
    } catch (error) {
      setMessage(errorMessage(error));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
  }, [id]);

  const repair = async () => {
    if (!id) return;
    if (!window.confirm("将从 Modrinth 或 MCIM 镜像查找并安装缺失的必需依赖，已存在的文件会跳过。是否继续？")) return;
    setRepairing(true);
    setMessage("");
    try {
      const result = await repairDependencies(id);
      const installed = result.installed.length ? `已补全：${result.installed.join("、")}` : "没有自动安装新依赖";
      const failed = result.failed.length ? `；未解决：${result.failed.join("；")}` : "";
      setMessage(`${installed}${failed}`);
      await refresh();
    } catch (error) {
      setMessage(errorMessage(error));
    } finally {
      setRepairing(false);
    }
  };

  return (
    <div className="animate-page mx-auto max-w-5xl">
      <Link to={id ? `/instances/${id}` : "/instances"} className="mb-5 inline-flex items-center gap-2 text-sm text-slate-500 hover:text-slate-200">
        <ArrowLeft size={16} />
        返回实例详情
      </Link>
      <PageHeader
        title="冲突报告"
        description="只做本地静态元数据比对，不删除模组，也不做运行时拦截。"
        action={
          <div className="flex gap-2">
            <button
              type="button"
              disabled={repairing}
              onClick={() => void repair()}
              className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:opacity-40"
            >
              <Download size={16} />
              {repairing ? "补全中..." : "自动补全缺失依赖"}
            </button>
            <button
              type="button"
              disabled={loading}
              onClick={() => void refresh()}
              className="inline-flex items-center gap-2 rounded-xl border border-white/10 bg-white/5 px-4 py-2.5 text-sm text-slate-300 transition hover:bg-white/8 disabled:opacity-40"
            >
              <RefreshCw size={16} className={loading ? "animate-spin" : ""} />
              刷新
            </button>
          </div>
        }
      />

      {message ? <div className="mb-4 rounded-2xl border border-rose-300/15 bg-rose-300/[0.06] px-4 py-3 text-sm text-rose-200">{message}</div> : null}

      {reports.length === 0 ? (
        <div className="glass-panel rounded-[30px] px-5 py-12 text-center text-sm text-slate-600">
          暂无冲突报告。启动或部署模组时会自动生成。
        </div>
      ) : (
        <div className="space-y-3">
          {reports.map((report) => (
            <div key={report.id} className="glass-panel rounded-[24px] p-4">
              <div className="flex items-start gap-3">
                <div className={[
                  "grid h-10 w-10 shrink-0 place-items-center rounded-2xl",
                  report.severity === "error" ? "bg-rose-400/10 text-rose-300" : "bg-amber-300/10 text-amber-200",
                ].join(" ")}>
                  {report.severity === "error" ? <ShieldAlert size={18} /> : <AlertTriangle size={18} />}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="text-sm font-semibold text-slate-100">{report.detail}</span>
                    <StatusPill tone={report.severity === "error" ? "rose" : "amber"}>{report.severity}</StatusPill>
                    <StatusPill tone="slate">{report.conflictType}</StatusPill>
                  </div>
                  <p className="mt-2 text-xs leading-5 text-slate-500">{report.suggestion}</p>
                  <div className="mt-2 text-[10px] text-slate-600">
                    {[report.sourceModId, report.targetModId].filter(Boolean).join(" → ")} · {formatDate(report.createdAt)}
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
