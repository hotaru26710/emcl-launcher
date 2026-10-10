import {
  Boxes,
  Gamepad2,
  LogIn,
  Settings,
  Waves,
} from "lucide-react";
import { NavLink } from "react-router-dom";

const navigation = [
  { to: "/instances", label: "实例", hint: "INSTANCES", icon: Gamepad2 },
  { to: "/versions", label: "版本", hint: "VERSIONS", icon: Boxes },
  { to: "/settings", label: "设置", hint: "SETTINGS", icon: Settings },
  { to: "/login", label: "账户", hint: "ACCOUNTS", icon: LogIn },
];

export function Sidebar() {
  return (
    <aside className="relative z-20 w-[17.5rem] shrink-0 p-3 pr-0">
      <div className="glass-panel flex h-full flex-col rounded-[30px] px-3 py-4">
        <div className="group flex items-center gap-3 rounded-3xl border border-white/[0.06] bg-white/[0.025] px-3 py-3">
          <div className="grid h-11 w-11 place-items-center rounded-2xl border border-cyan-300/20 bg-cyan-400/10 text-cyan-200 shadow-[0_0_35px_rgba(34,211,238,0.13)] transition duration-500 group-hover:rotate-3 group-hover:scale-105">
            <Waves size={22} />
          </div>
          <div>
            <div className="text-base font-semibold tracking-[0.08em] text-white">MCL</div>
            <div className="text-[9px] font-medium tracking-[0.25em] text-slate-600">
              LAUNCHER
            </div>
          </div>
        </div>

        <nav className="mt-7 space-y-1.5">
          {navigation.map((item) => {
            const Icon = item.icon;
            return (
              <NavLink
                key={item.to}
                to={item.to}
                className={({ isActive }) =>
                  [
                    "group relative flex items-center gap-3 overflow-hidden rounded-2xl px-3.5 py-3 text-sm transition-all duration-300",
                    isActive
                      ? "bg-gradient-to-r from-cyan-400/[0.13] to-blue-400/[0.035] text-cyan-100 shadow-[inset_0_0_0_1px_rgba(34,211,238,0.12)]"
                      : "text-slate-500 hover:translate-x-0.5 hover:bg-white/[0.045] hover:text-slate-100",
                  ].join(" ")
                }
              >
                {({ isActive }) => (
                  <>
                    <span
                      className={[
                        "absolute bottom-2 left-0 top-2 w-0.5 rounded-full bg-cyan-300 transition-all duration-300",
                        isActive ? "opacity-100" : "opacity-0 group-hover:opacity-40",
                      ].join(" ")}
                    />
                    <Icon
                      size={18}
                      className="transition-transform duration-300 group-hover:scale-110"
                    />
                    <span className="flex-1 font-medium">{item.label}</span>
                    <span className="text-[9px] tracking-[0.16em] text-slate-700 transition group-hover:text-slate-500">
                      {item.hint}
                    </span>
                  </>
                )}
              </NavLink>
            );
          })}
        </nav>

        <div className="mt-auto rounded-3xl border border-white/[0.06] bg-gradient-to-br from-white/[0.04] to-transparent p-4">
          <div className="flex items-center gap-2 text-xs font-medium text-slate-400">
            <span className="animate-soft-pulse h-2 w-2 rounded-full bg-emerald-400 shadow-[0_0_12px_rgba(52,211,153,0.8)]" />
            本地模式
          </div>
          <p className="mt-2 text-[11px] leading-5 text-slate-600">
            账户、实例与下载记录都留在本机。
          </p>
          <div className="mt-3 h-1 overflow-hidden rounded-full bg-slate-950/70">
            <div className="progress-sheen h-full w-2/3 rounded-full bg-gradient-to-r from-emerald-400/70 to-cyan-300/70" />
          </div>
        </div>
      </div>
    </aside>
  );
}
