import { useEffect, useState } from "react";
import type { CustomThemeColors, ThemeMode } from "../api/types";
import { useLauncherStore } from "../stores/launcher";

const presets: Record<Exclude<ThemeMode, "system" | "custom">, CustomThemeColors & { panelSoft: string; border: string; control: string }> = {
  dark: {
    accent: "#22d3ee",
    background: "#07111f",
    panel: "#0b1929",
    panelSoft: "rgba(15, 23, 42, 0.72)",
    text: "#e2e8f0",
    muted: "#94a3b8",
    border: "rgba(255, 255, 255, 0.09)",
    control: "rgba(15, 23, 42, 0.55)",
  },
  light: {
    accent: "#0891b2",
    background: "#eef4f8",
    panel: "rgba(255, 255, 255, 0.9)",
    panelSoft: "rgba(255, 255, 255, 0.74)",
    text: "#1e293b",
    muted: "#64748b",
    border: "rgba(15, 23, 42, 0.12)",
    control: "rgba(255, 255, 255, 0.78)",
  },
  pink: {
    accent: "#ec4899",
    background: "#fff1f6",
    panel: "rgba(255, 255, 255, 0.92)",
    panelSoft: "rgba(255, 255, 255, 0.76)",
    text: "#4a1028",
    muted: "#9d5774",
    border: "rgba(190, 24, 93, 0.14)",
    control: "rgba(255, 255, 255, 0.78)",
  },
  ocean: {
    accent: "#38bdf8",
    background: "#061827",
    panel: "#0a263a",
    panelSoft: "rgba(8, 47, 73, 0.72)",
    text: "#e0f2fe",
    muted: "#7dd3fc",
    border: "rgba(56, 189, 248, 0.16)",
    control: "rgba(8, 47, 73, 0.58)",
  },
  forest: {
    accent: "#34d399",
    background: "#071710",
    panel: "#0d2419",
    panelSoft: "rgba(6, 78, 59, 0.38)",
    text: "#dcfce7",
    muted: "#86efac",
    border: "rgba(52, 211, 153, 0.16)",
    control: "rgba(6, 78, 59, 0.36)",
  },
  amber: {
    accent: "#f59e0b",
    background: "#1b1206",
    panel: "#2a1b09",
    panelSoft: "rgba(120, 53, 15, 0.34)",
    text: "#fef3c7",
    muted: "#fbbf24",
    border: "rgba(245, 158, 11, 0.18)",
    control: "rgba(120, 53, 15, 0.34)",
  },
};

function parseCustom(json: string | null | undefined): Partial<CustomThemeColors> {
  if (!json) return {};
  try {
    return JSON.parse(json) as Partial<CustomThemeColors>;
  } catch {
    return {};
  }
}

export function ThemeManager() {
  const settings = useLauncherStore((state) => state.settings);
  const [systemTick, setSystemTick] = useState(0);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const listener = () => setSystemTick((value) => value + 1);
    media.addEventListener("change", listener);
    return () => media.removeEventListener("change", listener);
  }, []);

  useEffect(() => {
    const root = document.documentElement;
    const mode = settings?.theme ?? "system";
    const systemDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    const resolved = mode === "system" ? (systemDark ? "dark" : "light") : mode;

    let colors: CustomThemeColors & { panelSoft: string; border: string; control: string };
    if (resolved === "custom") {
      const custom = parseCustom(settings?.customTheme);
      colors = {
        accent: custom.accent ?? presets.dark.accent,
        background: custom.background ?? presets.dark.background,
        panel: custom.panel ?? presets.dark.panel,
        panelSoft: custom.panel ?? presets.dark.panelSoft,
        text: custom.text ?? presets.dark.text,
        muted: custom.muted ?? presets.dark.muted,
        border: "rgba(127, 127, 127, 0.22)",
        control: custom.panel ?? presets.dark.control,
      };
      root.dataset.theme = "custom";
    } else {
      colors = presets[resolved as keyof typeof presets] ?? presets.dark;
      root.dataset.theme = resolved;
    }

    root.style.setProperty("--theme-accent", colors.accent);
    root.style.setProperty("--theme-bg", colors.background);
    root.style.setProperty("--theme-panel", colors.panel);
    root.style.setProperty("--theme-panel-soft", colors.panelSoft);
    root.style.setProperty("--theme-text", colors.text);
    root.style.setProperty("--theme-text-strong", colors.text);
    root.style.setProperty("--theme-muted", colors.muted);
    root.style.setProperty("--theme-border", colors.border);
    root.style.setProperty("--theme-control", colors.control);
  }, [settings?.theme, settings?.customTheme, systemTick]);

  return null;
}
