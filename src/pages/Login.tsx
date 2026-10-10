import { useState } from "react";
import {
  Check,
  LogIn,
  Plus,
  ShieldCheck,
  Trash2,
  UserRound,
} from "lucide-react";
import { PageHeader } from "../components/PageHeader";
import { StatusPill } from "../components/StatusPill";
import { formatDate } from "../lib/format";
import { useLauncherStore } from "../stores/launcher";

export function Login() {
  const accounts = useLauncherStore((state) => state.accounts);
  const createOfflineAccount = useLauncherStore((state) => state.createOfflineAccount);
  const setActiveAccount = useLauncherStore((state) => state.setActiveAccount);
  const deleteAccount = useLauncherStore((state) => state.deleteAccount);
  const [username, setUsername] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const createOffline = async () => {
    if (!username.trim()) return;
    setSubmitting(true);
    try {
      await createOfflineAccount(username.trim());
      setUsername("");
    } catch {
      // The shared error banner owns the message.
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="animate-page mx-auto max-w-4xl">
      <PageHeader
        title="账户"
        description="使用离线账户进行单人游戏和测试，或接入微软正版账户。账户数据只写入本机 SQLite，不会上传到任何第三方服务。"
      />

      <div className="grid gap-4 md:grid-cols-[1.1fr_0.9fr]">
        <section className="glass-panel rounded-[28px] p-6">
          <div className="grid h-11 w-11 place-items-center rounded-2xl bg-cyan-300/10 text-cyan-200">
            <UserRound size={21} />
          </div>
          <h2 className="mt-4 text-base font-semibold text-white">离线登录</h2>
          <p className="mt-2 text-sm leading-6 text-slate-400">
            输入用户名后按 Minecraft 标准生成离线 UUID，可直接用于单人游戏。
          </p>
          <div className="mt-5 flex gap-3">
            <input
              value={username}
              onChange={(event) => setUsername(event.currentTarget.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") void createOffline();
              }}
              maxLength={16}
              placeholder="输入用户名"
              className="min-w-0 flex-1 rounded-xl border border-white/10 bg-slate-950/55 px-3.5 py-2.5 text-sm text-white outline-none placeholder:text-slate-700 focus:border-cyan-300/40"
            />
            <button
              type="button"
              disabled={submitting || !username.trim()}
              onClick={() => void createOffline()}
              className="inline-flex shrink-0 items-center gap-2 rounded-xl bg-cyan-400 px-4 py-2.5 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:opacity-40"
            >
              <Plus size={16} />
              {submitting ? "创建中" : "创建"}
            </button>
          </div>
        </section>

        <section className="glass-panel rounded-[28px] p-6">
          <div className="grid h-11 w-11 place-items-center rounded-2xl bg-slate-800 text-slate-400">
            <ShieldCheck size={21} />
          </div>
          <h2 className="mt-4 text-base font-semibold text-white">微软登录</h2>
          <p className="mt-2 text-sm leading-6 text-slate-400">
            通过设备码流程登录正版账户，并把刷新令牌保存在本机。
          </p>
          <button
            type="button"
            disabled
            className="mt-5 inline-flex items-center gap-2 rounded-xl border border-white/8 bg-white/5 px-4 py-2.5 text-sm text-slate-500"
          >
            <LogIn size={16} />
            第 8 步接入
          </button>
        </section>
      </div>

      <section className="glass-panel mt-5 rounded-[28px] p-6">
        <div className="flex items-center justify-between gap-4">
          <div>
            <h2 className="text-base font-semibold text-white">本机账户</h2>
            <p className="mt-1 text-xs text-slate-500">当前启动游戏时使用选中的账户。</p>
          </div>
          <StatusPill tone="slate">{`${accounts.length} 个`}</StatusPill>
        </div>

        {accounts.length === 0 ? (
          <div className="mt-5 rounded-2xl border border-dashed border-white/10 bg-slate-950/25 px-4 py-8 text-center text-sm text-slate-600">
            还没有账户，先创建一个离线账户。
          </div>
        ) : (
          <div className="mt-5 space-y-3">
            {accounts.map((account) => (
              <div
                key={account.id}
                className={[
                  "flex items-center gap-4 rounded-2xl border px-4 py-3.5 transition",
                  account.active
                    ? "border-cyan-300/20 bg-cyan-300/[0.055]"
                    : "border-white/7 bg-slate-950/25",
                ].join(" ")}
              >
                <div className="grid h-10 w-10 shrink-0 place-items-center rounded-2xl bg-slate-800 text-slate-400">
                  {account.accountType === "offline" ? (
                    <UserRound size={18} />
                  ) : (
                    <ShieldCheck size={18} />
                  )}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="truncate text-sm font-medium text-slate-100">
                      {account.username}
                    </span>
                    {account.active ? <StatusPill tone="emerald">当前使用</StatusPill> : null}
                  </div>
                  <div className="mt-1 truncate font-mono text-[11px] text-slate-600">
                    {account.uuid} · {formatDate(account.createdAt)}
                  </div>
                </div>
                {!account.active ? (
                  <button
                    type="button"
                    onClick={() => void setActiveAccount(account.id)}
                    className="rounded-lg p-2 text-slate-500 transition hover:bg-cyan-300/10 hover:text-cyan-200"
                    aria-label="使用这个账户"
                  >
                    <Check size={16} />
                  </button>
                ) : null}
                <button
                  type="button"
                  onClick={() => void deleteAccount(account.id)}
                  className="rounded-lg p-2 text-slate-600 transition hover:bg-rose-400/10 hover:text-rose-300"
                  aria-label="删除账户"
                >
                  <Trash2 size={16} />
                </button>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
