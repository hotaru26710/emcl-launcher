import type { LoaderType } from "../api/types";
import { Box, Flame, Hammer, Layers, Sparkles, Wrench } from "lucide-react";

export type LoaderChoice = "vanilla" | LoaderType | "optifine";

const config: Record<LoaderChoice, { label: string; sub: string; className: string; icon: typeof Box }> = {
  vanilla: {
    label: "V",
    sub: "Vanilla",
    className: "from-slate-500/30 to-slate-700/30 text-slate-200 border-slate-300/15",
    icon: Layers,
  },
  fabric: {
    label: "F",
    sub: "Fabric",
    className: "from-cyan-300/20 to-blue-500/10 text-cyan-200 border-cyan-300/20",
    icon: Box,
  },
  quilt: {
    label: "Q",
    sub: "Quilt",
    className: "from-fuchsia-300/20 to-purple-500/10 text-fuchsia-200 border-fuchsia-300/20",
    icon: Sparkles,
  },
  neoforge: {
    label: "N",
    sub: "NeoForge",
    className: "from-orange-300/20 to-amber-500/10 text-orange-200 border-orange-300/20",
    icon: Flame,
  },
  forge: {
    label: "⚒",
    sub: "Forge",
    className: "from-red-300/20 to-orange-500/10 text-orange-200 border-orange-300/20",
    icon: Hammer,
  },
  optifine: {
    label: "O",
    sub: "OptiFine",
    className: "from-emerald-300/20 to-lime-500/10 text-emerald-200 border-emerald-300/20",
    icon: Wrench,
  },
};

export function LoaderIcon({ loader, size = "md" }: { loader: LoaderChoice; size?: "sm" | "md" | "lg" }) {
  const item = config[loader];
  const Icon = item.icon;
  const dimensions = {
    sm: "h-9 w-9 rounded-xl",
    md: "h-12 w-12 rounded-2xl",
    lg: "h-14 w-14 rounded-2xl",
  }[size];
  const iconSize = size === "sm" ? 15 : size === "lg" ? 21 : 18;
  return (
    <div className={`grid shrink-0 place-items-center border bg-gradient-to-br ${dimensions} ${item.className}`}>
      <div className="relative grid place-items-center">
        <Icon size={iconSize} />
        <span className="absolute -bottom-3 rounded-full bg-slate-950/90 px-1.5 py-0.5 text-[8px] font-bold leading-none">
          {item.label}
        </span>
      </div>
    </div>
  );
}

export function loaderLabel(loader: LoaderChoice): string {
  return config[loader].sub;
}
