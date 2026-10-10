import { LogIn, ShieldCheck, UserRound } from "lucide-react";
import { PageHeader } from "../components/PageHeader";

export function Login() {
  return (
    <div className="animate-page mx-auto max-w-4xl">
      <PageHeader
        title="账户"
        description="支持正版与离线账户。账户数据只写入本机 SQLite，不会上传到任何第三方服务。"
      />
      <div className="grid gap-4 md:grid-cols-2">
        <div className="glass-panel hover-lift rounded-[28px] border-cyan-300/15 p-6">
          <div className="grid h-11 w-11 place-items-center rounded-2xl bg-cyan-300/10 text-cyan-200">
            <UserRound size={21} />
          </div>
          <h2 className="mt-4 text-base font-semibold text-white">离线登录</h2>
          <p className="mt-2 text-sm leading-6 text-slate-400">
            输入用户名后生成本地 UUID，用于单人游戏和离线测试。
          </p>
          <button
            type="button"
            disabled
            className="mt-5 inline-flex items-center gap-2 rounded-xl border border-white/8 bg-white/5 px-4 py-2.5 text-sm text-slate-500"
          >
            <LogIn size={16} />
            第 6 步接入
          </button>
        </div>
        <div className="glass-panel hover-lift rounded-[28px] p-6">
          <div className="grid h-11 w-11 place-items-center rounded-2xl bg-slate-800 text-slate-400">
            <ShieldCheck size={21} />
          </div>
          <h2 className="mt-4 text-base font-semibold text-white">微软登录</h2>
          <p className="mt-2 text-sm leading-6 text-slate-400">
            通过设备码流程登录正版账户，并自动刷新访问令牌。
          </p>
          <button
            type="button"
            disabled
            className="mt-5 inline-flex items-center gap-2 rounded-xl border border-white/8 bg-white/5 px-4 py-2.5 text-sm text-slate-500"
          >
            <LogIn size={16} />
            第 8 步接入
          </button>
        </div>
      </div>
    </div>
  );
}
