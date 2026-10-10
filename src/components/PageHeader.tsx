import type { ReactNode } from "react";

interface PageHeaderProps {
  title: string;
  description?: string;
  action?: ReactNode;
}

export function PageHeader({ title, description, action }: PageHeaderProps) {
  return (
    <div className="mb-7 flex items-start justify-between gap-6">
      <div className="min-w-0">
        <div className="mb-2 h-1 w-10 rounded-full bg-gradient-to-r from-cyan-300 to-blue-400 shadow-[0_0_18px_rgba(34,211,238,0.4)]" />
        <h1 className="text-[28px] font-semibold tracking-[-0.025em] text-white">{title}</h1>
        {description ? (
          <p className="mt-2 max-w-3xl text-sm leading-6 text-slate-400">{description}</p>
        ) : null}
      </div>
      {action ? <div className="shrink-0 pt-2">{action}</div> : null}
    </div>
  );
}
