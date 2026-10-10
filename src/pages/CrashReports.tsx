import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  BrainCircuit,
  CheckCircle2,
  FileWarning,
  PackageSearch,
  RefreshCw,
  ShieldAlert,
  Trash2,
  UploadCloud,
} from "lucide-react";
import { api, errorMessage } from "../api/tauri";
import type { CrashAnalysisReport, CrashFinding, SuspectMod } from "../api/types";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { useLauncherStore } from "../stores/launcher";
import { formatDate } from "../lib/format";

function severityTone(severity: string) {
  if (severity === "error") return "rose" as const;
  if (severity === "warning") return "amber" as const;
  return "slate" as const;
}

function severityText(severity: string) {
  if (severity === "error") return "严重";
  if (severity === "warning") return "警告";
  return "信息";
}

function FindingCard({ finding }: { finding: CrashFinding }) {
  return (
    <article className="glass-panel rounded-[24px] p-4">
      <div className="flex flex-wrap items-center gap-2">
        <div className={[
          "grid h-9 w-9 place-items-center rounded-xl",
          finding.severity === "error" ? "bg-rose-400/10 text-rose-300" : "bg-amber-300/10 text-amber-200",
        ].join(" ")}>
          {finding.severity === "error" ? <ShieldAlert size={17} /> : <AlertTriangle size={17} />}
        </div>
        <h3 className="font-semibold text-slate-100">{finding.title}</h3>
        <StatusPill tone={severityTone(finding.severity)}>{severityText(finding.severity)}</StatusPill>
        <span className="text-xs text-slate-500">置信度 {finding.confidence}%</span>
      </div>
      <p className="mt-3 text-sm leading-6 text-slate-400">{finding.explanation}</p>
      <div className="mt-3 rounded-2xl border border-white/[0.06] bg-black/10 p-3">
        <div className="text-[10px] font-semibold tracking-[0.14em] text-slate-500">建议</div>
        <p className="mt-1 text-xs leading-5 text-cyan-100/80">{finding.suggestion}</p>
      </div>
      {finding.evidence.length > 0 ? (
        <div className="mt-3 space-y-1.5">
          {finding.evidence.slice(0, 4).map((line, index) => (
            <code key={`${finding.category}-${index}`} className="block overflow-hidden text-ellipsis whitespace-nowrap rounded-lg bg-slate-950/45 px-2.5 py-1.5 text-[11px] text-slate-400">
              {line}
            </code>
          ))}
        </div>
      ) : null}
    </article>
  );
}

function SuspectCard({ suspect, rank }: { suspect: SuspectMod; rank: number }) {
  const confidence = suspect.confidence === "high" ? "高" : suspect.confidence === "medium" ? "中" : "低";
  return (
    <article className="rounded-2xl border border-cyan-300/10 bg-cyan-300/[0.035] p-3">
      <div className="flex items-start gap-3">
        <div className="grid h-8 w-8 shrink-0 place-items-center rounded-xl bg-cyan-300/10 text-sm font-bold text-cyan-200">{rank}</div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="truncate text-sm font-semibold text-slate-100">{suspect.name || suspect.modId}</span>
            <StatusPill tone={suspect.confidence === "high" ? "rose" : suspect.confidence === "medium" ? "amber" : "slate"}>
              {confidence}
            </StatusPill>
          </div>
          <div className="mt-1 text-[11px] text-slate-500">
            {suspect.modId}{suspect.version ? ` · ${suspect.version}` : ""} · 评分 {suspect.score}
          </div>
          {suspect.reasons.length > 0 ? (
            <ul className="mt-2 space-y-1 text-[11px] leading-5 text-slate-400">
              {suspect.reasons.slice(0, 3).map((reason) => <li key={reason}>· {reason}</li>)}
            </ul>
          ) : null}
        </div>
      </div>
    </article>
  );
}
export function CrashReports() {
  const [reports, setReports] = useState<CrashAnalysisReport[]>([]);
  const [selected, setSelected] = useState<CrashAnalysisReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState("");
  const progress = useLauncherStore((state) => state.crashAnalysisProgress);
  const ready = useLauncherStore((state) => state.crashReportReady);
  const clearReady = useLauncherStore((state) => state.clearCrashReportReady);

  const loadReports = async () => {
    try {
      const next = await api.listCrashReports();
      setReports(next);
      if (!selected && next[0]) setSelected(next[0]);
    } catch (error) {
      setMessage(errorMessage(error));
    }
  };

  useEffect(() => {
    void loadReports();
  }, []);

  useEffect(() => {
    if (!ready) return;
    setSelected(ready.report);
    setMessage(`已自动分析实例 ${ready.instanceId} 的崩溃报告。`);
    clearReady();
    void loadReports();
  }, [ready, clearReady]);

  useEffect(() => {
    if (progress?.status === "complete") void loadReports();
  }, [progress?.status]);

  const analyzePath = async (path: string) => {
    setLoading(true);
    setMessage("正在分析错误报告...");
    try {
      const report = await api.analyzeCrashArchive(path, crypto.randomUUID(), selected?.instanceId ?? null);
      setSelected(report);
      setMessage("分析完成。可以查看疑似模组和证据片段。");
      await loadReports();
    } catch (error) {
      setMessage(errorMessage(error));
    } finally {
      setLoading(false);
    }
  };

  const chooseArchive = async () => {
    const path = await open({
      multiple: false,
      filters: [{ name: "错误报告", extensions: ["zip", "log", "txt"] }],
    });
    if (typeof path === "string") await analyzePath(path);
  };

  const removeReport = async (id: string) => {
    try {
      await api.deleteCrashReport(id);
      if (selected?.id === id) setSelected(null);
      await loadReports();
    } catch (error) {
      setMessage(errorMessage(error));
    }
  };

  const running = progress?.status === "running" || loading;
  const report = selected;

  return (
    <div className="animate-page mx-auto max-w-6xl">
      <PageHeader
        title="错误报告分析"
        description="自动导出、离线分析游戏崩溃日志。尝试从堆栈、Mixin、依赖和模组元数据中定位可疑模组。"
        action={
          <button type="button" disabled={running} onClick={() => void chooseArchive()} className="inline-flex items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:opacity-40">
            {running ? <RefreshCw size={16} className="animate-spin" /> : <UploadCloud size={16} />}
            {running ? "分析中..." : "选择错误报告"}
          </button>
        }
      />

      {message ? <div className="mb-4 rounded-2xl border border-cyan-300/10 bg-cyan-300/[0.045] px-4 py-3 text-sm text-cyan-100/80">{message}</div> : null}

      {running && progress ? (
        <div className="mb-5 rounded-[24px] border border-cyan-300/15 bg-cyan-300/[0.045] p-4">
          <div className="flex items-center justify-between gap-3 text-sm">
            <span className="flex items-center gap-2 text-cyan-100"><BrainCircuit size={16} className="animate-pulse" />{progress.message}</span>
            <span className="tabular-nums text-cyan-200">{progress.percent.toFixed(0)}%</span>
          </div>
          <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-slate-950/70">
            <div className="progress-sheen h-full rounded-full bg-gradient-to-r from-cyan-400 to-blue-400 transition-[width] duration-300" style={{ width: `${Math.max(0, Math.min(100, progress.percent))}%` }} />
          </div>
          <div className="mt-2 text-[10px] tracking-[0.14em] text-slate-500">{progress.stage}</div>
        </div>
      ) : null}

      <div className="grid gap-5 lg:grid-cols-[18rem_1fr]">
        <aside className="space-y-3">
          <button
            type="button"
            onClick={() => void chooseArchive()}
            onDragOver={(event) => event.preventDefault()}
            onDrop={(event) => {
              event.preventDefault();
              const file = event.dataTransfer.files[0] as File & { path?: string };
              if (file?.path) void analyzePath(file.path);
            }}
            className="group flex w-full flex-col items-center rounded-[25px] border border-dashed border-cyan-300/20 bg-cyan-300/[0.025] px-4 py-7 text-center transition hover:border-cyan-300/45 hover:bg-cyan-300/[0.06]"
          >
            <div className="grid h-12 w-12 place-items-center rounded-2xl bg-cyan-300/10 text-cyan-200 transition group-hover:scale-105"><UploadCloud size={22} /></div>
            <div className="mt-3 text-sm font-semibold text-slate-200">拖入或选择错误报告</div>
            <div className="mt-1 text-[11px] leading-5 text-slate-500">支持 `.zip`、`latest.log` 和崩溃报告 `.txt`</div>
          </button>

          <div className="glass-panel rounded-[25px] p-3">
            <div className="mb-2 flex items-center justify-between px-1">
              <span className="text-xs font-semibold tracking-[0.12em] text-slate-400">最近分析</span>
              <button type="button" onClick={() => void loadReports()} className="rounded-lg p-1 text-slate-500 transition hover:bg-white/5 hover:text-slate-200" aria-label="刷新报告列表"><RefreshCw size={13} /></button>
            </div>
            {reports.length === 0 ? (
              <p className="px-2 py-5 text-center text-xs leading-5 text-slate-600">还没有分析记录</p>
            ) : (
              <div className="max-h-[36rem] space-y-1.5 overflow-y-auto pr-0.5">
                {reports.map((item) => (
                  <button key={item.id} type="button" onClick={() => setSelected(item)} className={["group flex w-full items-start gap-2 rounded-2xl px-2.5 py-2.5 text-left transition", selected?.id === item.id ? "bg-cyan-300/10 text-cyan-100" : "text-slate-400 hover:bg-white/[0.04] hover:text-slate-200"].join(" ")}>
                    <FileWarning size={15} className="mt-0.5 shrink-0" />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-xs font-medium">{item.instanceName || item.summary}</span>
                      <span className="mt-1 block truncate text-[10px] text-slate-600">{formatDate(item.createdAt)} · {item.source === "auto" ? "自动" : "手动"}</span>
                    </span>
                    {item.severity === "error" ? <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-rose-300" /> : null}
                  </button>
                ))}
              </div>
            )}
          </div>
        </aside>

        <main className="min-w-0">
          {!report ? (
            <div className="glass-panel flex min-h-[28rem] flex-col items-center justify-center rounded-[30px] px-8 text-center">
              <PackageSearch size={34} className="text-cyan-300/50" />
              <h2 className="mt-4 text-sm font-semibold text-slate-300">等待错误报告</h2>
              <p className="mt-2 max-w-md text-xs leading-6 text-slate-500">游戏异常退出时，EMCL 会自动导出并分析日志。你也可以在左侧拖入或选择错误报告压缩包。</p>
            </div>
          ) : (
            <ReportView report={report} onDelete={() => void removeReport(report.id)} />
          )}
        </main>
      </div>
    </div>
  );
}
function ReportView({ report, onDelete }: { report: CrashAnalysisReport; onDelete: () => void }) {
  const confidence = report.confidence >= 80 ? "高" : report.confidence >= 60 ? "中" : "低";
  return (
    <div className="space-y-5">
      <section className="relative overflow-hidden rounded-[30px] border border-white/[0.07] bg-gradient-to-br from-slate-900/80 to-slate-950/65 p-5 shadow-[0_20px_60px_rgba(0,0,0,0.25)]">
        <div className="pointer-events-none absolute -right-20 -top-24 h-52 w-52 rounded-full bg-rose-400/[0.08] blur-3xl" />
        <div className="relative flex flex-wrap items-start justify-between gap-4">
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <StatusPill tone={severityTone(report.severity)}>{severityText(report.severity)}</StatusPill>
              <StatusPill tone="cyan">{report.source === "auto" ? "自动分析" : "手动分析"}</StatusPill>
              <span className="text-[11px] text-slate-500">报告置信度 {report.confidence}% · {confidence}</span>
            </div>
            <h2 className="mt-3 text-xl font-semibold tracking-tight text-slate-100">{report.summary}</h2>
            <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-[11px] text-slate-500">
              <span>{report.instanceName || "独立错误报告"}</span>
              <span>{formatDate(report.createdAt)}</span>
              <span>已分析 {report.analyzedLines.toLocaleString()} 行</span>
            </div>
          </div>
          <button type="button" onClick={onDelete} className="grid h-9 w-9 place-items-center rounded-xl border border-rose-300/10 text-rose-300/70 transition hover:bg-rose-300/10 hover:text-rose-200" aria-label="删除分析记录"><Trash2 size={15} /></button>
        </div>

        <div className="relative mt-5 grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
          {[
            ["Minecraft", report.game.minecraftVersion || "未知"],
            ["加载器", report.game.loader ? `${report.game.loader} ${report.game.loaderVersion || ""}`.trim() : "原版"],
            ["Java", report.game.javaVersion || "未识别"],
            ["系统", report.game.operatingSystem || "未识别"],
          ].map(([label, value]) => (
            <div key={label} className="rounded-2xl border border-white/[0.055] bg-white/[0.025] px-3 py-2.5">
              <div className="text-[9px] font-semibold tracking-[0.14em] text-slate-600">{label}</div>
              <div className="mt-1 truncate text-xs text-slate-300" title={value}>{value}</div>
            </div>
          ))}
        </div>
      </section>

      <section className="glass-panel rounded-[28px] p-5">
        <div className="flex items-center gap-2">
          <BrainCircuit size={17} className="text-cyan-300" />
          <h2 className="text-sm font-semibold text-slate-200">疑似导致崩溃的模组</h2>
          <span className="text-[10px] text-slate-600">按堆栈命中与异常距离排序</span>
        </div>
        {report.suspectedMods.length === 0 ? (
          <div className="mt-4 rounded-2xl border border-white/[0.05] bg-black/10 px-4 py-5 text-center text-xs leading-5 text-slate-500">没有从现有证据中定位到明确的模组。可以尝试提供更完整的 `latest.log` 和 `crash-report.txt`。</div>
        ) : (
          <div className="mt-4 grid gap-2.5 md:grid-cols-2">
            {report.suspectedMods.map((suspect, index) => <SuspectCard key={`${suspect.modId}-${index}`} suspect={suspect} rank={index + 1} />)}
          </div>
        )}
      </section>

      <section className="glass-panel rounded-[28px] p-5">
        <div className="flex items-center gap-2">
          <AlertTriangle size={17} className="text-amber-200" />
          <h2 className="text-sm font-semibold text-slate-200">多维度分析结果</h2>
          <span className="text-[10px] text-slate-600">{report.findings.length} 项命中</span>
        </div>
        <div className="mt-4 space-y-3">
          {report.findings.map((finding, index) => <FindingCard key={`${finding.category}-${index}`} finding={finding} />)}
        </div>
      </section>

      <section className="glass-panel rounded-[28px] p-5">
        <div className="flex items-center gap-2">
          <CheckCircle2 size={17} className="text-emerald-300" />
          <h2 className="text-sm font-semibold text-slate-200">修复建议</h2>
        </div>
        <ol className="mt-4 space-y-2.5">
          {report.recommendations.map((recommendation, index) => (
            <li key={`${index}-${recommendation}`} className="flex gap-3 rounded-2xl border border-white/[0.05] bg-white/[0.02] px-3 py-3 text-xs leading-5 text-slate-300">
              <span className="grid h-5 w-5 shrink-0 place-items-center rounded-lg bg-emerald-300/10 text-[10px] font-bold text-emerald-200">{index + 1}</span>
              {recommendation}
            </li>
          ))}
        </ol>
      </section>

      <details className="glass-panel rounded-[28px] p-5">
        <summary className="cursor-pointer text-sm font-semibold text-slate-300">查看关键日志证据与包含文件</summary>
        <div className="mt-4 grid gap-4 lg:grid-cols-[1fr_16rem]">
          <pre className="max-h-80 overflow-auto rounded-2xl bg-slate-950/55 p-4 text-[11px] leading-5 text-slate-400">{report.evidence.join("\n") || "没有可展示的证据行"}</pre>
          <div className="rounded-2xl border border-white/[0.05] p-3">
            <div className="text-[10px] font-semibold tracking-[0.14em] text-slate-600">包含文件</div>
            <div className="mt-2 max-h-72 space-y-1 overflow-y-auto">
              {report.includedFiles.map((file) => <div key={file} className="truncate text-[11px] text-slate-500" title={file}>{file}</div>)}
            </div>
          </div>
        </div>
      </details>
    </div>
  );
}
