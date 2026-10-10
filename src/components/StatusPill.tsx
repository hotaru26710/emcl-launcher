interface StatusPillProps {
  children: string;
  tone?: "cyan" | "emerald" | "amber" | "slate" | "rose";
}

export function StatusPill({ children, tone = "slate" }: StatusPillProps) {
  const colors = {
    cyan: "border-cyan-300/20 bg-cyan-300/10 text-cyan-200 shadow-[0_0_18px_rgba(34,211,238,0.08)]",
    emerald:
      "border-emerald-300/20 bg-emerald-300/10 text-emerald-200 shadow-[0_0_18px_rgba(52,211,153,0.08)]",
    amber: "border-amber-300/20 bg-amber-300/10 text-amber-100",
    slate: "border-white/10 bg-white/5 text-slate-300",
    rose: "border-rose-300/20 bg-rose-300/10 text-rose-200",
  };
  return (
    <span
      className={`inline-flex rounded-full border px-2.5 py-1 text-[10px] font-medium tracking-wide ${colors[tone]}`}
    >
      {children}
    </span>
  );
}
