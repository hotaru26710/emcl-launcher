import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description: string;
  action?: ReactNode;
}

export function EmptyState({ icon: Icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="animate-card-in glass-panel grid min-h-80 place-items-center rounded-[30px] border-dashed p-8 text-center">
      <div>
        <div className="mx-auto grid h-14 w-14 place-items-center rounded-3xl border border-white/[0.07] bg-gradient-to-br from-slate-800 to-slate-900 text-slate-400 shadow-xl transition duration-500 hover:-rotate-3 hover:scale-105">
          <Icon size={24} />
        </div>
        <h2 className="mt-5 text-base font-semibold text-slate-200">{title}</h2>
        <p className="mx-auto mt-2 max-w-md text-sm leading-6 text-slate-500">{description}</p>
        {action ? <div className="mt-6">{action}</div> : null}
      </div>
    </div>
  );
}
