import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";

export function Layout() {
  return (
    <div className="relative flex h-screen overflow-hidden bg-transparent text-slate-100">
      <div className="pointer-events-none fixed inset-0 overflow-hidden">
        <div className="animate-drift absolute -right-24 -top-28 h-96 w-96 rounded-full bg-cyan-400/[0.075] blur-3xl" />
        <div
          className="animate-drift absolute -bottom-40 left-36 h-[28rem] w-[28rem] rounded-full bg-blue-500/[0.07] blur-3xl"
          style={{ animationDelay: "-6s" }}
        />
        <div className="absolute inset-0 bg-[linear-gradient(rgba(148,163,184,0.025)_1px,transparent_1px),linear-gradient(90deg,rgba(148,163,184,0.025)_1px,transparent_1px)] bg-[size:36px_36px] [mask-image:linear-gradient(to_bottom,black,transparent_88%)]" />
      </div>

      <Sidebar />
      <main className="relative z-10 flex min-w-0 flex-1 flex-col">
        <header className="flex h-16 shrink-0 items-center justify-between border-b border-white/[0.055] bg-[#07111f]/62 px-7 backdrop-blur-xl">
          <div className="flex items-center gap-3">
            <span className="animate-soft-pulse h-1.5 w-1.5 rounded-full bg-cyan-300 shadow-[0_0_12px_rgba(34,211,238,0.9)]" />
            <div className="text-[10px] font-semibold tracking-[0.22em] text-slate-500">
              MINECRAFT LOCAL WORKSPACE
            </div>
          </div>
          <div className="rounded-full border border-white/[0.07] bg-white/[0.035] px-3 py-1.5 text-[10px] tracking-[0.16em] text-slate-500">
            TAURI 2
          </div>
        </header>
        <div className="min-h-0 flex-1 overflow-y-auto p-6 lg:p-8">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
